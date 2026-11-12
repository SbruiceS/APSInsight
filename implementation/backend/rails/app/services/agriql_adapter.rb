# frozen_string_literal: true

# Full AgriQL adapter.
# Talks to the real AgriQL engine (v3.10.1) via the official Python client
# when available. Never invents grammar or results.
# Supported real capabilities (from AgriQL docs):
#   SHOW VERSION / GRAMMAR / SCHEMAS / CATALOG
#   CREATE SCHEMA / CREATE TABLE (UUID, GEOPOINT(4326), FLOAT64, ...)
#   IMPORT CSV 'path' INTO schema.table
#   SELECT with AVG, COUNT, MIN, MAX, STDDEV, WHERE, GROUP BY
#   information_schema.columns
#   CONVERT units, PROFILE
# Results are TSV text that the client converts to records.

class AgriqlAdapter
  class Unavailable < StandardError; end
  class QueryError < StandardError; end

  def initialize(target = nil)
    @target = target || ENV.fetch("AGRIQL_TARGET", ":memory:")
  end

  def available?
    python_available? && package_available?
  end

  def capabilities
    {
      available: available?,
      engine: "AgriQL",
      version: "3.10.1",
      grammar: "1.0",
      connection: @target,
      supported: [
        "SHOW VERSION", "SHOW GRAMMAR", "SHOW SCHEMAS", "SHOW CATALOG",
        "CREATE SCHEMA", "CREATE TABLE",
        "IMPORT CSV",
        "SELECT AVG/COUNT/MIN/MAX/STDDEV + WHERE + GROUP BY",
        "information_schema",
        "CONVERT units",
        "PROFILE",
        "GEOPOINT types"
      ],
      note: available? ? "Connected via Python client" : "AgriQL Python package or engine not present — test double mode"
    }
  end

  def query(sql)
    raise Unavailable, "AgriQL not available" unless available?

    script = <<~PY
      import json, sys
      try:
          import agriql
          with agriql.connect(#{@target.to_json}) as conn:
              result = conn.query(#{sql.to_json})
              records = result.to_records()
              print(json.dumps({"ok": True, "text": result.text, "records": records}))
      except Exception as e:
          print(json.dumps({"ok": False, "error": str(e)}))
          sys.exit(1)
    PY

    output = `python3 -c #{script.shellescape}`
    data = JSON.parse(output) rescue { "ok" => false, "error" => output }
    raise QueryError, data["error"] unless data["ok"]

    {
      sql: sql,
      text: data["text"],
      records: data["records"],
      row_count: data["records"]&.length || 0
    }
  end

  # Explicit test double that mirrors the real result shape
  def query_double(sql)
    {
      sql: sql,
      text: "status\tvalue\ntest_double\tok\n",
      records: [{ "status" => "test_double", "value" => "ok" }],
      row_count: 1,
      note: "Test double — real AgriQL engine not present"
    }
  end

  private

  def python_available?
    system("python3 --version > /dev/null 2>&1")
  end

  def package_available?
    system("python3 -c 'import agriql' > /dev/null 2>&1")
  end
end
