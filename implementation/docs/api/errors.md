# Error Responses

All error responses are JSON.

## Status codes

| Code | Meaning | Typical body |
|------|---------|--------------|
| 400 | Bad request | `{ "error": "param is missing or the value is empty: ..." }` |
| 404 | Not found | `{ "error": "Not found" }` |
| 422 | Validation failed | `{ "error": "...", "details": ["Name can't be blank", ...] }` or `{ "errors": ["..."] }` |
| 503 | External service unavailable | `{ "error": "...", "capabilities": { ... } }` (AgriQL / AgriCUDa) |

## Examples

Validation failure (422):
```json
{
  "errors": ["Slug has already been taken", "Name can't be blank"]
}
```

AgriCUDa unavailable (503):
```json
{
  "error": "agricuda binary not available",
  "capabilities": {
    "available": false,
    "not_implemented": ["GPU backend", "FPGA backend"]
  }
}
```
