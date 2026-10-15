# Acceptance criteria

## Per feature

A feature is **implemented and tested** only when every item below holds. Otherwise its status in [docs/STATUS.md](../docs/STATUS.md) is lower, and its gap note in [docs/traceability.csv](../docs/traceability.csv) says what is missing.

1. **Implementation reference:** the engine, server, CLI or contract code is named in the matrix.
2. **Automated test reference:** at least one named test exists (enforced by `every_referenced_test_exists`) and passes in `node tools/verify.mjs`.
3. **Failure-case coverage:** the tests include rejected input or denied access, not only the happy path.
4. **Documentation reference:** a generated domain document plus the capability document.
5. **Traceability status:** computed, never hand-entered ([ADR-0007](../01_governance/adr/0007-honest-feature-status.md)).

## Per capability (measurable)

| Capability | Acceptance test (expected outcome) |
|---|---|
| Registration and identity | ids match `rec_` + 32 hex characters; a duplicate source identity returns `conflict` |
| Schema contract | a breaking schema needs `allow_breaking`; malformed schemas are rejected |
| Validation rules | `error` makes a record invalid, `warning` does not; reports are machine-readable |
| CRUD and lifecycle | archived records are read-only; delete needs archived state and no legal hold |
| Search and filtering | exact, range, facet and pagination results are correct; the limit is at most 500 |
| Bulk operations | a dry run changes nothing; partial failures are reported per item; the idempotency key prevents re-application |
| Versioning | every change increments `version`; a rollback creates a new version |
| Access control | cross-tenant access returns `not_found`; each role is allowed and denied per the matrix |
| Concurrency control | 8 × 25 concurrent increments give exactly 200 |
| Export / import | CSV and JSON round trips are lossless |
| Migration support | a failed migration changes nothing; rollback restores |
| ASPF contracts | default adapters and correct mocks pass conformance; broken mocks fail |
| Units | a dimension mismatch is rejected; conversions round-trip (property test) |

## Release

A release needs, in addition: `node tools/verify.mjs` passing, the CI security jobs passing, a tested backup restore ([runbook](../20_backend_observability/runbooks/backup-restore.md)) with its evidence recorded, the SBOM attached, and a CHANGELOG entry.
