import React, { useEffect, useState } from "react";
import { api } from "../../api/client";

export function SoilPage() {
  const [samples, setSamples] = useState<any[]>([]);
  const [observations, setObservations] = useState<any[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    Promise.all([api.soilSamples(), api.observations()])
      .then(([s, o]) => {
        setSamples(s);
        setObservations(o);
      })
      .catch((e) => setError(e.message));
  }, []);

  if (error) return <div>Error: {error}</div>;

  return (
    <div>
      <h2>Soil Samples & Observations</h2>
      <h3>Samples ({samples.length})</h3>
      {samples.length === 0 ? <p>No soil samples.</p> : (
        <ul>
          {samples.map((s) => (
            <li key={s.id}>
              {s.sample_code || `#${s.id}`} — {s.sampled_at} — depth {s.depth_cm ?? "?"} cm
              {s.field?.name ? ` — ${s.field.name}` : ""}
            </li>
          ))}
        </ul>
      )}
      <h3>Observations ({observations.length})</h3>
      {observations.length === 0 ? <p>No observations.</p> : (
        <table style={{ width: "100%", borderCollapse: "collapse" }}>
          <thead>
            <tr style={{ textAlign: "left", borderBottom: "1px solid #475569" }}>
              <th style={{ padding: "0.5rem" }}>Variable</th>
              <th>Value</th>
              <th>Unit</th>
              <th>Observed</th>
            </tr>
          </thead>
          <tbody>
            {observations.slice(0, 50).map((o) => (
              <tr key={o.id} style={{ borderBottom: "1px solid #334155" }}>
                <td style={{ padding: "0.5rem" }}>{o.variable}</td>
                <td>{o.value}</td>
                <td>{o.unit}</td>
                <td>{new Date(o.observed_at).toLocaleString()}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
