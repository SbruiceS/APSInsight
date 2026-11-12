"""Full AgriQL client for APSInsight workers.

Uses the official agriql package from SbruiceS/AgriQL (bindings/python).
Supported operations match the real engine (v3.10.1 / grammar 1.0):
  SHOW VERSION / GRAMMAR / SCHEMAS / CATALOG
  CREATE SCHEMA / CREATE TABLE
  IMPORT CSV 'path' INTO schema.table
  SELECT ... AVG/COUNT/MIN/MAX/STDDEV ... WHERE ... GROUP BY
  information_schema
  CONVERT units
  PROFILE
"""

from typing import Any
import json


def is_available() -> bool:
    try:
        import agriql  # noqa: F401
        return True
    except ImportError:
        return False


def query(sql: str, target: str = ":memory:") -> dict[str, Any]:
    """Execute an AgriQL query and return typed records."""
    if not is_available():
        raise ImportError(
            "agriql package not installed. "
            "Install from https://github.com/SbruiceS/AgriQL (bindings/python) "
            "after building libagriql_ffi."
        )

    import agriql

    with agriql.connect(target) as conn:
        result = conn.query(sql)
        records = result.to_records()
        return {
            "sql": sql,
            "text": result.text,
            "records": records,
            "row_count": len(records),
            "engine": "AgriQL",
            "version": "3.10.1",
        }


def schema_discovery(target: str = ":memory:") -> dict[str, Any]:
    sql = (
        "SELECT table_schema, table_name, column_name, data_type "
        "FROM information_schema.columns"
    )
    return query(sql, target)
