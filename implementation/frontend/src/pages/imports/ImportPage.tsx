import React, { useState } from "react";
import { api } from "../../api/client";

export function ImportPage() {
  const [datasetId, setDatasetId] = useState("");
  const [projectId, setProjectId] = useState("");
  const [filename, setFilename] = useState("");
  const [message, setMessage] = useState("");

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    try {
      const result = await api.createImport({
        dataset_id: Number(datasetId),
        project_id: Number(projectId),
        source_type: "csv",
        original_filename: filename || "upload.csv",
      });
      setMessage(`Import accepted: id=${result.id}, status=${result.status}`);
    } catch (err: any) {
      setMessage(`Error: ${err.message}`);
    }
  }

  return (
    <div>
      <h2>Import Dataset</h2>
      <form onSubmit={handleSubmit} style={{ display: "flex", flexDirection: "column", gap: "1rem", maxWidth: 400 }}>
        <label>
          Dataset ID
          <input value={datasetId} onChange={(e) => setDatasetId(e.target.value)} required style={{ display: "block", width: "100%", padding: "0.5rem" }} />
        </label>
        <label>
          Project ID
          <input value={projectId} onChange={(e) => setProjectId(e.target.value)} required style={{ display: "block", width: "100%", padding: "0.5rem" }} />
        </label>
        <label>
          Filename
          <input value={filename} onChange={(e) => setFilename(e.target.value)} placeholder="soil_samples.csv" style={{ display: "block", width: "100%", padding: "0.5rem" }} />
        </label>
        <button type="submit" style={{ padding: "0.75rem", background: "#3b82f6", color: "white", border: "none", borderRadius: 4 }}>
          Start Import
        </button>
      </form>
      {message && <p style={{ marginTop: "1rem" }}>{message}</p>}
    </div>
  );
}
