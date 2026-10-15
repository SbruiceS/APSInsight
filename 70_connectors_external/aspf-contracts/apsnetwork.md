# APSNetwork contract v1.0

**Status:** contract implemented and tested; product planned, not yet created. Default adapter: `Unavailable(Product::Apsnetwork)`.

## `TelemetryEnvelope` (inbound)

| Field | Rules |
|---|---|
| `contract` | must satisfy the consumer's version, otherwise `incompatible` |
| `device_id`, `sequence` | sequence is monotonic per device (used for deduplication and gap detection) |
| `event_time`, `received_time` | RFC 3339. An event time more than 300 s after the receive time is rejected (unsynchronised clock); `lateness_s()` reports late arrivals |
| `measurements` | name → `Quantity {value, unit, quality: good\|suspect\|estimated\|missing, uncertainty?}`. A null value requires `missing`; `missing` requires a null value; uncertainty must be ≥ 0 |
| `link` `{rssi_dbm, snr_db, packet_loss_ratio}` | finite; loss ratio within [0, 1] |

## `EdgeJob` (outbound) and `CommandAck`

`EdgeJob {job_id, target_device, kind, payload, expires_at}`; `CommandAck {command_id, status: accepted|rejected|executed|timed_out|queued, at, detail?}`. The ack must reference the delivered job.

## Port

`EdgeNetworkPort::deliver`. Conformance: `conformance::check_edge`.

## Tests

`telemetry_contract_rules`, plus the shared conformance tests. The broken mock acknowledges a different job id and is rejected.
