# ADR-0007: Four-value feature status, derived from code and tests

- Status: Accepted
- Date: 2026-10-15

## Decision

Every register feature has exactly one of these statuses: **implemented and tested**, **implemented but not fully tested**, **specified only** or **blocked by an external dependency**. The status is computed in `crates/apsi-core/src/traceability.rs`:

1. Each of the 31 capabilities has a base status and a gap note, which is empty only when the capability is fully tested.
2. A **domain scope** caps the status of a domain whose substance needs something this build lacks, for example PostgreSQL, OIDC, raster I/O or a job queue. A cap can only lower a status (a test enforces this). It does not apply to schema contract, metadata model and documentation, whose evidence holds regardless.
3. In the planned-product domains, only the ten capabilities a contract can satisfy are implemented. Every other capability in those domains is blocked.

## Guards

- `every_referenced_test_exists` fails if any feature names a test that does not exist.
- `every_feature_has_evidence` fails if a status short of "implemented and tested" has no gap note.
- `scopes_reference_real_domains_and_never_raise_status` fails if a scope raises a status or reports a planned product as working.
- `generated_docs_are_up_to_date` fails if `docs/STATUS.md`, `docs/traceability.csv` or the domain documents drift from the code.

## Consequences

Completion metrics are reproducible (`apsi features status`), and they can only improve when code and tests improve.
