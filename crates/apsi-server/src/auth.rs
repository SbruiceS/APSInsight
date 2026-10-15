//! Bearer-token authentication and per-token rate limiting.

use crate::AppState;
use crate::api::ApiError;
use apsi_core::ApsiError;
use apsi_core::auth::{Principal, hash_token};
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct RateLimiter {
    per_minute: u32,
    windows: Mutex<HashMap<String, (Instant, u32)>>,
}

impl RateLimiter {
    pub fn new(per_minute: u32) -> Self {
        Self { per_minute: per_minute.max(1), windows: Mutex::new(HashMap::new()) }
    }

    /// Fixed one-minute window per key. Returns false when the key is over budget.
    pub fn check(&self, key: &str) -> bool {
        let now = Instant::now();
        let mut w = self.windows.lock();
        if w.len() > 100_000 {
            w.retain(|_, (start, _)| now.duration_since(*start) < Duration::from_secs(60));
        }
        let entry = w.entry(key.to_string()).or_insert((now, 0));
        if now.duration_since(entry.0) >= Duration::from_secs(60) {
            *entry = (now, 0);
        }
        entry.1 += 1;
        entry.1 <= self.per_minute
    }
}

/// Extractor: authenticated, rate-limited principal.
pub struct Auth(pub Principal);

impl FromRequestParts<AppState> for Auth {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        // Rejections happen before the engine sees the request, so they are counted
        // here (operation `http.auth`, no domain) for the denied/rate-limited alerts.
        let reject = |e: ApsiError| {
            state.engine.metrics().count("http.auth", "-", e.code());
            ApiError(e)
        };
        let header = parts.headers.get(axum::http::header::AUTHORIZATION).and_then(|v| v.to_str().ok()).unwrap_or("");
        let token = header.strip_prefix("Bearer ").map(str::trim).filter(|t| !t.is_empty()).ok_or_else(|| reject(ApsiError::Unauthenticated))?;
        let principal = state.tokens.resolve(token).map_err(reject)?;
        if !state.limiter.check(&hash_token(token)) {
            return Err(reject(ApsiError::RateLimited));
        }
        Ok(Auth(principal))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limiter_blocks_after_budget() {
        let l = RateLimiter::new(3);
        assert!(l.check("a") && l.check("a") && l.check("a"));
        assert!(!l.check("a"));
        assert!(l.check("b"), "keys are independent");
    }
}
