# ASPF integration contracts (v1.0)

> **The products below are planned and not yet created.** APSInsight ships contracts, default *unavailable* adapters, a conformance harness and contract tests, and nothing else. No APSInsight feature needs any of these products to be present.

| Product | Spec | Code | Wire examples | Operations |
|---|---|---|---|---|
| AgriQL | [agriql.md](agriql.md) | `apsi_contracts::agriql` | `agriql.query-submission.json` | `query.submit` |
| KutajKutambi AI | [kutajkutambi.md](kutajkutambi.md) | `apsi_contracts::kutajkutambi` | `kutajkutambi.inference-request.json` | `inference.run`, `agent.task` |
| AgriCUDA | [agricuda.md](agricuda.md) | `apsi_contracts::agricuda` | `agricuda.compute-job.json` | `compute.execute`, `compute.cpu-fallback` |
| APSNetwork | [apsnetwork.md](apsnetwork.md) | `apsi_contracts::apsnetwork` | `apsnetwork.edge-job.json` | `telemetry.ingest`, `edge-job.deliver` |
| Nityaga RTOS | [nityaga.md](nityaga.md) | `apsi_contracts::nityaga` | `nityaga.device-capability.json`, `nityaga.command-envelope.json` | `device.capabilities`, `device.command` |
| Accelerators (e.g. GreenPlus FPGA) | [accelerator.md](accelerator.md) | `apsi_contracts::accelerator` | – | `accelerator.load`, `compute.execute` |

Wire examples are in [crates/apsi-contracts/contracts/v1](../../crates/apsi-contracts/contracts/v1). They are synthetic and verified against the types by `published_wire_examples_match_contract_types`.

## Rules common to every contract

- **Versioning:** `major.minor`. A provider satisfies a consumer when the majors are equal and the provider's minor is ≥ the consumer's (`ContractVersion::satisfies`). An incompatible provider is detected at discovery (`CapabilityReport::check_compatible`).
- **Discovery:** `GET /api/v1/integrations` or `apsi integrations` returns a `CapabilityReport` for each product: `availability` (`unavailable` with a reason, or `available` with the offered contract), operations, and `optional: true`.
- **Errors:** `ContractError` is `invalid` (permanent, the request violates the contract), `unavailable` (the product is not present), `incompatible` (version mismatch) or `provider` (with an explicit `retryable` flag).
- **Values:** every quantity has a UCUM-style unit (`1` when dimensionless); a missing value is `null` with quality `missing`; CRS is `EPSG:<code>` or `OGC:CRS84`; timestamps are RFC 3339; hashes are lowercase SHA-256 hex.
- **Validation first:** an adapter must reject malformed requests as `invalid` before doing any provider work.

## Accepting a future adapter

An adapter is accepted only when the matching `apsi_contracts::conformance::check_*` returns no findings. The harness has been proven in both directions: the default adapters and correct reference mocks pass, and deliberately broken mocks fail (`broken_mocks_fail_conformance`).
