//! Typed errors shared by every capability (feature family "Failure handling").
//!
//! Each error carries a stable machine code, an HTTP status mapping and an explicit
//! retryable flag so callers can distinguish transient from permanent failures.

use crate::validation::ValidationReport;
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum ApsiError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("validation failed")]
    Validation(Box<ValidationReport>),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("version conflict: expected {expected}, current {current}")]
    VersionConflict { expected: u64, current: u64 },
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("lifecycle violation: {0}")]
    Lifecycle(String),
    #[error("record is under legal hold: {0}")]
    LegalHold(String),
    #[error("authentication required")]
    Unauthenticated,
    #[error("forbidden: {0}")]
    Forbidden(String),
    #[error("rate limit exceeded")]
    RateLimited,
    #[error("payload too large: {0}")]
    PayloadTooLarge(String),
    #[error("storage unavailable: {0}")]
    Storage(String),
    #[error("internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, ApsiError>;

/// Serializable error body used in API envelopes and CLI JSON output.
#[derive(Debug, Clone, Serialize)]
pub struct ErrorBody {
    pub code: &'static str,
    pub message: String,
    pub retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

/// Owned, deserializable variant of [`ErrorBody`] for stored reports.
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct ErrorBodyOwned {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl From<&ApsiError> for ErrorBodyOwned {
    fn from(e: &ApsiError) -> Self {
        let b = e.body();
        Self { code: b.code.into(), message: b.message, retryable: b.retryable, details: b.details }
    }
}

impl ApsiError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidInput(_) => "invalid_input",
            Self::Validation(_) => "validation_failed",
            Self::NotFound(_) => "not_found",
            Self::VersionConflict { .. } => "version_conflict",
            Self::Conflict(_) => "conflict",
            Self::Lifecycle(_) => "lifecycle_violation",
            Self::LegalHold(_) => "legal_hold",
            Self::Unauthenticated => "unauthenticated",
            Self::Forbidden(_) => "forbidden",
            Self::RateLimited => "rate_limited",
            Self::PayloadTooLarge(_) => "payload_too_large",
            Self::Storage(_) => "storage_unavailable",
            Self::Internal(_) => "internal",
        }
    }

    /// Retryable failures may succeed if repeated (possibly after a delay or after
    /// refreshing the expected version); permanent ones never will.
    pub fn retryable(&self) -> bool {
        matches!(self, Self::VersionConflict { .. } | Self::RateLimited | Self::Storage(_))
    }

    pub fn http_status(&self) -> u16 {
        match self {
            Self::InvalidInput(_) => 400,
            Self::Validation(_) | Self::Lifecycle(_) => 422,
            Self::NotFound(_) => 404,
            Self::VersionConflict { .. } | Self::Conflict(_) | Self::LegalHold(_) => 409,
            Self::Unauthenticated => 401,
            Self::Forbidden(_) => 403,
            Self::RateLimited => 429,
            Self::PayloadTooLarge(_) => 413,
            Self::Storage(_) => 503,
            Self::Internal(_) => 500,
        }
    }

    /// Stable CLI exit code per error family (0 = success; clap usage errors also exit with 2).
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::InvalidInput(_) => 2,
            Self::Validation(_) => 3,
            Self::NotFound(_) => 4,
            Self::VersionConflict { .. } | Self::Conflict(_) | Self::LegalHold(_) => 5,
            Self::Lifecycle(_) => 6,
            Self::Unauthenticated | Self::Forbidden(_) => 7,
            Self::RateLimited | Self::PayloadTooLarge(_) => 8,
            Self::Storage(_) => 9,
            Self::Internal(_) => 10,
        }
    }

    pub fn body(&self) -> ErrorBody {
        let details = match self {
            Self::Validation(report) => serde_json::to_value(report).ok(),
            Self::VersionConflict { expected, current } => Some(serde_json::json!({ "expected": expected, "current": current })),
            _ => None,
        };
        ErrorBody { code: self.code(), message: self.to_string(), retryable: self.retryable(), details }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retryable_classification_is_explicit() {
        assert!(ApsiError::VersionConflict { expected: 1, current: 2 }.retryable());
        assert!(ApsiError::Storage("disk".into()).retryable());
        assert!(ApsiError::RateLimited.retryable());
        assert!(!ApsiError::NotFound("x".into()).retryable());
        assert!(!ApsiError::Forbidden("x".into()).retryable());
        assert!(!ApsiError::InvalidInput("x".into()).retryable());
    }

    #[test]
    fn codes_and_statuses_are_stable() {
        let e = ApsiError::VersionConflict { expected: 3, current: 4 };
        assert_eq!(e.code(), "version_conflict");
        assert_eq!(e.http_status(), 409);
        assert_eq!(e.exit_code(), 5);
        assert_eq!(e.body().details.unwrap()["current"], 4);
    }
}
