# Schema ownership map

| Schema | Owner | Source of truth | Versioning | Migration path |
|---|---|---|---|---|
| Record envelope | Backend | `crates/apsi-core/src/record.rs`; published in [record.v1.schema.json](metadata-schemas/record.v1.schema.json) | the `v1` suffix; additive fields only within v1 | a new envelope version, with the API version bumped |
| Per-entity attribute schemas (800) | the tenant admin of each tenant | `EntitySchema::default_for`, plus versions published through `POST .../schemas` | integer `schema_version`; `compatibility()` classifies changes as compatible or breaking | `POST .../migrations` (dry run, apply, rollback) |
| Validation rules | the tenant admin | `PUT .../rules` | replaced as a set; audited | – |
| Notification rules, workflows | tenant editors | API | workflow definition hash recorded per run | re-register |
| Export format `apsinsight.export` | Backend | `crates/apsi-core/src/exchange.rs` | header `format_version` | importer accepts the same or an older version |
| Engine persistence (`state.json`, journal) | Backend | `engine.rs` | release-bound ([compatibility](../01_governance/release/COMPATIBILITY.md)) | snapshot rewrite on upgrade |
| ASPF contracts | Integrations | `crates/apsi-contracts`; examples in `contracts/v1/` | `major.minor` per product | the new major runs alongside the old one |
| OpenAPI | Backend | [openapi.v1.yaml](../10_backend_api/openapi.v1.yaml) | the API path prefix | [API compatibility policy](../10_backend_api/api-compatibility-policy.md) |
| PostgreSQL / PostGIS target schema | Data platform | `32_storage_postgres/migrations`, `33_storage_postgis/migrations` | numbered migrations | **specified only**, not executed |
| Units catalogue | Science | `crates/apsi-science/src/units.rs` | adding a unit is compatible; changing a factor is breaking | via a schema migration `scale` operation |
