//! Execution backends for numerical kernels.
//!
//! * `cpu`: pure Rust, data-parallel over fixed-size chunks with Rayon.
//! * `native`: the C++17 kernels in `apsi-native` (when compiled in).
//! * `accelerator`: any [`ComputePort`] from the AgriCUDA contract. The default
//!   port is `Unavailable`, because no AgriCUDA product exists yet; a job sent
//!   there falls back to the CPU and the profile says so.
//!
//! Every backend must agree with the CPU reference within the contract
//! tolerance; the tests check native against reference bit for bit.

use crate::{EngineError, Result};
use apsi_contracts::agricuda::{BufferDescriptor, ComputeJob, ComputePort, DType, Role, Tolerance};
use apsi_contracts::{Adapter, Availability, CapabilityReport, Product, Unavailable};
use rayon::prelude::*;
use serde::Serialize;
use std::sync::Arc;

/// Rows per parallel chunk. Fixed, so results never depend on thread count.
pub const CHUNK: usize = 16_384;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendKind {
    Cpu,
    Native,
    Accelerator,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BackendInfo {
    pub kind: BackendKind,
    pub available: bool,
    pub detail: String,
    pub kernels: Vec<String>,
}

#[derive(Clone)]
pub struct Backends {
    pub threads: usize,
    accelerator: Arc<dyn ComputePort>,
}

impl Default for Backends {
    fn default() -> Self {
        Self::new(Arc::new(Unavailable(Product::Agricuda)))
    }
}

impl Backends {
    pub fn new(accelerator: Arc<dyn ComputePort>) -> Self {
        Self { threads: rayon::current_num_threads(), accelerator }
    }

    pub fn native_available(&self) -> bool {
        apsi_native::available()
    }

    pub fn accelerator_report(&self) -> CapabilityReport {
        self.accelerator.discover()
    }

    pub fn accelerator_available(&self) -> bool {
        matches!(self.accelerator.discover().availability, Availability::Available { .. })
    }

    pub fn describe(&self) -> Vec<BackendInfo> {
        let acc = self.accelerator_report();
        vec![
            BackendInfo {
                kind: BackendKind::Cpu,
                available: true,
                detail: format!("Rust, Rayon data parallelism over {} threads, {CHUNK}-row chunks", self.threads),
                kernels: ["ndvi", "moving_mean", "fft_power", "filter", "project", "aggregate", "join", "sort"].map(String::from).to_vec(),
            },
            BackendInfo {
                kind: BackendKind::Native,
                available: self.native_available(),
                detail: format!("C++17 kernels, ABI v{} ({})", apsi_native::ABI_VERSION, apsi_native::build_info()),
                kernels: apsi_native::KERNELS.map(String::from).to_vec(),
            },
            BackendInfo {
                kind: BackendKind::Accelerator,
                available: self.accelerator_available(),
                detail: match acc.availability {
                    Availability::Available { provider_version, .. } => format!("{} {provider_version}", acc.product_name),
                    Availability::Unavailable { reason } => format!("{}: {reason}; CPU fallback is used", acc.product_name),
                },
                kernels: acc.operations,
            },
        ]
    }

    /// NDVI on the chosen backend. Returns the values and the backend that
    /// actually ran, plus a fallback note when it differs from the request.
    pub fn ndvi(&self, want: BackendKind, red: &[f64], nir: &[f64], nodata: Option<f64>) -> Result<(Vec<f64>, BackendKind, Option<String>)> {
        match want {
            BackendKind::Accelerator => match self.ndvi_accelerator(red, nir, nodata) {
                Ok(v) => Ok((v, BackendKind::Accelerator, None)),
                Err(e) => {
                    let (v, used, _) = self.ndvi(if self.native_available() { BackendKind::Native } else { BackendKind::Cpu }, red, nir, nodata)?;
                    Ok((v, used, Some(format!("accelerator unavailable ({e}); fell back to {used:?}").to_lowercase())))
                }
            },
            BackendKind::Native if self.native_available() => Ok((apsi_native::ndvi(red, nir, nodata).map_err(kernel)?, BackendKind::Native, None)),
            BackendKind::Native => {
                let (v, ..) = self.ndvi(BackendKind::Cpu, red, nir, nodata)?;
                Ok((v, BackendKind::Cpu, Some(format!("native kernels not built ({}); used cpu", apsi_native::build_info()))))
            }
            BackendKind::Cpu => {
                let mut out = vec![0.0; red.len()];
                out.par_chunks_mut(CHUNK).enumerate().for_each(|(c, chunk)| {
                    let s = c * CHUNK;
                    let r = apsi_native::reference::ndvi(&red[s..s + chunk.len()], &nir[s..s + chunk.len()], nodata);
                    chunk.copy_from_slice(&r);
                });
                Ok((out, BackendKind::Cpu, None))
            }
        }
    }

    fn ndvi_accelerator(&self, red: &[f64], nir: &[f64], nodata: Option<f64>) -> Result<Vec<f64>> {
        // The contract operation has no nodata flag: mask first, so flagged
        // pixels reach the device as NaN and come back undefined.
        let mask = |v: &[f64]| -> Vec<f64> { v.iter().map(|x| if nodata == Some(*x) { f64::NAN } else { *x }).collect() };
        let len = red.len() as u64;
        let buf = |name: &str, role| BufferDescriptor { name: name.into(), role, dtype: DType::F64, len, alignment: 8, unit: "1".into() };
        let job = ComputeJob {
            operation: "index.ndvi".into(),
            buffers: vec![buf("red", Role::Input), buf("nir", Role::Input), buf("ndvi", Role::Output)],
            scalars: Default::default(),
            tolerance: Tolerance { abs: 1e-9, rel: 1e-9 },
        };
        let r = self.accelerator.execute(&job, &[mask(red), mask(nir)]).map_err(|e| EngineError::Kernel(e.to_string()))?;
        r.outputs.into_iter().next().ok_or_else(|| EngineError::Kernel("accelerator returned no output".into()))
    }

    pub fn moving_mean(&self, want: BackendKind, x: &[f64], window: usize) -> Result<(Vec<f64>, BackendKind)> {
        if want != BackendKind::Cpu && self.native_available() {
            return Ok((apsi_native::moving_mean(x, window).map_err(kernel)?, BackendKind::Native));
        }
        Ok((apsi_native::reference::moving_mean(x, window), BackendKind::Cpu))
    }

    pub fn fft_power(&self, want: BackendKind, x: &[f64]) -> Result<(Vec<f64>, BackendKind)> {
        if x.len() < 2 || !x.len().is_power_of_two() {
            return Err(EngineError::Kernel(format!("fft_power needs a power-of-two row count >= 2, got {}", x.len())));
        }
        if want != BackendKind::Cpu && self.native_available() {
            return Ok((apsi_native::fft_power(x).map_err(kernel)?, BackendKind::Native));
        }
        Ok((apsi_native::reference::fft_power(x), BackendKind::Cpu))
    }
}

fn kernel(e: apsi_native::KernelError) -> EngineError {
    EngineError::Kernel(e.to_string())
}

/// Deterministic sum used by aggregates: native when available, else the
/// identical Rust algorithm.
pub fn sum(values: &[f64]) -> f64 {
    apsi_native::sum(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backends_describe_themselves_honestly() {
        let b = Backends::default();
        let d = b.describe();
        assert_eq!(d.len(), 3);
        assert!(d[0].available);
        assert_eq!(d[1].available, apsi_native::available());
        assert!(!d[2].available, "no AgriCUDA product exists, so the accelerator must not claim availability");
        assert!(d[2].detail.contains("CPU fallback"));
    }

    #[test]
    fn every_backend_computes_the_same_ndvi() {
        let red: Vec<f64> = (0..40_000).map(|i| (i % 97) as f64 / 200.0).collect();
        let nir: Vec<f64> = (0..40_000).map(|i| (i % 89) as f64 / 150.0).collect();
        let b = Backends::default();
        let (cpu, k, _) = b.ndvi(BackendKind::Cpu, &red, &nir, Some(0.0)).unwrap();
        assert_eq!(k, BackendKind::Cpu);
        let reference = apsi_native::reference::ndvi(&red, &nir, Some(0.0));
        let same = |a: &[f64], b: &[f64]| a.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits() || (x.is_nan() && y.is_nan()));
        assert!(same(&cpu, &reference));
        let (native, _, _) = b.ndvi(BackendKind::Native, &red, &nir, Some(0.0)).unwrap();
        assert!(same(&native, &reference));
        // The accelerator is unavailable: the job falls back and says why.
        let (acc, used, note) = b.ndvi(BackendKind::Accelerator, &red, &nir, Some(0.0)).unwrap();
        assert_ne!(used, BackendKind::Accelerator);
        assert!(note.unwrap().contains("fell back"));
        assert!(same(&acc, &reference));
    }

    #[test]
    fn a_working_accelerator_port_is_used() {
        let b = Backends::new(Arc::new(apsi_contracts::agricuda::CpuFallback));
        assert!(b.accelerator_available());
        let (v, used, note) = b.ndvi(BackendKind::Accelerator, &[0.1, 0.0, 9.0], &[0.5, 0.0, 9.0], Some(9.0)).unwrap();
        assert_eq!(used, BackendKind::Accelerator);
        assert!(note.is_none());
        assert!((v[0] - 0.4 / 0.6).abs() < 1e-12 && v[1].is_nan() && v[2].is_nan());
    }
}
