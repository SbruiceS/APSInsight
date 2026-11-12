import React, { useEffect, useState } from "react";
import { api } from "../api/client";

export function DashboardPage() {
  const [status, setStatus] = useState<string>("checking...");
  const [datasetCount, setDatasetCount] = useState(0);

  useEffect(() => {
    api.health().then((r) => setStatus(r.status)).catch(() => setStatus("unreachable"));
    api.datasets().then((d) => setDatasetCount(d.length)).catch(() => {});
  }, []);

  return (
    <div>
      <h2>Dashboard</h2>
      <p>Backend health: <strong>{status}</strong></p>
      <p>Datasets in catalog: <strong>{datasetCount}</strong></p>
      <p>APSInsight agricultural analytics platform — Phase 1 foundation active.</p>
    </div>
  );
}
