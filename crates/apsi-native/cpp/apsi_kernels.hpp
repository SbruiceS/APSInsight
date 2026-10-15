// APSInsight native scientific kernels (C++17, no third-party dependencies).
//
// A narrow, versioned C ABI: every function takes raw buffers plus explicit
// lengths, never allocates memory that the caller must free, never throws
// across the boundary, and returns a status code. The Rust crate apsi-native
// wraps these in safe, length-checked functions and holds a pure-Rust reference
// implementation of every kernel that the tests compare against.
#pragma once

#include <cstddef>

#ifdef __cplusplus
extern "C" {
#endif

enum apsi_status {
  APSI_OK = 0,
  APSI_INVALID_ARGUMENT = 1,
  APSI_INTERNAL_ERROR = 2,
};

/// Bumped whenever a signature or semantics changes.
int apsi_native_abi_version(void);

/// Pairwise (cascade) summation in a fixed order: deterministic and with an
/// O(log n) error bound instead of the O(n) of a naive loop. NaN propagates.
int apsi_sum(const double* x, std::size_t n, double* out);

/// Dot product using the same deterministic pairwise reduction.
int apsi_dot(const double* a, const double* b, std::size_t n, double* out);

/// Row-major C(m x n) = A(m x k) * B(k x n). Cache-blocked; rows are split
/// across up to `threads` worker threads (0 = hardware concurrency). Each
/// output element is produced by exactly one thread in a fixed k order, so the
/// result does not depend on the thread count.
int apsi_gemm(std::size_t m, std::size_t k, std::size_t n, const double* a, const double* b, double* c, std::size_t threads);

/// NDVI = (nir - red) / (nir + red). NaN where an input is NaN, equals
/// `nodata`, or the denominator is zero (undefined, not a fake 0).
int apsi_ndvi(const double* red, const double* nir, double* out, std::size_t n, double nodata, int has_nodata);

/// Trailing moving mean over `window` samples; the first window-1 outputs are
/// NaN. Uses a fresh pairwise sum per window block to avoid drift.
int apsi_moving_mean(const double* x, std::size_t n, std::size_t window, double* out);

/// One-sided power spectrum |X_k|^2 / n for k = 0..n/2 of a real signal, via
/// an iterative radix-2 FFT. `n` must be a power of two >= 2; `out` holds
/// n/2 + 1 values.
int apsi_fft_power(const double* x, std::size_t n, double* out);

#ifdef __cplusplus
}
#endif
