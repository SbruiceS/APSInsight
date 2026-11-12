# AgriQL as APSInsight Database Query Language

AgriQL (SbruiceS/AgriQL, engine 3.10.1, grammar 1.0) is the agricultural database query language used by APSInsight.

## Real Capabilities (from AgriQL source & docs)

- Connection: `:memory:`, file path (`.aqdb`), or `agriql://host:5471`
- Meta: `SHOW VERSION;`, `SHOW GRAMMAR;`, `SHOW SCHEMAS;`, `SHOW CATALOG;`
- DDL: `CREATE SCHEMA`, `CREATE TABLE` (types include `UUID`, `GEOPOINT(4326)`, `FLOAT64`, …)
- Import: `IMPORT CSV 'path' INTO schema.table;`
- Query: `SELECT` with `AVG`, `COUNT`, `MIN`, `MAX`, `STDDEV`, `WHERE`, `GROUP BY`
- Catalog: `information_schema.columns`
- Other: unit `CONVERT`, `PROFILE`, spatial types

Results are returned as TSV text. The official Python client (`agriql.connect` → `.query().to_records()`) converts them to records.

## APSInsight Implementation

| Component | Location |
|-----------|----------|
| Rails adapter | `app/services/agriql_adapter.rb` |
| Query provenance | `QueryRun` model + migration |
| API | `POST /api/v1/agriql/query`, `GET /api/v1/agriql/capabilities`, `GET /api/v1/agriql/schema` |
| Python client | `python/src/apsinsight/integrations/agriql_client.py` |
| Frontend workspace | `/agriql` page |

When the AgriQL Python package / engine is not present the adapter uses an explicit test double that returns the same result shape and labels itself as a test double. No query syntax or results are invented.

## Installation for real execution

```bash
# From AgriQL repo
cargo build -p agriql-ffi
pip install git+https://github.com/SbruiceS/agriql-python.git
# or install the local bindings/python
```

Set `AGRIQL_TARGET=:memory:` (or a file / agriql:// URL).
