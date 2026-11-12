# frozen_string_literal: true

class SoilAnalyticsService
  # First agricultural analytics module.
  # Computes basic nutrient summary from a dataset version.
  # Assumptions and units are explicit.

  NUTRIENT_RANGES = {
    "ph" => { low: 5.5, high: 7.5, unit: "pH" },
    "organic_carbon" => { low: 0.4, high: 1.0, unit: "%" },
    "nitrogen" => { low: 100, high: 300, unit: "kg/ha" },
    "phosphorus" => { low: 10, high: 25, unit: "kg/ha" },
    "potassium" => { low: 100, high: 250, unit: "kg/ha" }
  }.freeze

  def initialize(dataset)
    @dataset = dataset
  end

  def summary
    version = @dataset.current_version
    return { error: "No version available" } unless version

    # In production this would query the Parquet/DuckDB or AgriQL backend.
    # Here we return a structured result with explicit assumptions.
    {
      dataset_id: @dataset.id,
      version_id: version.id,
      methodology: "threshold_classification_v1",
      assumptions: [
        "Thresholds are illustrative regional defaults for Uttar Pradesh wheat systems",
        "Do not treat as universal agronomic recommendations",
        "Real values must come from validated laboratory or sensor observations"
      ],
      nutrients: NUTRIENT_RANGES.transform_values { |r| r.merge(status: "pending_data") },
      generated_at: Time.current.iso8601
    }
  end
end
