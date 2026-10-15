# APSInsight

The agriculture data analytics and intelligence platform of the Agriculture Service of Product Farm (ASPF). It ingests, validates, catalogues, versions, analyses and governs agricultural data, and it is built from the 3,100-feature register in [`register/APSInsight_Feature_Register.json`](register/APSInsight_Feature_Register.json).

The register is exactly **100 domains × 31 capabilities** over **800 entity kinds** (proved by [`tools/analyse-register.mjs`](tools/analyse-register.mjs)). Each capability is implemented once, in a generic engine, and applied to every entity kind. Domain science lives in a separate library ([ADR-0001](01_governance/adr/0001-modular-monolith-generic-engine.md)).

## What is and isn't done

Every feature has one honest status: **implemented and tested**, **implemented but not fully tested**, **specified only** or **blocked by an external dependency**. Status comes from code and tests, never from the existence of a directory or document ([ADR-0007](01_governance/adr/0007-honest-feature-status.md)). Current counts, per-domain tables and the reason for each gap are in **[docs/STATUS.md](docs/STATUS.md)**; per-feature evidence is in [docs/traceability.csv](docs/traceability.csv). `apsi features status` prints the counts.

Not in this build, with the reason recorded per feature:

- **Blocked by an external dependency:** PostgreSQL/PostGIS (the target schemas are specified only), object storage, Arrow/Parquet/DuckDB, raster I/O, OIDC/SAML/MFA, satellite and agency connectors, NL assistance and computer vision.
- **Specified only:** realtime channels, job queues, ML training and the map UI.
- **Planned ASPF products:** AgriQL, KutajKutambi AI, AgriCUDA, APSNetwork and Nityaga RTOS **do not exist yet**. APSInsight ships only their versioned contracts, capability discovery, unavailable default adapters, a CPU fallback for AgriCUDA and conformance tests ([contracts](70_connectors_external/aspf-contracts/README.md)).

## Components

| Component | Path | What it is |
|---|---|---|
| Engine | [crates/apsi-core](crates/apsi-core) | records, versioned schemas, validation, search, bulk/import/export, versioning, provenance, quality, retention and legal hold, notifications, workflows, migrations, hash-chained audit, metrics, traceability |
| Science | [crates/apsi-science](crates/apsi-science) | spectral indices, geodesy and reprojection, statistics, time series, FAO-56 ET₀, water balance, soil texture, units with dimensional analysis, economics, emissions |
| Contracts | [crates/apsi-contracts](crates/apsi-contracts) | v1.0 contracts for the planned ASPF products and optional accelerators; conformance harness |
| API | [crates/apsi-server](crates/apsi-server) | `/api/v1` over axum: bearer tokens, tenant isolation, limits, envelopes; serves the UI; [OpenAPI](10_backend_api/openapi.v1.yaml) |
| CLI | [crates/apsi-cli](crates/apsi-cli) | `apsi`, with JSON output and stable exit codes |
| UI | [web](web) | Angular 22: dashboards, records, history, feature register with status; accessible; English and Hindi |

The repository also has the **72 canonical directories** (`01_governance` … `72_economics_markets`). Each has a generated README giving its ownership, purpose, where the implementation lives, the status counts of the register domains it owns, and known gaps. They come from [01_governance/structure.json](01_governance/structure.json) and are checked by `node tools/structure.mjs --check`.

## Prerequisites

- Rust 1.88 or newer (developed on 1.99), with `rustfmt` and `clippy`.
- Node.js 24 and npm.
- Optional: Docker 24 or newer.

## Local development

```sh
cargo build --release
(cd web && npm ci && npm run build)
./target/release/apsi-server --dev --data-dir .apsi-data --web-dir web/dist/web/browser
# open http://127.0.0.1:8080 and paste the printed admin token
```

The CLI works on the same data directory. Don't run it while the server is writing to that directory.

```sh
./target/release/apsi --tenant demo records create soil-profile-and-classification ph \
  --json '{"name":"Synthetic plot topsoil","attributes":{"value":6.4,"unit":"pH"}}'
./target/release/apsi compute units.convert --json '{"value":120,"from":"lb/ac","to":"kg/ha"}'
./target/release/apsi integrations        # every ASPF product: optional, unavailable
```

To run in a container (no secrets are baked in; tokens come from the environment):

```sh
export APSI_TOKENS="$(openssl rand -hex 24):demo:ada:admin"
docker compose up --build
```

## Tests and checks

```sh
node tools/verify.mjs          # everything CI runs; stops at the first failure
node tools/verify.mjs --skip-web
```

`verify.mjs` runs, in order:

1. register analysis
2. the 72-directory structure check
3. `rustfmt`
4. clippy with every warning as an error
5. all Rust tests (unit, integration, HTTP contract, property, ASPF contract conformance, docs drift, OpenAPI route coverage)
6. the SBOM and licence gate
7. UI static and accessibility checks
8. UI unit tests
9. the UI production build

CI also runs an MSRV job, `cargo deny`, `npm audit`, gitleaks, and a container build with a smoke test ([.github/workflows/ci.yml](.github/workflows/ci.yml)).

## Documentation

- [Architecture](docs/ARCHITECTURE.md), with context and component diagrams, data flow and trust boundaries, plus the [ADRs](01_governance/adr/README.md).
- [API](docs/API.md), [OpenAPI](10_backend_api/openapi.v1.yaml) and the [compatibility policy](10_backend_api/api-compatibility-policy.md).
- [CLI](docs/CLI.md) · [UI](docs/UI.md) · [accessibility checklist](05_frontend_design_system/accessibility-checklist.md).
- [Security review](docs/SECURITY.md) · [threat model](01_governance/threat-model.md).
- [Operations](docs/OPERATIONS.md), [SLOs](20_backend_observability/slo.md), [alerts](20_backend_observability/alerts/apsinsight.rules.yml), [dashboard](20_backend_observability/dashboards/apsinsight-overview.json), and runbooks for [backup/restore](20_backend_observability/runbooks/backup-restore.md), [incidents](20_backend_observability/runbooks/incident-response.md) and [rollback](20_backend_observability/runbooks/deployment-rollback.md).
- [Data dictionary](03_domain_model/data-dictionary.md) · [units](docs/UNITS.md) · [CRS policy](03_domain_model/crs-policy.md) · [schema ownership](03_domain_model/schema-ownership.md) · [record JSON Schema](03_domain_model/metadata-schemas/record.v1.schema.json).
- [Testing](docs/TESTING.md) · [Performance](docs/PERFORMANCE.md) · [Interoperability](docs/INTEROPERABILITY.md).
- [Scientific validation plan](65_scientific_workflows/validation-plan.md) · [model card template](59_ml_platform/model-card-template.md).
- [Release manifest](01_governance/release/manifest.json) · [compatibility matrix](01_governance/release/COMPATIBILITY.md) · [CHANGELOG](CHANGELOG.md) · [CONTRIBUTING](CONTRIBUTING.md).

## Security

Report vulnerabilities privately as described in [SECURITY.md](SECURITY.md). Please don't open public issues for them.

## Licence

Apache-2.0.
