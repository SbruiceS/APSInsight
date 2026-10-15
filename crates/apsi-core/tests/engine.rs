//! Engine integration tests: one or more tests per capability family. Test names are
//! referenced from the traceability matrix (docs/traceability.csv).

use apsi_core::auth::{ActorKind, Principal, Role};
use apsi_core::bulk::{BulkRequest, ItemStatus};
use apsi_core::catalog::catalog;
use apsi_core::engine::{ImportBody, ManualClock};
use apsi_core::exchange::{FieldMapping, Format};
use apsi_core::migration::{MigrationOp, MigrationPlan};
use apsi_core::notify::{Channel, EventKind, NotificationRule};
use apsi_core::record::{Lifecycle, NewRecord, RecordPatch, SourceRef};
use apsi_core::retention::RetentionPolicy;
use apsi_core::schema::{EntitySchema, FieldDef, FieldType};
use apsi_core::search::{Filter, Op, Query};
use apsi_core::validation::{RuleKind, Severity, ValidationRule};
use apsi_core::workflow::{Binding, ComputeRegistry, FunctionSpec, Step, StepOp, WorkflowDefinition};
use apsi_core::{ApsiError, Engine};
use serde_json::{Value, json};
use std::sync::Arc;
use time::{Duration, OffsetDateTime};

const T: &str = "farm-co";
const D: &str = "soil-profile-and-classification";
const E: &str = "ph";

fn admin() -> Principal {
    Principal::new("ada", T, &[Role::Admin], ActorKind::User)
}
fn editor() -> Principal {
    Principal::new("eve", T, &[Role::Editor], ActorKind::User)
}
fn viewer() -> Principal {
    Principal::new("vic", T, &[Role::Viewer], ActorKind::User)
}
fn outsider() -> Principal {
    Principal::new("mallory", "other-co", &[Role::Admin], ActorKind::User)
}

fn rec(name: &str, attrs: Value) -> NewRecord {
    NewRecord { name: name.into(), attributes: attrs.as_object().unwrap().clone(), ..Default::default() }
}

fn clocked() -> (Engine, ManualClock) {
    let clock = ManualClock::new(OffsetDateTime::from_unix_timestamp(1_780_000_000).unwrap());
    (Engine::in_memory().with_clock(Arc::new(clock.clone())), clock)
}

#[test]
fn registration_assigns_stable_ids_and_preserves_source_identity() {
    let e = Engine::in_memory();
    let mut n = rec("Plot 7 topsoil", json!({"value": 6.4, "unit": "pH"}));
    n.source = Some(SourceRef { system: "lims".into(), id: "S-0007".into() });
    let r = e.create(&editor(), T, D, E, n.clone()).unwrap();
    assert!(r.id.starts_with("rec_"));
    assert_eq!(r.version, 1);
    assert_eq!(r.source.as_ref().unwrap().id, "S-0007");
    assert_eq!(e.get(&viewer(), T, &r.id).unwrap(), r);
    // same source identity again is a conflict, not a duplicate
    assert!(matches!(e.create(&editor(), T, D, E, n), Err(ApsiError::Conflict(_))));
    let audit = e.audit(&admin(), T, 10, Some("record.create")).unwrap();
    assert_eq!(audit.len(), 2);
}

#[test]
fn unknown_entity_kinds_and_malformed_input_are_rejected() {
    let e = Engine::in_memory();
    assert!(matches!(e.create(&editor(), T, "no-such-domain", E, rec("x", json!({}))), Err(ApsiError::NotFound(_))));
    assert!(matches!(e.create(&editor(), T, D, "nope", rec("x", json!({}))), Err(ApsiError::NotFound(_))));
    assert!(matches!(e.create(&editor(), T, D, E, rec("", json!({}))), Err(ApsiError::Validation(_))));
    assert!(matches!(e.create(&editor(), T, D, E, rec("x", json!({"value": "acidic"}))), Err(ApsiError::Validation(_))));
    assert!(matches!(e.get(&viewer(), T, "../../etc/passwd"), Err(ApsiError::InvalidInput(_))));
}

#[test]
fn crud_lifecycle_and_archive_constraints() {
    let e = Engine::in_memory();
    let mut n = rec("draft", json!({}));
    n.lifecycle = Some(Lifecycle::Draft);
    let r = e.create(&editor(), T, D, E, n).unwrap();
    assert_eq!(r.lifecycle, Lifecycle::Draft);
    let r = e.transition(&editor(), T, &r.id, 1, Lifecycle::Active).unwrap();
    let r = e.update(&editor(), T, &r.id, 2, RecordPatch { name: Some("renamed".into()), ..Default::default() }).unwrap();
    assert_eq!((r.version, r.name.as_str()), (3, "renamed"));
    let r = e.transition(&editor(), T, &r.id, 3, Lifecycle::Archived).unwrap();
    assert!(r.archived_at.is_some());
    // archived records are read-only
    assert!(matches!(e.update(&editor(), T, &r.id, 4, RecordPatch { name: Some("x".into()), ..Default::default() }), Err(ApsiError::Lifecycle(_))));
    // restore needs an administrator
    assert!(matches!(e.transition(&editor(), T, &r.id, 4, Lifecycle::Active), Err(ApsiError::Forbidden(_))));
    let r = e.transition(&admin(), T, &r.id, 4, Lifecycle::Active).unwrap();
    assert_eq!(r.lifecycle, Lifecycle::Active);
    assert!(matches!(e.transition(&admin(), T, &r.id, 5, Lifecycle::Draft), Err(ApsiError::Lifecycle(_))));
}

#[test]
fn access_control_allows_and_denies_across_roles_and_tenants() {
    let e = Engine::in_memory();
    assert!(matches!(e.create(&viewer(), T, D, E, rec("x", json!({}))), Err(ApsiError::Forbidden(_))));
    let r = e.create(&editor(), T, D, E, rec("x", json!({}))).unwrap();
    assert!(e.get(&viewer(), T, &r.id).is_ok());
    // other tenants cannot read, even as admin of their own tenant, nor probe ids
    assert!(matches!(e.get(&outsider(), T, &r.id), Err(ApsiError::Forbidden(_))));
    assert!(matches!(e.get(&outsider(), "other-co", &r.id), Err(ApsiError::NotFound(_))));
    assert!(matches!(e.audit(&editor(), T, 10, None), Err(ApsiError::Forbidden(_))));
    assert!(matches!(e.set_rules(&editor(), T, D, E, vec![]), Err(ApsiError::Forbidden(_))));
    let denied = e.audit(&admin(), T, 100, None).unwrap().into_iter().filter(|a| a.outcome == apsi_core::audit::Outcome::Denied).count();
    assert!(denied >= 3, "denials are audited");
}

#[test]
fn optimistic_concurrency_rejects_stale_writes() {
    let e = Engine::in_memory();
    let r = e.create(&editor(), T, D, E, rec("x", json!({"value": 6.0}))).unwrap();
    e.update(&editor(), T, &r.id, 1, RecordPatch { attributes: Some(json!({"value": 6.1}).as_object().unwrap().clone()), ..Default::default() }).unwrap();
    let err = e
        .update(&editor(), T, &r.id, 1, RecordPatch { attributes: Some(json!({"value": 9.9}).as_object().unwrap().clone()), ..Default::default() })
        .unwrap_err();
    assert!(matches!(err, ApsiError::VersionConflict { expected: 1, current: 2 }));
    assert!(err.retryable());
    assert_eq!(e.get(&viewer(), T, &r.id).unwrap().attributes["value"], 6.1);
}

#[test]
fn concurrent_writers_never_lose_updates() {
    let e = Arc::new(Engine::in_memory());
    let r = e.create(&editor(), T, D, E, rec("counter", json!({"value": 0}))).unwrap();
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let e = e.clone();
            let id = r.id.clone();
            std::thread::spawn(move || {
                let mut done = 0;
                while done < 25 {
                    let cur = e.get(&editor(), T, &id).unwrap();
                    let v = cur.attributes["value"].as_i64().unwrap() + 1;
                    let patch = RecordPatch { attributes: Some(json!({"value": v}).as_object().unwrap().clone()), ..Default::default() };
                    match e.update(&editor(), T, &id, cur.version, patch) {
                        Ok(_) => done += 1,
                        Err(ApsiError::VersionConflict { .. }) => continue,
                        Err(other) => panic!("{other}"),
                    }
                }
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }
    let r = e.get(&viewer(), T, &r.id).unwrap();
    assert_eq!(r.attributes["value"], 200);
    assert_eq!(r.version, 201);
}

#[test]
fn search_supports_exact_ranges_facets_and_pagination() {
    let e = Engine::in_memory();
    for i in 0..30 {
        let mut n = rec(&format!("sample {i:02}"), json!({"value": 4.0 + f64::from(i) * 0.2, "texture": if i % 3 == 0 { "clay" } else { "loam" }}));
        n.tags = vec![if i % 2 == 0 { "even" } else { "odd" }.into()];
        e.create(&editor(), T, D, E, n).unwrap();
    }
    let q = Query {
        domain: D.into(),
        entity: Some(E.into()),
        filters: vec![Filter { field: "texture".into(), op: Op::Eq, value: json!("loam") }, Filter { field: "value".into(), op: Op::Gte, value: json!(6.0) }],
        facets: vec!["tags".into(), "texture".into()],
        limit: Some(5),
        ..Default::default()
    };
    let page = e.search(&viewer(), T, &q).unwrap();
    assert_eq!(page.total, 14);
    assert_eq!(page.items.len(), 5);
    assert_eq!(page.next_offset, Some(5));
    assert_eq!(page.facets["texture"]["loam"], 14);
    assert_eq!(page.facets["tags"]["even"] + page.facets["tags"]["odd"], 14);
    let last = e.search(&viewer(), T, &Query { offset: 10, ..q.clone() }).unwrap();
    assert_eq!((last.items.len(), last.next_offset), (4, None));
    let text = e.search(&viewer(), T, &Query { domain: D.into(), text: Some("SAMPLE 2".into()), ..Default::default() }).unwrap();
    assert_eq!(text.total, 10);
    assert!(e.search(&viewer(), T, &Query { domain: D.into(), limit: Some(10_000), ..Default::default() }).is_err());
}

#[test]
fn bulk_import_reports_partial_failures_dry_runs_and_is_idempotent() {
    let e = Engine::in_memory();
    let items = vec![rec("ok-1", json!({"value": 5.5})), rec("", json!({})), rec("ok-2", json!({"value": "bad"})), rec("ok-3", json!({}))];
    let preview = e.bulk(&editor(), T, D, E, BulkRequest { items: items.clone(), dry_run: true, ..Default::default() }).unwrap();
    assert_eq!((preview.created, preview.failed), (2, 2));
    assert_eq!(e.search(&viewer(), T, &Query { domain: D.into(), ..Default::default() }).unwrap().total, 0, "dry run persists nothing");
    let req = BulkRequest { items: items.clone(), idempotency_key: Some("batch-1".into()), ..Default::default() };
    let first = e.bulk(&editor(), T, D, E, req.clone()).unwrap();
    assert_eq!(first.results[1].status, ItemStatus::Failed);
    assert_eq!(first.results[1].error.as_ref().unwrap().code, "validation_failed");
    let replay = e.bulk(&editor(), T, D, E, req).unwrap();
    assert!(replay.replayed);
    assert_eq!(e.search(&viewer(), T, &Query { domain: D.into(), ..Default::default() }).unwrap().total, 2);
    let changed = BulkRequest { items: vec![rec("other", json!({}))], idempotency_key: Some("batch-1".into()), ..Default::default() };
    assert!(matches!(e.bulk(&editor(), T, D, E, changed), Err(ApsiError::Conflict(_))));
}

#[test]
fn bulk_upsert_by_source_identity() {
    let e = Engine::in_memory();
    let mut a = rec("A", json!({"value": 1}));
    a.source = Some(SourceRef { system: "erp".into(), id: "1".into() });
    let r1 = e.bulk(&editor(), T, D, E, BulkRequest { items: vec![a.clone()], upsert: true, ..Default::default() }).unwrap();
    assert_eq!(r1.created, 1);
    let r2 = e.bulk(&editor(), T, D, E, BulkRequest { items: vec![a.clone()], upsert: true, ..Default::default() }).unwrap();
    assert_eq!(r2.unchanged, 1);
    a.attributes.insert("value".into(), json!(2));
    let r3 = e.bulk(&editor(), T, D, E, BulkRequest { items: vec![a], upsert: true, ..Default::default() }).unwrap();
    assert_eq!(r3.updated, 1);
    assert_eq!(r3.results[0].id, r1.results[0].id);
}

#[test]
fn export_import_roundtrip_is_lossless_in_csv_and_json() {
    let e = Engine::in_memory();
    let mut n = rec(
        "Field, \"north\" block",
        json!({"value": 6.25, "unit": "pH", "location": {"lat": 26.9, "lon": 75.8}, "notes": "multi\nline", "nested": {"k": [1, 2]}}),
    );
    n.tags = vec!["a".into(), "b".into()];
    n.source = Some(SourceRef { system: "lims".into(), id: "X,1".into() });
    let original = e.create(&editor(), T, D, E, n).unwrap();
    for format in [Format::Csv, Format::Json] {
        let text = e.export(&viewer(), T, D, E, format, false).unwrap();
        let target = Engine::in_memory();
        let report = target.import(&editor(), T, D, E, ImportBody { format, text: &text }, BulkRequest::default()).unwrap();
        assert_eq!(report.created, 1, "{format:?}: {report:?}");
        let copy = target.get(&viewer(), T, report.results[0].id.as_ref().unwrap()).unwrap();
        assert_eq!(copy.name, original.name);
        assert_eq!(copy.attributes, original.attributes);
        assert_eq!(copy.tags, original.tags);
        assert_eq!(copy.source, original.source);
    }
    let geo: Value = serde_json::from_str(&e.export(&viewer(), T, D, E, Format::Geojson, false).unwrap()).unwrap();
    assert_eq!(geo["features"][0]["geometry"]["coordinates"], json!([75.8, 26.9]));
    assert_eq!(geo["apsinsight"]["format_version"], 1);
    let nd = e.export(&viewer(), T, D, E, Format::Ndjson, false).unwrap();
    assert!(nd.lines().next().unwrap().contains("\"encoding\":\"utf-8\""));
}

#[test]
fn import_mapping_preview_and_row_errors() {
    let e = Engine::in_memory();
    let csv = "name,pH_value,junk\nA,6.5,x\nB,abc,y\nC,7,z,extra\n";
    let mapping = FieldMapping {
        rename: [("pH_value".to_string(), "value".to_string())].into(),
        defaults: json!({"unit": "pH"}).as_object().unwrap().clone(),
        drop: vec!["junk".into()],
    };
    let preview = e
        .import(
            &editor(),
            T,
            D,
            E,
            ImportBody { format: Format::Csv, text: csv },
            BulkRequest { dry_run: true, mapping: Some(mapping.clone()), ..Default::default() },
        )
        .unwrap();
    assert_eq!((preview.created, preview.failed), (1, 2));
    assert!(preview.results[2].error.as_ref().unwrap().message.contains("expected 3 cells"));
    let done =
        e.import(&editor(), T, D, E, ImportBody { format: Format::Csv, text: csv }, BulkRequest { mapping: Some(mapping), ..Default::default() }).unwrap();
    let r = e.get(&viewer(), T, done.results[0].id.as_ref().unwrap()).unwrap();
    assert_eq!(r.attributes["value"], 6.5);
    assert_eq!(r.attributes["unit"], "pH");
    assert!(!r.attributes.contains_key("junk"));
}

#[test]
fn versioning_history_diff_and_rollback() {
    let e = Engine::in_memory();
    let r = e.create(&editor(), T, D, E, rec("v1", json!({"value": 5.0}))).unwrap();
    e.update(
        &editor(),
        T,
        &r.id,
        1,
        RecordPatch {
            attributes: Some(json!({"value": 5.5, "unit": "pH"}).as_object().unwrap().clone()),
            transformation: Some("recalibrated".into()),
            ..Default::default()
        },
    )
    .unwrap();
    let h = e.history(&viewer(), T, &r.id).unwrap();
    assert_eq!(h.len(), 2);
    let d = e.diff(&viewer(), T, &r.id, 1, 2).unwrap();
    let paths: Vec<&str> = d.iter().map(|c| c.path.as_str()).collect();
    assert_eq!(paths, vec!["attributes.unit", "attributes.value"]);
    let back = e.rollback(&editor(), T, &r.id, 1, 2).unwrap();
    assert_eq!(back.version, 3);
    assert_eq!(back.attributes["value"], 5.0);
    assert!(!back.attributes.contains_key("unit"));
    assert!(back.provenance.iter().any(|p| p.transformation.as_deref() == Some("recalibrated")));
}

#[test]
fn schema_publication_enforces_compatibility() {
    let e = Engine::in_memory();
    let mut v2 = EntitySchema::default_for(D, E);
    v2.version = 2;
    v2.fields.push(FieldDef {
        name: "depth_cm".into(),
        ty: FieldType::Number,
        required: false,
        description: None,
        unit: Some("cm".into()),
        min: Some(0.0),
        max: Some(500.0),
        max_length: None,
    });
    assert!(matches!(e.publish_schema(&editor(), T, v2.clone(), false), Err(ApsiError::Forbidden(_))));
    let c = e.publish_schema(&admin(), T, v2.clone(), false).unwrap();
    assert_eq!(c.level, apsi_core::schema::CompatibilityLevel::BackwardCompatible);
    let mut v3 = v2.clone();
    v3.version = 3;
    v3.fields.last_mut().unwrap().required = true;
    assert!(matches!(e.publish_schema(&admin(), T, v3.clone(), false), Err(ApsiError::Conflict(_))));
    assert!(e.publish_schema(&admin(), T, v3, true).is_ok());
    let mut skip = v2;
    skip.version = 9;
    assert!(matches!(e.publish_schema(&admin(), T, skip, false), Err(ApsiError::InvalidInput(_))));
    assert_eq!(e.schemas(&viewer(), T, D, E).unwrap().len(), 3);
    // new records now validate against v3 (depth_cm required)
    assert!(matches!(e.create(&editor(), T, D, E, rec("x", json!({}))), Err(ApsiError::Validation(_))));
    let r = e.create(&editor(), T, D, E, rec("x", json!({"depth_cm": 30}))).unwrap();
    assert_eq!(r.schema_version, 3);
}

#[test]
fn configurable_rules_with_severities() {
    let e = Engine::in_memory();
    e.set_rules(
        &admin(),
        T,
        D,
        E,
        vec![
            ValidationRule {
                id: "ph-range".into(),
                kind: RuleKind::Range { field: "value".into(), min: Some(0.0), max: Some(14.0) },
                severity: Severity::Error,
                message: None,
                enabled: true,
            },
            ValidationRule {
                id: "needs-unit".into(),
                kind: RuleKind::RequiresUnit { field: "value".into() },
                severity: Severity::Warning,
                message: None,
                enabled: true,
            },
        ],
    )
    .unwrap();
    let report = e.validate_preview(&viewer(), T, D, E, &rec("x", json!({"value": 15}))).unwrap();
    assert!(!report.valid);
    assert_eq!((report.errors, report.warnings), (1, 1));
    assert!(e.create(&editor(), T, D, E, rec("x", json!({"value": 7}))).is_ok(), "warnings do not block");
    assert!(
        e.set_rules(
            &admin(),
            T,
            D,
            E,
            vec![ValidationRule { id: "BAD ID".into(), kind: RuleKind::MinTags { min: 1 }, severity: Severity::Info, message: None, enabled: true }]
        )
        .is_err()
    );
}

#[test]
fn quality_indicators_with_documented_formulas_and_empty_behavior() {
    let e = Engine::in_memory();
    let empty = e.quality(&viewer(), T, D, E).unwrap();
    assert_eq!(empty.records, 0);
    assert!(empty.completeness.is_none() && empty.validity.is_none());
    assert!(empty.formulas.contains_key("completeness"));
    let mut a = rec("full", json!({"description": "d", "unit": "pH", "value": 6.0, "observed_at": "2026-01-01T00:00:00Z", "location": {"lat": 1, "lon": 2}}));
    a.source = Some(SourceRef { system: "lims".into(), id: "1".into() });
    e.create(&editor(), T, D, E, a).unwrap();
    e.create(&editor(), T, D, E, rec("sparse", json!({"value": 6.0}))).unwrap();
    let q = e.quality(&viewer(), T, D, E).unwrap();
    assert_eq!(q.records, 2);
    assert!((q.completeness.unwrap() - 0.6).abs() < 1e-9, "(5/5 + 1/5) / 2");
    assert_eq!(q.validity, Some(1.0));
    assert_eq!(q.provenance_coverage, Some(0.5));
}

#[test]
fn retention_archives_purges_and_respects_legal_hold() {
    let (e, clock) = clocked();
    let keep = e.create(&editor(), T, D, E, rec("held", json!({}))).unwrap();
    let go = e.create(&editor(), T, D, E, rec("expiring", json!({}))).unwrap();
    e.set_legal_hold(&admin(), T, &keep.id, true).unwrap();
    e.set_retention(&admin(), T, D, E, RetentionPolicy { archive_after_days: Some(30), purge_archived_after_days: Some(60) }).unwrap();
    clock.advance(Duration::days(31));
    let preview = e.apply_retention(&admin(), T, true).unwrap();
    assert_eq!((preview.archived.len(), preview.held.len()), (1, 1));
    assert_eq!(e.get(&viewer(), T, &go.id).unwrap().lifecycle, Lifecycle::Active, "dry run changes nothing");
    let applied = e.apply_retention(&admin(), T, false).unwrap();
    assert_eq!(applied.archived, vec![go.id.clone()]);
    clock.advance(Duration::days(61));
    let purged = e.apply_retention(&admin(), T, false).unwrap();
    assert_eq!(purged.purged, vec![go.id.clone()]);
    assert!(matches!(e.get(&viewer(), T, &go.id), Err(ApsiError::NotFound(_))));
    // legal hold blocks manual deletion too
    let r = e.transition(&admin(), T, &keep.id, 1, Lifecycle::Archived).unwrap();
    assert!(matches!(e.delete(&admin(), T, &r.id), Err(ApsiError::LegalHold(_))));
    e.set_legal_hold(&admin(), T, &r.id, false).unwrap();
    e.delete(&admin(), T, &r.id).unwrap();
}

#[test]
fn notification_rules_deduplicate_and_audit() {
    let (e, clock) = clocked();
    let rule = e
        .add_notification_rule(
            &editor(),
            T,
            NotificationRule {
                id: String::new(),
                tenant: String::new(),
                domain: D.into(),
                entity: Some(E.into()),
                events: vec![EventKind::Updated],
                condition: Some(Filter { field: "value".into(), op: Op::Gt, value: json!(8.0) }),
                channel: Channel::InApp,
                dedup_window_secs: 600,
                created_by: String::new(),
            },
        )
        .unwrap();
    let r = e.create(&editor(), T, D, E, rec("x", json!({"value": 6.0}))).unwrap();
    let set = |v: f64, ver: u64| {
        e.update(&editor(), T, &r.id, ver, RecordPatch { attributes: Some(json!({"value": v}).as_object().unwrap().clone()), ..Default::default() }).unwrap()
    };
    set(7.0, 1); // condition false
    set(9.0, 2); // fires
    set(9.5, 3); // deduplicated
    assert_eq!(e.notifications(&viewer(), T, 50).unwrap().len(), 1);
    clock.advance(Duration::minutes(11));
    set(9.7, 4);
    let ns = e.notifications(&viewer(), T, 50).unwrap();
    assert_eq!(ns.len(), 2);
    assert_eq!(ns[0].rule_id, rule.id);
    assert_eq!(e.audit(&admin(), T, 100, Some("notification.suppressed")).unwrap().len(), 1);
    let bad = NotificationRule { channel: Channel::Webhook { url: "http://insecure".into() }, ..rule };
    assert!(e.add_notification_rule(&editor(), T, bad).is_err());
    assert!(
        e.add_notification_rule(
            &viewer(),
            T,
            NotificationRule {
                id: String::new(),
                tenant: String::new(),
                domain: D.into(),
                entity: None,
                events: vec![EventKind::Created],
                condition: None,
                channel: Channel::InApp,
                dedup_window_secs: 0,
                created_by: String::new()
            }
        )
        .is_err()
    );
}

struct Mean;
impl ComputeRegistry for Mean {
    fn functions(&self) -> Vec<FunctionSpec> {
        vec![FunctionSpec { name: "mean".into(), description: "arithmetic mean".into(), input: "number[]".into(), output: "number".into() }]
    }
    fn call(&self, name: &str, input: &Value) -> Result<Value, String> {
        if name != "mean" {
            return Err("unknown".into());
        }
        let xs: Vec<f64> = input.as_array().ok_or("expected array")?.iter().filter_map(Value::as_f64).collect();
        if xs.is_empty() {
            return Err("empty input".into());
        }
        Ok(json!(xs.iter().sum::<f64>() / xs.len() as f64))
    }
}

#[test]
fn workflows_are_type_checked_and_reproducible() {
    let e = Engine::in_memory().with_compute(Arc::new(Mean));
    for v in [5.0, 6.0, 7.0] {
        e.create(&editor(), T, D, E, rec("s", json!({"value": v}))).unwrap();
    }
    let def = WorkflowDefinition {
        id: "mean-ph".into(),
        name: "Mean pH".into(),
        version: 0,
        steps: vec![
            Step { id: "find".into(), op: StepOp::Search { query: Query { domain: D.into(), entity: Some(E.into()), ..Default::default() } } },
            Step { id: "values".into(), op: StepOp::ExtractNumbers { from: "find".into(), field: "value".into() } },
            Step { id: "mean".into(), op: StepOp::Compute { function: "mean".into(), input: Binding::Step { step: "values".into() } } },
            Step { id: "quality".into(), op: StepOp::Quality { domain: D.into(), entity: E.into() } },
        ],
    };
    let reg = e.register_workflow(&editor(), T, def.clone()).unwrap();
    assert_eq!(reg.version, 1);
    let a = e.run_workflow(&editor(), T, "mean-ph", None).unwrap();
    let b = e.run_workflow(&editor(), T, "mean-ph", Some(1)).unwrap();
    assert_eq!(a.outputs["mean"], 6.0);
    assert_eq!((a.definition_hash.clone(), a.data_fingerprint.clone(), a.outputs.clone()), (b.definition_hash, b.data_fingerprint, b.outputs));
    // type errors are caught at registration
    let mut bad = def.clone();
    bad.steps[1] = Step { id: "values".into(), op: StepOp::ExtractNumbers { from: "mean".into(), field: "value".into() } };
    assert!(e.register_workflow(&editor(), T, bad).is_err());
    let mut unknown = def;
    unknown.steps[2] = Step { id: "mean".into(), op: StepOp::Compute { function: "median".into(), input: Binding::Literal { value: json!([1]) } } };
    assert!(e.register_workflow(&editor(), T, unknown).is_err());
}

#[test]
fn migrations_preview_apply_and_roll_back() {
    let e = Engine::in_memory();
    let ids: Vec<String> =
        (0..3).map(|i| e.create(&editor(), T, D, E, rec(&format!("y{i}"), json!({"yield_lb_ac": 100.0 * f64::from(i + 1)}))).unwrap().id).collect();
    let mut v2 = EntitySchema::default_for(D, E);
    v2.version = 2;
    v2.fields.iter_mut().find(|f| f.name == "value").unwrap().required = true;
    e.publish_schema(&admin(), T, v2, true).unwrap();
    let plan = MigrationPlan {
        id: String::new(),
        domain: D.into(),
        entity: E.into(),
        to_version: 2,
        ops: vec![
            MigrationOp::Rename { from: "yield_lb_ac".into(), to: "value".into() },
            MigrationOp::Scale { field: "value".into(), factor: 1.120_851 },
            MigrationOp::SetUnit { unit: "kg/ha".into() },
        ],
    };
    let preview = e.migrate(&admin(), T, plan.clone(), true).unwrap();
    assert_eq!((preview.total, preview.migratable, preview.applied), (3, 3, false));
    let report = e.migrate(&admin(), T, plan, false).unwrap();
    assert!(report.applied);
    let r = e.get(&viewer(), T, &ids[0]).unwrap();
    assert_eq!((r.schema_version, r.attributes["unit"].as_str()), (2, Some("kg/ha")));
    // a record edited after migration is reported as a conflict, others restored
    e.update(&editor(), T, &ids[2], 2, RecordPatch { name: Some("edited".into()), ..Default::default() }).unwrap();
    let rb = e.rollback_migration(&admin(), T, &report.id).unwrap();
    assert_eq!((rb.restored.len(), rb.conflicts,), (2, vec![ids[2].clone()]));
    let r = e.get(&viewer(), T, &ids[0]).unwrap();
    assert_eq!(r.schema_version, 1);
    assert_eq!(r.attributes["yield_lb_ac"], 100.0);
    assert!(matches!(e.rollback_migration(&admin(), T, &report.id), Err(ApsiError::Conflict(_))));
}

#[test]
fn failed_migrations_change_nothing() {
    let e = Engine::in_memory();
    let r = e.create(&editor(), T, D, E, rec("x", json!({"value": "n/a"}))).unwrap_err();
    assert!(matches!(r, ApsiError::Validation(_)));
    let ok = e.create(&editor(), T, D, E, rec("x", json!({"reading": "high"}))).unwrap();
    let mut v2 = EntitySchema::default_for(D, E);
    v2.version = 2;
    e.publish_schema(&admin(), T, v2, false).unwrap();
    let plan = MigrationPlan {
        id: String::new(),
        domain: D.into(),
        entity: E.into(),
        to_version: 2,
        ops: vec![MigrationOp::Scale { field: "reading".into(), factor: 2.0 }],
    };
    let report = e.migrate(&admin(), T, plan, false).unwrap();
    assert!(!report.applied);
    assert_eq!(report.failures.len(), 1);
    assert_eq!(e.get(&viewer(), T, &ok.id).unwrap().version, 1);
}

#[test]
fn persistence_survives_restart_and_audit_chain_verifies() {
    let dir = tempfile::tempdir().unwrap();
    let id = {
        let e = Engine::open(dir.path()).unwrap();
        let r = e.create(&editor(), T, D, E, rec("durable", json!({"value": 6.6, "texture": "silt"}))).unwrap();
        e.update(&editor(), T, &r.id, 1, RecordPatch { name: Some("durable 2".into()), ..Default::default() }).unwrap();
        r.id
    };
    let e = Engine::open(dir.path()).unwrap();
    let r = e.get(&viewer(), T, &id).unwrap();
    assert_eq!((r.name.as_str(), r.version), ("durable 2", 2));
    let hit = e
        .search(
            &viewer(),
            T,
            &Query { domain: D.into(), filters: vec![Filter { field: "texture".into(), op: Op::Eq, value: json!("silt") }], ..Default::default() },
        )
        .unwrap();
    assert_eq!(hit.total, 1, "indexes are rebuilt on load");
    assert!(e.verify_audit(&admin(), T).unwrap().is_ok());
    drop(e);
    // tampering with the stored audit log is detected on open
    let path = dir.path().join("journal.ndjson");
    let text = std::fs::read_to_string(&path).unwrap().replacen("record.update", "record.forged", 1);
    std::fs::write(&path, text).unwrap();
    assert!(matches!(Engine::open(dir.path()), Err(ApsiError::Storage(_))));
}

#[test]
fn journal_compaction_torn_writes_and_metadata_survive_restart() {
    let dir = tempfile::tempdir().unwrap();
    let (kept, deleted) = {
        let e = Engine::open(dir.path()).unwrap();
        let mut v2 = EntitySchema::default_for(D, E);
        v2.version = 2;
        e.publish_schema(&admin(), T, v2, false).unwrap();
        e.add_notification_rule(
            &editor(),
            T,
            NotificationRule {
                id: String::new(),
                tenant: String::new(),
                domain: D.into(),
                entity: None,
                events: vec![EventKind::Created],
                condition: None,
                channel: Channel::InApp,
                dedup_window_secs: 0,
                created_by: String::new(),
            },
        )
        .unwrap();
        let kept = e.create(&editor(), T, D, E, rec("kept", json!({"value": 1}))).unwrap().id;
        e.compact_now().unwrap();
        // changes after the snapshot live only in the journal
        let gone = e.create(&editor(), T, D, E, rec("gone", json!({}))).unwrap();
        e.transition(&editor(), T, &gone.id, 1, Lifecycle::Archived).unwrap();
        e.delete(&admin(), T, &gone.id).unwrap();
        e.update(&editor(), T, &kept, 1, RecordPatch { name: Some("kept v2".into()), ..Default::default() }).unwrap();
        (kept, gone.id)
    };
    // simulate a crash mid-append: a torn, unterminated final line is ignored
    let journal = dir.path().join("journal.ndjson");
    let mut text = std::fs::read_to_string(&journal).unwrap();
    let valid = text.clone();
    text.push_str("{\"upserts\":[{\"rec");
    std::fs::write(&journal, &text).unwrap();
    let e = Engine::open(dir.path()).unwrap();
    assert_eq!(e.get(&viewer(), T, &kept).unwrap().name, "kept v2");
    assert!(matches!(e.get(&viewer(), T, &deleted), Err(ApsiError::NotFound(_))));
    assert_eq!(e.schemas(&viewer(), T, D, E).unwrap().len(), 2, "metadata persisted");
    assert_eq!(e.notification_rules(&viewer(), T).unwrap().len(), 1);
    assert_eq!(e.notifications(&viewer(), T, 10).unwrap().len(), 2);
    assert!(e.verify_audit(&admin(), T).unwrap().is_ok());
    drop(e);
    // replaying the journal twice (crash between snapshot rename and truncate) is idempotent
    std::fs::write(&journal, format!("{valid}{valid}")).unwrap();
    let e = Engine::open(dir.path()).unwrap();
    assert_eq!(e.get(&viewer(), T, &kept).unwrap().version, 2);
    assert!(e.verify_audit(&admin(), T).unwrap().is_ok());
    // corruption that is not a torn tail is an error, never silently skipped
    drop(e);
    std::fs::write(&journal, format!("garbage\n{valid}")).unwrap();
    assert!(matches!(Engine::open(dir.path()), Err(ApsiError::Storage(_))));
}

#[test]
fn failed_transactions_roll_back_completely() {
    let e = Engine::in_memory();
    let before = e.overview(&viewer(), T).unwrap().records;
    let err = e.bulk(&editor(), T, D, E, BulkRequest { items: vec![rec("a", json!({}))], idempotency_key: Some("\u{7}".into()), ..Default::default() });
    assert!(err.is_err());
    assert_eq!(e.overview(&viewer(), T).unwrap().records, before);
}

#[test]
fn observability_counts_operations_without_payloads() {
    let e = Engine::in_memory();
    e.create(&editor(), T, D, E, rec("secret-name", json!({"value": 1}))).unwrap();
    let _ = e.create(&viewer(), T, D, E, rec("x", json!({})));
    let m = e.metrics();
    assert_eq!(m.counter("record.create", D, "ok"), 1);
    assert_eq!(m.counter("record.create", D, "forbidden"), 1);
    let text = m.render();
    assert!(!text.contains("secret-name"));
}

#[test]
fn dashboard_and_overview_summaries() {
    let e = Engine::in_memory();
    e.create(&editor(), T, D, E, rec("a", json!({}))).unwrap();
    let r = e.create(&editor(), T, D, "bulk-density", rec("b", json!({}))).unwrap();
    e.transition(&editor(), T, &r.id, 1, Lifecycle::Archived).unwrap();
    let d = e.dashboard(&viewer(), T, D).unwrap();
    assert_eq!(d.total, 2);
    assert_eq!(d.entities.len(), catalog().domain(D).unwrap().entities.len());
    let bd = d.entities.iter().find(|x| x.entity == "bulk-density").unwrap();
    assert_eq!((bd.archived, bd.active), (1, 0));
    assert!(!d.recent_activity.is_empty());
    let o = e.overview(&viewer(), T).unwrap();
    assert_eq!(o.domains.len(), 100);
    assert_eq!(o.records, 2);
    assert!(o.audit_chain_valid);
}

/// Acceptance sweep: every one of the 800 entity kinds supports the full record
/// lifecycle, search, quality, export and validation through the same engine.
#[test]
fn acceptance_every_entity_kind_supports_core_capabilities() {
    let e = Engine::in_memory();
    let mut kinds = 0;
    for d in &catalog().domains {
        for k in &d.entities {
            let r = e.create(&editor(), T, &d.slug, &k.slug, rec(&k.name, json!({"value": 1.5, "unit": "1"}))).unwrap();
            let r = e.update(&editor(), T, &r.id, 1, RecordPatch { tags: Some(vec!["checked".into()]), ..Default::default() }).unwrap();
            let page = e
                .search(&viewer(), T, &Query { domain: d.slug.clone(), entity: Some(k.slug.clone()), tags: vec!["checked".into()], ..Default::default() })
                .unwrap();
            assert_eq!(page.total, 1, "{}/{}", d.slug, k.slug);
            assert_eq!(e.quality(&viewer(), T, &d.slug, &k.slug).unwrap().validity, Some(1.0));
            assert!(e.export(&viewer(), T, &d.slug, &k.slug, Format::Csv, false).unwrap().lines().count() == 2);
            e.transition(&editor(), T, &r.id, 2, Lifecycle::Archived).unwrap();
            kinds += 1;
        }
    }
    assert_eq!(kinds, 800);
    assert!(e.verify_audit(&admin(), T).unwrap().is_ok());
}

/// The published record JSON Schema (03_domain_model/metadata-schemas) must describe
/// exactly the envelope the engine produces: every emitted key is declared, every
/// required key is emitted, and enumerations and the id pattern hold.
#[test]
fn record_envelope_matches_published_json_schema() {
    let schema: Value = serde_json::from_str(include_str!("../../../03_domain_model/metadata-schemas/record.v1.schema.json")).unwrap();
    let e = Engine::in_memory();
    let mut n = rec("Synthetic plot", json!({"value": 6.1, "unit": "pH"}));
    n.source = Some(SourceRef { system: "synthetic-lims".into(), id: "S-1".into() });
    let fresh = e.create(&editor(), T, D, E, n).unwrap();
    let archived = e.transition(&editor(), T, &fresh.id, 1, Lifecycle::Archived).unwrap();
    let props = schema["properties"].as_object().unwrap();
    let required: Vec<&str> = schema["required"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
    let prov = &schema["$defs"]["provenance"];
    for record in [&fresh, &archived] {
        let v = serde_json::to_value(record).unwrap();
        let obj = v.as_object().unwrap();
        for k in obj.keys() {
            assert!(props.contains_key(k), "engine emits undeclared field {k}");
        }
        for k in &required {
            assert!(obj.contains_key(*k), "required field {k} missing");
        }
        let id = obj["id"].as_str().unwrap();
        assert!(id.len() == 36 && id.starts_with("rec_") && id[4..].bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()));
        let lifecycles: Vec<&Value> = props["lifecycle"]["enum"].as_array().unwrap().iter().collect();
        assert!(lifecycles.contains(&&obj["lifecycle"]));
        for p in obj["provenance"].as_array().unwrap() {
            let p = p.as_object().unwrap();
            for k in p.keys() {
                assert!(prov["properties"].as_object().unwrap().contains_key(k), "undeclared provenance field {k}");
            }
            for k in prov["required"].as_array().unwrap() {
                assert!(p.contains_key(k.as_str().unwrap()));
            }
            assert!(prov["properties"]["actor_kind"]["enum"].as_array().unwrap().contains(&p["actor_kind"]));
        }
    }
    assert!(serde_json::to_value(&archived).unwrap().get("archived_at").is_some());
}
