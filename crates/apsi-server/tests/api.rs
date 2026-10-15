//! HTTP contract tests against the full router (no network; tower `oneshot`).

use apsi_core::Engine;
use apsi_core::auth::TokenRegistry;
use apsi_core::catalog::catalog;
use apsi_science::ScienceRegistry;
use apsi_server::{AppState, router};
use axum::Router;
use axum::body::Body;
use axum::http::{HeaderMap, Method, Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

const ADMIN: &str = "admin-token-000000001";
const EDITOR: &str = "editor-token-00000001";
const VIEWER: &str = "viewer-token-00000001";
const AUDITOR: &str = "auditor-token-0000001";
const OTHER: &str = "other-tenant-token-01";

const D: &str = "crop-yield-and-production";
const E: &str = "yield-observations";

fn app_with_limit(limit: u32) -> Router {
    let tokens = TokenRegistry::from_spec(&format!(
        "{ADMIN}:acme:ada:admin,{EDITOR}:acme:eve:editor,{VIEWER}:acme:vic:viewer,{AUDITOR}:acme:svc-prom:auditor,{OTHER}:rival:rex:admin"
    ))
    .unwrap();
    router(AppState::new(Engine::in_memory().with_compute(Arc::new(ScienceRegistry)), tokens, limit), None)
}

fn app() -> Router {
    app_with_limit(10_000)
}

struct Resp {
    status: StatusCode,
    headers: HeaderMap,
    text: String,
}

impl Resp {
    fn json(&self) -> Value {
        serde_json::from_str(&self.text).unwrap_or_else(|_| panic!("not JSON: {}", self.text))
    }
    fn data(&self) -> Value {
        self.json()["data"].clone()
    }
    fn code(&self) -> String {
        self.json()["error"]["code"].as_str().unwrap_or("").to_string()
    }
}

async fn call(app: &Router, method: Method, uri: &str, token: Option<&str>, body: Option<Value>) -> Resp {
    let mut b = Request::builder().method(method).uri(uri);
    if let Some(t) = token {
        b = b.header("authorization", format!("Bearer {t}"));
    }
    let req = match body {
        Some(v) => b.header("content-type", "application/json").body(Body::from(v.to_string())).unwrap(),
        None => b.body(Body::empty()).unwrap(),
    };
    raw(app, req).await
}

async fn raw(app: &Router, req: Request<Body>) -> Resp {
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let headers = resp.headers().clone();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    Resp { status, headers, text: String::from_utf8_lossy(&bytes).into_owned() }
}

fn records() -> String {
    format!("/api/v1/domains/{D}/entities/{E}/records")
}

#[tokio::test]
async fn api_create_get_and_envelopes() {
    let app = app();
    let r = call(
        &app,
        Method::POST,
        &records(),
        Some(EDITOR),
        Some(json!({"name": "North field 2026", "attributes": {"value": 6.2, "unit": "t/ha"}, "source": {"system": "combine", "id": "J-17"}})),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.text);
    assert_eq!(r.json()["meta"]["api_version"], "v1");
    assert!(r.headers.contains_key("x-request-id"));
    let rec = r.data();
    assert_eq!(rec["tenant"], "acme");
    let id = rec["id"].as_str().unwrap();
    let g = call(&app, Method::GET, &format!("/api/v1/records/{id}"), Some(VIEWER), None).await;
    assert_eq!(g.status, StatusCode::OK);
    assert_eq!(g.headers["etag"], "\"1\"");
    assert_eq!(g.data()["attributes"]["value"], 6.2);
    let h = call(&app, Method::GET, "/health", None, None).await;
    assert_eq!(h.data()["status"], "ok");
    let c = call(&app, Method::GET, "/api/v1/catalog", Some(VIEWER), None).await;
    assert_eq!((c.data()["features"].as_u64(), c.data()["entity_kinds"].as_u64()), (Some(3100), Some(800)));
}

#[tokio::test]
async fn api_authentication_and_tenant_isolation() {
    let app = app();
    let none = call(&app, Method::GET, &records(), None, None).await;
    assert_eq!((none.status, none.code().as_str()), (StatusCode::UNAUTHORIZED, "unauthenticated"));
    assert_eq!(none.headers["www-authenticate"], "Bearer");
    let bad = call(&app, Method::GET, &records(), Some("not-a-real-token-xyz"), None).await;
    assert_eq!(bad.status, StatusCode::UNAUTHORIZED);
    let created = call(&app, Method::POST, &records(), Some(EDITOR), Some(json!({"name": "x"}))).await.data();
    let id = created["id"].as_str().unwrap();
    // viewer cannot write; other tenant cannot see the record at all
    let w = call(&app, Method::POST, &records(), Some(VIEWER), Some(json!({"name": "y"}))).await;
    assert_eq!((w.status, w.code().as_str()), (StatusCode::FORBIDDEN, "forbidden"));
    let other = call(&app, Method::GET, &format!("/api/v1/records/{id}"), Some(OTHER), None).await;
    assert_eq!(other.status, StatusCode::NOT_FOUND);
    let list = call(&app, Method::GET, &records(), Some(OTHER), None).await;
    assert_eq!(list.data()["total"], 0);
    let audit = call(&app, Method::GET, "/api/v1/audit?limit=50", Some(AUDITOR), None).await;
    assert!(audit.data().as_array().unwrap().iter().any(|e| e["outcome"] == "denied"));
    let me = call(&app, Method::GET, "/api/v1/me", Some(AUDITOR), None).await;
    assert_eq!(me.data()["kind"], "service");
}

#[tokio::test]
async fn api_errors_use_typed_envelopes() {
    let app = app();
    let bad_json = raw(
        &app,
        Request::post(records())
            .header("authorization", format!("Bearer {EDITOR}"))
            .header("content-type", "application/json")
            .body(Body::from("{not json"))
            .unwrap(),
    )
    .await;
    assert_eq!((bad_json.status, bad_json.code().as_str()), (StatusCode::BAD_REQUEST, "invalid_input"));
    let invalid = call(&app, Method::POST, &records(), Some(EDITOR), Some(json!({"name": "", "attributes": {"value": "lots"}}))).await;
    assert_eq!(invalid.status, StatusCode::UNPROCESSABLE_ENTITY);
    let err = invalid.json()["error"].clone();
    assert_eq!(err["retryable"], false);
    assert!(err["details"]["issues"].as_array().unwrap().len() >= 2);
    let nf = call(&app, Method::GET, "/api/v1/domains/no-such/entities/x/records", Some(VIEWER), None).await;
    assert_eq!((nf.status, nf.code().as_str()), (StatusCode::NOT_FOUND, "not_found"));
    let route = call(&app, Method::GET, "/api/v1/nothing-here", Some(VIEWER), None).await;
    assert_eq!(route.status, StatusCode::NOT_FOUND);
    assert_eq!(route.code(), "not_found");
    let malformed_id = call(&app, Method::GET, "/api/v1/records/..%2F..%2Fetc", Some(VIEWER), None).await;
    assert_eq!(malformed_id.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn api_lifecycle_concurrency_and_history() {
    let app = app();
    let id = call(&app, Method::POST, &records(), Some(EDITOR), Some(json!({"name": "plot", "attributes": {"value": 5.0}}))).await.data()["id"]
        .as_str()
        .unwrap()
        .to_string();
    let rec = format!("/api/v1/records/{id}");
    let missing = call(&app, Method::PATCH, &rec, Some(EDITOR), Some(json!({"name": "x"}))).await;
    assert_eq!(missing.code(), "invalid_input", "expected_version is mandatory");
    let p = raw(
        &app,
        Request::patch(&rec)
            .header("authorization", format!("Bearer {EDITOR}"))
            .header("if-match", "\"1\"")
            .header("content-type", "application/json")
            .body(Body::from(json!({"attributes": {"value": 5.5}}).to_string()))
            .unwrap(),
    )
    .await;
    assert_eq!(p.status, StatusCode::OK, "{}", p.text);
    let stale = call(&app, Method::PATCH, &rec, Some(EDITOR), Some(json!({"expected_version": 1, "attributes": {"value": 9.0}}))).await;
    assert_eq!((stale.status, stale.code().as_str()), (StatusCode::CONFLICT, "version_conflict"));
    assert_eq!(stale.json()["error"]["retryable"], true);
    assert_eq!(stale.json()["error"]["details"]["current"], 2);
    let diff = call(&app, Method::GET, &format!("{rec}/diff?from=1&to=2"), Some(VIEWER), None).await;
    assert_eq!(diff.data()[0]["path"], "attributes.value");
    let rb = call(&app, Method::POST, &format!("{rec}/rollback"), Some(EDITOR), Some(json!({"to_version": 1, "expected_version": 2}))).await;
    assert_eq!(rb.data()["attributes"]["value"], 5.0);
    let arch = call(&app, Method::POST, &format!("{rec}/lifecycle"), Some(EDITOR), Some(json!({"to": "archived", "expected_version": 3}))).await;
    assert_eq!(arch.data()["lifecycle"], "archived");
    assert_eq!(call(&app, Method::GET, &format!("{rec}/history"), Some(VIEWER), None).await.data().as_array().unwrap().len(), 4);
    let hold = call(&app, Method::PUT, &format!("{rec}/legal-hold"), Some(ADMIN), Some(json!({"hold": true}))).await;
    assert_eq!(hold.data()["legal_hold"], true);
    let del = call(&app, Method::DELETE, &rec, Some(ADMIN), None).await;
    assert_eq!((del.status, del.code().as_str()), (StatusCode::CONFLICT, "legal_hold"));
}

#[tokio::test]
async fn api_schema_rules_and_validation() {
    let app = app();
    let base = format!("/api/v1/domains/{D}/entities/{E}");
    let schemas = call(&app, Method::GET, &format!("{base}/schemas"), Some(VIEWER), None).await.data();
    assert_eq!(schemas[0]["version"], 1);
    let mut v2 = schemas[0].clone();
    v2["version"] = json!(2);
    v2["fields"].as_array_mut().unwrap().push(json!({"name": "moisture_pct", "type": {"kind": "number"}, "min": 0, "max": 60, "unit": "%"}));
    let pub_ = call(&app, Method::POST, &format!("{base}/schemas"), Some(ADMIN), Some(v2)).await;
    assert_eq!(pub_.status, StatusCode::CREATED, "{}", pub_.text);
    assert_eq!(pub_.data()["level"], "backward_compatible");
    let rules = call(
        &app,
        Method::PUT,
        &format!("{base}/rules"),
        Some(ADMIN),
        Some(json!([{"id": "yield-max", "kind": "range", "field": "value", "max": 30, "severity": "error"}])),
    )
    .await;
    assert_eq!(rules.status, StatusCode::OK, "{}", rules.text);
    let report =
        call(&app, Method::POST, &format!("{base}/validate"), Some(VIEWER), Some(json!({"name": "x", "attributes": {"value": 45, "moisture_pct": 80}})))
            .await
            .data();
    assert_eq!(report["valid"], false);
    let rules: Vec<&str> = report["issues"].as_array().unwrap().iter().map(|i| i["rule"].as_str().unwrap()).collect();
    assert!(rules.contains(&"yield-max") && rules.iter().any(|r| r.starts_with("schema.v2")), "{rules:?}");
    let editor_rules = call(&app, Method::PUT, &format!("{base}/rules"), Some(EDITOR), Some(json!([]))).await;
    assert_eq!(editor_rules.status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn api_search_bulk_import_export() {
    let app = app();
    let base = format!("/api/v1/domains/{D}/entities/{E}");
    let items: Vec<Value> = (0..12)
        .map(|i| json!({"name": format!("obs {i}"), "attributes": {"value": i, "crop": if i % 2 == 0 {"wheat"} else {"rice"}}, "tags": ["2026"]}))
        .collect();
    let bulk = call(&app, Method::POST, &format!("{base}/bulk"), Some(EDITOR), Some(json!({"items": items, "idempotency_key": "k1"}))).await.data();
    assert_eq!(bulk["created"], 12);
    let again = call(&app, Method::POST, &format!("{base}/bulk"), Some(EDITOR), Some(json!({"items": items, "idempotency_key": "k1"}))).await.data();
    assert_eq!(again["replayed"], true);
    let page = call(&app, Method::GET, &format!("{base}/records?tags=2026&facets=crop&limit=5&sort=name"), Some(VIEWER), None).await.data();
    assert_eq!((page["total"].as_u64(), page["items"].as_array().unwrap().len(), page["next_offset"].as_u64()), (Some(12), 5, Some(5)));
    assert_eq!(page["facets"]["crop"]["wheat"], 6);
    let s = call(
        &app,
        Method::POST,
        &format!("/api/v1/domains/{D}/search"),
        Some(VIEWER),
        Some(json!({"entity": E, "filters": [{"field": "crop", "op": "eq", "value": "rice"}, {"field": "value", "op": "gt", "value": 5}]})),
    )
    .await
    .data();
    assert_eq!(s["total"], 3);
    let csv = call(&app, Method::GET, &format!("{base}/export?format=csv"), Some(VIEWER), None).await;
    assert!(csv.headers["content-type"].to_str().unwrap().starts_with("text/csv"));
    assert!(csv.headers["content-disposition"].to_str().unwrap().contains(".csv"));
    assert_eq!(csv.text.lines().count(), 13);
    // import the export into another tenant — lossless and validated
    let imp = raw(
        &app,
        Request::post(format!("{base}/import?format=csv&dry_run=true"))
            .header("authorization", format!("Bearer {OTHER}"))
            .body(Body::from(csv.text.clone()))
            .unwrap(),
    )
    .await;
    assert_eq!(imp.data()["created"], 12, "{}", imp.text);
    assert_eq!(call(&app, Method::GET, &format!("{base}/records"), Some(OTHER), None).await.data()["total"], 0, "dry run");
    let geo = call(&app, Method::GET, &format!("{base}/export?format=geojson"), Some(VIEWER), None).await;
    assert_eq!(geo.headers["content-type"], "application/geo+json");
    let bad = call(&app, Method::GET, &format!("{base}/export?format=xls"), Some(VIEWER), None).await;
    assert_eq!(bad.code(), "invalid_input");
}

#[tokio::test]
async fn api_metrics_security_headers_and_limits() {
    let app = app();
    let forbidden = call(&app, Method::GET, "/metrics", Some(VIEWER), None).await;
    assert_eq!(forbidden.status, StatusCode::FORBIDDEN);
    call(&app, Method::POST, &records(), Some(EDITOR), Some(json!({"name": "secret plot name"}))).await;
    let m = call(&app, Method::GET, "/metrics", Some(AUDITOR), None).await;
    assert!(m.text.contains("apsi_operations_total{operation=\"record.create\""));
    assert!(!m.text.contains("secret plot name"));
    let r = call(&app, Method::GET, "/api/v1/me", Some(VIEWER), None).await;
    assert_eq!(r.headers["x-content-type-options"], "nosniff");
    assert_eq!(r.headers["x-frame-options"], "DENY");
    assert_eq!(r.headers["cache-control"], "no-store");
    assert!(r.headers["content-security-policy"].to_str().unwrap().contains("default-src 'none'"));
    let huge = "x".repeat(apsi_server::MAX_BODY_BYTES + 10);
    let big = raw(
        &app,
        Request::post(records())
            .header("authorization", format!("Bearer {EDITOR}"))
            .header("content-type", "application/json")
            .body(Body::from(format!("{{\"name\":\"{huge}\"}}")))
            .unwrap(),
    )
    .await;
    assert_eq!(big.status, StatusCode::PAYLOAD_TOO_LARGE);
    let limited = app_with_limit(2);
    for _ in 0..2 {
        assert_eq!(call(&limited, Method::GET, "/api/v1/me", Some(VIEWER), None).await.status, StatusCode::OK);
    }
    let over = call(&limited, Method::GET, "/api/v1/me", Some(VIEWER), None).await;
    assert_eq!((over.status, over.code().as_str()), (StatusCode::TOO_MANY_REQUESTS, "rate_limited"));
    assert_eq!(over.headers["retry-after"], "60");
    // Rejections before the engine are still counted, so the alert rules can see them.
    let lm = call(&limited, Method::GET, "/metrics", Some(AUDITOR), None).await;
    assert!(lm.text.contains("apsi_operations_total{operation=\"http.auth\",domain=\"-\",outcome=\"rate_limited\"} 1"), "{}", lm.text);
    call(&app, Method::GET, "/api/v1/me", None, None).await;
    call(&app, Method::GET, "/api/v1/me", Some("not-a-real-token-000000"), None).await;
    let m = call(&app, Method::GET, "/metrics", Some(AUDITOR), None).await;
    assert!(m.text.contains("apsi_operations_total{operation=\"http.auth\",domain=\"-\",outcome=\"unauthenticated\"} 2"), "{}", m.text);
    assert!(!m.text.contains("not-a-real-token"));
    let page = call(&app, Method::GET, &format!("{}?limit=100000", records()), Some(VIEWER), None).await;
    assert_eq!(page.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn api_dashboard_notifications_and_workflows() {
    let app = app();
    let rule = call(
        &app,
        Method::POST,
        "/api/v1/notification-rules",
        Some(EDITOR),
        Some(json!({"domain": D, "entity": E, "events": ["created"], "channel": {"kind": "in_app"}})),
    )
    .await;
    assert_eq!(rule.status, StatusCode::CREATED, "{}", rule.text);
    for v in [4.0, 5.0, 6.0] {
        call(&app, Method::POST, &records(), Some(EDITOR), Some(json!({"name": "obs", "attributes": {"value": v}}))).await;
    }
    assert_eq!(call(&app, Method::GET, "/api/v1/notifications", Some(VIEWER), None).await.data().as_array().unwrap().len(), 3);
    let dash = call(&app, Method::GET, &format!("/api/v1/domains/{D}/dashboard"), Some(VIEWER), None).await.data();
    assert_eq!(dash["total"], 3);
    assert_eq!(dash["notifications"], 3);
    let ov = call(&app, Method::GET, "/api/v1/overview", Some(VIEWER), None).await.data();
    assert_eq!(ov["domains"].as_array().unwrap().len(), 100);
    let fns = call(&app, Method::GET, "/api/v1/compute", Some(VIEWER), None).await.data();
    assert!(fns.as_array().unwrap().iter().any(|f| f["name"] == "stats.describe"));
    let ndvi =
        call(&app, Method::POST, "/api/v1/compute/indices.compute", Some(VIEWER), Some(json!({"index": "ndvi", "pixel": {"red": 0.05, "nir": 0.45}}))).await;
    assert!((ndvi.data().as_f64().unwrap() - 0.8).abs() < 1e-9);
    let bad = call(&app, Method::POST, "/api/v1/compute/geo.area", Some(VIEWER), Some(json!({"ring": [{"lon": 0, "lat": 0}]}))).await;
    assert_eq!(bad.code(), "invalid_input");
    let wf = json!({"id": "yield-summary", "name": "Yield summary", "steps": [
        {"id": "find", "op": "search", "query": {"domain": D, "entity": E}},
        {"id": "vals", "op": "extract_numbers", "from": "find", "field": "value"},
        {"id": "desc", "op": "compute", "function": "stats.describe", "input": {"kind": "step", "step": "vals"}}
    ]});
    let reg = call(&app, Method::POST, "/api/v1/workflows", Some(EDITOR), Some(wf)).await;
    assert_eq!(reg.status, StatusCode::CREATED, "{}", reg.text);
    let run = call(&app, Method::POST, "/api/v1/workflows/yield-summary/runs", Some(EDITOR), None).await.data();
    assert_eq!(run["status"], "succeeded");
    assert_eq!(run["outputs"]["desc"]["mean"], 5.0);
    assert_eq!(call(&app, Method::GET, "/api/v1/workflow-runs", Some(VIEWER), None).await.data().as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn api_acceptance_sweep_all_domains() {
    let app = app();
    for d in &catalog().domains {
        let e = &d.entities[0].slug;
        let base = format!("/api/v1/domains/{}/entities/{e}", d.slug);
        let c = call(&app, Method::POST, &format!("{base}/records"), Some(EDITOR), Some(json!({"name": format!("{} sample", d.name)}))).await;
        assert_eq!(c.status, StatusCode::CREATED, "{}: {}", d.slug, c.text);
        assert_eq!(call(&app, Method::GET, &format!("{base}/quality"), Some(VIEWER), None).await.data()["records"], 1);
        assert_eq!(call(&app, Method::GET, &format!("/api/v1/domains/{}/dashboard", d.slug), Some(VIEWER), None).await.data()["total"], 1);
    }
    let f = call(&app, Method::GET, "/api/v1/features/APSI-F2109", Some(VIEWER), None).await.data();
    assert_eq!(f["feature"]["domain"], "Crop yield and production");
    assert!(f["evidence"]["api"][0].as_str().unwrap().contains("/domains/crop-yield-and-production/"));
    let list = call(&app, Method::GET, "/api/v1/features?capability=versioning&limit=500", Some(VIEWER), None).await.data();
    assert_eq!(list["total"], 100);
    let trace = call(&app, Method::GET, "/api/v1/traceability", Some(VIEWER), None).await;
    assert_eq!(trace.text.lines().count(), 3101);
    let verify = call(&app, Method::GET, "/api/v1/audit/verify", Some(AUDITOR), None).await.data();
    assert_eq!(verify["valid"], true);
}

#[tokio::test]
async fn api_integrations_report_unavailable_products() {
    let app = app();
    assert_eq!(call(&app, Method::GET, "/api/v1/integrations", None, None).await.status, StatusCode::UNAUTHORIZED);
    let r = call(&app, Method::GET, "/api/v1/integrations", Some(VIEWER), None).await;
    assert_eq!(r.status, StatusCode::OK);
    let items = r.data();
    let items = items.as_array().unwrap();
    assert_eq!(items.len(), 6);
    for it in items {
        assert_eq!(it["availability"]["state"], "unavailable", "{it}");
        assert_eq!(it["optional"], true);
        assert_eq!(it["contract_version"], "1.0");
        assert!(!it["operations"].as_array().unwrap().is_empty());
    }
    let names: Vec<_> = items.iter().map(|i| i["product"].as_str().unwrap().to_string()).collect();
    assert_eq!(names, ["agriql", "kutajkutambi-ai", "agricuda", "apsnetwork", "nityaga-rtos", "accelerator"]);
}

/// Every route registered in the router must appear in 10_backend_api/openapi.v1.yaml
/// with each of its methods, and the spec must not document routes that do not exist.
#[test]
fn openapi_documents_every_route() {
    let src = include_str!("../src/api.rs");
    let spec = include_str!("../../../10_backend_api/openapi.v1.yaml");
    let kind = "/api/v1/domains/{domain}/entities/{entity}";
    let mut routes: Vec<(String, Vec<&str>)> = Vec::new();
    for line in src.lines().map(str::trim).filter(|l| l.starts_with(".route(")) {
        let path = if let Some(rest) = line.strip_prefix(".route(&format!(\"{kind}") {
            format!("{kind}{}", &rest[..rest.find('\"').unwrap()])
        } else {
            let rest = line.strip_prefix(".route(\"").unwrap();
            rest[..rest.find('\"').unwrap()].to_string()
        };
        if path.contains('*') {
            continue; // catch-all 404 for unknown API paths
        }
        let methods = ["get", "post", "put", "patch", "delete"]
            .into_iter()
            .filter(|m| line.contains(&format!(" {m}(")) || line.contains(&format!("({m}(")) || line.contains(&format!(".{m}(")))
            .collect();
        routes.push((path, methods));
    }
    assert!(routes.len() >= 39, "parsed only {} routes", routes.len());
    // Index the spec: "  /path:" lines and the "    method:" lines under them.
    let mut documented: Vec<(String, Vec<String>)> = Vec::new();
    let mut in_paths = false;
    for line in spec.lines() {
        if line == "paths:" {
            in_paths = true;
        } else if in_paths && !line.starts_with(' ') && !line.is_empty() {
            in_paths = false;
        } else if in_paths && line.starts_with("  /") {
            documented.push((line.trim().trim_end_matches(':').to_string(), Vec::new()));
        } else if in_paths && line.starts_with("    ") && !line.starts_with("     ") {
            let key = line.trim().split(':').next().unwrap().to_string();
            if ["get", "post", "put", "patch", "delete"].contains(&key.as_str()) {
                documented.last_mut().unwrap().1.push(key);
            }
        }
    }
    for (path, methods) in &routes {
        let doc = documented.iter().find(|(p, _)| p == path).unwrap_or_else(|| panic!("{path} is not in openapi.v1.yaml"));
        for m in methods {
            assert!(doc.1.iter().any(|d| d == m), "{} {path} is not documented", m.to_uppercase());
        }
        assert_eq!(doc.1.len(), methods.len(), "{path}: documented methods {:?} differ from router {:?}", doc.1, methods);
    }
    for (p, _) in &documented {
        assert!(routes.iter().any(|(r, _)| r == p), "openapi documents {p}, which the router does not serve");
    }
}

#[tokio::test]
async fn api_status_reports_honest_completion_metrics() {
    let app = app();
    assert_eq!(call(&app, Method::GET, "/api/v1/status", None, None).await.status, StatusCode::UNAUTHORIZED);
    let r = call(&app, Method::GET, "/api/v1/status", Some(VIEWER), None).await;
    assert_eq!(r.status, StatusCode::OK);
    let d = r.data();
    assert_eq!(d["total"], 3100);
    let by: Vec<&Value> = d["by_status"].as_array().unwrap().iter().collect();
    assert_eq!(by.len(), 4);
    assert_eq!(by.iter().map(|s| s["features"].as_u64().unwrap()).sum::<u64>(), 3100);
    let domains = d["domains"].as_array().unwrap();
    assert_eq!(domains.len(), 100);
    for dom in domains {
        let n: u64 = dom["counts"].as_object().unwrap().values().map(|v| v.as_u64().unwrap()).sum();
        assert_eq!(n, 31, "{}", dom["domain"]);
    }
    let agriql = domains.iter().find(|x| x["domain"] == "future-agriql-integration-contract").unwrap();
    assert_eq!(agriql["counts"]["blocked-external"], 21, "a planned product is never reported as working");
}
