# APSInsight Core Data Model

## Identity & Access
- **User** — platform user (email, name, external_id for OIDC)
- **Role** — named role with stable key (org_admin, researcher, …)
- **Membership** — links User to Organization (and optionally Project) with a Role

## Catalog
- **Organization** → **Project** → **Dataset** → **DatasetVersion**
- **DatasetSchema** + **DatasetColumn** — typed schema per dataset version
- **Import** — file/source import job
- **DataQualityReport** — quality summary linked to Dataset / DatasetVersion / Import

## Agricultural Domain
- **Farm** (belongs to Organization, optional Project)
- **Field** (belongs to Farm)
- **Plot** (belongs to Field)
- **SoilSample** (optional Field / Plot / Dataset; sampled_at, depth, location)
- **Observation** (typed measurement: variable, value, unit, observed_at, quality_flag, methodology, uncertainty)

## Analytics & Provenance
- **QueryRun** — AgriQL or analytical query with result and provenance
- **AuditEvent** — append-only audit log

## Relationships (simplified)
Organization
  ├── Memberships → Users + Roles
  ├── Projects
  ├── Farms → Fields → Plots
  └── Datasets
        ├── DatasetVersions
        ├── DatasetSchemas → DatasetColumns
        ├── Imports
        ├── DataQualityReports
        ├── SoilSamples → Observations
        └── QueryRuns

All models include timestamps. Important domain fields use typed columns; flexible metadata uses JSONB only where needed.
