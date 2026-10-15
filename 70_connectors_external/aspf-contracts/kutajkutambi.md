# KutajKutambi AI contract v1.0

**Status:** contract implemented and tested; product planned, not yet created. Default adapter: `Unavailable(Product::KutajkutambiAi)`.

## `InferenceRequest`

| Field | Rules |
|---|---|
| `tenant` | 1–64 bytes |
| `task` | `classification`, `forecast`, `interpretation`, `recommendation` or `agent_task` |
| `instruction` | 1 byte–32 KiB |
| `evidence[]` `{record_id, version, content_sha256}` | at least one; pinned to a record version and its content hash |
| `tools[]` `{tool, scope}` | `scope` is `read` or `propose`. **There is no write scope**, so an agent cannot change records autonomously |
| `deadline_ms` | 1–600,000 |

## `InferenceResponse`

| Field | Rules |
|---|---|
| `kind` | `prediction`, `ai_interpretation` or `recommendation`. **There is no `observation` kind**, so outputs can never be presented as observations |
| `model` `{name, version, evaluation_ref}` | all required; `evaluation_ref` points to a [model card](../../59_ml_platform/model-card-template.md) |
| `uncertainty` `{method, confidence?, interval?}` | method required; confidence within [0, 1]; interval finite with lo ≤ hi |
| `evidence_used[]` | a subset of the request evidence; must not be empty unless `status` is `refused` |
| `requires_human_approval` | must be `true` whenever the task or the output is a recommendation |
| `status` | `completed`, `partial` or `refused` |

## Port

`InferencePort::infer`. Conformance: `conformance::check_inference`.

## Tests

`inference_contract_rules`, plus the shared conformance tests. The broken mock omits human approval for a recommendation and is rejected.
