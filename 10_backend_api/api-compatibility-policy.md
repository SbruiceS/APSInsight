# API compatibility policy

The API is versioned by path prefix (`/api/v1`). Within a major version:

**Allowed without a version change:** new routes; new optional request fields and query parameters; new response fields; new error `details` keys; new enum values in fields that are documented as open (`operation` metric labels, compute function names).

**Breaking changes, which need `/api/v2` with v1 kept alongside it:** removing or renaming a route, field or parameter; making an optional field required; changing a field's type, unit or meaning; adding a value to a closed enum (`lifecycle`, error `code`, `actor_kind`); changing an error's HTTP status or retryability; tightening a limit below its documented value.

**Deprecation:** a deprecated route or field is marked in [openapi.v1.yaml](openapi.v1.yaml) and in the CHANGELOG. It keeps working for at least 2 minor releases, or 6 months, whichever is longer, and returns a `Deprecation` header during that time.

**Guards:**

- `openapi_documents_every_route` fails if the router and the spec disagree.
- `api_errors_use_typed_envelopes` fixes the error envelope.
- `api_acceptance_sweep_all_domains` exercises every entity kind through HTTP.

**Units and CRS:** the unit of a field is part of its contract. Converting a stored unit is a schema migration (`scale` + `set_unit`), not a silent change.
