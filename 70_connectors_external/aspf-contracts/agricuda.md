# AgriCUDA contract v1.0

**Status:** contract and **CPU fallback** implemented and tested; the AgriCUDA product is planned and not yet created.

## `ComputeJob`

| Field | Rules |
|---|---|
| `operation` | `vector.add` (2 inputs, 1 output), `vector.scale` (1 input, 1 output, scalar `factor`), `index.ndvi` (inputs red, nir; 1 output, unit `1`) |
| `buffers[]` `{name, role: input\|output, dtype: f32\|f64, len, alignment, unit}` | `len` 1–2²⁸; `alignment` a power of two ≥ the element size; unit required; equal lengths for element-wise operations |
| `scalars` | the operation's required scalars, all finite |
| `tolerance` `{abs, rel}` | non-negative; `\|actual − reference\| ≤ abs + rel·\|reference\|`; NaN matches only NaN |

Inputs passed to `execute` must match the input descriptors in count and length.

## `ComputeResult`

`outputs`, `fallback_used`, `profile {backend, device, elapsed_ns}`.

## CPU fallback

`agricuda::CpuFallback` is deterministic and computes in `f64`. It returns NaN where NDVI is undefined (red + nir = 0) so the caller can flag the value as missing; it never returns a fake 0. It is the reference every accelerator is compared against, and `accelerator::execute_with_fallback` uses it when no accelerator is available.

## Tests

`compute_job_rules`, `cpu_fallback_is_the_compute_reference`, plus the shared conformance tests. The broken mock is off by 0.1 in one element and is rejected by the tolerance check.
