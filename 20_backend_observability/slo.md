# Service level objectives

These are targets to measure against, not results. No production traffic exists yet. The metrics they use are the ones `apsi-server` actually exports.

| SLI | Definition (PromQL over `/metrics`) | Objective | Window |
|---|---|---|---|
| Availability | `up{job="apsinsight"}` | 99.5% | 30 days |
| Operation success | `1 - sum(rate(apsi_operations_total{outcome=~"storage_unavailable\|internal"}[5m])) / sum(rate(apsi_operations_total[5m]))` | 99.9% | 30 days |
| Mean operation latency | `sum(rate(apsi_operation_seconds_sum[5m])) / sum(rate(apsi_operation_seconds_count[5m]))` | < 50 ms | 5 minutes |
| Audit integrity | `GET /api/v1/audit/verify` returns `valid: true` (probe) | 100% | continuous |
| Backup restorability | latest restore drill succeeded ([runbook](runbooks/backup-restore.md)) | ≤ 30 days old | monthly |

Client errors (`invalid_input`, `validation_failed`, `forbidden`, `not_found`, `version_conflict`, `rate_limited`, and so on) are excluded from the success SLI, because they reflect requests rather than the service.

## Known limitations

- The server exports a summary (count and sum), not a histogram, so **percentile latency cannot be computed yet**. Adding histogram buckets is a prerequisite for a p95/p99 SLO.
- There is no queue-depth metric, because there is no job queue. The notification outbox size is not exported.
- Prometheus and Grafana are not part of this repository's tests. The [alert rules](alerts/apsinsight.rules.yml) and [dashboard](dashboards/apsinsight-overview.json) are definitions and have not been exercised against a running stack.
