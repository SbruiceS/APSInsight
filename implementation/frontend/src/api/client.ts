const API_BASE = import.meta.env.VITE_API_BASE_URL || "http://localhost:3000/api/v1";

async function request<T>(path: string, options: RequestInit = {}): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`, {
    headers: { "Content-Type": "application/json", ...options.headers },
    ...options,
  });
  if (!res.ok) {
    const err = await res.json().catch(() => ({ error: res.statusText }));
    throw new Error(err.error || err.message || "Request failed");
  }
  return res.json();
}

export const api = {
  health: () => request<{ status: string }>("/health/live"),

  organizations: () => request<any[]>("/organizations"),
  projects: () => request<any[]>("/projects"),
  datasets: () => request<any[]>("/datasets"),
  dataset: (id: number) => request<any>(`/datasets/${id}`),

  farms: () => request<any[]>("/farms"),
  farm: (id: number) => request<any>(`/farms/${id}`),
  fields: () => request<any[]>("/fields"),
  soilSamples: () => request<any[]>("/soil_samples"),
  observations: () => request<any[]>("/observations"),
  dataQualityReports: () => request<any[]>("/data_quality_reports"),

  createImport: (body: object) =>
    request<any>("/imports", { method: "POST", body: JSON.stringify({ import: body }) }),

  agricudaCapabilities: () => request<any>("/agricuda/capabilities"),
  agriqlCapabilities: () => request<any>("/agriql/capabilities"),
};
