import { Routes, Route, Navigate } from "react-router-dom";
import { Layout } from "../layouts/Layout";
import { DashboardPage } from "../pages/dashboard/DashboardPage";
import { DatasetsPage } from "../pages/datasets/DatasetsPage";
import { ImportPage } from "../pages/imports/ImportPage";
import { AgriqlWorkspace } from "../pages/agriql/AgriqlWorkspace";
import { FarmsPage } from "../pages/farms/FarmsPage";
import { SoilPage } from "../pages/soil/SoilPage";
import { IntegrationsPage } from "../pages/integrations/IntegrationsPage";
import { ComputingLayout } from "../pages/computing/ComputingLayout";
import { QueryCliPage } from "../pages/computing/QueryCliPage";
import { KernelProgrammingPage } from "../pages/computing/KernelProgrammingPage";

export function AppRouter() {
  return (
    <Routes>
      <Route element={<Layout />}>
        <Route path="/" element={<DashboardPage />} />
        <Route path="/datasets" element={<DatasetsPage />} />
        <Route path="/imports" element={<ImportPage />} />
        <Route path="/farms" element={<FarmsPage />} />
        <Route path="/soil" element={<SoilPage />} />
        <Route path="/agriql" element={<AgriqlWorkspace />} />
        <Route path="/integrations" element={<IntegrationsPage />} />
        <Route path="/computing" element={<ComputingLayout />}>
          <Route index element={<Navigate to="/computing/query" replace />} />
          <Route path="query" element={<QueryCliPage />} />
          <Route path="kernel" element={<KernelProgrammingPage />} />
        </Route>
        <Route path="/projects" element={<div><h2>Projects</h2><p>Loaded from API when backend is running.</p></div>} />
      </Route>
    </Routes>
  );
}