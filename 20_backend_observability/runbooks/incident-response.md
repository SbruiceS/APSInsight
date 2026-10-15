# Runbook: incident response

## Severity

| Sev | Examples | Response |
|---|---|---|
| 1 | data loss or corruption, a cross-tenant exposure, a leaked admin token, audit chain broken | page immediately; incident commander assigned; customer notification assessed within 24 h |
| 2 | service down (`ApsInsightDown`), sustained `storage_unavailable` | page; restore service first |
| 3 | elevated failures or latency, rate limiting, denied-request spike | ticket; next working day |

## First 15 minutes

1. **Acknowledge** the alert and open an incident log with UTC timestamps.
2. **Stabilise.** For `storage_unavailable`, check free disk space and I/O errors on the data volume. Writes are rolled back, so no partial state exists; free space and the service recovers. For a crash loop, check the logs (`--log-json`) for exit code 9, which means the engine refused to open a corrupt journal. In that case do **not** delete files: go to [backup-restore](backup-restore.md).
3. **Check integrity:** `GET /api/v1/audit/verify` (auditor token), or `apsi audit verify` on a copy of the data directory.

## Security incidents

- **Leaked token:** remove it from `APSI_TOKENS` and restart. Tokens are static, so a restart is what revokes them. Then review `GET /api/v1/audit?limit=500` for that subject's actions. The audit chain proves those events have not been altered.
- **Suspected cross-tenant access:** capture the request ids (`x-request-id`) from the proxy logs and correlate them with the audit events. Tenant isolation is enforced by `Principal::authorize`; the regression tests are `cross_tenant_access_is_denied_even_for_admin` and `api_authentication_and_tenant_isolation`.
- **Vulnerability report:** follow [SECURITY.md](../../SECURITY.md).

## Close

Write a blameless review within 5 working days covering the timeline, impact (tenants, records, duration), root cause, what detected it, and follow-up actions with owners. Add a regression test for the cause, and an alert if detection was late.
