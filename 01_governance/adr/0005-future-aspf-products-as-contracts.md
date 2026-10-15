# ADR-0005: Future ASPF products are optional, versioned contracts

- Status: Accepted
- Date: 2026-10-15

## Context

AgriQL, KutajKutambi AI, AgriCUDA, APSNetwork and Nityaga RTOS are planned and do not exist yet. GreenPlus FPGA and other hardware may follow. APSInsight must not pretend they exist or depend on them.

## Decision

- `crates/apsi-contracts` defines a v1.0 wire contract for each product, with serde types and validation, a port trait, and capability discovery (`Adapter::discover`).
- The default adapter for every product is `Unavailable`. It validates the request first, so malformed calls fail as `Invalid`, and then refuses the call as `Unavailable`. Nothing in APSInsight requires an available product.
- `conformance::check_*` is the acceptance test for any future adapter. It is proven in both directions: the default adapters and correct reference mocks pass, and deliberately broken mocks fail.
- Contract versions are `major.minor`. A provider satisfies a consumer when the majors are equal and the provider's minor is at least the consumer's.
- The AgriCUDA contract requires a CPU fallback and a numerical-tolerance contract, so `CpuFallback` is implemented. It is the reference that any accelerator is compared against.
- The KutajKutambi contract has no `observation` output kind and no write tool scope. Recommendations always require human approval.
- The Nityaga command envelope requires an acknowledgement, named interlocks, a bounded duration, an expiry of at most 900 s and an actuator the device declares.

## Consequences

- The product domains report contract capabilities as implemented and tested. Every capability that needs the product itself is reported as blocked.
- Changing a contract changes its published wire example in `crates/apsi-contracts/contracts/v1/`, and a test makes that change visible.
