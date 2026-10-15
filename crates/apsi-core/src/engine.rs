//! The APSInsight engine: one generic implementation of all 31 capabilities, applied
//! to every one of the 800 entity kinds in the catalogue.
//!
//! Concurrency model: every mutation runs as a transaction under a single write lock.
//! Transactions are all-or-nothing by construction: each operation validates
//! everything first and only then mutates (multi-item dry runs work on a scratch
//! copy). Lost updates are prevented by optimistic concurrency (`expected_version`).
//!
//! Persistence (`Engine::open`): `state.json` is a snapshot; `journal.ndjson`
//! receives one fsynced line per committed transaction holding only what changed
//! (records upserted/deleted, metadata when dirty, new audit events). Opening
//! replays the journal over the snapshot; the journal is compacted into a new
//! snapshot every `COMPACT_EVERY` commits. If a journal write fails, memory is
//! reloaded from disk and a retryable `storage_unavailable` error is returned.

use crate::audit::{AuditEvent, AuditInput, AuditLog, Outcome};
use crate::auth::{Permission, Principal};
use crate::bulk::{BulkReport, BulkRequest, ItemResult, ItemStatus, MAX_BULK_ITEMS};
use crate::catalog::catalog;
use crate::dashboard::{DomainCount, DomainDashboard, EntitySummary, Overview};
use crate::error::{ApsiError, ErrorBodyOwned, Result};
use crate::exchange::{self, ExportHeader, Format};
use crate::ids::{new_id, new_record_id, validate_record_id, validate_slug};
use crate::metrics::Metrics;
use crate::migration::{MigrationFailure, MigrationPlan, MigrationReport, apply_ops};
use crate::notify::{EventKind, Notification, NotificationRule};
use crate::quality::{self, QualityReport};
use crate::record::{FieldChange, Lifecycle, NewRecord, ProvenanceEvent, Record, RecordPatch, RecordVersion, apply_patch, diff};
use crate::retention::{RetentionPolicy, RetentionReport};
use crate::schema::{Compatibility, CompatibilityLevel, EntitySchema};
use crate::search::{self, Op, Query, SearchPage, index_key};
use crate::validation::{ValidationReport, ValidationRule, validate};
use crate::workflow::{Binding, ComputeRegistry, RunStatus, StepOp, WorkflowDefinition, WorkflowRun};
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use time::{Duration, OffsetDateTime};

pub trait Clock: Send + Sync {
    fn now(&self) -> OffsetDateTime;
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }
}

/// Deterministic clock for tests and reproducible simulations.
#[derive(Clone)]
pub struct ManualClock(Arc<Mutex<OffsetDateTime>>);

impl ManualClock {
    pub fn new(start: OffsetDateTime) -> Self {
        Self(Arc::new(Mutex::new(start)))
    }

    pub fn advance(&self, d: Duration) {
        *self.0.lock() += d;
    }
}

impl Clock for ManualClock {
    fn now(&self) -> OffsetDateTime {
        *self.0.lock()
    }
}

const SEP: char = '\u{1f}';

fn pkey(tenant: &str, domain: &str, entity: &str) -> String {
    format!("{tenant}/{domain}/{entity}")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Stored {
    record: Record,
    history: Vec<RecordVersion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct IdempotencyEntry {
    payload_hash: String,
    report: BulkReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationRecord {
    pub tenant: String,
    pub plan: MigrationPlan,
    pub report: MigrationReport,
    pub rolled_back: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationRollbackReport {
    pub id: String,
    pub restored: Vec<String>,
    /// Records changed after the migration; they were left untouched.
    pub conflicts: Vec<String>,
}

/// Everything except records and the audit log. Only reachable mutably through
/// [`State::meta_mut`], which marks it for persistence.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Meta {
    schemas: HashMap<String, Vec<EntitySchema>>,
    rules: HashMap<String, Vec<ValidationRule>>,
    retention: HashMap<String, RetentionPolicy>,
    notification_rules: HashMap<String, Vec<NotificationRule>>,
    outbox: HashMap<String, Vec<Notification>>,
    dedup: HashMap<String, i64>,
    idempotency: HashMap<String, IdempotencyEntry>,
    workflows: HashMap<String, Vec<WorkflowDefinition>>,
    runs: HashMap<String, Vec<WorkflowRun>>,
    migrations: HashMap<String, MigrationRecord>,
}

/// Changes since the last durable commit.
#[derive(Debug, Clone, Default)]
struct Dirty {
    upserts: BTreeSet<String>,
    deletes: BTreeSet<String>,
    meta: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct State {
    records: HashMap<String, Stored>,
    meta: Meta,
    audit: AuditLog,
    #[serde(skip)]
    dirty: Dirty,
    #[serde(skip)]
    persisted_audit: usize,
    #[serde(skip)]
    journal_lines: usize,
    #[serde(skip)]
    partitions: HashMap<String, BTreeSet<String>>,
    #[serde(skip)]
    eq_index: HashMap<String, BTreeSet<String>>,
    #[serde(skip)]
    source_index: HashMap<String, String>,
}

/// One journal line. Serialized from borrowed state, replayed as owned values.
#[derive(Serialize)]
struct JournalOut<'a> {
    upserts: Vec<&'a Stored>,
    deletes: Vec<&'a String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    meta: Option<&'a Meta>,
    audit: &'a [AuditEvent],
}

#[derive(Deserialize)]
struct JournalIn {
    upserts: Vec<Stored>,
    deletes: Vec<String>,
    #[serde(default)]
    meta: Option<Meta>,
    audit: Vec<AuditEvent>,
}

/// Commits between snapshot compactions.
const COMPACT_EVERY: usize = 2000;

fn storage<E: std::fmt::Display>(e: E) -> ApsiError {
    ApsiError::Storage(e.to_string())
}

/// Loads snapshot + journal. A torn final journal line (crash mid-write, never
/// acknowledged) is ignored; corruption anywhere else is an error.
fn load_state(dir: &Path) -> Result<State> {
    let snap = dir.join("state.json");
    let mut st = if snap.exists() {
        serde_json::from_slice::<State>(&std::fs::read(&snap).map_err(storage)?).map_err(|e| ApsiError::Storage(format!("corrupt snapshot: {e}")))?
    } else {
        State::default()
    };
    let journal = dir.join("journal.ndjson");
    if journal.exists() {
        let text = std::fs::read_to_string(&journal).map_err(storage)?;
        let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
        for (i, line) in lines.iter().enumerate() {
            let entry: JournalIn = match serde_json::from_str(line) {
                Ok(e) => e,
                Err(_) if i + 1 == lines.len() && !text.ends_with('\n') => break,
                Err(e) => return Err(ApsiError::Storage(format!("corrupt journal line {}: {e}", i + 1))),
            };
            for s in entry.upserts {
                st.records.insert(s.record.id.clone(), s);
            }
            for id in entry.deletes {
                st.records.remove(&id);
            }
            if let Some(m) = entry.meta {
                st.meta = m;
            }
            for e in entry.audit {
                st.audit.restore(e);
            }
            st.journal_lines += 1;
        }
    }
    st.rebuild_indexes();
    if let Err(seq) = st.audit.verify() {
        return Err(ApsiError::Storage(format!("audit chain broken at event {seq}")));
    }
    st.persisted_audit = st.audit.len();
    Ok(st)
}

impl State {
    fn meta_mut(&mut self) -> &mut Meta {
        self.dirty.meta = true;
        &mut self.meta
    }

    fn put(&mut self, s: Stored) {
        self.dirty.upserts.insert(s.record.id.clone());
        self.records.insert(s.record.id.clone(), s);
    }

    fn touch(&mut self, id: &str) -> Option<&mut Stored> {
        self.dirty.upserts.insert(id.to_string());
        self.records.get_mut(id)
    }

    fn gone(&mut self, id: &str) {
        self.dirty.upserts.remove(id);
        self.dirty.deletes.insert(id.to_string());
        self.records.remove(id);
    }

    fn index(&mut self, r: &Record) {
        let pk = pkey(&r.tenant, &r.domain, &r.entity);
        self.partitions.entry(pk.clone()).or_default().insert(r.id.clone());
        for (k, v) in &r.attributes {
            if let Some(ik) = index_key(v) {
                self.eq_index.entry(format!("{pk}{SEP}{k}{SEP}{ik}")).or_default().insert(r.id.clone());
            }
        }
        if let Some(s) = &r.source {
            self.source_index.insert(format!("{pk}{SEP}{}{SEP}{}", s.system, s.id), r.id.clone());
        }
    }

    fn unindex(&mut self, r: &Record) {
        let pk = pkey(&r.tenant, &r.domain, &r.entity);
        if let Some(set) = self.partitions.get_mut(&pk) {
            set.remove(&r.id);
        }
        for (k, v) in &r.attributes {
            if let Some(ik) = index_key(v) {
                let key = format!("{pk}{SEP}{k}{SEP}{ik}");
                if let Some(set) = self.eq_index.get_mut(&key) {
                    set.remove(&r.id);
                    if set.is_empty() {
                        self.eq_index.remove(&key);
                    }
                }
            }
        }
        if let Some(s) = &r.source {
            self.source_index.remove(&format!("{pk}{SEP}{}{SEP}{}", s.system, s.id));
        }
    }

    fn rebuild_indexes(&mut self) {
        let records: Vec<Record> = self.records.values().map(|s| s.record.clone()).collect();
        for r in &records {
            self.index(r);
        }
    }

    fn schema_versions(&self, tenant: &str, domain: &str, entity: &str) -> Vec<EntitySchema> {
        self.meta.schemas.get(&pkey(tenant, domain, entity)).cloned().unwrap_or_else(|| vec![EntitySchema::default_for(domain, entity)])
    }

    fn current_schema(&self, tenant: &str, domain: &str, entity: &str) -> EntitySchema {
        self.schema_versions(tenant, domain, entity).pop().expect("at least v1")
    }

    fn schema_at(&self, tenant: &str, domain: &str, entity: &str, version: u32) -> Result<EntitySchema> {
        self.schema_versions(tenant, domain, entity)
            .into_iter()
            .find(|s| s.version == version)
            .ok_or_else(|| ApsiError::NotFound(format!("schema version {version}")))
    }

    fn rules_for(&self, tenant: &str, domain: &str, entity: &str) -> Vec<ValidationRule> {
        self.meta.rules.get(&pkey(tenant, domain, entity)).cloned().unwrap_or_default()
    }

    fn stored(&self, tenant: &str, id: &str) -> Result<&Stored> {
        validate_record_id(id)?;
        // Records of other tenants are reported as not found so ids cannot be probed.
        self.records.get(id).filter(|s| s.record.tenant == tenant).ok_or_else(|| ApsiError::NotFound(format!("record {id}")))
    }
}

fn check_kind(domain: &str, entity: &str) -> Result<()> {
    catalog().entity(domain, entity).map(|_| ()).ok_or_else(|| ApsiError::NotFound(format!("entity kind {domain}/{entity}")))
}

fn check_domain(domain: &str) -> Result<()> {
    catalog().domain(domain).map(|_| ()).ok_or_else(|| ApsiError::NotFound(format!("domain {domain}")))
}

fn sha256_json<T: Serialize>(v: &T) -> String {
    hex::encode(Sha256::digest(serde_json::to_vec(v).unwrap_or_default()))
}

/// Context passed into transactions.
struct Tx<'a> {
    p: &'a Principal,
    tenant: &'a str,
    now: OffsetDateTime,
    compute: Option<&'a dyn ComputeRegistry>,
}

/// Text to import together with its format.
#[derive(Debug, Clone, Copy)]
pub struct ImportBody<'a> {
    pub format: Format,
    pub text: &'a str,
}

pub struct Engine {
    state: RwLock<State>,
    data_dir: Option<PathBuf>,
    clock: Arc<dyn Clock>,
    metrics: Metrics,
    compute: Option<Arc<dyn ComputeRegistry>>,
}

impl Default for Engine {
    fn default() -> Self {
        Self::in_memory()
    }
}

impl Engine {
    pub fn in_memory() -> Self {
        Self { state: RwLock::new(State::default()), data_dir: None, clock: Arc::new(SystemClock), metrics: Metrics::default(), compute: None }
    }

    /// Opens (or initialises) a persistent engine in `dir` (`state.json` + `journal.ndjson`).
    pub fn open(dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(dir).map_err(storage)?;
        let state = load_state(dir)?;
        Ok(Self { state: RwLock::new(state), data_dir: Some(dir.to_path_buf()), clock: Arc::new(SystemClock), metrics: Metrics::default(), compute: None })
    }

    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    pub fn with_compute(mut self, registry: Arc<dyn ComputeRegistry>) -> Self {
        self.compute = Some(registry);
        self
    }

    pub fn compute(&self) -> Option<&Arc<dyn ComputeRegistry>> {
        self.compute.as_ref()
    }

    pub fn metrics(&self) -> &Metrics {
        &self.metrics
    }

    /// Appends the transaction's changes to the journal (fsynced) and compacts the
    /// journal into a snapshot every `COMPACT_EVERY` commits.
    fn persist(&self, st: &mut State) -> Result<()> {
        let Some(dir) = &self.data_dir else {
            st.dirty = Dirty::default();
            st.persisted_audit = st.audit.len();
            return Ok(());
        };
        let new_audit = &st.audit.events()[st.persisted_audit..];
        if st.dirty.upserts.is_empty() && st.dirty.deletes.is_empty() && !st.dirty.meta && new_audit.is_empty() {
            return Ok(());
        }
        let entry = JournalOut {
            upserts: st.dirty.upserts.iter().filter_map(|id| st.records.get(id)).collect(),
            deletes: st.dirty.deletes.iter().collect(),
            meta: st.dirty.meta.then_some(&st.meta),
            audit: new_audit,
        };
        let mut line = serde_json::to_vec(&entry).map_err(|e| ApsiError::Internal(e.to_string()))?;
        line.push(b'\n');
        {
            use std::io::Write as _;
            let mut f = std::fs::OpenOptions::new().create(true).append(true).open(dir.join("journal.ndjson")).map_err(storage)?;
            f.write_all(&line).map_err(storage)?;
            f.sync_data().map_err(storage)?;
        }
        st.dirty = Dirty::default();
        st.persisted_audit = st.audit.len();
        st.journal_lines += 1;
        if st.journal_lines >= COMPACT_EVERY {
            Self::compact(dir, st)?;
        }
        Ok(())
    }

    /// Writes a full snapshot atomically (temp + rename), then truncates the journal.
    /// Replaying a stale journal over a newer snapshot is harmless (idempotent upserts,
    /// audit events deduplicated by sequence number), so a crash between the two
    /// steps loses nothing.
    fn compact(dir: &Path, st: &mut State) -> Result<()> {
        let tmp = dir.join("state.json.tmp");
        let bytes = serde_json::to_vec(&*st).map_err(|e| ApsiError::Internal(e.to_string()))?;
        {
            use std::io::Write as _;
            let mut f = std::fs::File::create(&tmp).map_err(storage)?;
            f.write_all(&bytes).map_err(storage)?;
            f.sync_all().map_err(storage)?;
        }
        std::fs::rename(&tmp, dir.join("state.json")).map_err(storage)?;
        std::fs::write(dir.join("journal.ndjson"), b"").map_err(storage)?;
        st.journal_lines = 0;
        Ok(())
    }

    /// Forces a snapshot compaction (e.g. before backups).
    pub fn compact_now(&self) -> Result<()> {
        let Some(dir) = &self.data_dir else { return Ok(()) };
        let mut st = self.state.write();
        self.persist(&mut st)?;
        Self::compact(dir, &mut st)
    }

    fn outcome_of(e: &ApsiError) -> Outcome {
        if matches!(e, ApsiError::Forbidden(_) | ApsiError::Unauthenticated) { Outcome::Denied } else { Outcome::Failed }
    }

    /// Runs `f` as an audited, authorized, all-or-nothing transaction.
    fn tx<T>(&self, p: &Principal, tenant: &str, perm: Permission, action: &str, target: &str, f: impl FnOnce(&mut State, &Tx<'_>) -> Result<T>) -> Result<T> {
        let started = Instant::now();
        let domain_label = target.split('/').next().unwrap_or("-").to_string();
        let mut st = self.state.write();
        let now = self.clock.now();
        // Operations validate before mutating, so an Err leaves the state untouched.
        let result = validate_slug("tenant", tenant).and_then(|_| p.authorize(tenant, perm)).and_then(|_| {
            let ctx = Tx { p, tenant, now, compute: self.compute.as_deref() };
            f(&mut st, &ctx)
        });
        let (outcome, detail) = match &result {
            Ok(_) => (Outcome::Allowed, None),
            Err(e) => (Self::outcome_of(e), Some(e.code().to_string())),
        };
        st.audit.append(now, AuditInput { tenant, actor: &p.subject, action, target, outcome, detail });
        let result = match (result, self.persist(&mut st)) {
            (Ok(v), Ok(())) => Ok(v),
            (Ok(_), Err(e)) => {
                // Commit failed: reload the last durable state so memory and disk agree.
                if let Some(dir) = &self.data_dir
                    && let Ok(durable) = load_state(dir)
                {
                    *st = durable;
                }
                Err(e)
            }
            (Err(e), _) => Err(e),
        };
        drop(st);
        let code = result.as_ref().map_or_else(|e| e.code(), |_| "ok");
        self.metrics.count(action, &domain_label, code);
        self.metrics.observe_seconds(action, started.elapsed().as_secs_f64());
        tracing::info!(target: "apsi::engine", action, tenant, actor = %p.subject, outcome = code, "operation");
        result
    }

    /// Read-only operation. Denials are still audited (under a short write lock).
    fn read<T>(
        &self,
        p: &Principal,
        tenant: &str,
        perm: Permission,
        action: &str,
        target: &str,
        f: impl FnOnce(&State, OffsetDateTime) -> Result<T>,
    ) -> Result<T> {
        let now = self.clock.now();
        let auth = validate_slug("tenant", tenant).and_then(|_| p.authorize(tenant, perm));
        let result = match auth {
            Ok(()) => f(&self.state.read(), now),
            Err(e) => {
                let mut st = self.state.write();
                st.audit.append(now, AuditInput { tenant, actor: &p.subject, action, target, outcome: Outcome::Denied, detail: Some(e.code().into()) });
                let _ = self.persist(&mut st);
                Err(e)
            }
        };
        let domain_label = target.split('/').next().unwrap_or("-");
        self.metrics.count(action, domain_label, result.as_ref().map_or_else(|e| e.code(), |_| "ok"));
        result
    }

    // ---------------------------------------------------------------- records

    pub fn create(&self, p: &Principal, tenant: &str, domain: &str, entity: &str, new: NewRecord) -> Result<Record> {
        self.tx(p, tenant, Permission::Write, "record.create", &format!("{domain}/{entity}"), |st, cx| create_in(st, cx, domain, entity, new))
    }

    pub fn get(&self, p: &Principal, tenant: &str, id: &str) -> Result<Record> {
        self.read(p, tenant, Permission::Read, "record.get", "-", |st, _| Ok(st.stored(tenant, id)?.record.clone()))
    }

    pub fn update(&self, p: &Principal, tenant: &str, id: &str, expected_version: u64, patch: RecordPatch) -> Result<Record> {
        self.tx(p, tenant, Permission::Write, "record.update", &self.target_of(tenant, id), |st, cx| {
            update_in(st, cx, id, Some(expected_version), patch).map(|(r, _)| r)
        })
    }

    pub fn transition(&self, p: &Principal, tenant: &str, id: &str, expected_version: u64, to: Lifecycle) -> Result<Record> {
        let perm = if to == Lifecycle::Archived { Permission::Archive } else { Permission::Write };
        self.tx(p, tenant, perm, "record.transition", &self.target_of(tenant, id), |st, cx| {
            let mut r = st.stored(tenant, id)?.record.clone();
            check_version(&r, expected_version)?;
            if r.lifecycle == to {
                return Ok(r);
            }
            if !r.lifecycle.can_transition(to) {
                return Err(ApsiError::Lifecycle(format!("cannot move from {} to {}", r.lifecycle.as_str(), to.as_str())));
            }
            if r.lifecycle == Lifecycle::Archived {
                cx.p.authorize(tenant, Permission::Administer)?;
            }
            r.lifecycle = to;
            r.archived_at = (to == Lifecycle::Archived).then_some(cx.now);
            let action = format!("lifecycle.{}", to.as_str());
            commit_change(st, cx, r, &action, None).inspect(|r| {
                notify_in(
                    st,
                    cx.now,
                    tenant,
                    if to == Lifecycle::Archived {
                        EventKind::Archived
                    } else if to == Lifecycle::Active {
                        EventKind::Restored
                    } else {
                        EventKind::Updated
                    },
                    r,
                )
            })
        })
    }

    pub fn set_legal_hold(&self, p: &Principal, tenant: &str, id: &str, hold: bool) -> Result<Record> {
        self.tx(
            p,
            tenant,
            Permission::Administer,
            if hold { "record.legal_hold.set" } else { "record.legal_hold.release" },
            &self.target_of(tenant, id),
            |st, cx| {
                st.stored(tenant, id)?;
                let stored = st.touch(id).expect("checked above");
                stored.record.legal_hold = hold;
                stored.record.provenance.push(ProvenanceEvent {
                    at: cx.now,
                    actor: cx.p.subject.clone(),
                    actor_kind: cx.p.kind,
                    action: if hold { "legal_hold.set".into() } else { "legal_hold.release".into() },
                    source: None,
                    transformation: None,
                    inputs: vec![],
                });
                Ok(stored.record.clone())
            },
        )
    }

    /// Permanent deletion: only archived records without legal hold.
    pub fn delete(&self, p: &Principal, tenant: &str, id: &str) -> Result<()> {
        self.tx(p, tenant, Permission::Administer, "record.delete", &self.target_of(tenant, id), |st, cx| {
            let r = st.stored(tenant, id)?.record.clone();
            if r.legal_hold {
                return Err(ApsiError::LegalHold(id.into()));
            }
            if r.lifecycle != Lifecycle::Archived {
                return Err(ApsiError::Lifecycle("only archived records can be deleted".into()));
            }
            st.unindex(&r);
            st.gone(id);
            notify_in(st, cx.now, tenant, EventKind::Deleted, &r);
            Ok(())
        })
    }

    fn target_of(&self, tenant: &str, id: &str) -> String {
        let st = self.state.read();
        st.records
            .get(id)
            .filter(|s| s.record.tenant == tenant)
            .map(|s| format!("{}/{}/{}", s.record.domain, s.record.entity, id))
            .unwrap_or_else(|| format!("-/-/{}", id.chars().take(40).collect::<String>()))
    }

    // -------------------------------------------------------------- versioning

    pub fn history(&self, p: &Principal, tenant: &str, id: &str) -> Result<Vec<RecordVersion>> {
        self.read(p, tenant, Permission::Read, "record.history", "-", |st, _| Ok(st.stored(tenant, id)?.history.clone()))
    }

    pub fn diff(&self, p: &Principal, tenant: &str, id: &str, from: u64, to: u64) -> Result<Vec<FieldChange>> {
        self.read(p, tenant, Permission::Read, "record.diff", "-", |st, _| {
            let h = &st.stored(tenant, id)?.history;
            let find = |v: u64| h.iter().find(|x| x.version == v).ok_or_else(|| ApsiError::NotFound(format!("version {v}")));
            Ok(diff(find(from)?, find(to)?))
        })
    }

    pub fn rollback(&self, p: &Principal, tenant: &str, id: &str, to_version: u64, expected_version: u64) -> Result<Record> {
        self.tx(p, tenant, Permission::Write, "record.rollback", &self.target_of(tenant, id), |st, cx| {
            let stored = st.stored(tenant, id)?;
            let mut r = stored.record.clone();
            check_version(&r, expected_version)?;
            if r.lifecycle == Lifecycle::Archived {
                return Err(ApsiError::Lifecycle("archived records are read-only".into()));
            }
            let snap = stored.history.iter().find(|v| v.version == to_version).cloned().ok_or_else(|| ApsiError::NotFound(format!("version {to_version}")))?;
            if snap.schema_version != r.schema_version {
                return Err(ApsiError::Conflict("target version uses a different schema version; roll back the migration instead".into()));
            }
            let schema = st.schema_at(tenant, &r.domain, &r.entity, r.schema_version)?;
            validate(&snap.name, &snap.attributes, &snap.tags, &schema, &st.rules_for(tenant, &r.domain, &r.entity)).into_result()?;
            r.name = snap.name;
            r.attributes = snap.attributes;
            r.tags = snap.tags;
            commit_change(st, cx, r, &format!("rollback to v{to_version}"), Some(format!("rollback to version {to_version}")))
        })
    }

    // ------------------------------------------------------------------ search

    pub fn search(&self, p: &Principal, tenant: &str, q: &Query) -> Result<SearchPage> {
        self.read(p, tenant, Permission::Read, "record.search", &q.domain, |st, _| search_in(st, tenant, q))
    }

    // ----------------------------------------------------------- bulk/import

    pub fn bulk(&self, p: &Principal, tenant: &str, domain: &str, entity: &str, req: BulkRequest) -> Result<BulkReport> {
        let items = req.items.iter().cloned().map(Ok).collect();
        self.bulk_items(p, tenant, domain, entity, items, &req)
    }

    /// Imports CSV or JSON text (parse errors become per-row failures).
    pub fn import(&self, p: &Principal, tenant: &str, domain: &str, entity: &str, input: ImportBody<'_>, options: BulkRequest) -> Result<BulkReport> {
        let ImportBody { format, text: body } = input;
        let items = match format {
            Format::Csv => exchange::csv_to_records(body)?,
            Format::Json => {
                let v: Value = serde_json::from_str(body).map_err(|e| ApsiError::InvalidInput(format!("invalid JSON: {e}")))?;
                exchange::json_to_records(&v)?
            }
            Format::Ndjson => body
                .lines()
                .filter(|l| !l.trim().is_empty())
                .enumerate()
                .filter(|(_, l)| !l.starts_with("{\"header\""))
                .map(|(i, l)| serde_json::from_str::<NewRecord>(l).map_err(|e| format!("line {}: {e}", i + 1)))
                .collect(),
            Format::Geojson => return Err(ApsiError::InvalidInput("GeoJSON is an export-only format".into())),
        };
        self.bulk_items(p, tenant, domain, entity, items, &options)
    }

    fn bulk_items(
        &self,
        p: &Principal,
        tenant: &str,
        domain: &str,
        entity: &str,
        items: Vec<std::result::Result<NewRecord, String>>,
        req: &BulkRequest,
    ) -> Result<BulkReport> {
        if items.len() > MAX_BULK_ITEMS {
            return Err(ApsiError::PayloadTooLarge(format!("at most {MAX_BULK_ITEMS} items per request")));
        }
        let action = if req.dry_run { "bulk.preview" } else { "bulk.import" };
        self.tx(p, tenant, Permission::Write, action, &format!("{domain}/{entity}"), |st, cx| {
            check_kind(domain, entity)?;
            if let Some(m) = &req.mapping {
                m.check()?;
            }
            let payload_hash = sha256_json(&(domain, entity, &items, req.upsert, &req.mapping));
            let idem_key = match &req.idempotency_key {
                Some(k) if k.is_empty() || k.len() > 128 || k.chars().any(char::is_control) => {
                    return Err(ApsiError::InvalidInput("idempotency key must be 1..128 printable characters".into()));
                }
                Some(k) if !req.dry_run => Some(format!("{tenant}{SEP}{k}")),
                _ => None,
            };
            if let Some(k) = &idem_key
                && let Some(entry) = st.meta.idempotency.get(k)
            {
                if entry.payload_hash != payload_hash {
                    return Err(ApsiError::Conflict("idempotency key reused with a different payload".into()));
                }
                let mut report = entry.report.clone();
                report.replayed = true;
                return Ok(report);
            }
            let mut scratch: State;
            let target: &mut State = if req.dry_run {
                scratch = st.clone();
                &mut scratch
            } else {
                &mut *st
            };
            let mut report = BulkReport { dry_run: req.dry_run, ..Default::default() };
            for (index, item) in items.into_iter().enumerate() {
                let result = item.map_err(ApsiError::InvalidInput).and_then(|mut n| {
                    if let Some(m) = &req.mapping {
                        n.attributes = m.apply(&n.attributes);
                    }
                    bulk_one(target, cx, domain, entity, n, req.upsert)
                });
                report.push(match result {
                    Ok((status, id)) => ItemResult { index, status, id: Some(id), error: None },
                    Err(e) => ItemResult { index, status: ItemStatus::Failed, id: None, error: Some(ErrorBodyOwned::from(&e)) },
                });
            }
            if !req.dry_run
                && let Some(k) = idem_key
            {
                st.meta_mut().idempotency.insert(k, IdempotencyEntry { payload_hash, report: report.clone() });
            }
            Ok(report)
        })
    }

    pub fn export(&self, p: &Principal, tenant: &str, domain: &str, entity: &str, format: Format, include_archived: bool) -> Result<String> {
        self.tx(p, tenant, Permission::Read, "export", &format!("{domain}/{entity}"), |st, cx| {
            check_kind(domain, entity)?;
            let mut records: Vec<Record> =
                partition(st, tenant, domain, entity).filter(|r| include_archived || r.lifecycle != Lifecycle::Archived).cloned().collect();
            search::sort(&mut records, search::SortKey::CreatedAt, false);
            let header = ExportHeader {
                format: exchange::EXPORT_FORMAT.into(),
                format_version: exchange::EXPORT_FORMAT_VERSION,
                encoding: "utf-8".into(),
                exported_at: cx.now,
                tenant: tenant.into(),
                domain: domain.into(),
                entity: entity.into(),
                schema_version: st.current_schema(tenant, domain, entity).version,
                count: records.len(),
                provenance_included: true,
            };
            exchange::render(format, &header, &records)
        })
    }

    // ------------------------------------------------------- schema & rules

    pub fn schemas(&self, p: &Principal, tenant: &str, domain: &str, entity: &str) -> Result<Vec<EntitySchema>> {
        self.read(p, tenant, Permission::Read, "schema.list", domain, |st, _| {
            check_kind(domain, entity)?;
            Ok(st.schema_versions(tenant, domain, entity))
        })
    }

    pub fn current_schema(&self, p: &Principal, tenant: &str, domain: &str, entity: &str) -> Result<EntitySchema> {
        self.schemas(p, tenant, domain, entity).map(|mut v| v.pop().expect("v1 exists"))
    }

    /// Publishes the next schema version. Breaking changes require `allow_breaking`
    /// and are expected to be followed by a migration.
    pub fn publish_schema(&self, p: &Principal, tenant: &str, schema: EntitySchema, allow_breaking: bool) -> Result<Compatibility> {
        let target = format!("{}/{}", schema.domain, schema.entity);
        self.tx(p, tenant, Permission::Administer, "schema.publish", &target, |st, _| {
            check_kind(&schema.domain, &schema.entity)?;
            schema.check_well_formed()?;
            let mut versions = st.schema_versions(tenant, &schema.domain, &schema.entity);
            let current = versions.last().expect("v1");
            if schema.version != current.version + 1 {
                return Err(ApsiError::InvalidInput(format!("next schema version must be {}", current.version + 1)));
            }
            let compat = current.compatibility(&schema);
            if compat.level == CompatibilityLevel::Breaking && !allow_breaking {
                return Err(ApsiError::Conflict(format!("breaking schema change: {}", compat.changes.join("; "))));
            }
            versions.push(schema.clone());
            st.meta_mut().schemas.insert(pkey(tenant, &schema.domain, &schema.entity), versions);
            Ok(compat)
        })
    }

    pub fn rules(&self, p: &Principal, tenant: &str, domain: &str, entity: &str) -> Result<Vec<ValidationRule>> {
        self.read(p, tenant, Permission::Read, "rules.get", domain, |st, _| {
            check_kind(domain, entity)?;
            Ok(st.rules_for(tenant, domain, entity))
        })
    }

    pub fn set_rules(&self, p: &Principal, tenant: &str, domain: &str, entity: &str, rules: Vec<ValidationRule>) -> Result<Vec<ValidationRule>> {
        self.tx(p, tenant, Permission::Administer, "rules.set", &format!("{domain}/{entity}"), |st, _| {
            check_kind(domain, entity)?;
            if rules.len() > 100 {
                return Err(ApsiError::InvalidInput("at most 100 rules per entity kind".into()));
            }
            let mut ids = BTreeSet::new();
            for r in &rules {
                r.check_well_formed()?;
                if !ids.insert(r.id.clone()) {
                    return Err(ApsiError::InvalidInput(format!("duplicate rule id {}", r.id)));
                }
            }
            st.meta_mut().rules.insert(pkey(tenant, domain, entity), rules.clone());
            Ok(rules)
        })
    }

    pub fn validate_preview(&self, p: &Principal, tenant: &str, domain: &str, entity: &str, new: &NewRecord) -> Result<ValidationReport> {
        self.read(p, tenant, Permission::Read, "record.validate", domain, |st, _| {
            check_kind(domain, entity)?;
            let schema = st.current_schema(tenant, domain, entity);
            Ok(validate(&new.name, &new.attributes, &new.tags, &schema, &st.rules_for(tenant, domain, entity)))
        })
    }

    // ------------------------------------------------------------- quality

    pub fn quality(&self, p: &Principal, tenant: &str, domain: &str, entity: &str) -> Result<QualityReport> {
        self.read(p, tenant, Permission::Read, "quality", domain, |st, now| {
            check_kind(domain, entity)?;
            Ok(quality_in(st, now, tenant, domain, entity))
        })
    }

    // ----------------------------------------------------------- retention

    pub fn set_retention(&self, p: &Principal, tenant: &str, domain: &str, entity: &str, policy: RetentionPolicy) -> Result<RetentionPolicy> {
        self.tx(p, tenant, Permission::Administer, "retention.set", &format!("{domain}/{entity}"), |st, _| {
            check_kind(domain, entity)?;
            policy.check()?;
            st.meta_mut().retention.insert(pkey(tenant, domain, entity), policy);
            Ok(policy)
        })
    }

    pub fn retention(&self, p: &Principal, tenant: &str, domain: &str, entity: &str) -> Result<RetentionPolicy> {
        self.read(p, tenant, Permission::Read, "retention.get", domain, |st, _| {
            check_kind(domain, entity)?;
            Ok(st.meta.retention.get(&pkey(tenant, domain, entity)).copied().unwrap_or_default())
        })
    }

    pub fn apply_retention(&self, p: &Principal, tenant: &str, dry_run: bool) -> Result<RetentionReport> {
        self.tx(p, tenant, Permission::Administer, if dry_run { "retention.preview" } else { "retention.apply" }, "-", |st, cx| {
            let mut scratch: State;
            let target: &mut State = if dry_run {
                scratch = st.clone();
                &mut scratch
            } else {
                &mut *st
            };
            let mut report = RetentionReport { dry_run, ..Default::default() };
            let policies: Vec<(String, RetentionPolicy)> =
                target.meta.retention.iter().filter(|(k, _)| k.split('/').next() == Some(tenant)).map(|(k, v)| (k.clone(), *v)).collect();
            for (key, policy) in policies {
                let ids: Vec<String> = target.partitions.get(&key).map(|s| s.iter().cloned().collect()).unwrap_or_default();
                for id in ids {
                    let r = target.records[&id].record.clone();
                    let days = |t: OffsetDateTime| (cx.now - t).whole_days();
                    if r.lifecycle != Lifecycle::Archived {
                        if let Some(n) = policy.archive_after_days
                            && days(r.updated_at) >= i64::from(n)
                        {
                            if r.legal_hold {
                                report.held.push(id);
                                continue;
                            }
                            let mut a = r;
                            a.lifecycle = Lifecycle::Archived;
                            a.archived_at = Some(cx.now);
                            commit_change(target, cx, a, "retention.archive", Some("retention policy".into()))?;
                            report.archived.push(id);
                        }
                    } else if let Some(n) = policy.purge_archived_after_days
                        && r.archived_at.is_some_and(|t| days(t) >= i64::from(n))
                    {
                        if r.legal_hold {
                            report.held.push(id);
                            continue;
                        }
                        target.unindex(&r);
                        target.gone(&id);
                        notify_in(target, cx.now, tenant, EventKind::Deleted, &r);
                        report.purged.push(id);
                    }
                }
            }
            Ok(report)
        })
    }

    // -------------------------------------------------------- notifications

    pub fn add_notification_rule(&self, p: &Principal, tenant: &str, mut rule: NotificationRule) -> Result<NotificationRule> {
        let target = rule.domain.clone();
        self.tx(p, tenant, Permission::Write, "notification_rule.create", &target, |st, cx| {
            check_domain(&rule.domain)?;
            if let Some(e) = &rule.entity {
                check_kind(&rule.domain, e)?;
            }
            rule.check()?;
            rule.id = new_id("nrl");
            rule.tenant = tenant.into();
            rule.created_by = cx.p.subject.clone();
            let rules = st.meta_mut().notification_rules.entry(tenant.into()).or_default();
            if rules.len() >= 200 {
                return Err(ApsiError::InvalidInput("at most 200 notification rules per tenant".into()));
            }
            rules.push(rule.clone());
            Ok(rule)
        })
    }

    pub fn notification_rules(&self, p: &Principal, tenant: &str) -> Result<Vec<NotificationRule>> {
        self.read(p, tenant, Permission::Read, "notification_rule.list", "-", |st, _| Ok(st.meta.notification_rules.get(tenant).cloned().unwrap_or_default()))
    }

    pub fn delete_notification_rule(&self, p: &Principal, tenant: &str, id: &str) -> Result<()> {
        self.tx(p, tenant, Permission::Write, "notification_rule.delete", "-", |st, _| {
            let rules = st.meta_mut().notification_rules.entry(tenant.into()).or_default();
            let before = rules.len();
            rules.retain(|r| r.id != id);
            if rules.len() == before { Err(ApsiError::NotFound(format!("notification rule {id}"))) } else { Ok(()) }
        })
    }

    pub fn notifications(&self, p: &Principal, tenant: &str, limit: usize) -> Result<Vec<Notification>> {
        self.read(p, tenant, Permission::Read, "notification.list", "-", |st, _| {
            let all = st.meta.outbox.get(tenant).cloned().unwrap_or_default();
            Ok(all.into_iter().rev().take(limit.clamp(1, 500)).collect())
        })
    }

    // ------------------------------------------------------------ workflows

    pub fn register_workflow(&self, p: &Principal, tenant: &str, mut def: WorkflowDefinition) -> Result<WorkflowDefinition> {
        self.tx(p, tenant, Permission::Write, "workflow.register", "-", |st, cx| {
            def.check(cx.compute)?;
            for step in &def.steps {
                match &step.op {
                    StepOp::Search { query } => check_domain(&query.domain)?,
                    StepOp::Quality { domain, entity } => check_kind(domain, entity)?,
                    _ => {}
                }
            }
            let versions = st.meta_mut().workflows.entry(format!("{tenant}/{}", def.id)).or_default();
            def.version = versions.last().map_or(1, |d| d.version + 1);
            versions.push(def.clone());
            Ok(def)
        })
    }

    pub fn workflows(&self, p: &Principal, tenant: &str) -> Result<Vec<WorkflowDefinition>> {
        self.read(p, tenant, Permission::Read, "workflow.list", "-", |st, _| {
            let prefix = format!("{tenant}/");
            let mut out: Vec<WorkflowDefinition> =
                st.meta.workflows.iter().filter(|(k, _)| k.starts_with(&prefix)).filter_map(|(_, v)| v.last().cloned()).collect();
            out.sort_by(|a, b| a.id.cmp(&b.id));
            Ok(out)
        })
    }

    pub fn run_workflow(&self, p: &Principal, tenant: &str, id: &str, version: Option<u32>) -> Result<WorkflowRun> {
        self.tx(p, tenant, Permission::Write, "workflow.run", "-", |st, cx| {
            let versions = st.meta.workflows.get(&format!("{tenant}/{id}")).ok_or_else(|| ApsiError::NotFound(format!("workflow {id}")))?;
            let def = match version {
                Some(v) => versions.iter().find(|d| d.version == v).cloned().ok_or_else(|| ApsiError::NotFound(format!("workflow {id} v{v}")))?,
                None => versions.last().cloned().expect("non-empty"),
            };
            let run = execute_workflow(st, cx, &def);
            st.meta_mut().runs.entry(tenant.into()).or_default().push(run.clone());
            Ok(run)
        })
    }

    pub fn workflow_runs(&self, p: &Principal, tenant: &str) -> Result<Vec<WorkflowRun>> {
        self.read(p, tenant, Permission::Read, "workflow.runs", "-", |st, _| Ok(st.meta.runs.get(tenant).cloned().unwrap_or_default()))
    }

    // ----------------------------------------------------------- migrations

    pub fn migrate(&self, p: &Principal, tenant: &str, mut plan: MigrationPlan, dry_run: bool) -> Result<MigrationReport> {
        let target = format!("{}/{}", plan.domain, plan.entity);
        self.tx(p, tenant, Permission::Administer, if dry_run { "migration.preview" } else { "migration.apply" }, &target, |st, cx| {
            check_kind(&plan.domain, &plan.entity)?;
            for op in &plan.ops {
                op.check()?;
            }
            let schema = st.schema_at(tenant, &plan.domain, &plan.entity, plan.to_version)?;
            let rules = st.rules_for(tenant, &plan.domain, &plan.entity);
            plan.id = new_id("mig");
            let candidates: Vec<Record> = partition(st, tenant, &plan.domain, &plan.entity)
                .filter(|r| r.lifecycle != Lifecycle::Archived && r.schema_version < plan.to_version)
                .cloned()
                .collect();
            let mut report = MigrationReport {
                id: plan.id.clone(),
                dry_run,
                applied: false,
                total: candidates.len(),
                migratable: 0,
                failures: vec![],
                touched: BTreeMap::new(),
            };
            let mut migrated = Vec::new();
            for r in candidates {
                match apply_ops(&r.attributes, &plan.ops) {
                    Err(reason) => report.failures.push(MigrationFailure { record_id: r.id.clone(), reason, report: None }),
                    Ok(attrs) => {
                        let v = validate(&r.name, &attrs, &r.tags, &schema, &rules);
                        if v.valid {
                            report.migratable += 1;
                            migrated.push((r, attrs));
                        } else {
                            report.failures.push(MigrationFailure {
                                record_id: r.id.clone(),
                                reason: "fails target schema validation".into(),
                                report: Some(v),
                            });
                        }
                    }
                }
            }
            if dry_run || !report.failures.is_empty() {
                return Ok(report);
            }
            for (mut r, attrs) in migrated {
                let before = r.version;
                r.attributes = attrs;
                r.schema_version = plan.to_version;
                let r = commit_change(st, cx, r, &format!("migration {}", plan.id), Some(format!("schema migration to v{}", plan.to_version)))?;
                report.touched.insert(r.id.clone(), (before, r.version));
            }
            report.applied = true;
            st.meta_mut().migrations.insert(
                format!("{tenant}/{}", plan.id),
                MigrationRecord { tenant: tenant.into(), plan: plan.clone(), report: report.clone(), rolled_back: false },
            );
            Ok(report)
        })
    }

    pub fn rollback_migration(&self, p: &Principal, tenant: &str, id: &str) -> Result<MigrationRollbackReport> {
        self.tx(p, tenant, Permission::Administer, "migration.rollback", "-", |st, cx| {
            let key = format!("{tenant}/{id}");
            let m = st.meta.migrations.get(&key).cloned().ok_or_else(|| ApsiError::NotFound(format!("migration {id}")))?;
            if m.rolled_back {
                return Err(ApsiError::Conflict("migration already rolled back".into()));
            }
            let mut out = MigrationRollbackReport { id: id.into(), restored: vec![], conflicts: vec![] };
            let mut restore = Vec::new();
            for (rid, (before, after)) in &m.report.touched {
                let Some(stored) = st.records.get(rid).filter(|s| s.record.version == *after) else {
                    out.conflicts.push(rid.clone());
                    continue;
                };
                let snap = stored.history.iter().find(|v| v.version == *before).ok_or_else(|| ApsiError::Internal("missing history".into()))?;
                let mut r = stored.record.clone();
                r.attributes = snap.attributes.clone();
                r.schema_version = snap.schema_version;
                restore.push(r);
            }
            for r in restore {
                out.restored.push(r.id.clone());
                commit_change(st, cx, r, &format!("migration rollback {id}"), Some("migration rollback".into()))?;
            }
            if let Some(m) = st.meta_mut().migrations.get_mut(&key) {
                m.rolled_back = true;
            }
            Ok(out)
        })
    }

    // ------------------------------------------------------------ dashboards

    pub fn dashboard(&self, p: &Principal, tenant: &str, domain: &str) -> Result<DomainDashboard> {
        self.read(p, tenant, Permission::Read, "dashboard", domain, |st, now| {
            let d = catalog().domain(domain).ok_or_else(|| ApsiError::NotFound(format!("domain {domain}")))?;
            let mut dash = DomainDashboard { domain: d.slug.clone(), name: d.name.clone(), ..Default::default() };
            for e in &d.entities {
                let records: Vec<&Record> = partition(st, tenant, domain, &e.slug).collect();
                let q = quality_in(st, now, tenant, domain, &e.slug);
                let count = |l| records.iter().filter(|r| r.lifecycle == l).count();
                dash.total += records.len();
                dash.entities.push(EntitySummary {
                    entity: e.slug.clone(),
                    name: e.name.clone(),
                    schema_version: st.current_schema(tenant, domain, &e.slug).version,
                    total: records.len(),
                    draft: count(Lifecycle::Draft),
                    active: count(Lifecycle::Active),
                    archived: count(Lifecycle::Archived),
                    legal_hold: records.iter().filter(|r| r.legal_hold).count(),
                    validity: q.validity,
                    completeness: q.completeness,
                    last_updated: records.iter().map(|r| r.updated_at).max().and_then(|t| t.format(&time::format_description::well_known::Rfc3339).ok()),
                });
            }
            let prefix = format!("{domain}/");
            dash.recent_activity = st.audit.for_tenant(tenant).filter(|e| e.target.starts_with(&prefix)).rev().take(20).cloned().collect();
            dash.notifications =
                st.meta.outbox.get(tenant).map_or(0, |o| o.iter().filter(|n| st.records.get(&n.record_id).is_none_or(|s| s.record.domain == domain)).count());
            Ok(dash)
        })
    }

    pub fn overview(&self, p: &Principal, tenant: &str) -> Result<Overview> {
        self.read(p, tenant, Permission::Read, "overview", "-", |st, _| {
            let mut counts: HashMap<&str, usize> = HashMap::new();
            for s in st.records.values().filter(|s| s.record.tenant == tenant) {
                *counts.entry(s.record.domain.as_str()).or_default() += 1;
            }
            let domains: Vec<DomainCount> = catalog()
                .domains
                .iter()
                .map(|d| DomainCount {
                    domain: d.slug.clone(),
                    name: d.name.clone(),
                    entity_kinds: d.entities.len(),
                    records: counts.get(d.slug.as_str()).copied().unwrap_or(0),
                })
                .collect();
            Ok(Overview {
                tenant: tenant.into(),
                records: domains.iter().map(|d| d.records).sum(),
                domains,
                audit_events: st.audit.for_tenant(tenant).count(),
                audit_chain_valid: st.audit.verify().is_ok(),
            })
        })
    }

    // ----------------------------------------------------------------- audit

    pub fn audit(&self, p: &Principal, tenant: &str, limit: usize, action_prefix: Option<&str>) -> Result<Vec<AuditEvent>> {
        self.read(p, tenant, Permission::Audit, "audit.list", "-", |st, _| {
            Ok(st
                .audit
                .for_tenant(tenant)
                .filter(|e| action_prefix.is_none_or(|a| e.action.starts_with(a)))
                .rev()
                .take(limit.clamp(1, 1000))
                .cloned()
                .collect())
        })
    }

    pub fn verify_audit(&self, p: &Principal, tenant: &str) -> Result<std::result::Result<usize, u64>> {
        self.read(p, tenant, Permission::Audit, "audit.verify", "-", |st, _| Ok(st.audit.verify().map(|_| st.audit.len())))
    }
}

// --------------------------------------------------------------- internals

fn check_version(r: &Record, expected: u64) -> Result<()> {
    if r.version != expected { Err(ApsiError::VersionConflict { expected, current: r.version }) } else { Ok(()) }
}

fn partition<'a>(st: &'a State, tenant: &str, domain: &str, entity: &str) -> impl Iterator<Item = &'a Record> + use<'a> {
    st.partitions.get(&pkey(tenant, domain, entity)).into_iter().flat_map(|ids| ids.iter()).filter_map(|id| st.records.get(id).map(|s| &s.record))
}

fn create_in(st: &mut State, cx: &Tx<'_>, domain: &str, entity: &str, new: NewRecord) -> Result<Record> {
    check_kind(domain, entity)?;
    if new.lifecycle == Some(Lifecycle::Archived) {
        return Err(ApsiError::Lifecycle("records cannot be created archived".into()));
    }
    if let Some(s) = &new.source {
        s.check()?;
        let key = format!("{}{SEP}{}{SEP}{}", pkey(cx.tenant, domain, entity), s.system, s.id);
        if let Some(existing) = st.source_index.get(&key) {
            return Err(ApsiError::Conflict(format!("source {}:{} already registered as {existing}", s.system, s.id)));
        }
    }
    for input in &new.inputs {
        validate_record_id(input)?;
    }
    if new.transformation.as_ref().is_some_and(|t| t.len() > 1000) {
        return Err(ApsiError::InvalidInput("transformation description too long".into()));
    }
    let schema = st.current_schema(cx.tenant, domain, entity);
    let report = validate(&new.name, &new.attributes, &new.tags, &schema, &st.rules_for(cx.tenant, domain, entity)).into_result()?;
    let record = Record {
        id: new_record_id(),
        tenant: cx.tenant.into(),
        domain: domain.into(),
        entity: entity.into(),
        schema_version: schema.version,
        version: 1,
        lifecycle: new.lifecycle.unwrap_or(Lifecycle::Active),
        name: new.name.trim().to_string(),
        attributes: new.attributes,
        tags: new.tags,
        source: new.source.clone(),
        owner: cx.p.subject.clone(),
        legal_hold: false,
        created_at: cx.now,
        updated_at: cx.now,
        created_by: cx.p.subject.clone(),
        updated_by: cx.p.subject.clone(),
        archived_at: None,
        provenance: vec![ProvenanceEvent {
            at: cx.now,
            actor: cx.p.subject.clone(),
            actor_kind: cx.p.kind,
            action: "create".into(),
            source: new.source,
            transformation: new.transformation,
            inputs: new.inputs,
        }],
    };
    st.index(&record);
    st.put(Stored { history: vec![RecordVersion::of(&record, "created")], record: record.clone() });
    notify_in(st, cx.now, cx.tenant, EventKind::Created, &record);
    if report.warnings > 0 {
        notify_in(st, cx.now, cx.tenant, EventKind::ValidationWarning, &record);
    }
    Ok(record)
}

/// Returns the updated record and whether anything changed.
fn update_in(st: &mut State, cx: &Tx<'_>, id: &str, expected: Option<u64>, patch: RecordPatch) -> Result<(Record, bool)> {
    let old = st.stored(cx.tenant, id)?.record.clone();
    if let Some(e) = expected {
        check_version(&old, e)?;
    }
    if old.lifecycle == Lifecycle::Archived {
        return Err(ApsiError::Lifecycle("archived records are read-only; restore first".into()));
    }
    let mut r = old.clone();
    if let Some(n) = patch.name {
        r.name = n.trim().to_string();
    }
    if let Some(a) = &patch.attributes {
        apply_patch(&mut r.attributes, a, patch.replace_attributes);
    }
    if let Some(t) = patch.tags {
        r.tags = t;
    }
    if r.name == old.name && r.attributes == old.attributes && r.tags == old.tags {
        return Ok((old, false));
    }
    let schema = st.schema_at(cx.tenant, &r.domain, &r.entity, r.schema_version)?;
    let report = validate(&r.name, &r.attributes, &r.tags, &schema, &st.rules_for(cx.tenant, &r.domain, &r.entity)).into_result()?;
    let r = commit_change(st, cx, r, "update", patch.transformation)?;
    notify_in(st, cx.now, cx.tenant, EventKind::Updated, &r);
    if report.warnings > 0 {
        notify_in(st, cx.now, cx.tenant, EventKind::ValidationWarning, &r);
    }
    Ok((r, true))
}

/// Bumps version, stamps provenance, reindexes and appends history.
fn commit_change(st: &mut State, cx: &Tx<'_>, mut r: Record, action: &str, transformation: Option<String>) -> Result<Record> {
    let stored = st.records.get(&r.id).ok_or_else(|| ApsiError::NotFound(r.id.clone()))?;
    let old = stored.record.clone();
    r.version = old.version + 1;
    r.updated_at = cx.now;
    r.updated_by = cx.p.subject.clone();
    r.provenance.push(ProvenanceEvent {
        at: cx.now,
        actor: cx.p.subject.clone(),
        actor_kind: cx.p.kind,
        action: action.into(),
        source: None,
        transformation,
        inputs: vec![],
    });
    st.unindex(&old);
    st.index(&r);
    let stored = st.touch(&r.id).expect("present");
    stored.history.push(RecordVersion::of(&r, action));
    stored.record = r.clone();
    Ok(r)
}

fn bulk_one(st: &mut State, cx: &Tx<'_>, domain: &str, entity: &str, n: NewRecord, upsert: bool) -> Result<(ItemStatus, String)> {
    if let (true, Some(s)) = (upsert, &n.source) {
        s.check()?;
        let key = format!("{}{SEP}{}{SEP}{}", pkey(cx.tenant, domain, entity), s.system, s.id);
        if let Some(id) = st.source_index.get(&key).cloned() {
            let patch = RecordPatch {
                name: Some(n.name),
                attributes: Some(n.attributes),
                replace_attributes: true,
                tags: Some(n.tags),
                transformation: n.transformation.or(Some("bulk upsert".into())),
            };
            let (r, changed) = update_in(st, cx, &id, None, patch)?;
            return Ok((if changed { ItemStatus::Updated } else { ItemStatus::Unchanged }, r.id));
        }
    }
    create_in(st, cx, domain, entity, n).map(|r| (ItemStatus::Created, r.id))
}

fn search_in(st: &State, tenant: &str, q: &Query) -> Result<SearchPage> {
    q.check()?;
    let d = catalog().domain(&q.domain).ok_or_else(|| ApsiError::NotFound(format!("domain {}", q.domain)))?;
    let entities: Vec<&str> = match &q.entity {
        Some(e) => {
            check_kind(&q.domain, e)?;
            vec![e.as_str()]
        }
        None => d.entities.iter().map(|e| e.slug.as_str()).collect(),
    };
    // Partitions are disjoint, so candidates never repeat across entity kinds.
    let mut hits: Vec<&Record> = Vec::new();
    for e in &entities {
        let pk = pkey(tenant, &q.domain, e);
        let Some(base) = st.partitions.get(&pk) else { continue };
        let indexed: Vec<&BTreeSet<String>> = q
            .filters
            .iter()
            .filter(|f| f.op == Op::Eq && !matches!(f.field.as_str(), "name" | "version" | "schema_version"))
            .filter_map(|f| index_key(&f.value).map(|k| (f, k)))
            .map(|(f, k)| st.eq_index.get(&format!("{pk}{SEP}{}{SEP}{k}", f.field)).unwrap_or(&EMPTY))
            .collect();
        let ids: Box<dyn Iterator<Item = &String>> = match indexed.iter().min_by_key(|s| s.len()) {
            Some(smallest) => Box::new(smallest.iter().filter(|id| base.contains(*id))),
            None => Box::new(base.iter()),
        };
        hits.extend(ids.filter_map(|id| st.records.get(id)).map(|s| &s.record).filter(|r| search::matches(r, q)));
    }
    let mut facets: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    for f in &q.facets {
        let counts = facets.entry(f.clone()).or_default();
        for r in &hits {
            for v in search::facet_values(r, f) {
                *counts.entry(v).or_default() += 1;
            }
        }
    }
    search::sort(&mut hits, q.sort, q.descending);
    let total = hits.len();
    let limit = q.effective_limit();
    let items: Vec<Record> = hits.into_iter().skip(q.offset).take(limit).cloned().collect();
    let next_offset = (q.offset + items.len() < total).then_some(q.offset + items.len());
    Ok(SearchPage { items, total, offset: q.offset, limit, next_offset, facets })
}

static EMPTY: BTreeSet<String> = BTreeSet::new();

fn quality_in(st: &State, now: OffsetDateTime, tenant: &str, domain: &str, entity: &str) -> QualityReport {
    let schema = st.current_schema(tenant, domain, entity);
    let rules = st.rules_for(tenant, domain, entity);
    let records: Vec<&Record> = partition(st, tenant, domain, entity).filter(|r| r.lifecycle != Lifecycle::Archived).collect();
    let valid: Vec<bool> = records
        .iter()
        .map(|r| {
            let s = st.schema_at(tenant, domain, entity, r.schema_version).unwrap_or_else(|_| schema.clone());
            validate(&r.name, &r.attributes, &r.tags, &s, &rules).valid
        })
        .collect();
    quality::aggregate(&schema, &records, &valid, now)
}

fn notify_in(st: &mut State, now: OffsetDateTime, tenant: &str, event: EventKind, r: &Record) {
    let rules: Vec<NotificationRule> = st
        .meta
        .notification_rules
        .get(tenant)
        .map(|rs| {
            rs.iter()
                .filter(|rule| rule.domain == r.domain && rule.entity.as_ref().is_none_or(|e| *e == r.entity) && rule.events.contains(&event))
                .filter(|rule| rule.condition.as_ref().is_none_or(|c| search::matches_filter(r, c)))
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    for rule in rules {
        let key = format!("{}{SEP}{}{SEP}{event:?}", rule.id, r.id);
        let t = now.unix_timestamp();
        if st.meta.dedup.get(&key).is_some_and(|last| t - last < rule.dedup_window_secs as i64) {
            st.audit.append(
                now,
                AuditInput {
                    tenant,
                    actor: "notifier",
                    action: "notification.suppressed",
                    target: &format!("{}/{}/{}", r.domain, r.entity, r.id),
                    outcome: Outcome::Allowed,
                    detail: Some(rule.id.clone()),
                },
            );
            continue;
        }
        st.meta_mut().dedup.insert(key, t);
        let n = Notification {
            id: new_id("ntf"),
            rule_id: rule.id.clone(),
            tenant: tenant.into(),
            event,
            record_id: r.id.clone(),
            message: format!("{event:?} {} '{}' ({}/{})", r.id, r.name, r.domain, r.entity),
            channel: rule.channel.clone(),
            at: now,
        };
        st.audit.append(
            now,
            AuditInput {
                tenant,
                actor: "notifier",
                action: "notification.queued",
                target: &format!("{}/{}/{}", r.domain, r.entity, r.id),
                outcome: Outcome::Allowed,
                detail: Some(rule.id.clone()),
            },
        );
        let outbox = st.meta_mut().outbox.entry(tenant.into()).or_default();
        outbox.push(n);
        if outbox.len() > 10_000 {
            outbox.drain(..outbox.len() - 10_000);
        }
    }
}

fn execute_workflow(st: &State, cx: &Tx<'_>, def: &WorkflowDefinition) -> WorkflowRun {
    let mut outputs: BTreeMap<String, Value> = BTreeMap::new();
    let mut sets: HashMap<String, Vec<Record>> = HashMap::new();
    let mut read: BTreeSet<(String, u64)> = BTreeSet::new();
    let mut error = None;
    for step in &def.steps {
        let out: std::result::Result<Value, String> = match &step.op {
            StepOp::Search { query } => search_in(st, cx.tenant, query).map_err(|e| e.to_string()).map(|page| {
                read.extend(page.items.iter().map(|r| (r.id.clone(), r.version)));
                let v = serde_json::json!({ "total": page.total, "ids": page.items.iter().map(|r| r.id.clone()).collect::<Vec<_>>() });
                sets.insert(step.id.clone(), page.items);
                v
            }),
            StepOp::Quality { domain, entity } => {
                let q = quality_in(st, cx.now, cx.tenant, domain, entity);
                read.extend(partition(st, cx.tenant, domain, entity).map(|r| (r.id.clone(), r.version)));
                // Freshness depends on wall-clock time, so it is excluded from reproducible outputs.
                serde_json::to_value(&q)
                    .map(|mut v| {
                        v["mean_freshness_days"] = Value::Null;
                        v["max_freshness_days"] = Value::Null;
                        v
                    })
                    .map_err(|e| e.to_string())
            }
            StepOp::ExtractNumbers { from, field } => Ok(Value::from(
                sets.get(from).map(|rs| rs.iter().filter_map(|r| r.attributes.get(field).and_then(Value::as_f64)).collect::<Vec<f64>>()).unwrap_or_default(),
            )),
            StepOp::Compute { function, input } => {
                let arg = match input {
                    Binding::Literal { value } => value.clone(),
                    Binding::Step { step } => outputs.get(step).cloned().unwrap_or(Value::Null),
                };
                match cx.compute {
                    Some(reg) => reg.call(function, &arg),
                    None => Err("no compute registry configured".into()),
                }
            }
        };
        match out {
            Ok(v) => {
                outputs.insert(step.id.clone(), v);
            }
            Err(e) => {
                error = Some(format!("step {}: {e}", step.id));
                break;
            }
        }
    }
    WorkflowRun {
        id: new_id("run"),
        workflow_id: def.id.clone(),
        workflow_version: def.version,
        status: if error.is_some() { RunStatus::Failed } else { RunStatus::Succeeded },
        definition_hash: sha256_json(def),
        data_fingerprint: sha256_json(&read),
        outputs,
        error,
        actor: cx.p.subject.clone(),
        started_at: cx.now,
        finished_at: cx.now,
    }
}
