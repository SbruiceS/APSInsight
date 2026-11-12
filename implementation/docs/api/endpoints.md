# APSInsight API v1 — Endpoint Reference

Base path: `/api/v1`

See the full reference committed alongside this file. Summary of groups:

- Health: GET /health/live, /health/ready, /capabilities
- Organizations, Projects, Datasets (incl. schema, preview, analytics/soil_summary)
- Imports
- Users, Roles, Memberships
- Farms, Fields, Plots, Soil Samples, Observations
- Dataset Schemas, Data Quality Reports
- AgriQL: GET /agriql/capabilities, POST /agriql/query, GET /agriql/schema
- AgriCUDa: GET /agricuda/capabilities, POST /agricuda/run, POST /agricuda/check

All responses JSON. Errors: 400/404/422/503. Nested bodies use Rails-style wrapping.
Authentication (OIDC) deferred. Pagination not yet implemented (observations limited to 500).
