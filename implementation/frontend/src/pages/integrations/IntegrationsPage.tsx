import React, { useEffect, useState } from "react";
import { api } from "../../api/client";

export function IntegrationsPage() {
  const [agriql, setAgriql] = useState<any>(null);
  const [agricuda, setAgricuda] = useState<any>(null);

  useEffect(() => {
    api.agriqlCapabilities().then(setAgriql).catch(() => setAgriql({ available: false }));
    api.agricudaCapabilities().then(setAgricuda).catch(() => setAgricuda({ available: false }));
  }, []);

  return (
    <div>
      <h2>Integrations</h2>
      <div style={{ display: "grid", gap: "1.5rem", gridTemplateColumns: "1fr 1fr" }}>
        <div style={{ padding: "1rem", background: "#1e293b", borderRadius: 8 }}>
          <h3>AgriQL</h3>
          <p>Available: <strong>{agriql?.available ? "yes" : "no"}</strong></p>
          <p>Engine: {agriql?.engine} {agriql?.version}</p>
        </div>
        <div style={{ padding: "1rem", background: "#1e293b", borderRadius: 8 }}>
          <h3>AgriCUDa</h3>
          <p>Available: <strong>{agricuda?.available ? "yes" : "no"}</strong></p>
          <p>Language: {agricuda?.language} {agricuda?.version}</p>
          <p style={{ fontSize: "0.85rem", color: "#94a3b8" }}>
            GPU/FPGA: not implemented (CPU kernels only)
          </p>
        </div>
      </div>
    </div>
  );
}
