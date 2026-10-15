# Architecture decision records

Each ADR records one decision: its context, the decision itself and its consequences. ADRs are immutable once accepted. To change a decision, write a new ADR that supersedes the old one.

| # | Decision | Status |
|---|---|---|
| [0001](0001-modular-monolith-generic-engine.md) | Modular monolith with one generic capability engine | Accepted |
| [0002](0002-embedded-storage.md) | Embedded journal storage now; PostgreSQL/PostGIS behind a port later | Accepted |
| [0003](0003-functional-core.md) | Functional core, effects at the edges, validate before mutate | Accepted |
| [0004](0004-authentication.md) | Static hashed bearer tokens; OIDC through a proxy | Accepted |
| [0005](0005-future-aspf-products-as-contracts.md) | Future ASPF products are optional, versioned contracts | Accepted |
| [0006](0006-language-boundaries.md) | Rust and TypeScript only until a benchmark justifies C++, Python or formal methods | Accepted |
| [0007](0007-honest-feature-status.md) | Four-value feature status, derived from code and tests | Accepted |
| [0008](0008-units-and-crs.md) | Quantities carry units and dimensions; geometries carry a CRS | Accepted |
