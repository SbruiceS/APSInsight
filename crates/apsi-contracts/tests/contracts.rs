//! Contract tests for the planned ASPF products. None of the products exist; these
//! tests prove that (a) the default adapters degrade correctly, (b) the conformance
//! harness accepts a correct implementation and (c) it rejects incorrect ones.

use apsi_contracts::agricuda::{ComputeJob, ComputePort, ComputeResult, CpuFallback, Profile};
use apsi_contracts::agriql::{AgriQlPort, Column, ColumnType, QueryProvenance, QueryResult, QuerySubmission, query_sha256};
use apsi_contracts::apsnetwork::{AckStatus, CommandAck, EdgeJob, EdgeNetworkPort, TelemetryEnvelope};
use apsi_contracts::common::{QualityFlag, Quantity};
use apsi_contracts::conformance::{self, fixtures};
use apsi_contracts::kutajkutambi::{ExecutionStatus, InferencePort, InferenceRequest, InferenceResponse, ModelMeta, OutputKind, Uncertainty};
use apsi_contracts::nityaga::{CommandSafetyEnvelope, DeviceCapability, DevicePort};
use apsi_contracts::{Adapter, Availability, CapabilityReport, ContractError, ContractVersion, Product, Unavailable, accelerator};
use std::path::Path;

fn available(p: Product) -> CapabilityReport {
    let mut r = Unavailable(p).discover();
    r.availability = Availability::Available { provider_version: "test-mock".into(), offered_contract: ContractVersion::new(1, 0) };
    r
}

/// A test double behaving like a correct provider. `broken` makes it violate the contract.
struct Mock {
    product: Product,
    broken: bool,
}

impl Adapter for Mock {
    fn product(&self) -> Product {
        self.product
    }
    fn discover(&self) -> CapabilityReport {
        available(self.product)
    }
}

impl AgriQlPort for Mock {
    fn submit(&self, q: &QuerySubmission) -> Result<QueryResult, ContractError> {
        q.validate()?;
        Ok(QueryResult {
            columns: vec![
                Column { name: "field_id".into(), ty: ColumnType::Text, unit: None, crs: None },
                Column { name: "yield".into(), ty: ColumnType::Quantity, unit: if self.broken { None } else { Some("t/ha".into()) }, crs: None },
            ],
            rows: vec![vec!["field_synthetic_1".into(), serde_json::json!(3.1)], vec!["field_synthetic_2".into(), serde_json::Value::Null]],
            truncated: false,
            provenance: QueryProvenance {
                query_sha256: query_sha256(&q.query),
                engine_version: "mock-1".into(),
                dataset_versions: [("yield_observations".to_string(), "v7".to_string())].into(),
                executed_at: "2026-10-15T08:00:01Z".into(),
            },
        })
    }
}

impl InferencePort for Mock {
    fn infer(&self, req: &InferenceRequest) -> Result<InferenceResponse, ContractError> {
        req.validate()?;
        Ok(InferenceResponse {
            status: ExecutionStatus::Completed,
            kind: OutputKind::Recommendation,
            output: serde_json::json!({ "interval_days": 4 }),
            model: ModelMeta { name: "mock-model".into(), version: "0.1".into(), evaluation_ref: "model-cards/mock-0.1.md".into() },
            uncertainty: Uncertainty { method: "ensemble spread".into(), confidence: Some(0.7), interval: Some([3.0, 5.0]) },
            evidence_used: req.evidence.clone(),
            requires_human_approval: !self.broken,
        })
    }
}

impl ComputePort for Mock {
    fn execute(&self, job: &ComputeJob, inputs: &[Vec<f64>]) -> Result<ComputeResult, ContractError> {
        let mut r = CpuFallback.execute(job, inputs)?;
        if self.broken {
            r.outputs[0][0] += 0.1;
        }
        r.fallback_used = false;
        r.profile = Profile { backend: "mock-gpu".into(), device: Some("mock".into()), elapsed_ns: 1 };
        Ok(r)
    }
}

impl EdgeNetworkPort for Mock {
    fn deliver(&self, job: &EdgeJob) -> Result<CommandAck, ContractError> {
        job.validate()?;
        Ok(CommandAck {
            command_id: if self.broken { "other".into() } else { job.job_id.clone() },
            status: AckStatus::Queued,
            at: "2026-10-15T08:00:02Z".into(),
            detail: None,
        })
    }
}

impl DevicePort for Mock {
    fn capabilities(&self, device_id: &str) -> Result<DeviceCapability, ContractError> {
        if device_id.is_empty() {
            return Err(ContractError::invalid("device_id", "must not be empty"));
        }
        Ok(fixtures::device())
    }
    fn command(&self, cmd: &CommandSafetyEnvelope) -> Result<CommandAck, ContractError> {
        if !self.broken {
            cmd.validate_for(&fixtures::device())?;
        }
        Ok(CommandAck { command_id: cmd.command_id.clone(), status: AckStatus::Accepted, at: cmd.issued_at.clone(), detail: None })
    }
}

fn mock(product: Product, broken: bool) -> Mock {
    Mock { product, broken }
}

#[test]
fn unavailable_adapters_pass_conformance_and_degrade_gracefully() {
    assert_eq!(conformance::check_agriql(&Unavailable(Product::Agriql)), Vec::<String>::new());
    assert_eq!(conformance::check_inference(&Unavailable(Product::KutajkutambiAi)), Vec::<String>::new());
    assert_eq!(conformance::check_compute(&Unavailable(Product::Agricuda), Product::Agricuda), Vec::<String>::new());
    assert_eq!(conformance::check_compute(&Unavailable(Product::Accelerator), Product::Accelerator), Vec::<String>::new());
    assert_eq!(conformance::check_edge(&Unavailable(Product::Apsnetwork)), Vec::<String>::new());
    assert_eq!(conformance::check_device(&Unavailable(Product::NityagaRtos)), Vec::<String>::new());
    // A valid request reaches the adapter and is refused as unavailable, not as invalid.
    assert!(matches!(Unavailable(Product::Agriql).submit(&fixtures::query()), Err(ContractError::Unavailable { .. })));
    // Device commands are never sent when the device cannot be checked.
    assert!(matches!(Unavailable(Product::NityagaRtos).command(&fixtures::command()), Err(ContractError::Unavailable { .. })));
}

#[test]
fn reference_mocks_pass_conformance() {
    assert_eq!(conformance::check_agriql(&mock(Product::Agriql, false)), Vec::<String>::new());
    assert_eq!(conformance::check_inference(&mock(Product::KutajkutambiAi, false)), Vec::<String>::new());
    assert_eq!(conformance::check_compute(&mock(Product::Agricuda, false), Product::Agricuda), Vec::<String>::new());
    assert_eq!(conformance::check_edge(&mock(Product::Apsnetwork, false)), Vec::<String>::new());
    assert_eq!(conformance::check_device(&mock(Product::NityagaRtos, false)), Vec::<String>::new());
}

#[test]
fn broken_mocks_fail_conformance() {
    let cases = [
        conformance::check_agriql(&mock(Product::Agriql, true)),
        conformance::check_inference(&mock(Product::KutajkutambiAi, true)),
        conformance::check_compute(&mock(Product::Agricuda, true), Product::Agricuda),
        conformance::check_edge(&mock(Product::Apsnetwork, true)),
        conformance::check_device(&mock(Product::NityagaRtos, true)),
    ];
    for (i, findings) in cases.iter().enumerate() {
        assert!(!findings.is_empty(), "broken mock {i} passed conformance");
    }
    // Wrong product identity is caught too.
    assert!(!conformance::check_agriql(&mock(Product::Agricuda, false)).is_empty());
}

#[test]
fn cpu_fallback_is_the_compute_reference() {
    assert!(CpuFallback.discover().is_available());
    assert_eq!(conformance::check_compute(&CpuFallback, Product::Agricuda), Vec::<String>::new());
    let r = CpuFallback.execute(&fixtures::ndvi_job(3), &fixtures::ndvi_inputs()).unwrap();
    assert!(r.fallback_used);
    assert!((r.outputs[0][0] - (0.32 / 0.48)).abs() < 1e-12);
    assert!((r.outputs[0][1] - 0.5).abs() < 1e-12);
    assert!(r.outputs[0][2].is_nan(), "undefined NDVI must be NaN, never a fake 0");
    // Unavailable accelerator falls back to the CPU and says so.
    let fb = accelerator::execute_with_fallback(&Unavailable(Product::Accelerator), &fixtures::ndvi_job(3), &fixtures::ndvi_inputs()).unwrap();
    assert!(fb.fallback_used);
    // Validation errors are not hidden by the fallback.
    assert!(matches!(
        accelerator::execute_with_fallback(&Unavailable(Product::Accelerator), &fixtures::ndvi_job(2), &fixtures::ndvi_inputs()),
        Err(ContractError::Invalid { .. })
    ));
}

#[test]
fn compute_job_rules() {
    let mut j = fixtures::ndvi_job(3);
    assert!(j.validate().is_ok());
    j.buffers[2].unit = "kg".into();
    assert!(j.validate().is_err(), "NDVI output must be dimensionless");
    let mut j = fixtures::ndvi_job(3);
    j.buffers.pop();
    assert!(j.validate().is_err());
    let mut j = fixtures::ndvi_job(3);
    j.operation = "vector.scale".into();
    j.buffers.remove(1);
    assert!(j.validate().is_err(), "scale needs a factor");
    j.scalars.insert("factor".into(), 2.0);
    assert!(j.validate().is_ok());
    j.tolerance.abs = -1.0;
    assert!(j.validate().is_err());
    let mut j = fixtures::ndvi_job(3);
    j.operation = "matrix.invert".into();
    assert!(j.validate().is_err());
}

#[test]
fn inference_contract_rules() {
    let req = fixtures::inference();
    let good = mock(Product::KutajkutambiAi, false).infer(&req).unwrap();
    assert!(good.validate_for(&req).is_ok());
    let mut r = good.clone();
    r.evidence_used[0].version = 99;
    assert!(r.validate_for(&req).is_err(), "evidence not supplied in the request");
    let mut r = good.clone();
    r.uncertainty.confidence = Some(1.5);
    assert!(r.validate_for(&req).is_err());
    let mut r = good.clone();
    r.model.evaluation_ref.clear();
    assert!(r.validate_for(&req).is_err(), "outputs need evaluation evidence");
    let mut r = good;
    r.evidence_used.clear();
    assert!(r.validate_for(&req).is_err());
    // There is no `observation` output kind and no write tool scope on the wire.
    assert!(serde_json::from_str::<OutputKind>("\"observation\"").is_err());
    assert!(serde_json::from_str::<apsi_contracts::kutajkutambi::ToolScope>("\"write\"").is_err());
}

#[test]
fn query_contract_rules() {
    let q = fixtures::query();
    assert!(q.validate().is_ok());
    let mut bad = q.clone();
    bad.spatial.as_mut().unwrap().crs = "WGS84".into();
    assert!(bad.validate().is_err());
    let mut bad = q.clone();
    bad.spatial.as_mut().unwrap().bbox = [78.0, 28.0, 77.0, 29.0];
    assert!(bad.validate().is_err());
    let res = mock(Product::Agriql, false).submit(&q).unwrap();
    assert!(res.validate_for(&q, &query_sha256(&q.query)).is_ok());
    assert!(res.validate_for(&q, &query_sha256("another query")).is_err(), "result tied to the wrong query");
    let mut r = res.clone();
    r.columns.push(Column { name: "geom".into(), ty: ColumnType::Geometry, unit: None, crs: None });
    assert!(r.validate_for(&q, &query_sha256(&q.query)).is_err(), "geometry without CRS");
}

#[test]
fn telemetry_contract_rules() {
    let mut measurements = std::collections::BTreeMap::new();
    measurements.insert("soil_moisture".to_string(), Quantity { value: Some(0.21), unit: "m3/m3".into(), quality: QualityFlag::Good, uncertainty: Some(0.01) });
    measurements.insert("leaf_wetness".to_string(), Quantity { value: None, unit: "1".into(), quality: QualityFlag::Missing, uncertainty: None });
    let t = TelemetryEnvelope {
        contract: ContractVersion::new(1, 0),
        device_id: "dev_synthetic_1".into(),
        sequence: 42,
        event_time: "2026-10-15T07:50:00Z".into(),
        received_time: "2026-10-15T08:00:00Z".into(),
        measurements,
        link: None,
    };
    assert!(t.validate(ContractVersion::new(1, 0)).is_ok());
    assert_eq!(t.lateness_s().unwrap(), 600);
    assert!(matches!(t.validate(ContractVersion::new(2, 0)), Err(ContractError::Incompatible { .. })));
    let mut skewed = t.clone();
    skewed.event_time = "2026-10-15T09:00:00Z".into();
    assert!(skewed.validate(ContractVersion::new(1, 0)).is_err());
    let mut unitless = t;
    unitless.measurements.get_mut("soil_moisture").unwrap().unit.clear();
    assert!(unitless.validate(ContractVersion::new(1, 0)).is_err());
}

#[test]
fn device_command_safety_envelope_rules() {
    let d = fixtures::device();
    assert!(d.validate().is_ok());
    let ok = fixtures::command();
    assert!(ok.validate_for(&d).is_ok());
    let cases: [fn(&mut CommandSafetyEnvelope); 6] = [
        |c| c.actuator = "pump_9".into(),
        |c| c.requires_ack = false,
        |c| c.interlocks.clear(),
        |c| c.max_duration_ms = 0,
        |c| c.expires_at = "2026-10-15T07:59:00Z".into(),
        |c| c.expires_at = "2026-10-15T09:00:00Z".into(),
    ];
    for (i, mutate) in cases.iter().enumerate() {
        let mut c = ok.clone();
        mutate(&mut c);
        assert!(c.validate_for(&d).is_err(), "unsafe command {i} accepted");
    }
}

#[test]
fn accelerator_manifest_rules() {
    let m = accelerator::AcceleratorManifest {
        device_family: "synthetic-fpga".into(),
        bitstream_sha256: fixtures::HASH.into(),
        driver_contract: ContractVersion::new(1, 0),
        operations: vec!["index.ndvi".into()],
        safety_constraints: vec!["max_junction_temp_c=95".into()],
        test_vectors: vec!["ndvi-v1.json".into()],
    };
    let host = ContractVersion::new(1, 0);
    assert!(m.validate(host).is_ok());
    assert!(accelerator::AcceleratorManifest { operations: vec!["fft".into()], ..m.clone() }.validate(host).is_err());
    assert!(accelerator::AcceleratorManifest { test_vectors: vec![], ..m.clone() }.validate(host).is_err());
    assert!(accelerator::AcceleratorManifest { driver_contract: ContractVersion::new(2, 0), ..m }.validate(host).is_err());
}

/// The JSON wire examples published under `contracts/v1/` must match the fixtures,
/// so a change to a contract type is visible as a change to its published example.
/// Regenerate with `APSI_BLESS=1 cargo test -p apsi-contracts`.
#[test]
fn published_wire_examples_match_contract_types() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("contracts/v1");
    let examples: Vec<(&str, serde_json::Value)> = vec![
        ("agriql.query-submission.json", serde_json::to_value(fixtures::query()).unwrap()),
        ("kutajkutambi.inference-request.json", serde_json::to_value(fixtures::inference()).unwrap()),
        ("agricuda.compute-job.json", serde_json::to_value(fixtures::ndvi_job(3)).unwrap()),
        ("apsnetwork.edge-job.json", serde_json::to_value(fixtures::edge_job()).unwrap()),
        ("nityaga.device-capability.json", serde_json::to_value(fixtures::device()).unwrap()),
        ("nityaga.command-envelope.json", serde_json::to_value(fixtures::command()).unwrap()),
        ("capabilities.json", serde_json::to_value(apsi_contracts::discover_all()).unwrap()),
    ];
    if std::env::var_os("APSI_BLESS").is_some() {
        std::fs::create_dir_all(&dir).unwrap();
        for (name, v) in &examples {
            std::fs::write(dir.join(name), serde_json::to_string_pretty(v).unwrap() + "\n").unwrap();
        }
    }
    for (name, v) in &examples {
        let text = std::fs::read_to_string(dir.join(name)).unwrap_or_else(|_| panic!("missing contracts/v1/{name}; run with APSI_BLESS=1"));
        let published: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(&published, v, "contracts/v1/{name} is stale");
    }
    // Published examples also deserialize into the contract types and validate.
    let q: QuerySubmission = serde_json::from_str(&std::fs::read_to_string(dir.join("agriql.query-submission.json")).unwrap()).unwrap();
    assert!(q.validate().is_ok());
    let c: CommandSafetyEnvelope = serde_json::from_str(&std::fs::read_to_string(dir.join("nityaga.command-envelope.json")).unwrap()).unwrap();
    assert!(c.validate_for(&fixtures::device()).is_ok());
}
