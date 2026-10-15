//! Scientific computations used by APSInsight domains. Every function validates its
//! inputs, documents its formula and units, and returns a typed error instead of
//! silently producing NaN or infinities.

pub mod agronomy;
pub mod economics;
pub mod geo;
pub mod indices;
pub mod registry;
pub mod stats;
pub mod timeseries;
pub mod units;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ScienceError {
    #[error("invalid input: {0}")]
    Invalid(String),
    #[error("insufficient data: {0}")]
    Insufficient(String),
    #[error("undefined result: {0}")]
    Undefined(String),
}

pub type Result<T> = std::result::Result<T, ScienceError>;

pub(crate) fn finite(name: &str, v: f64) -> Result<f64> {
    if v.is_finite() { Ok(v) } else { Err(ScienceError::Invalid(format!("{name} must be finite"))) }
}

pub(crate) fn in_range(name: &str, v: f64, lo: f64, hi: f64) -> Result<f64> {
    finite(name, v)?;
    if v < lo || v > hi {
        return Err(ScienceError::Invalid(format!("{name} = {v} outside [{lo}, {hi}]")));
    }
    Ok(v)
}

pub use registry::ScienceRegistry;
