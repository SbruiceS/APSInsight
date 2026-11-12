"""CSV and Parquet ingestion helpers using Polars / DuckDB."""

from pathlib import Path
from typing import Any

import polars as pl
import duckdb


def profile_csv(path: str | Path) -> dict[str, Any]:
    """Load a CSV and return basic profile: schema, row count, null counts."""
    df = pl.read_csv(path, n_rows=100_000, ignore_errors=True)
    return {
        "row_count": df.height,
        "columns": df.columns,
        "schema": {col: str(dtype) for col, dtype in df.schema.items()},
        "null_counts": {col: int(df[col].null_count()) for col in df.columns},
    }


def query_parquet_with_duckdb(path: str | Path, sql: str) -> list[dict]:
    """Run an analytical SQL query against a Parquet file using DuckDB."""
    con = duckdb.connect()
    result = con.execute(sql).fetchdf()
    return result.to_dict(orient="records")


def descriptive_stats(series: list[float]) -> dict[str, float]:
    """Simple descriptive statistics (Python reference for C++ kernel)."""
    if not series:
        return {}
    n = len(series)
    mean = sum(series) / n
    variance = sum((x - mean) ** 2 for x in series) / (n - 1) if n > 1 else 0.0
    return {
        "count": n,
        "mean": mean,
        "variance": variance,
        "min": min(series),
        "max": max(series),
    }
