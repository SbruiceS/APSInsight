# ADR-0002: Embedded journal storage now; PostgreSQL/PostGIS behind a port later

- Status: Accepted
- Date: 2026-10-15

## Context

The master prompt names PostgreSQL, PostGIS, object storage, Arrow/Parquet and DuckDB. None of them is provisioned in the build environment, and a build that needs a database server to run its tests would not be self-verifying.

## Decision

- `Engine::open(dir)` persists to an fsynced append-only journal (`journal.ndjson`) and an atomically replaced snapshot (`state.json`). See the Persistence section of [docs/ARCHITECTURE.md](../../docs/ARCHITECTURE.md).
- The PostgreSQL and PostGIS target schemas are written as migrations in [32_storage_postgres](../../32_storage_postgres/README.md) and [33_storage_postgis](../../33_storage_postgis/README.md), with row-level tenant isolation. They are marked **specified only** and are not executed by any test.
- Object storage, Arrow/Parquet and DuckDB are not implemented. Their domains are recorded as blocked by an external dependency.

## Consequences

- The whole system runs and is tested without external services. Crash safety (torn writes, compaction, tamper detection) is tested.
- All records are held in memory and there is one writer per data directory ([docs/PERFORMANCE.md](../../docs/PERFORMANCE.md)). This limits the data size and write concurrency.
- Moving to PostgreSQL needs a storage port trait in the engine and an adapter in `14_backend_adapters`. The record envelope already matches the target schema.
