//! AgriCUDA contract v1.0: accelerated compute jobs over typed buffers.
//!
//! Buffers are described by name, element type, length, alignment and unit; a
//! result must match the CPU reference within the job's [`Tolerance`]. Because
//! AgriCUDA does not exist yet, [`CpuFallback`] is the only executor: it is the
//! reference implementation every accelerator is compared against.

use crate::common::{Adapter, Availability, CapabilityReport, ContractError, Product, Result, Unavailable, check_unit, finite, non_empty};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::Instant;

pub const MAX_ELEMENTS: u64 = 1 << 28;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DType {
    F32,
    F64,
}

impl DType {
    pub fn size(self) -> u32 {
        match self {
            Self::F32 => 4,
            Self::F64 => 8,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Input,
    Output,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BufferDescriptor {
    pub name: String,
    pub role: Role,
    pub dtype: DType,
    pub len: u64,
    /// Byte alignment; a power of two and at least the element size.
    pub alignment: u32,
    pub unit: String,
}

/// `|a - b| <= abs + rel * |b|` element-wise, where `b` is the CPU reference.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Tolerance {
    pub abs: f64,
    pub rel: f64,
}

impl Tolerance {
    pub fn within(&self, actual: f64, reference: f64) -> bool {
        if actual.is_nan() || reference.is_nan() {
            return actual.is_nan() && reference.is_nan();
        }
        (actual - reference).abs() <= self.abs + self.rel * reference.abs()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComputeJob {
    /// One of [`OPERATIONS`].
    pub operation: String,
    pub buffers: Vec<BufferDescriptor>,
    #[serde(default)]
    pub scalars: BTreeMap<String, f64>,
    pub tolerance: Tolerance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    pub backend: String,
    pub device: Option<String>,
    pub elapsed_ns: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComputeResult {
    pub outputs: Vec<Vec<f64>>,
    pub fallback_used: bool,
    pub profile: Profile,
}

/// Operations in contract v1.0, with (inputs, outputs, scalars).
pub const OPERATIONS: [(&str, usize, usize, &[&str]); 3] = [("vector.add", 2, 1, &[]), ("vector.scale", 1, 1, &["factor"]), ("index.ndvi", 2, 1, &[])];

impl ComputeJob {
    fn spec(&self) -> Result<(usize, usize, &'static [&'static str])> {
        OPERATIONS
            .iter()
            .find(|(n, ..)| *n == self.operation)
            .map(|(_, i, o, s)| (*i, *o, *s))
            .ok_or_else(|| ContractError::invalid("operation", format!("`{}` is not a v1.0 operation", self.operation)))
    }

    pub fn validate(&self) -> Result<()> {
        let (ins, outs, scalars) = self.spec()?;
        for (i, b) in self.buffers.iter().enumerate() {
            let f = format!("buffers[{i}]");
            non_empty(&format!("{f}.name"), &b.name, 64)?;
            check_unit(&format!("{f}.unit"), &b.unit)?;
            if b.len == 0 || b.len > MAX_ELEMENTS {
                return Err(ContractError::invalid(format!("{f}.len"), format!("must be within 1..={MAX_ELEMENTS}")));
            }
            if !b.alignment.is_power_of_two() || b.alignment < b.dtype.size() {
                return Err(ContractError::invalid(format!("{f}.alignment"), "must be a power of two >= element size"));
            }
        }
        let count = |r| self.buffers.iter().filter(|b| b.role == r).count();
        if count(Role::Input) != ins || count(Role::Output) != outs {
            return Err(ContractError::invalid("buffers", format!("{} needs {ins} input(s) and {outs} output(s)", self.operation)));
        }
        let len = self.buffers[0].len;
        if self.buffers.iter().any(|b| b.len != len) {
            return Err(ContractError::invalid("buffers", "all buffers must have the same length for element-wise operations"));
        }
        for s in scalars {
            let v = self.scalars.get(*s).ok_or_else(|| ContractError::invalid(format!("scalars.{s}"), "required"))?;
            finite(&format!("scalars.{s}"), *v)?;
        }
        finite("tolerance.abs", self.tolerance.abs)?;
        finite("tolerance.rel", self.tolerance.rel)?;
        if self.tolerance.abs < 0.0 || self.tolerance.rel < 0.0 {
            return Err(ContractError::invalid("tolerance", "must be non-negative"));
        }
        if self.operation == "index.ndvi" && self.buffers.iter().any(|b| b.role == Role::Output && b.unit != "1") {
            return Err(ContractError::invalid("buffers", "NDVI output is dimensionless (unit `1`)"));
        }
        Ok(())
    }

    fn check_inputs(&self, inputs: &[Vec<f64>]) -> Result<()> {
        self.validate()?;
        let descs: Vec<_> = self.buffers.iter().filter(|b| b.role == Role::Input).collect();
        if inputs.len() != descs.len() {
            return Err(ContractError::invalid("inputs", format!("expected {} input buffers", descs.len())));
        }
        for (i, (d, data)) in descs.iter().zip(inputs).enumerate() {
            if data.len() as u64 != d.len {
                return Err(ContractError::invalid(format!("inputs[{i}]"), format!("length {} does not match descriptor {}", data.len(), d.len)));
            }
        }
        Ok(())
    }
}

pub trait ComputePort: Adapter {
    fn execute(&self, job: &ComputeJob, inputs: &[Vec<f64>]) -> Result<ComputeResult>;
}

impl ComputePort for Unavailable {
    fn execute(&self, job: &ComputeJob, inputs: &[Vec<f64>]) -> Result<ComputeResult> {
        job.check_inputs(inputs)?;
        Err(self.error())
    }
}

/// The CPU reference executor. Deterministic, single-threaded, f64 throughout.
#[derive(Debug, Clone, Copy, Default)]
pub struct CpuFallback;

impl Adapter for CpuFallback {
    fn product(&self) -> Product {
        Product::Agricuda
    }

    fn discover(&self) -> CapabilityReport {
        CapabilityReport {
            product: Product::Agricuda,
            product_name: "AgriCUDA CPU fallback (built in)".into(),
            contract_version: Product::Agricuda.contract_version(),
            availability: Availability::Available {
                provider_version: env!("CARGO_PKG_VERSION").into(),
                offered_contract: Product::Agricuda.contract_version(),
            },
            operations: OPERATIONS.iter().map(|(n, ..)| n.to_string()).collect(),
            optional: true,
        }
    }
}

impl ComputePort for CpuFallback {
    fn execute(&self, job: &ComputeJob, inputs: &[Vec<f64>]) -> Result<ComputeResult> {
        job.check_inputs(inputs)?;
        let start = Instant::now();
        let out: Vec<f64> = match job.operation.as_str() {
            "vector.add" => inputs[0].iter().zip(&inputs[1]).map(|(a, b)| a + b).collect(),
            "vector.scale" => {
                let k = job.scalars["factor"];
                inputs[0].iter().map(|a| a * k).collect()
            }
            // inputs: red, nir. NaN where the index is undefined (red + nir == 0)
            // so the caller can flag missing values instead of receiving a fake 0.
            "index.ndvi" => inputs[0].iter().zip(&inputs[1]).map(|(r, n)| if r + n == 0.0 { f64::NAN } else { (n - r) / (n + r) }).collect(),
            other => return Err(ContractError::invalid("operation", format!("`{other}` is not supported"))),
        };
        Ok(ComputeResult {
            outputs: vec![out],
            fallback_used: true,
            profile: Profile { backend: "cpu-fallback".into(), device: None, elapsed_ns: start.elapsed().as_nanos().min(u64::MAX as u128) as u64 },
        })
    }
}
