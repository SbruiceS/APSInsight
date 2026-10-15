# Operations

## Run

```sh
cargo build --release
(cd web && npm ci && npm run build)
APSI_TOKENS="<32+ random chars>:acme:ada:admin,<random>:acme:svc-prom:auditor" \
  ./target/release/apsi-server --bind 0.0.0.0:8080 --data-dir /var/lib/apsinsight --web-dir web/dist/web/browser --log-json
```

`--dev` generates demo tokens for tenant `demo` and prints them to stderr. Without tokens, and without `--dev`, the server refuses to start (exit 2).

| Flag / env | Default | Meaning |
|---|---|---|
| `--bind` / `APSI_BIND` | `127.0.0.1:8080` | listen address |
| `--data-dir` / `APSI_DATA_DIR` | in-memory | persistent state |
| `--tokens` / `APSI_TOKENS` | – | `token:tenant:subject:role[+role]`, comma separated; subjects starting `svc-` are service accounts |
| `--web-dir` / `APSI_WEB_DIR` | – | serve the built UI (falls back to `index.html` for client-side routes) |
| `--rate-limit` / `APSI_RATE_LIMIT` | 600 | requests per token per minute |
| `--log-json` / `APSI_LOG_JSON` | off | JSON log lines |
| `RUST_LOG` | `info` | log filter |

Ctrl+C triggers a graceful shutdown that drains in-flight requests.

## Observability

- **Logs**: one structured line per engine operation (`action`, `tenant`, `actor`, `outcome`) and per HTTP request (request id, method, path). Payloads and credentials are never logged.
- **Metrics** (`GET /metrics`, auditor token): `apsi_operations_total{operation,domain,outcome}` and `apsi_operation_seconds_{count,sum}{operation}`. Requests rejected before they reach the engine (missing or unknown token, rate limit) are counted as `operation="http.auth", domain="-"`. The [SLOs](../20_backend_observability/slo.md), [alert rules](../20_backend_observability/alerts/apsinsight.rules.yml) and [Grafana dashboard](../20_backend_observability/dashboards/apsinsight-overview.json) use only these series.
- **Container**: `docker compose up --build`, with `APSI_TOKENS` exported ([compose.yaml](../compose.yaml)). The image runs as UID 10001 with a read-only root filesystem and keeps state on the `apsinsight-data` volume.
- **Runbooks**: [backup/restore](../20_backend_observability/runbooks/backup-restore.md), [incident response](../20_backend_observability/runbooks/incident-response.md), [deployment rollback](../20_backend_observability/runbooks/deployment-rollback.md).
- **Audit**: `GET /api/v1/audit`, `GET /api/v1/audit/verify`, or `apsi audit list|verify`.

## Backups

Stop the server, or quiesce writes, and copy the data directory: `state.json` plus `journal.ndjson`. The CLI can also open the directory directly, for example `apsi --data-dir ... audit verify`. Don't run the CLI against a data directory while the server is writing to it; there is a single writer per directory.

## Recovery behaviour

| Situation | Behaviour |
|---|---|
| Crash during a journal write | The torn last line is ignored on open; that commit was never acknowledged. |
| Crash during compaction | The old journal replays idempotently over the new snapshot. |
| Corrupt journal (not the tail) or a broken audit chain | The engine refuses to open (exit 9) instead of silently dropping data. |
| Disk full or I/O error on commit | Retryable `storage_unavailable`; memory is reloaded from disk. |
