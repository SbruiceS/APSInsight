//! Optional hardware-accelerator contract v1.0 (for example GreenPlus FPGA).
//!
//! An accelerator is loaded from a manifest that pins the bitstream by hash and
//! declares which AgriCUDA-contract operations it implements. Execution reuses
//! [`crate::agricuda::ComputeJob`], so every accelerator is checked against the
//! same CPU reference.

use crate::agricuda::{ComputeJob, ComputePort, ComputeResult, OPERATIONS};
use crate::common::{ContractError, ContractVersion, Result, Unavailable, check_sha256, non_empty};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AcceleratorManifest {
    pub device_family: String,
    pub bitstream_sha256: String,
    pub driver_contract: ContractVersion,
    pub operations: Vec<String>,
    /// Thermal, power or interlock constraints the host must enforce.
    pub safety_constraints: Vec<String>,
    /// Test vectors shipped with the bitstream (file names or ids).
    pub test_vectors: Vec<String>,
}

impl AcceleratorManifest {
    pub fn validate(&self, host_contract: ContractVersion) -> Result<()> {
        non_empty("device_family", &self.device_family, 64)?;
        check_sha256("bitstream_sha256", &self.bitstream_sha256)?;
        if !self.driver_contract.satisfies(host_contract) {
            return Err(ContractError::Incompatible { required: host_contract, offered: self.driver_contract });
        }
        if self.operations.is_empty() {
            return Err(ContractError::invalid("operations", "must declare at least one operation"));
        }
        if let Some(op) = self.operations.iter().find(|o| !OPERATIONS.iter().any(|(n, ..)| n == o)) {
            return Err(ContractError::invalid("operations", format!("`{op}` is not an AgriCUDA v1.0 operation")));
        }
        if self.test_vectors.is_empty() {
            return Err(ContractError::invalid("test_vectors", "an accelerator without test vectors cannot be verified"));
        }
        Ok(())
    }
}

pub trait AcceleratorPort: ComputePort {
    fn load(&self, manifest: &AcceleratorManifest) -> Result<()>;
}

impl AcceleratorPort for Unavailable {
    fn load(&self, manifest: &AcceleratorManifest) -> Result<()> {
        manifest.validate(crate::Product::Accelerator.contract_version())?;
        Err(self.error())
    }
}

/// Runs `job` on the accelerator if it accepts it, otherwise on the CPU fallback.
/// The returned result says which one ran (`fallback_used`).
pub fn execute_with_fallback(accel: &dyn ComputePort, job: &ComputeJob, inputs: &[Vec<f64>]) -> Result<ComputeResult> {
    match accel.execute(job, inputs) {
        Err(ContractError::Unavailable { .. }) => crate::agricuda::CpuFallback.execute(job, inputs),
        other => other,
    }
}
