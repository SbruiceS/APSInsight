# Data dictionary

Terms used across the code, API and documentation. The machine-readable definitions are the [record JSON Schema](metadata-schemas/record.v1.schema.json), the per-entity schemas (`GET /api/v1/domains/{domain}/entities/{entity}/schemas`), the [units catalogue](../docs/UNITS.md) and the [ASPF contract examples](../crates/apsi-contracts/contracts/v1).

## Core concepts

| Term | Definition | Defined in |
|---|---|---|
| Domain | One of the 100 register areas, such as `soil-chemistry-and-fertility` | `register/catalog.json` |
| Entity kind | One of the 800 record types within a domain, such as `ph` or `yield-observations` | `register/catalog.json` |
| Capability | One of the 31 behaviours applied to every entity kind, such as versioning or import | `crates/apsi-core/src/catalog.rs` |
| Feature | A domain × capability pair, `APSI-F0001`–`APSI-F3100` | register |
| Tenant | The isolation boundary. Every record, rule, workflow and audit event belongs to exactly one | `auth.rs` |
| Record | A versioned, tenant-scoped instance of an entity kind | `record.rs` |
| Schema | The versioned attribute definition for an entity kind: type, required, unit, range, enumeration | `schema.rs` |
| Rule | A configurable validation check with a severity (`error`, `warning`, `info`) | `validation.rs` |
| Provenance event | Who changed a record, how, from which source and inputs | `record.rs` |
| Audit event | A hash-chained log entry for every operation, including denials | `audit.rs` |
| Source identity | `(source.system, source.id)`, unique per tenant and entity kind; makes re-imports idempotent | `record.rs` |
| Legal hold | A flag that blocks archival, purge and deletion | `retention.rs` |
| Workflow | Typed, ordered steps whose runs record a definition hash and a data fingerprint | `workflow.rs` |

## Kinds of value

These must never be conflated. A field or output declares which kind it is through its name, its schema description or the contract type.

| Kind | Meaning | How it is marked |
|---|---|---|
| Observation | measured or recorded in the world | source identity and provenance `create`/`import`; the default for imported records |
| Derived measurement | computed deterministically from observations, such as NDVI or ET₀ | workflow run with definition hash and inputs; provenance `transformation` |
| Statistical estimate | an estimate with uncertainty, such as a mean or confidence interval | output carries `ci`, `se` or `p`; never stored as an observation |
| Prediction | a model output about an unobserved value | `OutputKind::Prediction` with model, version, evaluation and uncertainty |
| AI interpretation | a model-generated explanation | `OutputKind::AiInterpretation` |
| Human-approved recommendation | a recommendation accepted by a person | `OutputKind::Recommendation` + `requires_human_approval`; the approval is recorded as a provenance event |
| Synthetic | fabricated for tests or examples | identifiers contain `synthetic`; never imported into production tenants |

## Missing values

A missing value is absent or `null`, never `0`, `-9999` or NaN. Quality indicators over empty sets are `null`. Contract quantities with no value have `quality: "missing"`.

## Time

All timestamps are RFC 3339 in UTC (`2026-10-15T08:00:00Z`). Telemetry keeps `event_time` (when it was observed) separate from `received_time` (when it arrived). The time zone of a local observation is an attribute (`timezone`, IANA name) and is preserved.
