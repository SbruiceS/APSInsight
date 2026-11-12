import React, { useState } from "react";
import { api } from "../../api/client";

export function AgriqlWorkspace() {
  const [sql, setSql] = useState("SHOW VERSION;");
  const [result, setResult] = useState<any>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  async function runQuery(e: React.FormEvent) {
    e.preventDefault();
    setLoading(true);
    setError(null);
    try {
      const res = await fetch("/api/v1/agriql/query", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ sql }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Query failed");
      setResult(data);
    } catch (err: any) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  }

  return (
    <div>
      <h2>AgriQL Query Workspace</h2>
      <p style={{ color: "#94a3b8" }}>
        Agricultural database query language (AgriQL engine 3.10.1). 
        Examples: SHOW VERSION; · SHOW SCHEMAS; · SELECT AVG(moisture) FROM agriculture.soil GROUP BY plant_id;
      </p>
      <form onSubmit={runQuery} style={{ display: "flex", flexDirection: "column", gap: "0.75rem" }}>
        <textarea
          value={sql}
          onChange={(e) => setSql(e.target.value)}
          rows={6}
          style={{ fontFamily: "monospace", padding: "0.75rem", background: "#1e293b", color: "#e2e8f0", border: "1px solid #334155" }}
        />
        <button type="submit" disabled={loading} style={{ padding: "0.75rem", background: "#3b82f6", color: "white", border: "none", borderRadius: 4, maxWidth: 200 }}>
          {loading ? "Running…" : "Run Query"}
        </button>
      </form>
      {error && <pre style={{ color: "#f87171", marginTop: "1rem" }}>{error}</pre>}
      {result && (
        <div style={{ marginTop: "1.5rem" }}>
          <h3>Result (QueryRun #{result.query_run_id})</h3>
          <pre style={{ background: "#0f172a", padding: "1rem", overflow: "auto" }}>
            {JSON.stringify(result.result, null, 2)}
          </pre>
        </div>
      )}
    </div>
  );
}
