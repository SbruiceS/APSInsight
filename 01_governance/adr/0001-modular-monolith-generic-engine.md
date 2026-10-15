# ADR-0001: Modular monolith with one generic capability engine

- Status: Accepted
- Date: 2026-10-15

## Context

The register has 3,100 requirements. `tools/analyse-register.mjs` proves that they are exactly 100 domains × 31 capability templates over 800 entity kinds. Writing 3,100 separate implementations would produce 3,100 slightly different behaviours to test and secure. The master prompt asks for the simplest architecture that keeps future extraction boundaries, and warns against premature microservices.

## Decision

- Implement each capability once in `apsi-core::Engine` and apply it to every entity kind in `register/catalog.json`.
- Ship one server process (`apsi-server`) and one CLI (`apsi`). Both use the same engine library.
- Keep science (`apsi-science`) and integration contracts (`apsi-contracts`) as separate crates with no dependency on the server.
- Separate deployable workers are not created until a measured need exists (for example job queues or ingestion streaming).

## Consequences

- A capability that is tested once is tested for all 800 entity kinds. `acceptance_every_entity_kind_supports_core_capabilities` and `api_acceptance_sweep_all_domains` sweep every kind.
- A domain whose substance is more than record management (raster I/O, ML training, OIDC) is **not** completed by the generic engine. Those domains are capped in the traceability matrix ([ADR-0007](0007-honest-feature-status.md)).
- Crate boundaries are the future extraction boundaries: engine, science, contracts, API and CLI.
