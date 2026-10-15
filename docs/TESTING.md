# Testing

```sh
node tools/verify.mjs      # everything below, fails on the first problem
```

| Suite | Where | Kind |
|---|---|---|
| Unit | `#[cfg(test)]` modules in every crate | boundary cases, malformed inputs, reference values (FAO-56, USDA texture, t-tables, BH) |
| Property-based | `proptest` in `ids`, `validation`, `exchange`, `indices`, `geo`, `stats` | identifiers, validation totality, CSV round-trip, index bounds, reprojection round-trip, BH validity, t-CDF monotonicity |
| Engine integration | `crates/apsi-core/tests/engine.rs` | every capability family, including concurrency, persistence, crash recovery, and an acceptance sweep over **all 800 entity kinds** |
| HTTP contract | `crates/apsi-server/tests/api.rs` | envelopes, status codes, auth, tenant isolation, limits, headers, and a sweep over all 100 domains |
| CLI end-to-end | `crates/apsi-cli/tests/cli.rs` | JSON output, exit codes, files, benchmarks, generated-docs freshness |
| Traceability | `apsi-core::traceability::tests` | each of the 3,100 features has evidence and a status, and any status short of fully tested explains its gap; every Rust test named as evidence exists; domain scopes can only lower a status; planned products are never reported as working |
| ASPF contracts | `crates/apsi-contracts/tests/contracts.rs` | default adapters and reference mocks pass conformance; broken mocks fail; per-product validation (unit, CRS, safety envelope, AI output kinds, tolerance); published wire examples match the types |
| Units | `apsi-science::units::tests` | dimension mismatches rejected, temperature semantics, agreement with `agronomy::convert`, round-trip property test over the whole catalogue |
| Published contracts | `openapi_documents_every_route`, `record_envelope_matches_published_json_schema` | the OpenAPI document and the record JSON Schema match the code |
| Repository structure | `node tools/structure.mjs --check` | the 72 canonical directories exist with current READMEs; every referenced path exists; each register domain is assigned exactly once; no empty directories |
| Supply chain | `node tools/sbom.mjs` (+ `--self-test`) | CycloneDX SBOMs; every shipped dependency has an allowed licence; the SPDX expression evaluator is self-tested |
| UI | `web/src/**/*.spec.ts` (Vitest + jsdom), `web/scripts/check-ui.mjs` | loading, empty, ready, denied and error states; bearer header; labelled controls; i18n parity |

The build treats warnings as errors: the workspace lints are `warnings = "deny"` plus clippy `all = deny`, and TypeScript runs with `strict` and `strictTemplates`.
