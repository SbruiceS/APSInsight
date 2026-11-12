import React from "react";
import { Link, Outlet, useLocation } from "react-router-dom";

export function ComputingLayout() {
  const loc = useLocation();
  const tab = (path: string, label: string) => (
    <Link
      to={path}
      style={{
        padding: "0.5rem 1rem",
        textDecoration: "none",
        color: loc.pathname.startsWith(path) ? "#e2e8f0" : "#94a3b8",
        borderBottom: loc.pathname.startsWith(path) ? "2px solid #3b82f6" : "2px solid transparent",
      }}
    >
      {label}
    </Link>
  );

  return (
    <div>
      <h1 style={{ marginTop: 0 }}>Computing</h1>
      <nav style={{ display: "flex", gap: "0.25rem", borderBottom: "1px solid #334155", marginBottom: "1.5rem" }}>
        {tab("/computing/query", "Query CLI")}
        {tab("/computing/kernel", "Kernel Programming")}
      </nav>
      <Outlet />
    </div>
  );
}
