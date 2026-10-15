//! Native scientific kernels for the APSI Compute Engine.
//!
//! The kernels are written in C++17 (`cpp/`) and exposed through a narrow C
//! ABI, version [`ABI_VERSION`]. This crate is the only place in the workspace
//! that uses `unsafe`, and only in [`ffi`]: every public function checks
//! lengths and arguments before crossing the boundary.
//!
//! [`reference`] holds a pure-Rust implementation of every kernel using the
//! same algorithms. It is the fallback when the crate was built without a C++
//! compiler, and the oracle the tests compare the C++ code against.

pub mod reference;

use std::fmt;

/// ABI version this crate expects from the C++ library.
pub const ABI_VERSION: i32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelError {
    InvalidArgument(String),
    /// The C++ side reported an internal failure (e.g. allocation).
    Internal(&'static str),
}

impl fmt::Display for KernelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidArgument(m) => write!(f, "invalid kernel argument: {m}"),
            Self::Internal(k) => write!(f, "native kernel {k} failed internally"),
        }
    }
}

impl std::error::Error for KernelError {}

pub type Result<T> = std::result::Result<T, KernelError>;

/// Which implementation a call used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Implementation {
    /// The C++ library.
    Native,
    /// The pure-Rust reference kernels.
    Reference,
}

/// True when the C++ kernels were compiled into this build and report the
/// expected ABI version.
pub fn available() -> bool {
    native_abi() == Some(ABI_VERSION)
}

/// How this build obtained its kernels, for capability reports.
pub fn build_info() -> &'static str {
    env!("APSI_NATIVE_REASON")
}

/// ABI version reported by the linked C++ library, if any.
pub fn native_abi() -> Option<i32> {
    #[cfg(apsi_native)]
    {
        Some(ffi::abi_version())
    }
    #[cfg(not(apsi_native))]
    {
        None
    }
}

/// The implementation used by the dispatching functions below.
pub fn implementation() -> Implementation {
    if available() { Implementation::Native } else { Implementation::Reference }
}

/// Kernel names, for capability discovery.
pub const KERNELS: [&str; 6] = ["sum", "dot", "gemm", "ndvi", "moving_mean", "fft_power"];

fn same_len(what: &str, a: usize, b: usize) -> Result<()> {
    if a == b { Ok(()) } else { Err(KernelError::InvalidArgument(format!("{what}: lengths {a} and {b} differ"))) }
}

/// Deterministic pairwise sum.
pub fn sum(x: &[f64]) -> f64 {
    #[cfg(apsi_native)]
    if available() {
        return ffi::sum(x);
    }
    reference::sum(x)
}

/// Deterministic pairwise dot product.
pub fn dot(a: &[f64], b: &[f64]) -> Result<f64> {
    same_len("dot", a.len(), b.len())?;
    #[cfg(apsi_native)]
    if available() {
        return Ok(ffi::dot(a, b));
    }
    Ok(reference::dot(a, b))
}

/// Row-major matrix product `A (m x k) * B (k x n)`. `threads` = 0 uses every
/// core; the result does not depend on the thread count.
pub fn gemm(m: usize, k: usize, n: usize, a: &[f64], b: &[f64], threads: usize) -> Result<Vec<f64>> {
    if m == 0 || k == 0 || n == 0 {
        return Err(KernelError::InvalidArgument("gemm: dimensions must be positive".into()));
    }
    let mk = m.checked_mul(k).ok_or_else(|| KernelError::InvalidArgument("gemm: m*k overflows".into()))?;
    let kn = k.checked_mul(n).ok_or_else(|| KernelError::InvalidArgument("gemm: k*n overflows".into()))?;
    let mn = m.checked_mul(n).ok_or_else(|| KernelError::InvalidArgument("gemm: m*n overflows".into()))?;
    same_len("gemm A", a.len(), mk)?;
    same_len("gemm B", b.len(), kn)?;
    #[cfg(apsi_native)]
    if available() {
        let mut c = vec![0.0; mn];
        ffi::gemm(m, k, n, a, b, &mut c, threads)?;
        return Ok(c);
    }
    let _ = (threads, mn);
    Ok(reference::gemm(m, k, n, a, b))
}

/// NDVI per pixel; NaN where undefined or flagged as nodata.
pub fn ndvi(red: &[f64], nir: &[f64], nodata: Option<f64>) -> Result<Vec<f64>> {
    same_len("ndvi", red.len(), nir.len())?;
    #[cfg(apsi_native)]
    if available() {
        let mut out = vec![0.0; red.len()];
        ffi::ndvi(red, nir, &mut out, nodata);
        return Ok(out);
    }
    Ok(reference::ndvi(red, nir, nodata))
}

/// Trailing moving mean; the first `window - 1` values are NaN.
pub fn moving_mean(x: &[f64], window: usize) -> Result<Vec<f64>> {
    if window == 0 {
        return Err(KernelError::InvalidArgument("moving_mean: window must be at least 1".into()));
    }
    #[cfg(apsi_native)]
    if available() {
        let mut out = vec![0.0; x.len()];
        ffi::moving_mean(x, window, &mut out);
        return Ok(out);
    }
    Ok(reference::moving_mean(x, window))
}

/// One-sided power spectrum (`n/2 + 1` values) of a power-of-two-length signal.
pub fn fft_power(x: &[f64]) -> Result<Vec<f64>> {
    if x.len() < 2 || !x.len().is_power_of_two() {
        return Err(KernelError::InvalidArgument(format!("fft_power: length {} is not a power of two >= 2", x.len())));
    }
    #[cfg(apsi_native)]
    if available() {
        let mut out = vec![0.0; x.len() / 2 + 1];
        ffi::fft_power(x, &mut out)?;
        return Ok(out);
    }
    Ok(reference::fft_power(x))
}

/// The C ABI. Every wrapper is only called after the public functions above
/// have validated lengths, so the C++ side never reads or writes out of bounds.
#[cfg(apsi_native)]
#[allow(unsafe_code)]
mod ffi {
    use super::{KernelError, Result};

    const OK: i32 = 0;

    unsafe extern "C" {
        fn apsi_native_abi_version() -> i32;
        fn apsi_sum(x: *const f64, n: usize, out: *mut f64) -> i32;
        fn apsi_dot(a: *const f64, b: *const f64, n: usize, out: *mut f64) -> i32;
        fn apsi_gemm(m: usize, k: usize, n: usize, a: *const f64, b: *const f64, c: *mut f64, threads: usize) -> i32;
        fn apsi_ndvi(red: *const f64, nir: *const f64, out: *mut f64, n: usize, nodata: f64, has_nodata: i32) -> i32;
        fn apsi_moving_mean(x: *const f64, n: usize, window: usize, out: *mut f64) -> i32;
        fn apsi_fft_power(x: *const f64, n: usize, out: *mut f64) -> i32;
    }

    fn check(kernel: &'static str, status: i32) -> Result<()> {
        match status {
            OK => Ok(()),
            1 => Err(KernelError::InvalidArgument(format!("{kernel}: rejected by the native library"))),
            _ => Err(KernelError::Internal(kernel)),
        }
    }

    pub fn abi_version() -> i32 {
        // SAFETY: takes no arguments and only returns a constant.
        unsafe { apsi_native_abi_version() }
    }

    pub fn sum(x: &[f64]) -> f64 {
        let mut out = 0.0;
        // SAFETY: `x` is valid for `x.len()` reads; `out` is a valid f64. The
        // kernel only rejects null pointers, which a slice never provides.
        let status = unsafe { apsi_sum(x.as_ptr(), x.len(), &mut out) };
        debug_assert_eq!(status, OK);
        out
    }

    pub fn dot(a: &[f64], b: &[f64]) -> f64 {
        let mut out = 0.0;
        // SAFETY: the caller checked a.len() == b.len(); both are valid for
        // that many reads and `out` is a valid f64.
        let status = unsafe { apsi_dot(a.as_ptr(), b.as_ptr(), a.len(), &mut out) };
        debug_assert_eq!(status, OK);
        out
    }

    pub fn gemm(m: usize, k: usize, n: usize, a: &[f64], b: &[f64], c: &mut [f64], threads: usize) -> Result<()> {
        // SAFETY: the caller checked a.len() == m*k, b.len() == k*n and
        // c.len() == m*n without overflow; the kernel touches no other memory.
        check("gemm", unsafe { apsi_gemm(m, k, n, a.as_ptr(), b.as_ptr(), c.as_mut_ptr(), threads) })
    }

    pub fn ndvi(red: &[f64], nir: &[f64], out: &mut [f64], nodata: Option<f64>) {
        debug_assert!(red.len() == nir.len() && nir.len() == out.len());
        // SAFETY: all three slices have the same length, checked by the caller.
        let status = unsafe { apsi_ndvi(red.as_ptr(), nir.as_ptr(), out.as_mut_ptr(), out.len(), nodata.unwrap_or(0.0), i32::from(nodata.is_some())) };
        debug_assert_eq!(status, OK);
    }

    pub fn moving_mean(x: &[f64], window: usize, out: &mut [f64]) {
        debug_assert_eq!(x.len(), out.len());
        // SAFETY: `out` has x.len() elements and window >= 1 (checked by the caller).
        let status = unsafe { apsi_moving_mean(x.as_ptr(), x.len(), window, out.as_mut_ptr()) };
        debug_assert_eq!(status, OK);
    }

    pub fn fft_power(x: &[f64], out: &mut [f64]) -> Result<()> {
        debug_assert_eq!(out.len(), x.len() / 2 + 1);
        // SAFETY: x.len() is a power of two >= 2 and `out` holds x.len()/2 + 1
        // elements, exactly what the kernel writes.
        check("fft_power", unsafe { apsi_fft_power(x.as_ptr(), x.len(), out.as_mut_ptr()) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic pseudo-random data (no external crates needed).
    fn data(n: usize, seed: u64) -> Vec<f64> {
        let mut s = seed;
        (0..n)
            .map(|_| {
                s = s.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
                ((s >> 11) as f64 / (1u64 << 53) as f64) * 2.0 - 1.0
            })
            .collect()
    }

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a.is_nan() && b.is_nan()) || (a - b).abs() <= tol * (1.0 + b.abs())
    }

    #[test]
    fn native_kernels_are_built_on_this_toolchain() {
        // The repository's supported toolchains (MSVC, GCC, Clang) all compile
        // the kernels; a silent fallback here would hide a build regression.
        if std::env::var_os("APSI_NATIVE_DISABLE").is_none() {
            assert!(available(), "C++ kernels unavailable: {}", build_info());
            assert_eq!(implementation(), Implementation::Native);
        }
    }

    #[test]
    fn native_and_reference_agree() {
        for n in [0, 1, 7, 128, 129, 1000, 4097] {
            let x = data(n, 1);
            let y = data(n, 2);
            // Same pairwise order, no contraction: bit-identical.
            assert_eq!(sum(&x).to_bits(), reference::sum(&x).to_bits(), "sum n={n}");
            assert_eq!(dot(&x, &y).unwrap().to_bits(), reference::dot(&x, &y).to_bits(), "dot n={n}");
        }
        let (m, k, n) = (37, 70, 29);
        let a = data(m * k, 3);
        let b = data(k * n, 4);
        let reference = reference::gemm(m, k, n, &a, &b);
        for threads in [1, 2, 8] {
            let c = gemm(m, k, n, &a, &b, threads).unwrap();
            assert!(c.iter().zip(&reference).all(|(x, y)| x.to_bits() == y.to_bits()), "gemm threads={threads}");
        }
        let red = data(500, 5).iter().map(|v| v.abs()).collect::<Vec<_>>();
        let nir = data(500, 6).iter().map(|v| v.abs()).collect::<Vec<_>>();
        let got = ndvi(&red, &nir, Some(-1.0)).unwrap();
        assert!(got.iter().zip(reference::ndvi(&red, &nir, Some(-1.0))).all(|(a, b)| close(*a, b, 0.0)));
        let mm = moving_mean(&red, 5).unwrap();
        assert!(mm.iter().zip(reference::moving_mean(&red, 5)).all(|(a, b)| close(*a, b, 0.0)));
        let p = fft_power(&data(256, 7)).unwrap();
        assert!(p.iter().zip(reference::fft_power(&data(256, 7))).all(|(a, b)| close(*a, b, 1e-12)));
    }

    #[test]
    fn gemm_is_independent_of_thread_count_on_large_inputs() {
        // Large enough (> 1 MFLOP) that the C++ side really spawns threads.
        let (m, k, n) = (128, 96, 112);
        let a = data(m * k, 8);
        let b = data(k * n, 9);
        let one = gemm(m, k, n, &a, &b, 1).unwrap();
        let many = gemm(m, k, n, &a, &b, 0).unwrap();
        assert_eq!(one.iter().map(|v| v.to_bits()).collect::<Vec<_>>(), many.iter().map(|v| v.to_bits()).collect::<Vec<_>>());
        // Spot-check one element against a naive dot product.
        let naive: f64 = (0..k).map(|p| a[5 * k + p] * b[p * n + 7]).sum();
        assert!(close(one[5 * n + 7], naive, 1e-12));
    }

    #[test]
    fn known_values() {
        assert_eq!(sum(&[1.0, 2.0, 3.5]), 6.5);
        assert!(sum(&[1.0, f64::NAN]).is_nan());
        // Pairwise summation keeps 1e6 x 0.1 close to 1e5.
        assert!((sum(&vec![0.1; 1_000_000]) - 100_000.0).abs() < 1e-6);
        assert_eq!(gemm(2, 2, 2, &[1.0, 2.0, 3.0, 4.0], &[5.0, 6.0, 7.0, 8.0], 0).unwrap(), vec![19.0, 22.0, 43.0, 50.0]);
        let v = ndvi(&[0.1, 0.0, -9999.0], &[0.5, 0.0, 0.4], Some(-9999.0)).unwrap();
        assert!((v[0] - 0.4 / 0.6).abs() < 1e-15 && v[1].is_nan() && v[2].is_nan());
        let mm = moving_mean(&[1.0, 2.0, 3.0, 4.0], 2).unwrap();
        assert!(mm[0].is_nan() && mm[1..] == [1.5, 2.5, 3.5]);
        // A pure cosine at bin 2 of 8 puts all power in bin 2 (n/4 per the |X|²/n scaling).
        let x: Vec<f64> = (0..8).map(|i| (2.0 * std::f64::consts::PI * 2.0 * i as f64 / 8.0).cos()).collect();
        let p = fft_power(&x).unwrap();
        assert!((p[2] - 2.0).abs() < 1e-12 && p.iter().enumerate().all(|(i, v)| i == 2 || v.abs() < 1e-12), "{p:?}");
    }

    #[test]
    fn invalid_arguments_are_rejected_before_the_ffi_boundary() {
        assert!(dot(&[1.0], &[1.0, 2.0]).is_err());
        assert!(gemm(2, 2, 2, &[1.0; 3], &[1.0; 4], 1).is_err());
        assert!(gemm(0, 2, 2, &[], &[1.0; 4], 1).is_err());
        assert!(gemm(usize::MAX, 2, 2, &[], &[], 1).is_err());
        assert!(ndvi(&[1.0], &[], None).is_err());
        assert!(moving_mean(&[1.0], 0).is_err());
        assert!(fft_power(&[1.0; 6]).is_err());
        assert!(fft_power(&[1.0]).is_err());
    }
}
