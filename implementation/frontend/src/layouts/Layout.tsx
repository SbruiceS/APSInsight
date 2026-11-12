import React from "react";
import { Link, Outlet } from "react-router-dom";

export function Layout() {
  return (
    <div style={{ fontFamily: "system-ui, sans-serif", minHeight: "100vh", background: "#0f172a", color: "#e2e8f0" }}>
      <header style={{ padding: "1rem 2rem", borderBottom: "1px solid #334155", display: "flex", gap: "2rem", alignItems: "center" }}>
        <h1 style={{ margin: 0, fontSize: "1.5rem" }}>APSInsight</h1>
        <nav style={{ display: "flex", gap: "1.5rem" }}>
          <Link to="/" style={{ color: "#94a3b8", textDecoration: "none" }}>Dashboard</Link>
          <Link to="/datasets" style={{ color: "#94a3b8", textDecoration: "none" }}>Datasets</Link>
          <Link to="/imports" style={{ color: "#94a3b8", textDecoration: "none" }}>Import</Link>
          <Link to="/farms" style={{ color: "#94a3b8", textDecoration: "none" }}>Farms</Link>
          <Link to="/soil" style={{ color: "#94a3b8", textDecoration: "none" }}>Soil</Link>
          <Link to="/computing" style={{ color: "#94a3b8", textDecoration: "none" }}>Computing</Link>
        </nav>
      </header>
      <main style={{ padding: "2rem" }}>
        <Outlet />
      </main>
    </div>
  );
}
