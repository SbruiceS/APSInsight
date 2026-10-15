# Nityaga RTOS contract v1.0

**Status:** contract implemented and tested; product planned, not yet created. Default adapter: `Unavailable(Product::NityagaRtos)`. It never sends a command, because without the device's declared capabilities a command cannot be checked.

## `DeviceCapability`

`device_id`; `firmware {name, version, sha256}`; `sensors[] {name, unit, min, max, resolution}` (finite, min < max, resolution > 0); `actuators[] {name, safe_state}`; `time_sync {source, max_drift_ms}`.

## `CommandSafetyEnvelope`

Actuator commands are the highest-risk operation in the ecosystem. `validate_for(device)` requires:

| Rule | Why |
|---|---|
| `device_id` matches and `actuator` is declared by the device | no commands to unknown hardware |
| `requires_ack = true` | every actuation is confirmed |
| at least one named `interlocks` entry | the device verifies preconditions (e.g. `pressure_ok`) before acting |
| `max_duration_ms` within 1–3,600,000 | bounded actuation; the device returns to `safe_state` afterwards |
| `issued_at < expires_at ≤ issued_at + 900 s` | stale commands cannot be replayed |
| `issued_by` set | attributable |

## Port

`DevicePort::{capabilities, command}`. Conformance: `conformance::check_device`. The check includes refusing a command without interlocks.

## Tests

`device_command_safety_envelope_rules` (six unsafe variants rejected), plus the shared conformance tests. The broken mock accepts a command without interlocks and is rejected.
