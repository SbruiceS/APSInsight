//! Conformance harness. A future adapter for any ASPF product is accepted only if
//! the matching `check_*` function returns no findings. The same functions are run
//! in this crate's tests against the built-in adapters, against reference mocks and
//! against deliberately broken mocks (which must fail).
//!
//! Rules applied to every port:
//! 1. `discover()` names the right product and a compatible contract version.
//! 2. Malformed requests are rejected with `ContractError::Invalid` before any
//!    provider work happens.
//! 3. A canonical valid request either succeeds with a response that passes the
//!    contract's response checks, or fails with `Unavailable` **only** when the
//!    adapter reports itself unavailable.

use crate::agricuda::{BufferDescriptor, ComputeJob, ComputePort, CpuFallback, DType, Role, Tolerance};
use crate::agriql::{AgriQlPort, Parameter, QuerySubmission, SpatialContext};
use crate::apsnetwork::{EdgeJob, EdgeNetworkPort};
use crate::common::{Adapter, ContractError, Product};
use crate::kutajkutambi::{EvidenceRef, InferencePort, InferenceRequest, TaskKind, ToolPermission, ToolScope};
use crate::nityaga::{ActuatorCapability, CommandSafetyEnvelope, DeviceCapability, DevicePort, FirmwareIdentity, SensorCapability, TimeSync};
use std::collections::BTreeMap;

/// The canonical fixtures are synthetic. They describe no real farm, device or person.
pub mod fixtures {
    use super::*;

    pub const HASH: &str = "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08";

    pub fn query() -> QuerySubmission {
        let mut parameters = BTreeMap::new();
        parameters.insert("min_yield".into(), Parameter::Number { value: 2.5, unit: "t/ha".into() });
        parameters.insert("season_start".into(), Parameter::Timestamp { value: "2026-06-01T00:00:00Z".into() });
        QuerySubmission {
            tenant: "synthetic-tenant".into(),
            query: "FROM yield_observations WHERE yield >= :min_yield AND observed_at >= :season_start SELECT field_id, yield".into(),
            language_version: "1.0".into(),
            parameters,
            spatial: Some(SpatialContext { crs: "EPSG:4326".into(), bbox: [77.0, 28.0, 78.0, 29.0] }),
            max_rows: 1000,
            timeout_ms: 30_000,
        }
    }

    pub fn inference() -> InferenceRequest {
        InferenceRequest {
            tenant: "synthetic-tenant".into(),
            task: TaskKind::Recommendation,
            instruction: "Suggest an irrigation interval for the referenced synthetic field.".into(),
            evidence: vec![EvidenceRef { record_id: "rec_synthetic_1".into(), version: 3, content_sha256: HASH.into() }],
            tools: vec![ToolPermission { tool: "records.read".into(), scope: ToolScope::Read }],
            deadline_ms: 10_000,
        }
    }

    pub fn ndvi_job(len: u64) -> ComputeJob {
        let b = |name: &str, role, unit: &str| BufferDescriptor { name: name.into(), role, dtype: DType::F64, len, alignment: 8, unit: unit.into() };
        ComputeJob {
            operation: "index.ndvi".into(),
            buffers: vec![b("red", Role::Input, "1"), b("nir", Role::Input, "1"), b("ndvi", Role::Output, "1")],
            scalars: BTreeMap::new(),
            tolerance: Tolerance { abs: 1e-6, rel: 1e-6 },
        }
    }

    pub fn ndvi_inputs() -> Vec<Vec<f64>> {
        vec![vec![0.08, 0.10, 0.0], vec![0.40, 0.30, 0.0]]
    }

    pub fn edge_job() -> EdgeJob {
        EdgeJob {
            job_id: "job_synthetic_1".into(),
            target_device: "dev_synthetic_1".into(),
            kind: "sample.sensors".into(),
            payload: serde_json::json!({ "interval_s": 600 }),
            expires_at: "2026-10-15T09:00:00Z".into(),
        }
    }

    pub fn device() -> DeviceCapability {
        DeviceCapability {
            device_id: "dev_synthetic_1".into(),
            firmware: FirmwareIdentity { name: "synthetic-fw".into(), version: "1.0.0".into(), sha256: HASH.into() },
            sensors: vec![SensorCapability { name: "soil_moisture".into(), unit: "m3/m3".into(), min: 0.0, max: 0.6, resolution: 0.001 }],
            actuators: vec![ActuatorCapability { name: "valve_1".into(), safe_state: "closed".into() }],
            time_sync: TimeSync { source: "gnss".into(), max_drift_ms: 50 },
        }
    }

    pub fn command() -> CommandSafetyEnvelope {
        CommandSafetyEnvelope {
            command_id: "cmd_synthetic_1".into(),
            device_id: "dev_synthetic_1".into(),
            actuator: "valve_1".into(),
            action: "open".into(),
            max_duration_ms: 600_000,
            issued_at: "2026-10-15T08:00:00Z".into(),
            expires_at: "2026-10-15T08:05:00Z".into(),
            requires_ack: true,
            interlocks: vec!["pressure_ok".into()],
            issued_by: "user_synthetic".into(),
        }
    }
}

fn common(a: &dyn Adapter, product: Product, findings: &mut Vec<String>) -> bool {
    let r = a.discover();
    if r.product != product || a.product() != product {
        findings.push(format!("discover() reports {:?}, expected {product:?}", r.product));
    }
    if let Err(e) = r.check_compatible() {
        findings.push(format!("incompatible contract: {e}"));
    }
    if !r.optional {
        findings.push("integration must be optional".into());
    }
    r.is_available()
}

fn expect_invalid<T>(what: &str, r: Result<T, ContractError>, findings: &mut Vec<String>) {
    match r {
        Err(ContractError::Invalid { .. }) | Err(ContractError::Incompatible { .. }) => {}
        Err(e) => findings.push(format!("{what}: expected an Invalid error, got {e}")),
        Ok(_) => findings.push(format!("{what}: malformed request was accepted")),
    }
}

fn outcome<T>(what: &str, available: bool, r: Result<T, ContractError>, findings: &mut Vec<String>) -> Option<T> {
    match r {
        Ok(v) if available => Some(v),
        Ok(_) => {
            findings.push(format!("{what}: adapter reports unavailable but returned a result"));
            None
        }
        Err(ContractError::Unavailable { .. }) if !available => None,
        Err(e) => {
            findings.push(format!("{what}: canonical request failed: {e}"));
            None
        }
    }
}

pub fn check_agriql(p: &dyn AgriQlPort) -> Vec<String> {
    let mut f = Vec::new();
    let available = common(p, Product::Agriql, &mut f);
    let mut bad = fixtures::query();
    bad.max_rows = 0;
    expect_invalid("max_rows = 0", p.submit(&bad), &mut f);
    let mut bad = fixtures::query();
    bad.parameters.insert("x".into(), Parameter::Number { value: 1.0, unit: String::new() });
    expect_invalid("unitless number parameter", p.submit(&bad), &mut f);
    let q = fixtures::query();
    if let Some(res) = outcome("query.submit", available, p.submit(&q), &mut f)
        && let Err(e) = res.validate_for(&q, &crate::agriql::query_sha256(&q.query))
    {
        f.push(format!("query result violates contract: {e}"));
    }
    f
}

pub fn check_inference(p: &dyn InferencePort) -> Vec<String> {
    let mut f = Vec::new();
    let available = common(p, Product::KutajkutambiAi, &mut f);
    let mut bad = fixtures::inference();
    bad.evidence.clear();
    expect_invalid("no evidence", p.infer(&bad), &mut f);
    let req = fixtures::inference();
    if let Some(res) = outcome("inference.run", available, p.infer(&req), &mut f)
        && let Err(e) = res.validate_for(&req)
    {
        f.push(format!("inference response violates contract: {e}"));
    }
    f
}

pub fn check_compute(p: &dyn ComputePort, product: Product) -> Vec<String> {
    let mut f = Vec::new();
    let available = common(p, product, &mut f);
    let mut bad = fixtures::ndvi_job(3);
    bad.buffers[0].alignment = 3;
    expect_invalid("alignment 3", p.execute(&bad, &fixtures::ndvi_inputs()), &mut f);
    expect_invalid("input length mismatch", p.execute(&fixtures::ndvi_job(2), &fixtures::ndvi_inputs()), &mut f);
    let job = fixtures::ndvi_job(3);
    let inputs = fixtures::ndvi_inputs();
    if let Some(res) = outcome("compute.execute", available, p.execute(&job, &inputs), &mut f) {
        match CpuFallback.execute(&job, &inputs) {
            Ok(reference) => {
                let ok = res.outputs.len() == reference.outputs.len()
                    && res
                        .outputs
                        .iter()
                        .zip(&reference.outputs)
                        .all(|(a, b)| a.len() == b.len() && a.iter().zip(b).all(|(x, y)| job.tolerance.within(*x, *y)));
                if !ok {
                    f.push("compute outputs differ from the CPU reference beyond the job tolerance".into());
                }
            }
            Err(e) => f.push(format!("reference failed: {e}")),
        }
    }
    f
}

pub fn check_edge(p: &dyn EdgeNetworkPort) -> Vec<String> {
    let mut f = Vec::new();
    let available = common(p, Product::Apsnetwork, &mut f);
    let mut bad = fixtures::edge_job();
    bad.expires_at = "tomorrow".into();
    expect_invalid("bad expiry", p.deliver(&bad), &mut f);
    if let Some(ack) = outcome("edge-job.deliver", available, p.deliver(&fixtures::edge_job()), &mut f) {
        if ack.command_id != fixtures::edge_job().job_id {
            f.push("ack does not reference the delivered job".into());
        }
        if crate::common::parse_time("at", &ack.at).is_err() {
            f.push("ack timestamp is not RFC 3339".into());
        }
    }
    f
}

pub fn check_device(p: &dyn DevicePort) -> Vec<String> {
    let mut f = Vec::new();
    let available = common(p, Product::NityagaRtos, &mut f);
    expect_invalid("empty device id", p.capabilities(""), &mut f);
    if let Some(cap) = outcome("device.capabilities", available, p.capabilities("dev_synthetic_1"), &mut f) {
        if let Err(e) = cap.validate() {
            f.push(format!("capabilities violate contract: {e}"));
        }
        let mut unsafe_cmd = fixtures::command();
        unsafe_cmd.interlocks.clear();
        expect_invalid("command without interlocks", p.command(&unsafe_cmd), &mut f);
    }
    f
}
