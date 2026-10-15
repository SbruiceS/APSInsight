//! APSInsight HTTP API (`/api/v1`).
//!
//! * Authentication: `Authorization: Bearer <token>`; tokens map to a tenant-bound
//!   principal (see `apsi_core::auth::TokenRegistry`). The tenant is always taken
//!   from the token, never from the request, so cross-tenant access is impossible
//!   by construction and additionally denied by the engine.
//! * Envelopes: success `{ "data": ..., "meta": {...} }`, failure
//!   `{ "error": { code, message, retryable, details? }, "meta": {...} }`.
//! * Limits: request bodies <= 16 MiB, per-token rate limit (fixed one-minute window),
//!   search pages <= 500, bulk requests <= 10,000 items.

pub mod api;
pub mod auth;

use apsi_core::Engine;
use apsi_core::auth::TokenRegistry;
use axum::Router;
use axum::http::{HeaderName, HeaderValue, header};
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;

pub const MAX_BODY_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone)]
pub struct AppState {
    pub engine: Arc<Engine>,
    pub tokens: Arc<TokenRegistry>,
    pub limiter: Arc<auth::RateLimiter>,
}

impl AppState {
    pub fn new(engine: Engine, tokens: TokenRegistry, requests_per_minute: u32) -> Self {
        Self { engine: Arc::new(engine), tokens: Arc::new(tokens), limiter: Arc::new(auth::RateLimiter::new(requests_per_minute)) }
    }
}

pub fn router(state: AppState, web_dir: Option<PathBuf>) -> Router {
    let security =
        |name: &'static str, value: &'static str| SetResponseHeaderLayer::if_not_present(HeaderName::from_static(name), HeaderValue::from_static(value));
    let api = api::routes()
        .layer(security("x-content-type-options", "nosniff"))
        .layer(security("x-frame-options", "DENY"))
        .layer(security("content-security-policy", "default-src 'none'; frame-ancestors 'none'"))
        .layer(security("referrer-policy", "no-referrer"))
        .layer(SetResponseHeaderLayer::if_not_present(header::CACHE_CONTROL, HeaderValue::from_static("no-store")));
    let mut app = Router::new()
        .merge(api)
        .layer(axum::middleware::from_fn(api::request_id))
        .layer(RequestBodyLimitLayer::new(MAX_BODY_BYTES))
        .layer(TraceLayer::new_for_http())
        .with_state(state);
    if let Some(dir) = web_dir {
        // Angular single-page app: unknown paths fall back to index.html.
        let index = dir.join("index.html");
        app = app.fallback_service(ServeDir::new(dir).fallback(ServeFile::new(index)));
    }
    app
}
