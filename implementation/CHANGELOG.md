# Changelog

## [Unreleased]

### Added
- Background job: `ImportProcessingJob` (Sidekiq) that processes imports and records provenance.
- Object storage abstraction (`ObjectStorage`) with local filesystem backend.
- Authorization policies (`ApplicationPolicy`, `DatasetPolicy`, `ProjectPolicy`) using Pundit.
- First agricultural analytics module: `SoilAnalyticsService` + `GET /api/v1/datasets/:id/analytics/soil_summary`.
- API endpoint reference completed in `docs/api/endpoints.md` (all routes, request bodies, AgriQL and AgriCUDa).
- Restored missing AgriQL routes in `config/routes.rb`.
- Filled `docs/api/errors.md`, `pagination.md`, and `versioning.md`.
  - Query CLI page (`/computing/query`) — terminal-style AgriQL interface
  - Kernel Programming page (`/computing/kernel`) — AgriCUDa editor + run/check
  - Nested computing layout and navigation entry
  - Frontend pages: Farms, Soil (samples + observations), Integrations (AgriQL + AgriCUDa status)
  - Expanded API client
  - ImportProcessingJob now creates a DataQualityReport
  - Core model specs added (Organization, Dataset, Observation)
  - `AgricudaAdapter` that calls the real `agricuda` CLI (`run` / `check`) when present.
  - Endpoints: `/api/v1/agricuda/capabilities`, `/run`, `/check`.
  - Python client wrapper.
  - Explicit test double when binary is absent (no fabricated kernel results).
  - GPU/FPGA backends correctly reported as not implemented.
  - Identity: Users, Roles, Memberships
  - Agricultural domain: Farms, Fields, Plots, SoilSamples, Observations
  - Schema & quality: DatasetSchemas, DataQualityReports
  - Expanded update actions on Organizations, Projects, Datasets
  - Routes updated under /api/v1/
  - Identity: User, Role, Membership
  - Agricultural domain: Farm, Field, Plot, SoilSample, Observation
  - Schema & quality: DatasetSchema, DatasetColumn, DataQualityReport
  - Associations updated on Organization, Project, Dataset
  - Seeds extended; data model overview documented.
  - `AgriqlAdapter` that executes real AgriQL via the official Python client (or explicit test double).
  - `QueryRun` model for provenance (SQL, status, result, timestamps).
  - Endpoints: capabilities, query, schema discovery.
  - Frontend AgriQL Query Workspace at `/agriql`.
  - Supported real grammar subset documented from AgriQL source.
- Gemfile updated with Pundit.
