"""APSInsight Python scientific package."""

__version__ = "0.1.0"

from .ingestion.csv_profiler import profile_csv, query_parquet_with_duckdb, descriptive_stats

__all__ = ["profile_csv", "query_parquet_with_duckdb", "descriptive_stats"]
