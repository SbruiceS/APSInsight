import React, { useEffect, useState } from "react";
import { api } from "../../api/client";

export function DatasetsPage() {
  const [datasets, setDatasets] = useState<any[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api.datasets()
      .then(setDatasets)
      .catch((e) => setError(e.message));
  }, []);

  if (error) return <div>Error loading datasets: {error}</div>;

  return (
    <div>
      <h2>Datasets</h2>
      {datasets.length === 0 ? (
        <p>No datasets yet. Create one via the API or import flow.</p>
      ) : (
        <table style={{ width: "100%", borderCollapse: "collapse" }}>
          <thead>
            <tr style={{ textAlign: "left", borderBottom: "1px solid #475569" }}>
              <th style={{ padding: "0.5rem" }}>Name</th>
              <th>Project</th>
              <th>Status</th>
              <th>Created</th>
            </tr>
          </thead>
          <tbody>
            {datasets.map((d) => (
              <tr key={d.id} style={{ borderBottom: "1px solid #334155" }}>
                <td style={{ padding: "0.5rem" }}>{d.name}</td>
                <td>{d.project?.name || d.project_id}</td>
                <td>{d.status}</td>
                <td>{new Date(d.created_at).toLocaleDateString()}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
