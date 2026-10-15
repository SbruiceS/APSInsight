//! `apsi` — automation-friendly command line for APSInsight.
//!
//! * Output: one JSON document on stdout, `{"ok": true, "data": ...}` or
//!   `{"ok": false, "error": {code, message, retryable, details?}}`. Exports write
//!   raw file content (to `--out` or stdout).
//! * Exit codes: 0 ok, 2 invalid input or usage error, 3 validation, 4 not found,
//!   5 conflict, 6 lifecycle, 7 auth, 8 limits, 9 storage, 10 internal.
//! * Never prompts; every input comes from flags, files or stdin (`-`).

use apsi_core::auth::{ActorKind, Principal, Role};
use apsi_core::bulk::BulkRequest;
use apsi_core::catalog::{catalog, features};
use apsi_core::engine::ImportBody;
use apsi_core::exchange::Format;
use apsi_core::migration::MigrationPlan;
use apsi_core::notify::NotificationRule;
use apsi_core::record::{Lifecycle, NewRecord, RecordPatch};
use apsi_core::retention::RetentionPolicy;
use apsi_core::schema::EntitySchema;
use apsi_core::search::{Filter, Op, Query};
use apsi_core::validation::ValidationRule;
use apsi_core::workflow::{ComputeRegistry, WorkflowDefinition};
use apsi_core::{ApsiError, Engine, docs, traceability};
use apsi_science::ScienceRegistry;
use clap::{Parser, Subcommand};
use serde_json::{Value, json};
use std::io::Read;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Instant;

#[derive(Parser)]
#[command(name = "apsi", version, about = "APSInsight command line")]
struct Cli {
    /// Persistent data directory.
    #[arg(long, global = true, env = "APSI_DATA_DIR", default_value = ".apsi-data")]
    data_dir: PathBuf,
    /// Tenant to act in.
    #[arg(long, global = true, env = "APSI_TENANT", default_value = "default")]
    tenant: String,
    /// Actor recorded in provenance and audit.
    #[arg(long, global = true, env = "APSI_ACTOR", default_value = "cli")]
    actor: String,
    /// Role of the local operator: viewer, editor, auditor or admin.
    #[arg(long, global = true, env = "APSI_ROLE", default_value = "admin")]
    role: String,
    /// Pretty-print JSON output.
    #[arg(long, global = true)]
    pretty: bool,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Feature register: list, show, traceability.
    Features {
        #[command(subcommand)]
        cmd: FeatureCmd,
    },
    /// Domains and entity kinds.
    Catalog {
        /// Show entity kinds of this domain.
        domain: Option<String>,
    },
    Records {
        #[command(subcommand)]
        cmd: RecordCmd,
    },
    Schema {
        #[command(subcommand)]
        cmd: SchemaCmd,
    },
    Rules {
        #[command(subcommand)]
        cmd: RulesCmd,
    },
    /// Quality indicators for an entity kind.
    Quality { domain: String, entity: String },
    Retention {
        #[command(subcommand)]
        cmd: RetentionCmd,
    },
    /// Apply (or preview) a schema migration from a plan file `{to_version, ops}`.
    Migrate {
        domain: String,
        entity: String,
        #[arg(long)]
        file: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// Roll back a previously applied migration.
    MigrateRollback { id: String },
    Notify {
        #[command(subcommand)]
        cmd: NotifyCmd,
    },
    Workflow {
        #[command(subcommand)]
        cmd: WorkflowCmd,
    },
    /// Run a science function (`apsi compute list` to see them).
    Compute {
        function: String,
        /// JSON input (or @file / - for stdin).
        #[arg(long, default_value = "{}")]
        json: String,
    },
    /// Overview of all domains, or the dashboard of one domain.
    Dashboard { domain: Option<String> },
    Audit {
        #[command(subcommand)]
        cmd: AuditCmd,
    },
    /// Capability discovery for planned ASPF products (all optional; none exist yet).
    Integrations,
    /// Regenerate docs/domains/*.md, docs/traceability.csv, docs/STATUS.md and docs/UNITS.md.
    Docs {
        #[command(subcommand)]
        cmd: DocsCmd,
    },
    /// Measure representative operations on an in-memory engine.
    Bench {
        #[arg(long, default_value_t = 2000)]
        records: usize,
        /// Use a temporary persistent engine (journal + fsync per commit) instead of memory.
        #[arg(long)]
        persistent: bool,
    },
}

#[derive(Subcommand)]
enum FeatureCmd {
    List {
        #[arg(long)]
        domain: Option<String>,
        #[arg(long)]
        capability: Option<String>,
        #[arg(long)]
        q: Option<String>,
        #[arg(long, default_value_t = 3100)]
        limit: usize,
    },
    Show {
        id: String,
    },
    Trace {
        #[arg(long, default_value = "csv")]
        format: String,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Feature counts per honest status.
    Status,
}

#[derive(Subcommand)]
enum RecordCmd {
    Create {
        domain: String,
        entity: String,
        /// NewRecord JSON (or @file / -).
        #[arg(long)]
        json: String,
    },
    Get {
        id: String,
    },
    Update {
        id: String,
        #[arg(long)]
        expected_version: u64,
        /// RecordPatch JSON (or @file / -).
        #[arg(long)]
        json: String,
    },
    Archive {
        id: String,
        #[arg(long)]
        expected_version: u64,
    },
    Restore {
        id: String,
        #[arg(long)]
        expected_version: u64,
    },
    Delete {
        id: String,
    },
    Hold {
        id: String,
        #[arg(long)]
        release: bool,
    },
    Search {
        domain: String,
        #[arg(long)]
        entity: Option<String>,
        #[arg(long)]
        q: Option<String>,
        #[arg(long = "tag")]
        tags: Vec<String>,
        /// `field:op:value`, op one of eq ne gt gte lt lte contains in exists.
        #[arg(long = "filter")]
        filters: Vec<String>,
        #[arg(long = "facet")]
        facets: Vec<String>,
        #[arg(long)]
        include_archived: bool,
        #[arg(long, default_value_t = 0)]
        offset: usize,
        #[arg(long)]
        limit: Option<usize>,
    },
    Validate {
        domain: String,
        entity: String,
        #[arg(long)]
        json: String,
    },
    History {
        id: String,
    },
    Diff {
        id: String,
        #[arg(long)]
        from: u64,
        #[arg(long)]
        to: u64,
    },
    Rollback {
        id: String,
        #[arg(long)]
        to_version: u64,
        #[arg(long)]
        expected_version: u64,
    },
    /// Bulk create/upsert from a JSON file holding a BulkRequest or an array of records.
    Bulk {
        domain: String,
        entity: String,
        #[arg(long)]
        file: String,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        upsert: bool,
        #[arg(long)]
        idempotency_key: Option<String>,
    },
    Import {
        domain: String,
        entity: String,
        #[arg(long)]
        file: String,
        /// csv, json or ndjson; inferred from the file extension when omitted.
        #[arg(long)]
        format: Option<String>,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        upsert: bool,
    },
    Export {
        domain: String,
        entity: String,
        #[arg(long, default_value = "json")]
        format: String,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long)]
        include_archived: bool,
    },
}

#[derive(Subcommand)]
enum SchemaCmd {
    Show {
        domain: String,
        entity: String,
    },
    Publish {
        domain: String,
        entity: String,
        #[arg(long)]
        file: String,
        #[arg(long)]
        allow_breaking: bool,
    },
}

#[derive(Subcommand)]
enum RulesCmd {
    Show {
        domain: String,
        entity: String,
    },
    Set {
        domain: String,
        entity: String,
        #[arg(long)]
        file: String,
    },
}

#[derive(Subcommand)]
enum RetentionCmd {
    Show {
        domain: String,
        entity: String,
    },
    Set {
        domain: String,
        entity: String,
        #[arg(long)]
        archive_after_days: Option<u32>,
        #[arg(long)]
        purge_after_days: Option<u32>,
    },
    Apply {
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
enum NotifyCmd {
    Add {
        #[arg(long)]
        json: String,
    },
    List,
    Remove {
        id: String,
    },
    Outbox {
        #[arg(long, default_value_t = 100)]
        limit: usize,
    },
}

#[derive(Subcommand)]
enum WorkflowCmd {
    Register {
        #[arg(long)]
        file: String,
    },
    List,
    Run {
        id: String,
        #[arg(long)]
        version: Option<u32>,
    },
    Runs,
}

#[derive(Subcommand)]
enum AuditCmd {
    List {
        #[arg(long, default_value_t = 100)]
        limit: usize,
        #[arg(long)]
        action: Option<String>,
    },
    Verify,
}

#[derive(Subcommand)]
enum DocsCmd {
    Generate {
        #[arg(long, default_value = "docs")]
        out: PathBuf,
    },
}

/// Output of a command: structured JSON or raw text (exports).
enum Out {
    Json(Value),
    Raw(String),
}

fn read_input(spec: &str) -> Result<String, ApsiError> {
    if spec == "-" {
        let mut s = String::new();
        std::io::stdin().read_to_string(&mut s).map_err(|e| ApsiError::InvalidInput(format!("stdin: {e}")))?;
        Ok(s)
    } else if let Some(path) = spec.strip_prefix('@') {
        std::fs::read_to_string(path).map_err(|e| ApsiError::InvalidInput(format!("{path}: {e}")))
    } else {
        Ok(spec.to_string())
    }
}

fn read_file(path: &str) -> Result<String, ApsiError> {
    read_input(&if path == "-" { path.to_string() } else { format!("@{path}") })
}

fn parse<T: serde::de::DeserializeOwned>(text: &str) -> Result<T, ApsiError> {
    serde_json::from_str(text).map_err(|e| ApsiError::InvalidInput(format!("invalid JSON: {e}")))
}

fn to_json<T: serde::Serialize>(v: T) -> Result<Out, ApsiError> {
    serde_json::to_value(v).map(Out::Json).map_err(|e| ApsiError::Internal(e.to_string()))
}

fn parse_filter(s: &str) -> Result<Filter, ApsiError> {
    let mut parts = s.splitn(3, ':');
    let (Some(field), Some(op)) = (parts.next(), parts.next()) else {
        return Err(ApsiError::InvalidInput(format!("filter `{s}` must be field:op:value")));
    };
    let op: Op = serde_json::from_value(json!(op)).map_err(|_| ApsiError::InvalidInput(format!("unknown filter op {op}")))?;
    let raw = parts.next().unwrap_or("true");
    let value = serde_json::from_str(raw).unwrap_or_else(|_| Value::from(raw));
    Ok(Filter { field: field.into(), op, value })
}

fn run(cli: Cli) -> Result<Out, ApsiError> {
    let role = Role::parse(&cli.role)?;
    let p = Principal::new(&cli.actor, &cli.tenant, &[role], if cli.actor.starts_with("svc-") { ActorKind::Service } else { ActorKind::User });
    let t = cli.tenant.as_str();
    let open = || Engine::open(&cli.data_dir).map(|e| e.with_compute(Arc::new(ScienceRegistry)));
    match cli.cmd {
        Cmd::Features { cmd } => match cmd {
            FeatureCmd::List { domain, capability, q, limit } => {
                let c = catalog();
                let q = q.map(|s| s.to_lowercase());
                let items: Vec<_> = features()
                    .iter()
                    .filter(|f| {
                        let (d, cap) = c.feature_coordinates(&f.id).expect("valid ids");
                        domain.as_ref().is_none_or(|x| *x == d.slug)
                            && capability.as_ref().is_none_or(|x| *x == cap.slug)
                            && q.as_ref().is_none_or(|q| f.requirement.to_lowercase().contains(q) || f.id.to_lowercase().contains(q))
                    })
                    .take(limit)
                    .collect();
                to_json(json!({ "count": items.len(), "items": items }))
            }
            FeatureCmd::Show { id } => {
                let row = traceability::row(&id).ok_or_else(|| ApsiError::NotFound(format!("feature {id}")))?;
                to_json(json!({ "feature": features().iter().find(|f| f.id == id), "evidence": row }))
            }
            FeatureCmd::Trace { format, out } => {
                let text = match format.as_str() {
                    "csv" => traceability::csv(),
                    "json" => serde_json::to_string_pretty(&traceability::rows()).map_err(|e| ApsiError::Internal(e.to_string()))?,
                    other => return Err(ApsiError::InvalidInput(format!("unknown format {other}"))),
                };
                write_out(out, text)
            }
            FeatureCmd::Status => to_json(json!({
                "total": features().len(),
                "by_status": traceability::status_counts().into_iter().map(|(s, n)| json!({ "status": s, "label": s.label(), "features": n })).collect::<Vec<_>>(),
            })),
        },
        Cmd::Integrations => to_json(json!(apsi_contracts::discover_all())),
        Cmd::Catalog { domain } => match domain {
            None => to_json(
                catalog()
                    .domains
                    .iter()
                    .map(|d| json!({ "index": d.index, "slug": d.slug, "name": d.name, "entity_kinds": d.entities.len() }))
                    .collect::<Vec<_>>(),
            ),
            Some(d) => to_json(catalog().domain(&d).ok_or_else(|| ApsiError::NotFound(format!("domain {d}")))?),
        },
        Cmd::Records { cmd } => {
            let e = open()?;
            match cmd {
                RecordCmd::Create { domain, entity, json } => to_json(e.create(&p, t, &domain, &entity, parse::<NewRecord>(&read_input(&json)?)?)?),
                RecordCmd::Get { id } => to_json(e.get(&p, t, &id)?),
                RecordCmd::Update { id, expected_version, json } => {
                    to_json(e.update(&p, t, &id, expected_version, parse::<RecordPatch>(&read_input(&json)?)?)?)
                }
                RecordCmd::Archive { id, expected_version } => to_json(e.transition(&p, t, &id, expected_version, Lifecycle::Archived)?),
                RecordCmd::Restore { id, expected_version } => to_json(e.transition(&p, t, &id, expected_version, Lifecycle::Active)?),
                RecordCmd::Delete { id } => to_json(e.delete(&p, t, &id).map(|_| json!({ "deleted": id }))?),
                RecordCmd::Hold { id, release } => to_json(e.set_legal_hold(&p, t, &id, !release)?),
                RecordCmd::Search { domain, entity, q, tags, filters, facets, include_archived, offset, limit } => {
                    let filters = filters.iter().map(|f| parse_filter(f)).collect::<Result<Vec<_>, _>>()?;
                    let query = Query { domain, entity, text: q, tags, filters, facets, include_archived, offset, limit, ..Default::default() };
                    to_json(e.search(&p, t, &query)?)
                }
                RecordCmd::Validate { domain, entity, json } => {
                    to_json(e.validate_preview(&p, t, &domain, &entity, &parse::<NewRecord>(&read_input(&json)?)?)?)
                }
                RecordCmd::History { id } => to_json(e.history(&p, t, &id)?),
                RecordCmd::Diff { id, from, to } => to_json(e.diff(&p, t, &id, from, to)?),
                RecordCmd::Rollback { id, to_version, expected_version } => to_json(e.rollback(&p, t, &id, to_version, expected_version)?),
                RecordCmd::Bulk { domain, entity, file, dry_run, upsert, idempotency_key } => {
                    let v: Value = parse(&read_file(&file)?)?;
                    let mut req: BulkRequest =
                        if v.is_array() { BulkRequest { items: parse(&v.to_string())?, ..Default::default() } } else { parse(&v.to_string())? };
                    req.dry_run |= dry_run;
                    req.upsert |= upsert;
                    if idempotency_key.is_some() {
                        req.idempotency_key = idempotency_key;
                    }
                    to_json(e.bulk(&p, t, &domain, &entity, req)?)
                }
                RecordCmd::Import { domain, entity, file, format, dry_run, upsert } => {
                    let format = match format {
                        Some(f) => Format::parse(&f)?,
                        None if file.ends_with(".csv") => Format::Csv,
                        None if file.ends_with(".ndjson") => Format::Ndjson,
                        None => Format::Json,
                    };
                    let text = read_file(&file)?;
                    let options = BulkRequest { dry_run, upsert, ..Default::default() };
                    to_json(e.import(&p, t, &domain, &entity, ImportBody { format, text: &text }, options)?)
                }
                RecordCmd::Export { domain, entity, format, out, include_archived } => {
                    let text = e.export(&p, t, &domain, &entity, Format::parse(&format)?, include_archived)?;
                    write_out(out, text)
                }
            }
        }
        Cmd::Schema { cmd } => {
            let e = open()?;
            match cmd {
                SchemaCmd::Show { domain, entity } => to_json(e.schemas(&p, t, &domain, &entity)?),
                SchemaCmd::Publish { domain, entity, file, allow_breaking } => {
                    let mut s: EntitySchema = parse(&read_file(&file)?)?;
                    s.domain = domain;
                    s.entity = entity;
                    to_json(e.publish_schema(&p, t, s, allow_breaking)?)
                }
            }
        }
        Cmd::Rules { cmd } => {
            let e = open()?;
            match cmd {
                RulesCmd::Show { domain, entity } => to_json(e.rules(&p, t, &domain, &entity)?),
                RulesCmd::Set { domain, entity, file } => to_json(e.set_rules(&p, t, &domain, &entity, parse::<Vec<ValidationRule>>(&read_file(&file)?)?)?),
            }
        }
        Cmd::Quality { domain, entity } => to_json(open()?.quality(&p, t, &domain, &entity)?),
        Cmd::Retention { cmd } => {
            let e = open()?;
            match cmd {
                RetentionCmd::Show { domain, entity } => to_json(e.retention(&p, t, &domain, &entity)?),
                RetentionCmd::Set { domain, entity, archive_after_days, purge_after_days } => {
                    to_json(e.set_retention(&p, t, &domain, &entity, RetentionPolicy { archive_after_days, purge_archived_after_days: purge_after_days })?)
                }
                RetentionCmd::Apply { dry_run } => to_json(e.apply_retention(&p, t, dry_run)?),
            }
        }
        Cmd::Migrate { domain, entity, file, dry_run } => {
            let v: Value = parse(&read_file(&file)?)?;
            let plan =
                MigrationPlan { id: String::new(), domain, entity, to_version: parse(&v["to_version"].to_string())?, ops: parse(&v["ops"].to_string())? };
            to_json(open()?.migrate(&p, t, plan, dry_run)?)
        }
        Cmd::MigrateRollback { id } => to_json(open()?.rollback_migration(&p, t, &id)?),
        Cmd::Notify { cmd } => {
            let e = open()?;
            match cmd {
                NotifyCmd::Add { json } => to_json(e.add_notification_rule(&p, t, parse::<NotificationRule>(&read_input(&json)?)?)?),
                NotifyCmd::List => to_json(e.notification_rules(&p, t)?),
                NotifyCmd::Remove { id } => to_json(e.delete_notification_rule(&p, t, &id).map(|_| json!({ "deleted": id }))?),
                NotifyCmd::Outbox { limit } => to_json(e.notifications(&p, t, limit)?),
            }
        }
        Cmd::Workflow { cmd } => {
            let e = open()?;
            match cmd {
                WorkflowCmd::Register { file } => to_json(e.register_workflow(&p, t, parse::<WorkflowDefinition>(&read_file(&file)?)?)?),
                WorkflowCmd::List => to_json(e.workflows(&p, t)?),
                WorkflowCmd::Run { id, version } => to_json(e.run_workflow(&p, t, &id, version)?),
                WorkflowCmd::Runs => to_json(e.workflow_runs(&p, t)?),
            }
        }
        Cmd::Compute { function, json } => {
            let reg = ScienceRegistry;
            if function == "list" {
                return to_json(reg.functions());
            }
            let input: Value = parse(&read_input(&json)?)?;
            to_json(reg.call(&function, &input).map_err(ApsiError::InvalidInput)?)
        }
        Cmd::Dashboard { domain } => {
            let e = open()?;
            match domain {
                Some(d) => to_json(e.dashboard(&p, t, &d)?),
                None => to_json(e.overview(&p, t)?),
            }
        }
        Cmd::Audit { cmd } => {
            let e = open()?;
            match cmd {
                AuditCmd::List { limit, action } => to_json(e.audit(&p, t, limit, action.as_deref())?),
                AuditCmd::Verify => to_json(match e.verify_audit(&p, t)? {
                    Ok(n) => json!({ "valid": true, "events": n }),
                    Err(seq) => json!({ "valid": false, "broken_at": seq }),
                }),
            }
        }
        Cmd::Docs { cmd: DocsCmd::Generate { out } } => {
            let dir = out.join("domains");
            std::fs::create_dir_all(&dir).map_err(|e| ApsiError::Storage(e.to_string()))?;
            let write = |path: PathBuf, text: String| std::fs::write(&path, text).map_err(|e| ApsiError::Storage(format!("{}: {e}", path.display())));
            for d in &catalog().domains {
                write(dir.join(format!("{}.md", d.slug)), docs::domain_markdown(d))?;
            }
            write(dir.join("README.md"), docs::index_markdown())?;
            write(out.join("traceability.csv"), traceability::csv())?;
            write(out.join("STATUS.md"), traceability::status_markdown())?;
            write(out.join("UNITS.md"), apsi_science::units::catalogue_markdown())?;
            to_json(json!({ "domains": catalog().domains.len(), "out": out }))
        }
        Cmd::Bench { records, persistent } => bench(records, persistent),
    }
}

fn write_out(out: Option<PathBuf>, text: String) -> Result<Out, ApsiError> {
    match out {
        Some(path) => {
            std::fs::write(&path, &text).map_err(|e| ApsiError::Storage(format!("{}: {e}", path.display())))?;
            to_json(json!({ "written": path, "bytes": text.len() }))
        }
        None => Ok(Out::Raw(text)),
    }
}

/// Measures, never assumes: numbers come from timing this process on this machine.
fn bench(n: usize, persistent: bool) -> Result<Out, ApsiError> {
    if !(1..=200_000).contains(&n) {
        return Err(ApsiError::InvalidInput("records must be within 1..=200000".into()));
    }
    let dir = std::env::temp_dir().join(format!("apsi-bench-{}", std::process::id()));
    let e = if persistent { Engine::open(&dir)? } else { Engine::in_memory() };
    let p = Principal::new("bench", "bench", &[Role::Admin], ActorKind::Service);
    let (d, k) = ("crop-yield-and-production", "yield-observations");
    let mut results = Vec::new();
    let mut measure = |name: &str, ops: usize, f: &mut dyn FnMut() -> Result<(), ApsiError>| -> Result<(), ApsiError> {
        let start = Instant::now();
        f()?;
        let secs = start.elapsed().as_secs_f64();
        results.push(json!({ "operation": name, "ops": ops, "seconds": secs, "ops_per_second": if secs > 0.0 { ops as f64 / secs } else { 0.0 } }));
        Ok(())
    };
    let mut ids = Vec::with_capacity(n);
    measure("create", n, &mut || {
        for i in 0..n {
            let crop = ["wheat", "rice", "maize"][i % 3];
            let r = e.create(
                &p,
                "bench",
                d,
                k,
                NewRecord {
                    name: format!("obs {i}"),
                    attributes: json!({"value": (i % 97) as f64 / 10.0, "crop": crop}).as_object().cloned().unwrap_or_default(),
                    ..Default::default()
                },
            )?;
            ids.push(r.id);
        }
        Ok(())
    })?;
    let lookups = n.min(5000);
    measure("get", lookups, &mut || {
        for id in ids.iter().take(lookups) {
            e.get(&p, "bench", id)?;
        }
        Ok(())
    })?;
    let q = Query {
        domain: d.into(),
        entity: Some(k.into()),
        filters: vec![Filter { field: "crop".into(), op: Op::Eq, value: json!("rice") }],
        limit: Some(50),
        ..Default::default()
    };
    measure("indexed_search", 100, &mut || {
        for _ in 0..100 {
            e.search(&p, "bench", &q)?;
        }
        Ok(())
    })?;
    measure("export_csv", 1, &mut || e.export(&p, "bench", d, k, Format::Csv, false).map(|_| ()))?;
    drop(e);
    if persistent {
        let _ = std::fs::remove_dir_all(&dir);
    }
    to_json(json!({
        "workload": { "records": n, "entity_kind": format!("{d}/{k}"), "storage": if persistent { "persistent journal, fsync per commit" } else { "in-memory (no persistence)" }, "attributes_per_record": 2, "threads": 1 },
        "note": "Measured on this machine in this process; not a guarantee for other hardware or persistent storage.",
        "results": results,
    }))
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let pretty = cli.pretty;
    let render = |v: &Value| if pretty { serde_json::to_string_pretty(v) } else { serde_json::to_string(v) }.unwrap_or_default();
    match run(cli) {
        Ok(Out::Json(data)) => {
            println!("{}", render(&json!({ "ok": true, "data": data })));
            ExitCode::SUCCESS
        }
        Ok(Out::Raw(text)) => {
            print!("{text}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            println!("{}", render(&json!({ "ok": false, "error": e.body() })));
            ExitCode::from(u8::try_from(e.exit_code()).unwrap_or(10))
        }
    }
}
