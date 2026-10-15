//! Route table and handlers. Every handler authenticates, then delegates to the
//! engine on a blocking thread and wraps the outcome in the standard envelope.

use crate::AppState;
use crate::auth::Auth;
use apsi_core::auth::{Permission, Principal};
use apsi_engine::plan::Workflow;
use apsi_engine::records::{RecordSource, to_core};
use apsi_engine::source::DataSource;
use apsi_core::bulk::BulkRequest;
use apsi_core::catalog::{catalog, features};
use apsi_core::engine::ImportBody;
use apsi_core::exchange::Format;
use apsi_core::migration::MigrationPlan;
use apsi_core::notify::NotificationRule;
use apsi_core::record::{Lifecycle, NewRecord, RecordPatch};
use apsi_core::retention::RetentionPolicy;
use apsi_core::schema::EntitySchema;
use apsi_core::search::{Query, SortKey};
use apsi_core::validation::ValidationRule;
use apsi_core::workflow::WorkflowDefinition;
use apsi_core::{ApsiError, Engine, traceability};
use axum::Router;
use axum::extract::rejection::JsonRejection;
use axum::extract::{FromRequest, FromRequestParts, Path, Request, State};
use axum::http::request::Parts;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::{any, delete, get, post, put};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::sync::Arc;

pub const API_VERSION: &str = "v1";

/// Error wrapper rendering the standard failure envelope.
pub struct ApiError(pub ApsiError);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.0.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let mut resp = (status, axum::Json(json!({ "error": self.0.body(), "meta": { "api_version": API_VERSION } }))).into_response();
        if matches!(self.0, ApsiError::RateLimited) {
            resp.headers_mut().insert(header::RETRY_AFTER, HeaderValue::from_static("60"));
        }
        if matches!(self.0, ApsiError::Unauthenticated) {
            resp.headers_mut().insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
        }
        resp
    }
}

impl From<ApsiError> for ApiError {
    fn from(e: ApsiError) -> Self {
        ApiError(e)
    }
}

type ApiResult = Result<Response, ApiError>;

fn ok<T: serde::Serialize>(data: T) -> ApiResult {
    Ok(axum::Json(json!({ "data": data, "meta": { "api_version": API_VERSION } })).into_response())
}

fn created<T: serde::Serialize>(data: T) -> ApiResult {
    Ok((StatusCode::CREATED, axum::Json(json!({ "data": data, "meta": { "api_version": API_VERSION } }))).into_response())
}

async fn blocking<T: Send + 'static>(engine: &Arc<Engine>, f: impl FnOnce(&Engine) -> apsi_core::Result<T> + Send + 'static) -> Result<T, ApiError> {
    let engine = engine.clone();
    tokio::task::spawn_blocking(move || f(&engine)).await.map_err(|e| ApiError(ApsiError::Internal(e.to_string())))?.map_err(ApiError)
}

/// JSON body extractor whose rejections use the standard envelope.
pub struct ApiJson<T>(pub T);

impl<T: DeserializeOwned, S: Send + Sync> FromRequest<S> for ApiJson<T> {
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match axum::Json::<T>::from_request(req, state).await {
            Ok(axum::Json(v)) => Ok(ApiJson(v)),
            Err(rej) => Err(ApiError(json_rejection(rej))),
        }
    }
}

fn json_rejection(rej: JsonRejection) -> ApsiError {
    if rej.status() == StatusCode::PAYLOAD_TOO_LARGE {
        ApsiError::PayloadTooLarge(format!("request body exceeds {} bytes", crate::MAX_BODY_BYTES))
    } else {
        ApsiError::InvalidInput(rej.body_text())
    }
}

/// Query-string extractor with standard error envelopes.
pub struct ApiQuery<T>(pub T);

impl<T: DeserializeOwned, S: Send + Sync> FromRequestParts<S> for ApiQuery<T> {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        axum::extract::Query::<T>::try_from_uri(&parts.uri).map(|q| ApiQuery(q.0)).map_err(|e| ApiError(ApsiError::InvalidInput(e.body_text())))
    }
}

/// Adds/echoes `x-request-id` for correlation between clients, logs and audit.
pub async fn request_id(req: Request, next: Next) -> Response {
    let id = req
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .filter(|v| !v.is_empty() && v.len() <= 64 && v.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'))
        .map(str::to_string)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let span = tracing::info_span!("request", request_id = %id, method = %req.method(), path = %req.uri().path());
    let mut resp = tracing::Instrument::instrument(next.run(req), span).await;
    if let Ok(v) = HeaderValue::from_str(&id) {
        resp.headers_mut().insert("x-request-id", v);
    }
    resp
}

pub fn routes() -> Router<AppState> {
    let kind = "/api/v1/domains/{domain}/entities/{entity}";
    Router::new()
        .route("/health", get(health))
        .route("/metrics", get(metrics))
        .route("/api/v1/me", get(me))
        .route("/api/v1/catalog", get(catalog_h))
        .route("/api/v1/features", get(features_h))
        .route("/api/v1/features/{id}", get(feature_h))
        .route("/api/v1/traceability", get(traceability_h))
        .route("/api/v1/status", get(status_h))
        .route("/api/v1/overview", get(overview))
        .route("/api/v1/domains/{domain}/dashboard", get(dashboard))
        .route("/api/v1/domains/{domain}/search", post(search))
        .route(&format!("{kind}/records"), get(list_records).post(create_record))
        .route(&format!("{kind}/validate"), post(validate_h))
        .route(&format!("{kind}/bulk"), post(bulk))
        .route(&format!("{kind}/import"), post(import))
        .route(&format!("{kind}/export"), get(export))
        .route(&format!("{kind}/schemas"), get(schemas).post(publish_schema))
        .route(&format!("{kind}/rules"), get(get_rules).put(put_rules))
        .route(&format!("{kind}/quality"), get(quality))
        .route(&format!("{kind}/retention"), get(get_retention).put(put_retention))
        .route(&format!("{kind}/migrations"), post(migrate))
        .route("/api/v1/records/{id}", get(get_record).patch(patch_record).delete(delete_record))
        .route("/api/v1/records/{id}/lifecycle", post(lifecycle))
        .route("/api/v1/records/{id}/history", get(history))
        .route("/api/v1/records/{id}/diff", get(diff_h))
        .route("/api/v1/records/{id}/rollback", post(rollback))
        .route("/api/v1/records/{id}/legal-hold", put(legal_hold))
        .route("/api/v1/retention/apply", post(apply_retention))
        .route("/api/v1/migrations/{id}/rollback", post(rollback_migration))
        .route("/api/v1/notification-rules", get(list_nrules).post(create_nrule))
        .route("/api/v1/notification-rules/{id}", delete(delete_nrule))
        .route("/api/v1/notifications", get(notifications))
        .route("/api/v1/workflows", get(list_workflows).post(register_workflow))
        .route("/api/v1/workflows/{id}/runs", post(run_workflow))
        .route("/api/v1/workflow-runs", get(workflow_runs))
        .route("/api/v1/integrations", get(integrations_h))
        .route("/api/v1/compute", get(compute_list))
        .route("/api/v1/compute/{function}", post(compute))
        .route("/api/v1/engine", get(engine_capabilities))
        .route("/api/v1/engine/tables", get(engine_tables))
        .route("/api/v1/engine/plan", post(engine_plan))
        .route("/api/v1/engine/run", post(engine_run))
        .route("/api/v1/engine/sql", post(engine_sql))
        .route("/api/v1/audit", get(audit))
        .route("/api/v1/audit/verify", get(audit_verify))
        .route("/api/{*rest}", any(api_not_found))
}

async fn api_not_found() -> ApiError {
    ApiError(ApsiError::NotFound("no such API route".into()))
}

async fn health() -> ApiResult {
    ok(json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION") }))
}

/// Prometheus metrics. Requires the `audit` permission (auditor or admin).
async fn metrics(State(app): State<AppState>, Auth(p): Auth) -> ApiResult {
    p.authorize(&p.tenant, Permission::Audit)?;
    Ok(([(header::CONTENT_TYPE, "text/plain; version=0.0.4")], app.engine.metrics().render()).into_response())
}

async fn me(Auth(p): Auth) -> ApiResult {
    ok(p)
}

async fn catalog_h(Auth(_): Auth) -> ApiResult {
    let c = catalog();
    ok(json!({ "features": c.features, "entity_kinds": c.entity_kind_count(), "capabilities": c.capabilities, "domains": c.domains }))
}

#[derive(Deserialize)]
struct FeatureParams {
    domain: Option<String>,
    capability: Option<String>,
    q: Option<String>,
    #[serde(default)]
    offset: usize,
    limit: Option<usize>,
}

async fn features_h(Auth(_): Auth, ApiQuery(f): ApiQuery<FeatureParams>) -> ApiResult {
    let limit = f.limit.unwrap_or(100);
    if limit == 0 || limit > 500 {
        return Err(ApsiError::InvalidInput("limit must be within 1..=500".into()).into());
    }
    let c = catalog();
    let q = f.q.map(|s| s.to_lowercase());
    let hits: Vec<_> = features()
        .iter()
        .filter(|feat| {
            let (d, cap) = c.feature_coordinates(&feat.id).expect("register ids are valid");
            f.domain.as_ref().is_none_or(|x| *x == d.slug)
                && f.capability.as_ref().is_none_or(|x| *x == cap.slug)
                && q.as_ref()
                    .is_none_or(|q| feat.id.to_lowercase().contains(q) || feat.requirement.to_lowercase().contains(q) || feat.title.to_lowercase().contains(q))
        })
        .collect();
    let total = hits.len();
    let items: Vec<_> = hits.into_iter().skip(f.offset).take(limit).collect();
    ok(json!({ "total": total, "offset": f.offset, "limit": limit, "items": items }))
}

async fn feature_h(Auth(_): Auth, Path(id): Path<String>) -> ApiResult {
    let row = traceability::row(&id).ok_or_else(|| ApsiError::NotFound(format!("feature {id}")))?;
    let feature = features().iter().find(|f| f.id == id).cloned();
    ok(json!({ "feature": feature, "evidence": row }))
}

async fn traceability_h(Auth(_): Auth) -> ApiResult {
    Ok(([(header::CONTENT_TYPE, "text/csv; charset=utf-8")], traceability::csv()).into_response())
}

/// Honest completion metrics: feature counts per status, overall and per domain.
async fn status_h(Auth(_): Auth) -> ApiResult {
    let rows = traceability::rows();
    let statuses = traceability::Status::ALL;
    let by_status: Vec<Value> = traceability::status_counts().into_iter().map(|(s, n)| json!({ "status": s, "label": s.label(), "features": n })).collect();
    let domains: Vec<Value> = catalog()
        .domains
        .iter()
        .map(|d| {
            let counts: serde_json::Map<String, Value> = statuses
                .iter()
                .map(|st| {
                    let key = serde_json::to_value(st).ok().and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default();
                    (key, json!(rows.iter().filter(|r| r.domain_slug == d.slug && r.status == *st).count()))
                })
                .collect();
            json!({ "domain": d.slug, "name": d.name, "counts": counts })
        })
        .collect();
    ok(json!({ "total": rows.len(), "by_status": by_status, "domains": domains }))
}

async fn overview(State(app): State<AppState>, Auth(p): Auth) -> ApiResult {
    ok(blocking(&app.engine, move |e| e.overview(&p, &p.tenant)).await?)
}

async fn dashboard(State(app): State<AppState>, Auth(p): Auth, Path(domain): Path<String>) -> ApiResult {
    ok(blocking(&app.engine, move |e| e.dashboard(&p, &p.tenant, &domain)).await?)
}

async fn search(State(app): State<AppState>, Auth(p): Auth, Path(domain): Path<String>, ApiJson(mut q): ApiJson<Query>) -> ApiResult {
    q.domain = domain;
    ok(blocking(&app.engine, move |e| e.search(&p, &p.tenant, &q)).await?)
}

#[derive(Deserialize)]
struct ListParams {
    q: Option<String>,
    lifecycle: Option<String>,
    /// Comma-separated tags; all must match.
    tags: Option<String>,
    /// Comma-separated facet fields.
    facets: Option<String>,
    #[serde(default)]
    include_archived: bool,
    sort: Option<SortKey>,
    #[serde(default)]
    desc: bool,
    #[serde(default)]
    offset: usize,
    limit: Option<usize>,
}

fn split(s: Option<String>) -> Vec<String> {
    s.map(|s| s.split(',').map(str::trim).filter(|t| !t.is_empty()).map(str::to_string).collect()).unwrap_or_default()
}

async fn list_records(
    State(app): State<AppState>,
    Auth(p): Auth,
    Path((domain, entity)): Path<(String, String)>,
    ApiQuery(l): ApiQuery<ListParams>,
) -> ApiResult {
    let q = Query {
        domain,
        entity: Some(entity),
        text: l.q.filter(|s| !s.is_empty()),
        lifecycle: l.lifecycle.as_deref().map(Lifecycle::parse).transpose()?,
        include_archived: l.include_archived,
        tags: split(l.tags),
        filters: vec![],
        facets: split(l.facets),
        sort: l.sort.unwrap_or_default(),
        descending: l.desc,
        offset: l.offset,
        limit: l.limit,
    };
    ok(blocking(&app.engine, move |e| e.search(&p, &p.tenant, &q)).await?)
}

async fn create_record(
    State(app): State<AppState>,
    Auth(p): Auth,
    Path((domain, entity)): Path<(String, String)>,
    ApiJson(body): ApiJson<NewRecord>,
) -> ApiResult {
    created(blocking(&app.engine, move |e| e.create(&p, &p.tenant, &domain, &entity, body)).await?)
}

async fn validate_h(
    State(app): State<AppState>,
    Auth(p): Auth,
    Path((domain, entity)): Path<(String, String)>,
    ApiJson(body): ApiJson<NewRecord>,
) -> ApiResult {
    ok(blocking(&app.engine, move |e| e.validate_preview(&p, &p.tenant, &domain, &entity, &body)).await?)
}

async fn bulk(State(app): State<AppState>, Auth(p): Auth, Path((domain, entity)): Path<(String, String)>, ApiJson(body): ApiJson<BulkRequest>) -> ApiResult {
    ok(blocking(&app.engine, move |e| e.bulk(&p, &p.tenant, &domain, &entity, body)).await?)
}

#[derive(Deserialize)]
struct ImportParams {
    format: Option<String>,
    #[serde(default)]
    dry_run: bool,
    #[serde(default)]
    upsert: bool,
    idempotency_key: Option<String>,
}

async fn import(
    State(app): State<AppState>,
    Auth(p): Auth,
    Path((domain, entity)): Path<(String, String)>,
    ApiQuery(ip): ApiQuery<ImportParams>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> ApiResult {
    let format = match ip.format.as_deref() {
        Some(f) => Format::parse(f)?,
        None if headers.get(header::CONTENT_TYPE).and_then(|v| v.to_str().ok()).is_some_and(|c| c.starts_with("text/csv")) => Format::Csv,
        None => Format::Json,
    };
    let text = String::from_utf8(body.to_vec()).map_err(|_| ApsiError::InvalidInput("import body must be UTF-8".into()))?;
    let options = BulkRequest { items: vec![], dry_run: ip.dry_run, upsert: ip.upsert, idempotency_key: ip.idempotency_key, mapping: None };
    ok(blocking(&app.engine, move |e| e.import(&p, &p.tenant, &domain, &entity, ImportBody { format, text: &text }, options)).await?)
}

#[derive(Deserialize)]
struct ExportParams {
    format: Option<String>,
    #[serde(default)]
    include_archived: bool,
}

async fn export(State(app): State<AppState>, Auth(p): Auth, Path((domain, entity)): Path<(String, String)>, ApiQuery(xp): ApiQuery<ExportParams>) -> ApiResult {
    let format = Format::parse(xp.format.as_deref().unwrap_or("json"))?;
    let name = format!("{domain}-{entity}");
    let text = blocking(&app.engine, move |e| e.export(&p, &p.tenant, &domain, &entity, format, xp.include_archived)).await?;
    let ext = match format {
        Format::Json => "json",
        Format::Ndjson => "ndjson",
        Format::Csv => "csv",
        Format::Geojson => "geojson",
    };
    let disposition = format!("attachment; filename=\"{name}.{ext}\"");
    Ok(([(header::CONTENT_TYPE, format.content_type().to_string()), (header::CONTENT_DISPOSITION, disposition)], text).into_response())
}

async fn schemas(State(app): State<AppState>, Auth(p): Auth, Path((domain, entity)): Path<(String, String)>) -> ApiResult {
    ok(blocking(&app.engine, move |e| e.schemas(&p, &p.tenant, &domain, &entity)).await?)
}

#[derive(Deserialize)]
struct PublishParams {
    #[serde(default)]
    allow_breaking: bool,
}

async fn publish_schema(
    State(app): State<AppState>,
    Auth(p): Auth,
    Path((domain, entity)): Path<(String, String)>,
    ApiQuery(pp): ApiQuery<PublishParams>,
    ApiJson(mut s): ApiJson<EntitySchema>,
) -> ApiResult {
    s.domain = domain;
    s.entity = entity;
    created(blocking(&app.engine, move |e| e.publish_schema(&p, &p.tenant, s, pp.allow_breaking)).await?)
}

async fn get_rules(State(app): State<AppState>, Auth(p): Auth, Path((domain, entity)): Path<(String, String)>) -> ApiResult {
    ok(blocking(&app.engine, move |e| e.rules(&p, &p.tenant, &domain, &entity)).await?)
}

async fn put_rules(
    State(app): State<AppState>,
    Auth(p): Auth,
    Path((domain, entity)): Path<(String, String)>,
    ApiJson(rules): ApiJson<Vec<ValidationRule>>,
) -> ApiResult {
    ok(blocking(&app.engine, move |e| e.set_rules(&p, &p.tenant, &domain, &entity, rules)).await?)
}

async fn quality(State(app): State<AppState>, Auth(p): Auth, Path((domain, entity)): Path<(String, String)>) -> ApiResult {
    ok(blocking(&app.engine, move |e| e.quality(&p, &p.tenant, &domain, &entity)).await?)
}

async fn get_retention(State(app): State<AppState>, Auth(p): Auth, Path((domain, entity)): Path<(String, String)>) -> ApiResult {
    ok(blocking(&app.engine, move |e| e.retention(&p, &p.tenant, &domain, &entity)).await?)
}

async fn put_retention(
    State(app): State<AppState>,
    Auth(p): Auth,
    Path((domain, entity)): Path<(String, String)>,
    ApiJson(policy): ApiJson<RetentionPolicy>,
) -> ApiResult {
    ok(blocking(&app.engine, move |e| e.set_retention(&p, &p.tenant, &domain, &entity, policy)).await?)
}

#[derive(Deserialize)]
struct DryRun {
    #[serde(default)]
    dry_run: bool,
}

#[derive(Deserialize)]
struct MigrationBody {
    to_version: u32,
    ops: Vec<apsi_core::migration::MigrationOp>,
}

async fn migrate(
    State(app): State<AppState>,
    Auth(p): Auth,
    Path((domain, entity)): Path<(String, String)>,
    ApiQuery(d): ApiQuery<DryRun>,
    ApiJson(b): ApiJson<MigrationBody>,
) -> ApiResult {
    let plan = MigrationPlan { id: String::new(), domain, entity, to_version: b.to_version, ops: b.ops };
    ok(blocking(&app.engine, move |e| e.migrate(&p, &p.tenant, plan, d.dry_run)).await?)
}

async fn get_record(State(app): State<AppState>, Auth(p): Auth, Path(id): Path<String>) -> ApiResult {
    let r = blocking(&app.engine, move |e| e.get(&p, &p.tenant, &id)).await?;
    let etag = format!("\"{}\"", r.version);
    let mut resp = ok(r)?;
    if let Ok(v) = HeaderValue::from_str(&etag) {
        resp.headers_mut().insert(header::ETAG, v);
    }
    Ok(resp)
}

#[derive(Deserialize)]
struct PatchBody {
    #[serde(default)]
    expected_version: Option<u64>,
    #[serde(flatten)]
    patch: RecordPatch,
}

fn expected_version(headers: &HeaderMap, body: Option<u64>) -> Result<u64, ApiError> {
    let from_header = headers.get(header::IF_MATCH).and_then(|v| v.to_str().ok()).map(|s| s.trim_matches('"').parse::<u64>());
    match (body, from_header) {
        (Some(v), _) => Ok(v),
        (None, Some(Ok(v))) => Ok(v),
        (None, Some(Err(_))) => Err(ApsiError::InvalidInput("If-Match must be a record version such as \"3\"".into()).into()),
        (None, None) => Err(ApsiError::InvalidInput("expected_version (or If-Match) is required for updates".into()).into()),
    }
}

async fn patch_record(State(app): State<AppState>, Auth(p): Auth, Path(id): Path<String>, headers: HeaderMap, ApiJson(b): ApiJson<PatchBody>) -> ApiResult {
    let v = expected_version(&headers, b.expected_version)?;
    ok(blocking(&app.engine, move |e| e.update(&p, &p.tenant, &id, v, b.patch)).await?)
}

async fn delete_record(State(app): State<AppState>, Auth(p): Auth, Path(id): Path<String>) -> ApiResult {
    blocking(&app.engine, move |e| e.delete(&p, &p.tenant, &id)).await?;
    ok(json!({ "deleted": true }))
}

#[derive(Deserialize)]
struct LifecycleBody {
    to: Lifecycle,
    expected_version: u64,
}

async fn lifecycle(State(app): State<AppState>, Auth(p): Auth, Path(id): Path<String>, ApiJson(b): ApiJson<LifecycleBody>) -> ApiResult {
    ok(blocking(&app.engine, move |e| e.transition(&p, &p.tenant, &id, b.expected_version, b.to)).await?)
}

async fn history(State(app): State<AppState>, Auth(p): Auth, Path(id): Path<String>) -> ApiResult {
    ok(blocking(&app.engine, move |e| e.history(&p, &p.tenant, &id)).await?)
}

#[derive(Deserialize)]
struct DiffParams {
    from: u64,
    to: u64,
}

async fn diff_h(State(app): State<AppState>, Auth(p): Auth, Path(id): Path<String>, ApiQuery(d): ApiQuery<DiffParams>) -> ApiResult {
    ok(blocking(&app.engine, move |e| e.diff(&p, &p.tenant, &id, d.from, d.to)).await?)
}

#[derive(Deserialize)]
struct RollbackBody {
    to_version: u64,
    expected_version: u64,
}

async fn rollback(State(app): State<AppState>, Auth(p): Auth, Path(id): Path<String>, ApiJson(b): ApiJson<RollbackBody>) -> ApiResult {
    ok(blocking(&app.engine, move |e| e.rollback(&p, &p.tenant, &id, b.to_version, b.expected_version)).await?)
}

#[derive(Deserialize)]
struct HoldBody {
    hold: bool,
}

async fn legal_hold(State(app): State<AppState>, Auth(p): Auth, Path(id): Path<String>, ApiJson(b): ApiJson<HoldBody>) -> ApiResult {
    ok(blocking(&app.engine, move |e| e.set_legal_hold(&p, &p.tenant, &id, b.hold)).await?)
}

async fn apply_retention(State(app): State<AppState>, Auth(p): Auth, ApiQuery(d): ApiQuery<DryRun>) -> ApiResult {
    ok(blocking(&app.engine, move |e| e.apply_retention(&p, &p.tenant, d.dry_run)).await?)
}

async fn rollback_migration(State(app): State<AppState>, Auth(p): Auth, Path(id): Path<String>) -> ApiResult {
    ok(blocking(&app.engine, move |e| e.rollback_migration(&p, &p.tenant, &id)).await?)
}

async fn list_nrules(State(app): State<AppState>, Auth(p): Auth) -> ApiResult {
    ok(blocking(&app.engine, move |e| e.notification_rules(&p, &p.tenant)).await?)
}

async fn create_nrule(State(app): State<AppState>, Auth(p): Auth, ApiJson(rule): ApiJson<NotificationRule>) -> ApiResult {
    created(blocking(&app.engine, move |e| e.add_notification_rule(&p, &p.tenant, rule)).await?)
}

async fn delete_nrule(State(app): State<AppState>, Auth(p): Auth, Path(id): Path<String>) -> ApiResult {
    blocking(&app.engine, move |e| e.delete_notification_rule(&p, &p.tenant, &id)).await?;
    ok(json!({ "deleted": true }))
}

#[derive(Deserialize)]
struct LimitParams {
    limit: Option<usize>,
    action: Option<String>,
}

async fn notifications(State(app): State<AppState>, Auth(p): Auth, ApiQuery(l): ApiQuery<LimitParams>) -> ApiResult {
    ok(blocking(&app.engine, move |e| e.notifications(&p, &p.tenant, l.limit.unwrap_or(100))).await?)
}

async fn list_workflows(State(app): State<AppState>, Auth(p): Auth) -> ApiResult {
    ok(blocking(&app.engine, move |e| e.workflows(&p, &p.tenant)).await?)
}

async fn register_workflow(State(app): State<AppState>, Auth(p): Auth, ApiJson(def): ApiJson<WorkflowDefinition>) -> ApiResult {
    created(blocking(&app.engine, move |e| e.register_workflow(&p, &p.tenant, def)).await?)
}

#[derive(Deserialize)]
struct RunParams {
    version: Option<u32>,
}

async fn run_workflow(State(app): State<AppState>, Auth(p): Auth, Path(id): Path<String>, ApiQuery(rp): ApiQuery<RunParams>) -> ApiResult {
    created(blocking(&app.engine, move |e| e.run_workflow(&p, &p.tenant, &id, rp.version)).await?)
}

async fn workflow_runs(State(app): State<AppState>, Auth(p): Auth) -> ApiResult {
    ok(blocking(&app.engine, move |e| e.workflow_runs(&p, &p.tenant)).await?)
}

/// Capability discovery for planned ASPF products. Every product is optional and,
/// in this build, reported unavailable: no product implementation exists.
async fn integrations_h(Auth(_): Auth) -> ApiResult {
    ok(apsi_contracts::discover_all())
}

async fn compute_list(State(app): State<AppState>, Auth(_): Auth) -> ApiResult {
    ok(app.engine.compute().map(|c| c.functions()).unwrap_or_default())
}

async fn compute(State(app): State<AppState>, Auth(p): Auth, Path(function): Path<String>, ApiJson(input): ApiJson<Value>) -> ApiResult {
    p.authorize(&p.tenant, Permission::Read)?;
    let registry = app.engine.compute().cloned().ok_or_else(|| ApsiError::Internal("no compute registry configured".into()))?;
    let result = tokio::task::spawn_blocking(move || registry.call(&function, &input)).await.map_err(|e| ApsiError::Internal(e.to_string()))?;
    ok(result.map_err(ApsiError::InvalidInput)?)
}

async fn audit(State(app): State<AppState>, Auth(p): Auth, ApiQuery(l): ApiQuery<LimitParams>) -> ApiResult {
    ok(blocking(&app.engine, move |e| e.audit(&p, &p.tenant, l.limit.unwrap_or(100), l.action.as_deref())).await?)
}

async fn audit_verify(State(app): State<AppState>, Auth(p): Auth) -> ApiResult {
    let r = blocking(&app.engine, move |e| e.verify_audit(&p, &p.tenant)).await?;
    ok(match r {
        Ok(events) => json!({ "valid": true, "events": events }),
        Err(seq) => json!({ "valid": false, "broken_at": seq }),
    })
}

// ------------------------------------------------------- compute engine

/// Runs engine work for one principal on a blocking thread, over a fresh
/// per-request record snapshot.
async fn analytics<T: Send + 'static>(
    app: &AppState,
    p: Principal,
    f: impl FnOnce(&apsi_engine::ComputeEngine, &RecordSource) -> apsi_engine::Result<T> + Send + 'static,
) -> Result<T, ApiError> {
    p.authorize(&p.tenant, Permission::Read)?;
    let (engine, compute) = (app.engine.clone(), app.compute.clone());
    tokio::task::spawn_blocking(move || f(&compute, &RecordSource::new(engine, p)))
        .await
        .map_err(|e| ApiError(ApsiError::Internal(e.to_string())))?
        .map_err(|e| ApiError(to_core(e)))
}

fn parquet_response(name: &str, batch: &apsi_engine::types::Batch) -> ApiResult {
    let bytes = apsi_engine::sql::to_parquet(batch).map_err(|e| ApiError(to_core(e)))?;
    let safe: String = name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
    let disposition = format!("attachment; filename=\"{safe}.parquet\"");
    Ok(([(header::CONTENT_TYPE, "application/vnd.apache.parquet".to_string()), (header::CONTENT_DISPOSITION, disposition)], bytes).into_response())
}

async fn engine_capabilities(State(app): State<AppState>, Auth(p): Auth) -> ApiResult {
    p.authorize(&p.tenant, Permission::Read)?;
    ok(app.compute.capabilities())
}

async fn engine_tables(State(app): State<AppState>, Auth(p): Auth) -> ApiResult {
    let tables = analytics(&app, p, |_, src| {
        src.tables()?
            .into_iter()
            .map(|t| {
                let schema = src.schema(&t.domain, &t.entity)?;
                Ok(json!({ "table": t.table, "domain": t.domain, "entity": t.entity, "rows": t.rows, "columns": schema.fields }))
            })
            .collect::<apsi_engine::Result<Vec<Value>>>()
    })
    .await?;
    ok(tables)
}

async fn engine_plan(State(app): State<AppState>, Auth(p): Auth, ApiJson(wf): ApiJson<Workflow>) -> ApiResult {
    ok(analytics(&app, p, move |c, src| c.plan(&wf, src)).await?)
}

#[derive(Deserialize)]
struct EngineFormat {
    /// `json` (default) or `parquet`.
    format: Option<String>,
    /// Output to export as Parquet; defaults to the first output.
    output: Option<String>,
}

fn wants_parquet(f: &EngineFormat) -> Result<bool, ApiError> {
    match f.format.as_deref().unwrap_or("json") {
        "json" => Ok(false),
        "parquet" => Ok(true),
        other => Err(ApsiError::InvalidInput(format!("format must be json or parquet, got `{other}`")).into()),
    }
}

async fn engine_run(State(app): State<AppState>, Auth(p): Auth, ApiQuery(f): ApiQuery<EngineFormat>, ApiJson(wf): ApiJson<Workflow>) -> ApiResult {
    if !wants_parquet(&f)? {
        return ok(analytics(&app, p, move |c, src| c.run(&wf, src)).await?);
    }
    let name = f.output.clone().or_else(|| wf.outputs.first().cloned()).unwrap_or_default();
    if !wf.outputs.contains(&name) {
        return Err(ApsiError::InvalidInput(format!("`{name}` is not an output of this workflow")).into());
    }
    let wanted = name.clone();
    let batch = analytics(&app, p, move |c, src| {
        let (_, mut batches) = c.run_batches(&wf, src)?;
        batches.remove(&wanted).ok_or_else(|| apsi_engine::EngineError::Internal("output missing".into()))
    })
    .await?;
    parquet_response(&name, &batch)
}

#[derive(Deserialize)]
struct SqlBody {
    sql: String,
    #[serde(default)]
    max_rows: Option<usize>,
    #[serde(default)]
    explain: bool,
}

async fn engine_sql(State(app): State<AppState>, Auth(p): Auth, ApiQuery(f): ApiQuery<EngineFormat>, ApiJson(body): ApiJson<SqlBody>) -> ApiResult {
    p.authorize(&p.tenant, Permission::Read)?;
    let parquet = wants_parquet(&f)?;
    let max_rows = body.max_rows.unwrap_or(1000);
    if max_rows == 0 || max_rows > 100_000 {
        return Err(ApsiError::InvalidInput("max_rows must be within 1..=100000".into()).into());
    }
    let limits = apsi_engine::sql::SqlLimits { max_rows, ..Default::default() };
    let source: Arc<dyn apsi_engine::source::DataSource> = Arc::new(RecordSource::new(app.engine.clone(), p));
    let (result, batch) = apsi_engine::sql::query_with_batch(source, &body.sql, &limits, body.explain).await.map_err(|e| ApiError(to_core(e)))?;
    if parquet { parquet_response("query", &batch) } else { ok(result) }
}
