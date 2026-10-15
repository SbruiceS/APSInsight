# Compatibility matrix

| Component | Version | Compatible with | Verified by |
|---|---|---|---|
| Rust toolchain | MSRV 1.88; developed on 1.99.0 | edition 2024 | CI job `msrv` (`cargo check --workspace` on 1.88); `verify` on stable |
| Node.js | 24.x | Angular 22, TypeScript 6 | CI `verify` |
| HTTP API | `/api/v1` | web UI of the same release; any client following the [API compatibility policy](../../10_backend_api/api-compatibility-policy.md) | `crates/apsi-server/tests/api.rs`; `openapi_documents_every_route` |
| Export format | `apsinsight.export` v1 | import of the same or a later 0.x release | `export_import_roundtrip_is_lossless_in_csv_and_json` |
| Data directory | journal + `state.json` written by 0.1.x | opened by 0.1.x | `persistence_survives_restart_and_audit_chain_verifies` |
| ASPF contracts | 1.0 for every product | providers offering 1.x (same major, minor ≥ 0) | `version_compatibility_rules`, `published_wire_examples_match_contract_types` |
| Record JSON Schema | v1 | records returned by `/api/v1/records/{id}` | `record_envelope_matches_published_json_schema` |
| Browsers | current Chrome, Edge, Firefox, Safari | – | not tested automatically (no e2e suite) |
| PostgreSQL / PostGIS | target 16 / 3.4 | – | **specified only**; not executed |
