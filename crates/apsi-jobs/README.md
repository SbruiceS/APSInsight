# apsi-jobs

Pure, deterministic job-lifecycle primitives for future APSInsight worker and queue adapters.

The crate owns job validation, tenant-safe leasing, cancellation, retry backoff and expired-lease recovery. It deliberately has no clock, threads, network calls, database driver or server dependency: callers supply time and persist state through a future adapter.

This is not yet a deployed queue. It is not wired to the HTTP API, durable storage or a worker process, so it does not change feature-register status. Its contract is covered by unit tests and is intended to be reused by PostgreSQL, in-memory development and external-queue adapters.

## Safety properties

- Every job and worker identifier is validated as a safe slug.
- A worker can only claim jobs for explicitly authorized tenants.
- A lease is owned by one worker, expires deterministically and cannot be completed by another worker.
- Cancellation is tenant-scoped and cannot modify another tenant's job.
- Payloads are opaque and never logged by this crate; callers must avoid placing credentials in them.
