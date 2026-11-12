# APSInsight API v1 — Endpoint Reference

Base path: `/api/v1`

All responses are JSON. Errors follow the shape `{ "error": "...", "details": [...] }` with appropriate HTTP status codes (400, 404, 422, 503).

---

## Health

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health/live` | Liveness. Returns `{ "status": "ok", "timestamp": "..." }` |
| GET | `/health/ready` | Readiness. Returns `{ "status": "ready", ... }` |
| GET | `/capabilities` | Platform capabilities summary |

---

## Organizations

| Method | Path | Body | Description |
|--------|------|------|-------------|
| GET | `/organizations` | — | List all organizations |
| GET | `/organizations/:id` | — | Show one organization |
| POST | `/organizations` | `{ "organization": { "name", "slug", "description" } }` | Create |
| PATCH | `/organizations/:id` | `{ "organization": { ... } }` | Update |

---

## Projects

| Method | Path | Body | Description |
|--------|------|------|-------------|
| GET | `/projects` | — | List (includes organization) |
| GET | `/projects/:id` | — | Show |
| POST | `/projects` | `{ "project": { "organization_id", "name", "slug", "description" } }` | Create |
| PATCH | `/projects/:id` | `{ "project": { ... } }` | Update |

---

## Datasets

| Method | Path | Body | Description |
|--------|------|------|-------------|
| GET | `/datasets` | — | List (includes project, organization) |
| GET | `/datasets/:id` | — | Show with versions |
| POST | `/datasets` | `{ "dataset": { "project_id", "organization_id", "name", "slug", "description", "status", "metadata" } }` | Create |
| PATCH | `/datasets/:id` | `{ "dataset": { ... } }` | Update |
| GET | `/datasets/:id/schema` | — | Current schema snapshot + DatasetSchema with columns |
| GET | `/datasets/:id/preview` | — | Preview placeholder |
| GET | `/datasets/:id/analytics/soil_summary` | — | Soil analytics summary with assumptions |

---

## Imports

| Method | Path | Body | Description |
|--------|------|------|-------------|
| POST | `/imports` | `{ "import": { "dataset_id", "project_id", "source_type", "original_filename" } }` | Enqueue import (returns 202) |
| GET | `/imports/:id` | — | Import status and diagnostics |

---

## Identity

### Users
| Method | Path | Body |
|--------|------|------|
| GET | `/users` | — |
| GET | `/users/:id` | — (includes memberships) |
| POST | `/users` | `{ "user": { "email", "name", "external_id" } }` |
| PATCH | `/users/:id` | `{ "user": { ... } }` |

### Roles
| Method | Path | Body |
|--------|------|------|
| GET | `/roles` | — |
| GET | `/roles/:id` | — |
| POST | `/roles` | `{ "role": { "name", "key", "description" } }` |

### Memberships
| Method | Path | Body |
|--------|------|------|
| GET | `/memberships` | — (includes user, organization, project, role) |
| GET | `/memberships/:id` | — |
| POST | `/memberships` | `{ "membership": { "user_id", "organization_id", "project_id", "role_id" } }` |
| DELETE | `/memberships/:id` | — (204) |

---

## Agricultural Domain

### Farms
| Method | Path | Body |
|--------|------|------|
| GET | `/farms` | — |
| GET | `/farms/:id` | — (includes fields) |
| POST | `/farms` | `{ "farm": { "organization_id", "project_id", "name", "slug", "description", "area_hectares", "location" } }` |
| PATCH | `/farms/:id` | `{ "farm": { ... } }` |

### Fields
| Method | Path | Body |
|--------|------|------|
| GET | `/fields` | — |
| GET | `/fields/:id` | — (includes plots) |
| POST | `/fields` | `{ "field": { "farm_id", "name", "area_hectares", "boundary" } }` |

### Plots
| Method | Path | Body |
|--------|------|------|
| GET | `/plots` | — |
| GET | `/plots/:id` | — |
| POST | `/plots` | `{ "plot": { "field_id", "name", "area_hectares" } }` |

### Soil Samples
| Method | Path | Body |
|--------|------|------|
| GET | `/soil_samples` | — |
| GET | `/soil_samples/:id` | — (includes observations) |
| POST | `/soil_samples` | `{ "soil_sample": { "field_id", "plot_id", "dataset_id", "sampled_at", "depth_cm", "sample_code", "latitude", "longitude", "metadata" } }` |

### Observations
| Method | Path | Body |
|--------|------|------|
| GET | `/observations` | — (limited to 500) |
| GET | `/observations/:id` | — |
| POST | `/observations` | `{ "observation": { "soil_sample_id", "dataset_id", "variable", "value", "unit", "observed_at", "quality_flag", "methodology", "uncertainty" } }` |

---

## Schema & Quality

### Dataset Schemas
| Method | Path | Body |
|--------|------|------|
| GET | `/dataset_schemas` | — (includes columns) |
| GET | `/dataset_schemas/:id` | — |
| POST | `/dataset_schemas` | `{ "dataset_schema": { "dataset_id", "version", "raw_schema" } }` |

### Data Quality Reports
| Method | Path | Body |
|--------|------|------|
| GET | `/data_quality_reports` | — |
| GET | `/data_quality_reports/:id` | — |
| POST | `/data_quality_reports` | `{ "data_quality_report": { "dataset_id", "dataset_version_id", "import_id", "status", "row_count", "null_count", "summary", "issues" } }` |

---

## AgriQL (Query Language)

| Method | Path | Body | Description |
|--------|------|------|-------------|
| GET | `/agriql/capabilities` | — | Engine availability and supported grammar |
| POST | `/agriql/query` | `{ "sql": "...", "dataset_id": null, "project_id": null }` | Execute query; returns `{ "query_run_id", "result" }` |
| GET | `/agriql/schema` | — | Schema discovery via `information_schema` |

Supported real AgriQL grammar (engine 3.10.1): `SHOW VERSION/GRAMMAR/SCHEMAS/CATALOG`, `CREATE SCHEMA/TABLE`, `IMPORT CSV`, `SELECT` with aggregates, `information_schema`, unit conversion, `PROFILE`.

---

## AgriCUDa (Kernels)

| Method | Path | Body | Description |
|--------|------|------|-------------|
| GET | `/agricuda/capabilities` | — | Binary availability, features, not-implemented list |
| POST | `/agricuda/run` | `{ "source": "kernel fn ..." }` | Run AgriCUDa program (CPU device) |
| POST | `/agricuda/check` | `{ "source": "..." }` | Type-check only |

GPU and FPGA backends are not implemented. When the binary is absent the response is an explicit test double with no fabricated numeric results.

---

## Audit

Significant create/destroy/import/query/kernel actions write an `AuditEvent` (action, resource_type, resource_id, metadata).

---

## Notes

- Authentication is not yet enforced (OpenID Connect deferred).
- Pagination is not yet implemented on list endpoints (except observations limited to 500).
- All timestamps are UTC ISO-8601.
- Nested resources use Rails-style wrapping (`{ "model_name": { ... } }`).
