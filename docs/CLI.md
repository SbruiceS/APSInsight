# CLI (`apsi`)

Global flags (each also has an environment variable):

- `--data-dir` (`APSI_DATA_DIR`, default `.apsi-data`)
- `--tenant` (`APSI_TENANT`)
- `--actor` (`APSI_ACTOR`)
- `--role` (`APSI_ROLE`, default `admin`, because the local operator owns the data directory)
- `--pretty`

The CLI is non-interactive and never prompts. JSON inputs accept literal JSON, `@file`, or `-` for stdin.

## Output

The CLI prints one JSON document to stdout: `{"ok":true,"data":...}` on success, or `{"ok":false,"error":{code,message,retryable,details?}}` on failure. Exports write the raw file to `--out` or to stdout.

## Exit codes

| code | meaning |
|---|---|
| 0 | ok |
| 2 | invalid input / usage |
| 3 | validation failed |
| 4 | not found |
| 5 | conflict (version, duplicate, idempotency, legal hold) |
| 6 | lifecycle violation |
| 7 | unauthenticated / forbidden |
| 8 | limits (rate, size) |
| 9 | storage |
| 10 | internal |

## Commands

```
apsi features list [--domain D] [--capability C] [--q TEXT]
apsi features show APSI-F0001
apsi features trace [--format csv|json] [--out FILE]
apsi catalog [DOMAIN]
apsi records create D E --json '{"name":"...","attributes":{...}}'
apsi records get|history|delete ID
apsi records update ID --expected-version N --json '{"attributes":{...}}'
apsi records archive|restore ID --expected-version N
apsi records hold ID [--release]
apsi records search D [--entity E] [--q T] [--tag X]... [--filter field:op:value]... [--facet F]... [--limit N --offset N]
apsi records validate D E --json ...
apsi records diff ID --from 1 --to 2
apsi records rollback ID --to-version 1 --expected-version 3
apsi records bulk D E --file items.json [--dry-run] [--upsert] [--idempotency-key K]
apsi records import D E --file data.csv [--format csv|json|ndjson] [--dry-run] [--upsert]
apsi records export D E [--format json|ndjson|csv|geojson] [--out FILE] [--include-archived]
apsi schema show|publish D E [--file schema.json --allow-breaking]
apsi rules show|set D E [--file rules.json]
apsi quality D E
apsi retention show|set D E [--archive-after-days N --purge-after-days N]
apsi retention apply [--dry-run]
apsi migrate D E --file plan.json [--dry-run]
apsi migrate-rollback MIGRATION_ID
apsi notify add --json ... | list | remove ID | outbox
apsi workflow register --file wf.json | list | run ID [--version N] | runs
apsi compute list
apsi compute FUNCTION --json '{...}'
apsi features status
apsi integrations
apsi dashboard [DOMAIN]
apsi audit list [--limit N --action PREFIX] | verify
apsi docs generate [--out docs]
apsi bench [--records N] [--persistent]
```
