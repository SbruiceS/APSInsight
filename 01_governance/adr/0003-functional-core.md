# ADR-0003: Functional core, effects at the edges, validate before mutate

- Status: Accepted
- Date: 2026-10-15

## Decision

- Science functions (`apsi-science`), validation, schema compatibility, diffing, migration operations and contract validation are pure functions over immutable inputs. They return typed errors and never return NaN or a sentinel value for a missing result.
- Effects (the clock, files, the network, locks) live only in `Engine`, `apsi-server` and `apsi-cli`. The engine takes an injectable clock (`Engine::with_clock`), so tests are deterministic.
- Every engine operation validates fully before it mutates state, then commits in one transaction. A failed operation leaves nothing behind except its audit event.
- Parsing and validation are property-tested for totality: they never panic on arbitrary input.

## Consequences

- Rollback does not need state snapshots, which keeps writes O(change) instead of O(state) ([docs/PERFORMANCE.md](../../docs/PERFORMANCE.md)).
- There is no hidden global mutable state in domain logic. Metrics are the only shared mutable state, and they sit outside the domain.
