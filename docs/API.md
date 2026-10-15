# HTTP API (`/api/v1`)

Every route except `/health` requires `Authorization: Bearer <token>`. The tenant always comes from the token, never from the request.

## Envelopes

```json
{ "data": { ... }, "meta": { "api_version": "v1" } }
{ "error": { "code": "version_conflict", "message": "...", "retryable": true, "details": { "expected": 1, "current": 2 } }, "meta": { "api_version": "v1" } }
```

Exports return the raw file with `Content-Type` and `Content-Disposition` headers instead of an envelope. Every response carries `x-request-id`; a valid one you send is echoed back.

## Errors

| code | HTTP | retryable |
|---|---|---|
| `invalid_input` | 400 | no |
| `unauthenticated` | 401 | no |
| `forbidden` | 403 | no |
| `not_found` | 404 | no |
| `version_conflict` | 409 | yes |
| `conflict` | 409 | no |
| `legal_hold` | 409 | no |
| `payload_too_large` | 413 | no |
| `validation_failed` | 422 | no (`details` holds the full validation report) |
| `lifecycle_violation` | 422 | no |
| `rate_limited` | 429 | yes (`Retry-After: 60`) |
| `internal` | 500 | no |
| `storage_unavailable` | 503 | yes |

## Limits

- Request bodies: at most 16 MiB.
- Search page size: at most 500.
- Bulk requests: at most 10,000 items.
- Rate limit: 600 requests per token per minute by default (`--rate-limit`).

## Routes

The machine-readable contract is [10_backend_api/openapi.v1.yaml](../10_backend_api/openapi.v1.yaml). The test `openapi_documents_every_route` keeps it in step with the router. Compatibility rules are in the [API compatibility policy](../10_backend_api/api-compatibility-policy.md).

Throughout, `K` = `/api/v1/domains/{domain}/entities/{entity}`.

| Method | Path | Purpose |
|---|---|---|
| GET | `/health` | liveness |
| GET | `/metrics` | Prometheus metrics (auditor/admin) |
| GET | `/api/v1/me` | current principal |
| GET | `/api/v1/catalog` | domains, capabilities, entity kinds |
| GET | `/api/v1/features?domain=&capability=&q=&offset=&limit=` | feature register |
| GET | `/api/v1/features/{id}` | feature + evidence |
| GET | `/api/v1/traceability` | CSV matrix of all 3,100 features |
| GET | `/api/v1/overview` | records per domain, audit status |
| GET | `/api/v1/domains/{domain}/dashboard` | per-entity summaries, activity |
| POST | `/api/v1/domains/{domain}/search` | `Query` body (filters, facets, sort, paging) |
| GET / POST | `K/records` | list (`q, lifecycle, tags, facets, include_archived, sort, desc, offset, limit`) / create |
| POST | `K/validate` | validation report without saving |
| POST | `K/bulk` | `{items, dry_run, upsert, idempotency_key, mapping}` |
| POST | `K/import?format=csv\|json\|ndjson&dry_run=&upsert=&idempotency_key=` | raw body |
| GET | `K/export?format=json\|ndjson\|csv\|geojson&include_archived=` | file |
| GET / POST | `K/schemas` (`?allow_breaking=`) | versions / publish next version |
| GET / PUT | `K/rules` | validation rules |
| GET | `K/quality` | quality indicators |
| GET / PUT | `K/retention` | retention policy |
| POST | `K/migrations?dry_run=` | `{to_version, ops}` |
| GET / PATCH / DELETE | `/api/v1/records/{id}` | read (`ETag`) / update (`If-Match` or `expected_version`) / delete |
| POST | `/api/v1/records/{id}/lifecycle` | `{to, expected_version}` |
| GET | `/api/v1/records/{id}/history` | versions |
| GET | `/api/v1/records/{id}/diff?from=&to=` | field changes |
| POST | `/api/v1/records/{id}/rollback` | `{to_version, expected_version}` |
| PUT | `/api/v1/records/{id}/legal-hold` | `{hold}` |
| POST | `/api/v1/retention/apply?dry_run=` | run retention |
| POST | `/api/v1/migrations/{id}/rollback` | undo a migration |
| GET / POST / DELETE | `/api/v1/notification-rules[/{id}]` | rules |
| GET | `/api/v1/notifications` | outbox |
| GET / POST | `/api/v1/workflows` | list / register |
| POST | `/api/v1/workflows/{id}/runs?version=` | run |
| GET | `/api/v1/workflow-runs` | runs |
| GET | `/api/v1/status` | honest completion metrics: feature counts per traceability status, overall and per domain |
| GET | `/api/v1/integrations` | capability discovery for planned ASPF products (all optional; unavailable in this build) |
| GET | `/api/v1/compute` | science functions |
| POST | `/api/v1/compute/{function}` | run one (JSON in, JSON out) |
| GET | `/api/v1/audit?limit=&action=` | audit events (auditor/admin) |
| GET | `/api/v1/audit/verify` | hash-chain check |

## Schemas

The default v1 schema for every entity kind declares these optional attributes: `description`, `unit`, `value` (number), `observed_at` (RFC 3339) and `location` (`{lat, lon}`, WGS 84). Other attributes are accepted and preserved. Publish v2+ with `POST K/schemas`. Compatible changes (new optional fields, relaxed limits) need nothing more. Breaking changes (removed or retyped fields, new required fields, narrowed ranges, changed units) need `allow_breaking=true` and usually a migration.

## Validation

Rule kinds are `required`, `range`, `max_length`, `one_of`, `requires_unit` and `min_tags`. Each rule has a severity of `info`, `warning` or `error`; only `error` blocks a write. Reports list every issue with its rule id, field, code and message.

## Search

Filters have the form `{field, op, value}`, where `op` is one of `eq ne gt gte lt lte contains in exists`. `eq` on a scalar attribute uses the exact-match index. Facets may be any attribute, or `lifecycle`, `tags` or `entity`. Pagination uses `offset`/`limit`, and `next_offset` is present when more results exist.

## Bulk, import and export

Bulk import reports results per item, so valid items are applied even when others fail. `dry_run` previews without saving. `idempotency_key` with the same payload replays the stored report; the same key with a different payload returns `409 conflict`. `mapping` supports `rename`, `defaults` and `drop`.

Export headers carry the format and its version, the encoding (`utf-8`), the export time, the schema version and a provenance flag. CSV is RFC 4180, with JSON-encoded `attributes` and `provenance` cells, so a CSV round trip is lossless.

## Notifications

Events are `created`, `updated`, `archived`, `restored`, `deleted` and `validation_warning`. A rule can add an optional attribute condition and sets a dedup window (default 300 s). Matching notifications go to the outbox, and both deliveries and suppressions are audited. Webhook channels must use https and are queued for a delivery worker; the engine itself makes no network calls.
