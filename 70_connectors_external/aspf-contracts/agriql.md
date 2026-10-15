# AgriQL contract v1.0

**Status:** contract implemented and tested; product planned, not yet created. Default adapter: `Unavailable(Product::Agriql)`.

## `QuerySubmission`

| Field | Type | Rules |
|---|---|---|
| `tenant` | string | 1–64 bytes |
| `query` | string | 1 byte–64 KiB. Values are never interpolated into the text; they travel in `parameters` |
| `language_version` | string | 1–16 bytes |
| `parameters` | map name → `{type: text\|number\|boolean\|timestamp, value, unit?}` | names `[A-Za-z0-9_]+`; numbers are finite and **must** have a unit; timestamps are RFC 3339; text ≤ 4 KiB |
| `spatial` | `{crs, bbox: [min_x, min_y, max_x, max_y]}` (optional) | CRS checked; bbox finite with min ≤ max |
| `max_rows` | integer | 1–1,000,000 |
| `timeout_ms` | integer | 1–300,000 |

## `QueryResult`

| Field | Rules |
|---|---|
| `columns[]` `{name, type, unit?, crs?}` | at least one; `quantity` columns need a unit and only they may have one; `geometry` columns need a CRS |
| `rows[][]` | each row is as wide as the column list; `null` means missing; no more than `max_rows` rows |
| `truncated` | true when rows were cut at `max_rows` |
| `provenance` `{query_sha256, engine_version, dataset_versions, executed_at}` | `query_sha256` must equal the SHA-256 of the submitted query (`agriql::query_sha256`); `dataset_versions` pins every dataset read |

## Port

`AgriQlPort::submit(&QuerySubmission) -> Result<QueryResult, ContractError>`. Conformance: `conformance::check_agriql`.

## Tests

`query_contract_rules`, `unavailable_adapters_pass_conformance_and_degrade_gracefully`, `reference_mocks_pass_conformance`, `broken_mocks_fail_conformance` (the broken mock is a quantity column without a unit).
