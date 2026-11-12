import React, { useEffect, useState } from "react";
import { api } from "../../api/client";

export function FarmsPage() {
  const [farms, setFarms] = useState<any[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api.farms().then(setFarms).catch((e) => setError(e.message));
  }, []);

  if (error) return <div>Error: {error}</div>;

  return (
    <div>
      <h2>Farms</h2>
      {farms.length === 0 ? (
        <p>No farms yet.</p>
      ) : (
        <table style={{ width: "100%", borderCollapse: "collapse" }}>
          <thead>
            <tr style={{ textAlign: "left", borderBottom: "1px solid #475569" }}>
              <th style={{ padding: "0.5rem" }}>Name</th>
              <th>Organization</th>
              <th>Area (ha)</th>
              <th>Location</th>
            </tr>
          </thead>
          <tbody>
            {farms.map((f) => (
              <tr key={f.id} style={{ borderBottom: "1px solid #334155" }}>
                <td style={{ padding: "0.5rem" }}>{f.name}</td>
                <td>{f.organization?.name || f.organization_id}</td>
                <td>{f.area_hectares ?? "—"}</td>
                <td>{f.location || "—"}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
