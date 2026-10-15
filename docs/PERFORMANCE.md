# Performance

Performance claims are made only from measurements. Reproduce them with:

```sh
cargo build --release -p apsi-cli
./target/release/apsi bench --records 20000             # in-memory engine
./target/release/apsi bench --records 2000 --persistent # journal + fsync per commit
```

## Workload

- A single thread.
- One entity kind (`crop-yield-and-production/yield-observations`).
- Two attributes per record (`value`, `crop` in three categories).
- The default schema with no extra rules.
- `indexed_search` is an `eq` filter on `crop` (about a third of records match), page size 50.
- `export_csv` exports every record.

## Results

Measured on the development machine (Windows 11, release build) on 2026-10-15. Treat these as indicative only: they will differ on other hardware, and network and HTTP overhead are not included.

| storage | records | create/s | get/s | indexed search/s | CSV export |
|---|---|---|---|---|---|
| in-memory | 20,000 | ~103,000 | ~990,000 | ~324 | 0.076 s |
| persistent (fsync per commit) | 2,000 | ~1,230 | ~560,000 | ~5,100 | 0.012 s |

Persistent writes are bound by fsync latency. Batch writes through `bulk`, which is one transaction and one fsync per request, when ingesting large volumes.

## Regression found by measuring

The first implementation cloned the whole state per transaction (for rollback) and rewrote a full snapshot per commit. `apsi bench --records 20000` measured **31.7 creates/s**, which is quadratic. The engine now validates before mutating and appends change-only journal lines, giving the numbers above. A search regression from cloning every hit before pagination was fixed the same way, from 69/s to 324/s.

## Methodology

Rules for any performance claim in this repository:

1. **Reproducible command.** Every number comes from `apsi bench` (or a future harness in the same style) run on a release build. Record the exact command.
2. **Environment.** Record the hardware (CPU model, cores, RAM, storage type), OS, Rust version and the date. Numbers from different machines are not compared.
3. **Workload.** Record the dataset size, entity kind, attribute count, filter selectivity and page size, as in the Workload section above.
4. **Statistics.** `apsi bench` reports throughput over one run. Before a number is used as a target, run it at least 5 times and report the median and the min–max range. Latency percentiles (p50/p95/p99) need per-operation timing, which the harness does not collect yet. **No percentile claim is made anywhere in this repository.**
5. **Scope.** In-process engine numbers exclude HTTP, TLS and network costs. An HTTP load test (for example `oha` or `k6` against a running server) is required before claiming API throughput.
6. **Regressions.** A change that alters a hot path (transaction, journal, search, export) must re-run the affected benchmark and update this file if the result moves by more than 10%. CI does not gate on performance yet, because a shared CI runner is too noisy to give a stable baseline.

## Known limits

- One writer lock per engine: throughput does not scale with cores for writes.
- Search sorts all matching records before paginating, so it is O(m log m) in the number of matches. Use `eq` filters on indexed attributes to narrow `m`.
- All records are held in memory. Size the host for the dataset (roughly 1–2 KB per record plus its history).
