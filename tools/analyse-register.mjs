// Derives the domain catalogue from the feature register and proves that every
// requirement is (domain entities) x (capability template). Fails on any mismatch.
import { readFileSync, writeFileSync } from "node:fs";

const register = JSON.parse(readFileSync(new URL("../register/APSInsight_Feature_Register.json", import.meta.url)));

export const TEMPLATES = {
  "Registration and identity": "Implement validated registration and stable identifiers for {E}; preserve source identity and create auditable records.",
  "Schema contract": "Define versioned schemas for {E}, including required fields, optional fields, validation errors and compatibility rules.",
  "Metadata model": "Capture metadata for {E}, including provenance, ownership, timestamps, units or domain-specific attributes where applicable.",
  "Validation rules": "Provide configurable validation rules for {E}, with severity levels, explainable failures and machine-readable reports.",
  "CRUD and lifecycle": "Support controlled create, read, update and archive operations for {E}, with lifecycle constraints and audit records.",
  "Search and filtering": "Enable indexed search and filtering over {E}, including exact matches, ranges, facets and pagination.",
  "Bulk operations": "Support safe bulk import, update and export of {E}, with dry-run validation, partial-failure reporting and idempotency.",
  "Versioning": "Version {E} and expose change history, comparisons, compatibility metadata and safe rollback where rollback is meaningful.",
  "Access control": "Enforce tenant-aware permissions for {E} at API and service boundaries; test both allowed and denied access.",
  "Provenance": "Record source references, transformation history and actor or service identity for changes involving {E}.",
  "Quality indicators": "Calculate appropriate quality indicators for {E}; document formulas, missing-data behavior and limitations.",
  "Observability": "Emit structured logs, metrics and traces for operations on {E}, avoiding secrets and sensitive payloads.",
  "Failure handling": "Define typed errors and recovery behavior for failures involving {E}; distinguish retryable from permanent failures.",
  "Concurrency control": "Protect concurrent operations on {E} using transactions, optimistic concurrency or domain-appropriate locking.",
  "Retention policy": "Apply configurable retention and archival rules to {E}, with explicit legal-hold and deletion semantics where required.",
  "Export contract": "Provide documented exports for {E}, including schema version, encoding, timestamps and provenance where appropriate.",
  "Import contract": "Provide documented import mappings for {E}, with previews, validation summaries and recoverable error reports.",
  "API endpoint": "Expose versioned API operations for {E} with request limits, authentication, authorization and consistent response envelopes.",
  "CLI support": "Provide automation-friendly CLI operations for {E}, with structured output, stable exit codes and noninteractive support.",
  "UI workflow": "Provide an Angular workflow for users to inspect and manage {E}, including empty, loading, error and permission-denied states.",
  "Dashboard visibility": "Expose meaningful status and summary metrics for {E}, with drill-down to supporting records and definitions.",
  "Notification rules": "Allow authorized users to define relevant notifications for changes or conditions involving {E}, with deduplication and audit.",
  "Workflow integration": "Allow {E} to participate in reproducible jobs and workflows through typed input/output contracts.",
  "Test coverage": "Create unit, integration, contract and property-based tests for {E}; include boundary cases and malformed inputs.",
  "Security review": "Threat-model operations involving {E}; validate injection resistance, access boundaries, abuse limits and sensitive-data handling.",
  "Performance target": "Benchmark representative operations on {E}, define workload assumptions and prevent unmeasured performance claims.",
  "Documentation": "Document the purpose, schema, API, permissions, failure modes and operational behavior of {E}.",
  "Interoperability": "Define mappings between {E} and relevant external formats or standards without silently losing semantic information.",
  "Accessibility and localization": "Ensure user-facing operations for {E} support keyboard access, accessible status messages and localization-ready labels.",
  "Migration support": "Provide safe migrations for schema or behavior changes affecting {E}, including compatibility checks and rollback planning.",
  "Acceptance evidence": "Define measurable acceptance tests for {E}, including expected outputs, failure cases and reproducibility requirements.",
};
const CAPS = Object.keys(TEMPLATES);
const slug = (s) => s.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");

const domains = [];
const errors = [];
for (const f of register) {
  if (f.title === "Registration and identity") {
    const m = f.requirement.match(/identifiers for (.*); preserve source identity/);
    if (!m) { errors.push(`cannot parse entities for ${f.id}`); continue; }
    const entities = m[1].split(", ").map((name) => ({ slug: slug(name), name }));
    domains.push({ index: domains.length + 1, slug: slug(f.domain), name: f.domain, entitiesText: m[1], entities });
  }
}
const byName = new Map(domains.map((d) => [d.name, d]));
register.forEach((f, i) => {
  const d = byName.get(f.domain);
  const expectedId = `APSI-F${String(i + 1).padStart(4, "0")}`;
  const cap = CAPS[i % 31];
  if (f.id !== expectedId) errors.push(`id order ${f.id} != ${expectedId}`);
  if (f.title !== cap) errors.push(`${f.id} title ${f.title} != ${cap}`);
  if (!d) { errors.push(`${f.id} unknown domain`); return; }
  const expected = TEMPLATES[f.title].replace("{E}", d.entitiesText);
  if (f.requirement !== expected) errors.push(`${f.id} requirement mismatch`);
});
for (const d of domains) {
  const s = new Set(d.entities.map((e) => e.slug));
  if (s.size !== d.entities.length) errors.push(`duplicate entity slug in ${d.name}`);
}
if (errors.length) { console.error(errors.join("\n")); process.exit(1); }

const capabilities = CAPS.map((title, i) => ({ index: i + 1, slug: slug(title), title, template: TEMPLATES[title] }));
writeFileSync(new URL("../register/catalog.json", import.meta.url),
  JSON.stringify({ register: "APSInsight_Feature_Register.json", features: register.length, capabilities, domains: domains.map(({ entitiesText, ...d }) => d) }, null, 2) + "\n");
console.log(`OK: ${register.length} features = ${domains.length} domains x ${capabilities.length} capabilities; ${domains.reduce((a, d) => a + d.entities.length, 0)} entity kinds`);
