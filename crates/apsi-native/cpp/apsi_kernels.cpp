#include "apsi_kernels.hpp"

#include <algorithm>
#include <cmath>
#include <complex>
#include <exception>
#include <limits>
#include <thread>
#include <vector>

namespace {

constexpr int kAbiVersion = 1;
constexpr std::size_t kPairwiseBlock = 128;
constexpr std::size_t kTile = 64;
const double kNaN = std::numeric_limits<double>::quiet_NaN();

// Pure function: identical inputs give bit-identical outputs.
double pairwise_sum(const double* x, std::size_t n) {
  if (n <= kPairwiseBlock) {
    double s = 0.0;
    for (std::size_t i = 0; i < n; ++i) s += x[i];
    return s;
  }
  const std::size_t half = n / 2;
  return pairwise_sum(x, half) + pairwise_sum(x + half, n - half);
}

double pairwise_dot(const double* a, const double* b, std::size_t n) {
  if (n <= kPairwiseBlock) {
    double s = 0.0;
    for (std::size_t i = 0; i < n; ++i) s += a[i] * b[i];
    return s;
  }
  const std::size_t half = n / 2;
  return pairwise_dot(a, b, half) + pairwise_dot(a + half, b + half, n - half);
}

bool mul_overflows(std::size_t a, std::size_t b) { return a != 0 && b > std::numeric_limits<std::size_t>::max() / a; }

// C rows [r0, r1) = A rows * B, tiled over k and n for cache reuse. The k loop
// runs in ascending order for every element, which keeps results independent
// of how rows are split between threads.
void gemm_rows(std::size_t r0, std::size_t r1, std::size_t k, std::size_t n, const double* a, const double* b, double* c) {
  for (std::size_t i = r0; i < r1; ++i) std::fill(c + i * n, c + (i + 1) * n, 0.0);
  for (std::size_t kk = 0; kk < k; kk += kTile) {
    const std::size_t k_end = std::min(k, kk + kTile);
    for (std::size_t jj = 0; jj < n; jj += kTile) {
      const std::size_t j_end = std::min(n, jj + kTile);
      for (std::size_t i = r0; i < r1; ++i) {
        double* crow = c + i * n;
        const double* arow = a + i * k;
        for (std::size_t p = kk; p < k_end; ++p) {
          const double aip = arow[p];
          const double* brow = b + p * n;
          for (std::size_t j = jj; j < j_end; ++j) crow[j] += aip * brow[j];
        }
      }
    }
  }
}

bool is_power_of_two(std::size_t n) { return n >= 2 && (n & (n - 1)) == 0; }

template <typename F>
int guarded(F&& f) noexcept {
  try {
    return f();
  } catch (...) {
    return APSI_INTERNAL_ERROR;
  }
}

}  // namespace

extern "C" {

int apsi_native_abi_version(void) { return kAbiVersion; }

int apsi_sum(const double* x, std::size_t n, double* out) {
  if (out == nullptr || (n > 0 && x == nullptr)) return APSI_INVALID_ARGUMENT;
  *out = pairwise_sum(x, n);
  return APSI_OK;
}

int apsi_dot(const double* a, const double* b, std::size_t n, double* out) {
  if (out == nullptr || (n > 0 && (a == nullptr || b == nullptr))) return APSI_INVALID_ARGUMENT;
  *out = pairwise_dot(a, b, n);
  return APSI_OK;
}

int apsi_gemm(std::size_t m, std::size_t k, std::size_t n, const double* a, const double* b, double* c, std::size_t threads) {
  return guarded([&]() -> int {
    if (m == 0 || k == 0 || n == 0 || a == nullptr || b == nullptr || c == nullptr) return APSI_INVALID_ARGUMENT;
    if (mul_overflows(m, k) || mul_overflows(k, n) || mul_overflows(m, n)) return APSI_INVALID_ARGUMENT;
    std::size_t workers = threads == 0 ? std::max(1u, std::thread::hardware_concurrency()) : threads;
    // Below ~1 MFLOP the thread start-up costs more than it saves.
    if (m * n * k < (std::size_t{1} << 20)) workers = 1;
    workers = std::min(workers, m);
    if (workers <= 1) {
      gemm_rows(0, m, k, n, a, b, c);
      return APSI_OK;
    }
    std::vector<std::thread> pool;
    pool.reserve(workers);
    const std::size_t chunk = (m + workers - 1) / workers;
    for (std::size_t w = 0; w < workers; ++w) {
      const std::size_t r0 = w * chunk;
      const std::size_t r1 = std::min(m, r0 + chunk);
      if (r0 >= r1) break;
      pool.emplace_back(gemm_rows, r0, r1, k, n, a, b, c);
    }
    for (auto& t : pool) t.join();
    return APSI_OK;
  });
}

int apsi_ndvi(const double* red, const double* nir, double* out, std::size_t n, double nodata, int has_nodata) {
  if (n > 0 && (red == nullptr || nir == nullptr || out == nullptr)) return APSI_INVALID_ARGUMENT;
  for (std::size_t i = 0; i < n; ++i) {
    const double r = red[i];
    const double v = nir[i];
    const bool missing = std::isnan(r) || std::isnan(v) || (has_nodata != 0 && (r == nodata || v == nodata));
    const double den = v + r;
    out[i] = (missing || den == 0.0) ? kNaN : (v - r) / den;
  }
  return APSI_OK;
}

int apsi_moving_mean(const double* x, std::size_t n, std::size_t window, double* out) {
  if (window == 0 || (n > 0 && (x == nullptr || out == nullptr))) return APSI_INVALID_ARGUMENT;
  for (std::size_t i = 0; i < n; ++i) {
    out[i] = (i + 1 < window) ? kNaN : pairwise_sum(x + (i + 1 - window), window) / static_cast<double>(window);
  }
  return APSI_OK;
}

int apsi_fft_power(const double* x, std::size_t n, double* out) {
  return guarded([&]() -> int {
    if (x == nullptr || out == nullptr || !is_power_of_two(n)) return APSI_INVALID_ARGUMENT;
    std::vector<std::complex<double>> a(x, x + n);
    // Bit-reversal permutation.
    for (std::size_t i = 1, j = 0; i < n; ++i) {
      std::size_t bit = n >> 1;
      for (; j & bit; bit >>= 1) j ^= bit;
      j ^= bit;
      if (i < j) std::swap(a[i], a[j]);
    }
    const double pi = std::acos(-1.0);
    for (std::size_t len = 2; len <= n; len <<= 1) {
      const double angle = -2.0 * pi / static_cast<double>(len);
      const std::complex<double> wlen(std::cos(angle), std::sin(angle));
      for (std::size_t i = 0; i < n; i += len) {
        std::complex<double> w(1.0, 0.0);
        for (std::size_t j = 0; j < len / 2; ++j) {
          const std::complex<double> u = a[i + j];
          const std::complex<double> v = a[i + j + len / 2] * w;
          a[i + j] = u + v;
          a[i + j + len / 2] = u - v;
          w *= wlen;
        }
      }
    }
    for (std::size_t k = 0; k <= n / 2; ++k) out[k] = std::norm(a[k]) / static_cast<double>(n);
    return APSI_OK;
  });
}

}  // extern "C"
