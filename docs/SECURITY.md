# Security review and threat model

## Assets

- Tenant records and their history.
- The audit log.
- API tokens.
- Operational metadata: schemas, rules, workflows.

## Trust boundaries

1. Browser or API client to the HTTP API.
2. The API to the engine, through tenant-bound principals.
3. The engine to the data directory.
4. Local operator to the CLI. Anyone with filesystem access to the data directory is fully trusted.

## Threats and controls

| Threat | Control | Evidence |
|---|---|---|
| Cross-tenant read/write | Tenant comes only from the token. `Principal::authorize` checks tenant before role. Other tenants' records return `not_found`, so ids can't be probed. | `cross_tenant_access_is_denied_even_for_admin`, `api_authentication_and_tenant_isolation` |
| Privilege escalation | Fixed role matrix: viewer < editor < admin, plus auditor. Restore, delete, legal hold, schemas, rules, retention and migrations need admin. | `role_matrix_allowed_and_denied`, `access_control_allows_and_denies_across_roles_and_tenants` |
| Token theft at rest | Only SHA-256 hashes of tokens are kept. Tokens must be at least 16 characters. The UI keeps the token in `sessionStorage` only. | `token_registry_hashes_and_parses` |
| Injection (path, query, CSV, log) | No SQL or shell anywhere. Tenants, domains, entities, fields and rule ids must match `[a-z0-9][a-z0-9_-]{0,63}`. Record ids are strictly `rec_` + 32 hex characters. CSV cells are quoted. Filter fields are validated. | `slugs`, `validation_never_panics`, `api_errors_use_typed_envelopes` |
| Denial of service | 16 MiB body limit, per-token rate limit, page limit of 500, bulk limit of 10,000, schema limit of 256 fields, at most 100 rules and 200 notification rules, compute input limit of 8 MiB, outbox capped at 10,000 per tenant. | `api_metrics_security_headers_and_limits` |
| Secrets in logs | Logs, metrics and audit details carry slugs and error codes only. `metrics::redact` masks `token`, `password`, `secret`, `authorization` and `api_key` values, including `Bearer <x>`. | `redaction`, `observability_counts_operations_without_payloads` |
| Audit tampering | Hash-chained events, verified on every open and via `/api/v1/audit/verify`. | `chain_detects_tampering`, `persistence_survives_restart_and_audit_chain_verifies` |
| Clickjacking / MIME sniffing | `/api` responses send `X-Frame-Options: DENY`, `nosniff`, `CSP default-src 'none'`, `Referrer-Policy: no-referrer` and `Cache-Control: no-store`. | `api_metrics_security_headers_and_limits` |
| SSRF via webhooks | Webhook URLs must be https and are only queued; the engine never makes requests. | `notification_rules_deduplicate_and_audit` |
| Corrupt or partial writes | Journal fsynced per commit, atomic snapshot rename, torn-tail tolerance, and an error on any other corruption. | `journal_compaction_torn_writes_and_metadata_survive_restart` |

## Related documents

The [threat model](../01_governance/threat-model.md) adds data classification, the ASPF integration boundary and STRIDE threats beyond this table. The [vulnerability reporting and dependency update policy](../SECURITY.md) is at the repository root.

## Residual risks

- TLS is not terminated by `apsi-server`. Run it behind a TLS proxy.
- Tokens are static. Rotate them by restarting with a new `APSI_TOKENS`.
- OIDC/SAML federation is defined in the register but not provided by this server; put an authenticating proxy in front if you need it.
- The rate limiter is per process, so multiple replicas each get the full budget.
