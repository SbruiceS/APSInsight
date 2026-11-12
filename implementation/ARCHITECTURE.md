# APSInsight Architecture

## System Overview

APSInsight follows a polyglot modular architecture with clear language responsibilities:

- **TypeScript/React/Vite**: User interface, dashboards, data exploration, maps.
- **Ruby on Rails**: Application APIs, projects, organizations, permissions, workflows, metadata, job orchestration.
- **C++20/23**: High-performance numerical algorithms, sensor fusion kernels, selected geospatial computations.
- **Python**: Data engineering, ML pipelines, scientific libraries, analytical workers.
- **R**: Optional experimental design and statistical inference.
- **PostgreSQL**: Authoritative application metadata.
- **DuckDB + Parquet/GeoParquet**: Analytical queries and columnar scientific data.

## Component Boundaries

See `docs/architecture/component-boundaries.md`.

## Data Flow

1. Ingestion → Quality assessment → Catalog (PostgreSQL metadata + object storage / Parquet)
2. Analytical jobs (Python / C++ / R workers) → Results + Provenance
3. Frontend consumes versioned REST APIs under `/api/v1/`

## Deployment Topology

Docker Compose for local development. Optional services (Keycloak, R runtime, monitoring) can be enabled independently.

## Key Architectural Decisions

Recorded in `docs/decisions/` (ADRs).

## Important Rules

- Rails is the sole application backend.
- Computationally expensive work is never done in synchronous HTTP handlers.
- Optional languages (Rust, CUDA, Fortran, Java, Go) added only with documented justification.
- Scientific integrity: distinguish observations, estimates, simulations, and AI interpretations.
