//! AgriQL contract v1.0: query submission and typed, unit-carrying results.
//!
//! AgriQL (the planned agricultural query language and database engine) does not
//! exist yet. This module fixes what APSInsight will send and what it will accept.

use crate::common::{Adapter, ContractError, Result, Unavailable, check_crs, check_sha256, check_unit, non_empty};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const MAX_QUERY_BYTES: usize = 64 * 1024;
pub const MAX_ROWS: u32 = 1_000_000;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuerySubmission {
    pub tenant: String,
    /// AgriQL source text. APSInsight never interpolates values into it;
    /// values travel in `parameters`.
    pub query: String,
    pub language_version: String,
    #[serde(default)]
    pub parameters: BTreeMap<String, Parameter>,
    /// Required when the query has spatial predicates.
    #[serde(default)]
    pub spatial: Option<SpatialContext>,
    pub max_rows: u32,
    pub timeout_ms: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Parameter {
    Text { value: String },
    Number { value: f64, unit: String },
    Boolean { value: bool },
    Timestamp { value: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpatialContext {
    pub crs: String,
    /// `[min_x, min_y, max_x, max_y]` in `crs` axis order.
    pub bbox: [f64; 4],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Column {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: ColumnType,
    /// Mandatory for `quantity` columns, forbidden otherwise.
    #[serde(default)]
    pub unit: Option<String>,
    /// Mandatory for `geometry` columns.
    #[serde(default)]
    pub crs: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColumnType {
    Text,
    Integer,
    Quantity,
    Boolean,
    Timestamp,
    Geometry,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryResult {
    pub columns: Vec<Column>,
    /// Row-major; `null` marks a missing value.
    pub rows: Vec<Vec<serde_json::Value>>,
    pub truncated: bool,
    pub provenance: QueryProvenance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryProvenance {
    /// SHA-256 of the query text, so a result can be tied to its query.
    pub query_sha256: String,
    pub engine_version: String,
    /// Dataset id → version actually read.
    pub dataset_versions: BTreeMap<String, String>,
    pub executed_at: String,
}

impl QuerySubmission {
    pub fn validate(&self) -> Result<()> {
        non_empty("tenant", &self.tenant, 64)?;
        non_empty("query", &self.query, MAX_QUERY_BYTES)?;
        non_empty("language_version", &self.language_version, 16)?;
        if self.max_rows == 0 || self.max_rows > MAX_ROWS {
            return Err(ContractError::invalid("max_rows", format!("must be within 1..={MAX_ROWS}")));
        }
        if !(1..=300_000).contains(&self.timeout_ms) {
            return Err(ContractError::invalid("timeout_ms", "must be within 1..=300000"));
        }
        for (name, p) in &self.parameters {
            let f = format!("parameters.{name}");
            if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
                return Err(ContractError::invalid(f, "parameter names are [A-Za-z0-9_]+"));
            }
            match p {
                Parameter::Number { value, unit } => {
                    crate::common::finite(&f, *value)?;
                    check_unit(&format!("{f}.unit"), unit)?;
                }
                Parameter::Timestamp { value } => {
                    crate::common::parse_time(&f, value)?;
                }
                Parameter::Text { value } if value.len() > 4096 => return Err(ContractError::invalid(f, "text parameter longer than 4096 bytes")),
                _ => {}
            }
        }
        if let Some(s) = &self.spatial {
            check_crs("spatial.crs", &s.crs)?;
            if s.bbox.iter().any(|v| !v.is_finite()) || s.bbox[0] > s.bbox[2] || s.bbox[1] > s.bbox[3] {
                return Err(ContractError::invalid("spatial.bbox", "must be finite with min <= max"));
            }
        }
        Ok(())
    }
}

impl QueryResult {
    /// Checks a provider's result against the submission that produced it.
    pub fn validate_for(&self, q: &QuerySubmission, query_sha256: &str) -> Result<()> {
        if self.columns.is_empty() {
            return Err(ContractError::invalid("columns", "must not be empty"));
        }
        for (i, c) in self.columns.iter().enumerate() {
            let f = format!("columns[{i}]");
            non_empty(&format!("{f}.name"), &c.name, 128)?;
            match (c.ty, &c.unit) {
                (ColumnType::Quantity, Some(u)) => check_unit(&format!("{f}.unit"), u)?,
                (ColumnType::Quantity, None) => return Err(ContractError::invalid(format!("{f}.unit"), "quantity columns must declare a unit")),
                (_, Some(_)) => return Err(ContractError::invalid(format!("{f}.unit"), "only quantity columns carry a unit")),
                _ => {}
            }
            match (c.ty, &c.crs) {
                (ColumnType::Geometry, Some(crs)) => check_crs(&format!("{f}.crs"), crs)?,
                (ColumnType::Geometry, None) => return Err(ContractError::invalid(format!("{f}.crs"), "geometry columns must declare a CRS")),
                _ => {}
            }
        }
        if self.rows.len() > q.max_rows as usize {
            return Err(ContractError::invalid("rows", "more rows than max_rows"));
        }
        if let Some(i) = self.rows.iter().position(|r| r.len() != self.columns.len()) {
            return Err(ContractError::invalid(format!("rows[{i}]"), "row width differs from column count"));
        }
        check_sha256("provenance.query_sha256", &self.provenance.query_sha256)?;
        if self.provenance.query_sha256 != query_sha256 {
            return Err(ContractError::invalid("provenance.query_sha256", "does not match the submitted query"));
        }
        non_empty("provenance.engine_version", &self.provenance.engine_version, 64)?;
        crate::common::parse_time("provenance.executed_at", &self.provenance.executed_at)?;
        Ok(())
    }
}

/// Lowercase hex SHA-256 of the query text, as required in [`QueryProvenance`].
pub fn query_sha256(query: &str) -> String {
    use sha2::Digest;
    hex::encode(sha2::Sha256::digest(query.as_bytes()))
}

pub trait AgriQlPort: Adapter {
    fn submit(&self, q: &QuerySubmission) -> Result<QueryResult>;
}

impl AgriQlPort for Unavailable {
    fn submit(&self, q: &QuerySubmission) -> Result<QueryResult> {
        q.validate()?;
        Err(self.error())
    }
}
