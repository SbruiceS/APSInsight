# Hardware accelerator contract v1.0 (e.g. GreenPlus FPGA)

**Status:** contract implemented and tested; no accelerator exists. The contract is optional: without an accelerator, work runs on the AgriCUDA CPU fallback.

## `AcceleratorManifest`

| Field | Rules |
|---|---|
| `device_family` | 1–64 bytes |
| `bitstream_sha256` | lowercase SHA-256 hex; pins the exact bitstream |
| `driver_contract` | must satisfy the host's contract version |
| `operations[]` | non-empty; each must be an AgriCUDA v1.0 operation |
| `safety_constraints[]` | thermal, power or interlock limits the host enforces |
| `test_vectors[]` | non-empty; an accelerator without test vectors cannot be verified |

## Execution

`AcceleratorPort: ComputePort` adds `load(&manifest)`. Jobs use `agricuda::ComputeJob`, and results must match `CpuFallback` within the job tolerance (`conformance::check_compute`). `execute_with_fallback` runs on the CPU when the accelerator is unavailable and reports `fallback_used: true`. Validation errors are never hidden by the fallback.

## Tests

`accelerator_manifest_rules`, `cpu_fallback_is_the_compute_reference`, `unavailable_adapters_pass_conformance_and_degrade_gracefully`.
