import React, { useState, useRef, useEffect } from "react";

type Line = { kind: "input" | "output" | "error" | "info"; text: string };

export function QueryCliPage() {
  const [lines, setLines] = useState<Line[]>([
    { kind: "info", text: "APSInsight Query CLI — AgriQL 3.10.1" },
    { kind: "info", text: "Type SQL and press Enter. Examples: SHOW VERSION;  SHOW SCHEMAS;  SELECT AVG(moisture) FROM agriculture.soil;" },
  ]);
  const [input, setInput] = useState("");
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [lines]);

  async function run(e: React.FormEvent) {
    e.preventDefault();
    const sql = input.trim();
    if (!sql) return;
    setLines((prev) => [...prev, { kind: "input", text: `> ${sql}` }]);
    setInput("");

    try {
      const res = await fetch("/api/v1/agriql/query", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ sql }),
      });
      const data = await res.json();
      if (!res.ok) {
        setLines((prev) => [...prev, { kind: "error", text: data.error || "Query failed" }]);
        return;
      }
      const result = data.result;
      if (result.note) {
        setLines((prev) => [...prev, { kind: "info", text: result.note }]);
      }
      if (result.text) {
        setLines((prev) => [...prev, { kind: "output", text: result.text }]);
      } else if (result.records) {
        setLines((prev) => [...prev, { kind: "output", text: JSON.stringify(result.records, null, 2) }]);
      }
      setLines((prev) => [...prev, { kind: "info", text: `(${result.row_count ?? 0} rows, QueryRun #${data.query_run_id})` }]);
    } catch (err: any) {
      setLines((prev) => [...prev, { kind: "error", text: err.message }]);
    }
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "calc(100vh - 120px)" }}>
      <h2 style={{ marginTop: 0 }}>Query CLI</h2>
      <div
        style={{
          flex: 1,
          background: "#0f172a",
          border: "1px solid #334155",
          borderRadius: 6,
          padding: "1rem",
          fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace",
          fontSize: "0.9rem",
          overflow: "auto",
          marginBottom: "0.75rem",
        }}
      >
        {lines.map((line, i) => (
          <div
            key={i}
            style={{
              color:
                line.kind === "input" ? "#7dd3fc" :
                line.kind === "error" ? "#f87171" :
                line.kind === "info" ? "#94a3b8" : "#e2e8f0",
              whiteSpace: "pre-wrap",
              marginBottom: "0.25rem",
            }}
          >
            {line.text}
          </div>
        ))}
        <div ref={bottomRef} />
      </div>
      <form onSubmit={run} style={{ display: "flex", gap: "0.5rem" }}>
        <span style={{ alignSelf: "center", color: "#7dd3fc" }}>></span>
        <input
          value={input}
          onChange={(e) => setInput(e.target.value)}
          placeholder="SHOW VERSION;"
          autoFocus
          style={{
            flex: 1,
            padding: "0.6rem",
            background: "#1e293b",
            color: "#e2e8f0",
            border: "1px solid #334155",
            borderRadius: 4,
            fontFamily: "monospace",
          }}
        />
        <button type="submit" style={{ padding: "0.6rem 1rem", background: "#3b82f6", color: "white", border: "none", borderRadius: 4 }}>
          Run
        </button>
      </form>
    </div>
  );
}
