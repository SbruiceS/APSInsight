# ADR-0006: Rust and TypeScript only until a benchmark justifies C++, Python or formal methods

- Status: Accepted
- Date: 2026-10-15

## Context

The master prompt allows C++ kernels "only where benchmarks justify it", Python workers for libraries that "materially improve scientific capabilities", and Haskell or Idris 2 "only when value is demonstrated".

## Decision

No C++, Python, Haskell or Idris code exists in this build, because no measurement has yet justified any of them. The current computations (per-pixel indices, FAO-56, regression) run in well under a millisecond per call.

When a benchmark does justify a C++ kernel, it must follow these rules:

- **ABI:** a C ABI (`extern "C"`) with a version symbol `apsi_kernel_abi_version() -> u32`. The host refuses to load a kernel whose ABI major differs from its own.
- **Ownership:** the caller allocates and frees all buffers. Kernels never keep pointers after returning.
- **Buffers:** passed as `(ptr, len)` element counts with `len <= 2^28`. Alignment is declared and checked, using the same rules as `apsi_contracts::agricuda::BufferDescriptor`.
- **Numeric types:** `f32` or `f64` only, stated per buffer. Results are compared with the Rust reference within a declared tolerance.
- **Errors:** an `int32` status code (0 means ok) plus an optional message buffer. C++ exceptions are caught at the boundary and mapped to a status code; they never cross the ABI.
- **Panics:** Rust callbacks into C++ are not allowed.

Python workers, if added, run as separate processes with resource limits, a versioned JSON contract and a pinned lock file.

## Consequences

- No FFI exists, which is why `unsafe_code = "forbid"` holds across the workspace.
- A future kernel is accepted only together with the benchmark that justified it, recorded in [docs/PERFORMANCE.md](../../docs/PERFORMANCE.md).
