# Changelog

All notable changes are recorded here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses [Semantic Versioning](https://semver.org/). The HTTP API is versioned separately (`/api/v1`; see the [API compatibility policy](10_backend_api/api-compatibility-policy.md)).

## [Unreleased]

### Added

- The 72 canonical top-level directories, each with a generated README covering ownership, purpose, implementation pointers, register coverage and gaps (`01_governance/structure.json`, `tools/structure.mjs`).
- Honest per-feature status (implemented and tested / implemented but not fully tested / specified only / blocked by an external dependency), with a gap note per feature in `docs/traceability.csv`, generated `docs/STATUS.md`, `apsi features status` and a status column in each domain document.
- The `apsi-contracts` crate: versioned v1.0 contracts for AgriQL, KutajKutambi AI, AgriCUDA, APSNetwork, Nityaga RTOS and optional accelerators. It includes port traits, capability discovery, default unavailable adapters, the AgriCUDA CPU fallback, a conformance harness and published wire examples.
- `GET /api/v1/integrations` and `apsi integrations` for capability discovery.
- `apsi_science::units`: dimension-checked quantities, a units catalogue (`docs/UNITS.md`) and the `units.convert` compute function.
- OpenAPI 3.1 specification, with a test that every route is documented.
- ADRs, threat model, CONTRIBUTING, SECURITY policy, data dictionary, CRS policy, schema ownership map, record JSON Schema, accessibility checklist, SLOs, Prometheus alert rules, Grafana dashboard, runbooks, scientific validation plan, model card template, release manifest and compatibility matrix.
- Container definitions (`Dockerfile`, `compose.yaml`) without secrets; SBOM and licence report (`tools/sbom.mjs`); CI security jobs and an MSRV job.
- Specified, not executed: PostgreSQL/PostGIS target migrations.

### Changed

- The declared MSRV is now 1.88 (it was 1.85). The code already used let-chains, which need 1.88, so the old value was wrong.

## [0.1.0]

### Added

- Generic engine for 100 domains × 31 capabilities over 800 entity kinds, plus the science library, HTTP API, CLI and Angular UI.
