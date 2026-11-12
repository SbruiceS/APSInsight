import React, { useState } from "react";

const DEFAULT_KERNEL = `kernel fn saxpy(a: f64, x: Array<f64>, y: Array<f64>, z: Array<f64>) {
  let i = block_idx * block_dim + thread_idx;
  if i < x.length { z[i] = a * x[i] + y[i]; }
}

fn main() -> f64 {
  let x = [1.0, 2.0, 3.0, 4.0];
  let y = [10.0, 20.0, 30.0, 40.0];
  let z = array<f64>(4);
  launch saxpy(2.0, x, y, z) <<< 1, 4 >>>;
  return sum(z);
}`;

export function KernelProgrammingPage() {
  const [source, setSource] = useState(DEFAULT_KERNEL);
  const [output, setOutput] = useState<string>("");
  const [loading, setLoading] = useState(false);
  const [mode, setMode] = useState<"run" | "check">("run");

  async function execute() {
    setLoading(true);
    setOutput("");
    try {
      const endpoint = mode === "run" ? "/api/v1/agricuda/run" : "/api/v1/agricuda/check";
      const res = await fetch(endpoint, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ source }),
      });
      const data = await res.json();
      if (!res.ok) {
        setOutput(`Error: ${data.error || "failed"}\n${JSON.stringify(data.capabilities || {}, null, 2)}`);
      } else {
        setOutput(JSON.stringify(data, null, 2));
      }
    } catch (err: any) {
      setOutput(err.message);
    } finally {
      setLoading(false);
    }
  }

  return (
    <div>
      <h2 style={{ marginTop: 0 }}>AgriCUDa Kernel Programming</h2>
      <p style={{ color: "#94a3b8", marginTop: 0 }}>
        Agriculture-oriented kernel language. CPU device only — GPU/FPGA not implemented in AgriCUDa.
        Example SAXPY sum is 120 when the real binary is present.
      </p>
      <div style={{ display: "flex", gap: "0.5rem", marginBottom: "0.75rem" }}>
        <button
          onClick={() => setMode("check")}
          style={{
            padding: "0.4rem 0.8rem",
            background: mode === "check" ? "#3b82f6" : "#334155",
            color: "white",
            border: "none",
            borderRadius: 4,
          }}
        >
          Check
        </button>
        <button
          onClick={() => setMode("run")}
          style={{
            padding: "0.4rem 0.8rem",
            background: mode === "run" ? "#3b82f6" : "#334155",
            color: "white",
            border: "none",
            borderRadius: 4,
          }}
        >
          Run
        </button>
        <button
          onClick={execute}
          disabled={loading}
          style={{ padding: "0.4rem 1rem", background: "#22c55e", color: "white", border: "none", borderRadius: 4 }}
        >
          {loading ? "Running…" : "Execute"}
        </button>
      </div>
      <textarea
        value={source}
        onChange={(e) => setSource(e.target.value)}
        spellCheck={false}
        style={{
          width: "100%",
          height: 280,
          fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace",
          fontSize: "0.9rem",
          background: "#0f172a",
          color: "#e2e8f0",
          border: "1px solid #334155",
          borderRadius: 6,
          padding: "0.75rem",
          boxSizing: "border-box",
        }}
      />
      {output && (
        <pre
          style={{
            marginTop: "1rem",
            background: "#1e293b",
            padding: "1rem",
            borderRadius: 6,
            overflow: "auto",
            maxHeight: 240,
          }}
        >
          {output}
        </pre>
      )}
    </div>
  );
}
