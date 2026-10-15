// Types mirroring the /api/v1 contract (apsi-core serde shapes).

export type Lifecycle = 'draft' | 'active' | 'archived';
export type Role = 'viewer' | 'editor' | 'auditor' | 'admin';

export interface Envelope<T> {
  data: T;
  meta: { api_version: string };
}

export interface ErrorBody {
  code: string;
  message: string;
  retryable: boolean;
  details?: unknown;
}

export interface Principal {
  subject: string;
  tenant: string;
  roles: Role[];
  kind: 'user' | 'service';
}

export interface EntityKind {
  slug: string;
  name: string;
}

export interface Domain {
  index: number;
  slug: string;
  name: string;
  entities: EntityKind[];
}

export interface Capability {
  index: number;
  slug: string;
  title: string;
  template: string;
}

export interface Catalog {
  features: number;
  entity_kinds: number;
  capabilities: Capability[];
  domains: Domain[];
}

export interface Feature {
  id: string;
  domain: string;
  title: string;
  requirement: string;
}

export type FeatureStatus = 'implemented-and-tested' | 'implemented-not-fully-tested' | 'specified-only' | 'blocked-external';

export interface TraceRow {
  feature_id: string;
  domain: string;
  domain_slug: string;
  capability: string;
  status: FeatureStatus;
  status_label: string;
  gap: string;
  entity_kinds: string[];
  engine: string[];
  api: string[];
  cli: string[];
  ui: string;
  tests: string[];
  docs: string;
}

export interface Page<T> {
  total: number;
  offset: number;
  limit: number;
  items: T[];
}

export interface SourceRef {
  system: string;
  id: string;
}

export interface ProvenanceEvent {
  at: string;
  actor: string;
  actor_kind: 'user' | 'service';
  action: string;
  source?: SourceRef;
  transformation?: string;
  inputs?: string[];
}

export interface ApsiRecord {
  id: string;
  tenant: string;
  domain: string;
  entity: string;
  schema_version: number;
  version: number;
  lifecycle: Lifecycle;
  name: string;
  attributes: Record<string, unknown>;
  tags: string[];
  source?: SourceRef;
  owner: string;
  legal_hold: boolean;
  created_at: string;
  updated_at: string;
  created_by: string;
  updated_by: string;
  archived_at?: string;
  provenance: ProvenanceEvent[];
}

export interface SearchPage extends Page<ApsiRecord> {
  next_offset?: number;
  facets: Record<string, Record<string, number>>;
}

export interface RecordVersion {
  version: number;
  at: string;
  actor: string;
  change: string;
  schema_version: number;
  lifecycle: Lifecycle;
  name: string;
  attributes: Record<string, unknown>;
  tags: string[];
}

export interface FieldChange {
  path: string;
  before?: unknown;
  after?: unknown;
}

export interface QualityReport {
  domain: string;
  entity: string;
  schema_version: number;
  records: number;
  completeness: number | null;
  validity: number | null;
  mean_freshness_days: number | null;
  max_freshness_days: number | null;
  provenance_coverage: number | null;
  formulas: Record<string, string>;
  missing_data: string;
  limitations: string;
}

export interface EntitySummary {
  entity: string;
  name: string;
  schema_version: number;
  total: number;
  draft: number;
  active: number;
  archived: number;
  legal_hold: number;
  validity: number | null;
  completeness: number | null;
  last_updated?: string;
}

export interface AuditEvent {
  seq: number;
  at: string;
  tenant: string;
  actor: string;
  action: string;
  target: string;
  outcome: 'allowed' | 'denied' | 'failed';
  detail?: string;
}

export interface DomainDashboard {
  domain: string;
  name: string;
  total: number;
  entities: EntitySummary[];
  notifications: number;
  recent_activity: AuditEvent[];
}

export interface Overview {
  tenant: string;
  domains: { domain: string; name: string; entity_kinds: number; records: number }[];
  records: number;
  audit_events: number;
  audit_chain_valid: boolean;
}

export interface Notification {
  id: string;
  rule_id: string;
  event: string;
  record_id: string;
  message: string;
  at: string;
}

export interface NewRecord {
  name: string;
  attributes?: Record<string, unknown>;
  tags?: string[];
  lifecycle?: Lifecycle;
}
