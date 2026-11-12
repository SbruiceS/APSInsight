"""Unit tests for descriptive statistics."""

from apsinsight.ingestion.csv_profiler import descriptive_stats


def test_descriptive_stats_basic():
    result = descriptive_stats([1.0, 2.0, 3.0, 4.0])
    assert result["count"] == 4
    assert abs(result["mean"] - 2.5) < 1e-9
    assert result["min"] == 1.0
    assert result["max"] == 4.0
