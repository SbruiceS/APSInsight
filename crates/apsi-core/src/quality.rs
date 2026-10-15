//! Quality indicators with documented formulas ("Quality indicators" family).

use crate::record::Record;
use crate::schema::EntitySchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use time::OffsetDateTime;

pub const FORMULAS: [(&str, &str); 4] = [
    ("completeness", "mean over records of (declared schema fields present and non-null) / (declared schema fields); 1.0 when the schema declares no fields"),
    ("validity", "(records whose validation report has zero error-severity issues) / (records evaluated)"),
    ("freshness_days", "mean and max of (now - updated_at) in days"),
    ("provenance_coverage", "(records with a source reference or a documented transformation) / (records evaluated)"),
];

pub const MISSING_DATA: &str = "Absent attributes and explicit nulls both count as missing. With zero records every indicator is null rather than 0 or 1, so empty collections are never reported as perfect or failing.";
pub const LIMITATIONS: &str = "Indicators describe structure and documentation, not scientific correctness: a complete, valid record can still hold a wrong measurement. Archived records are excluded. Freshness says nothing about whether old data is still representative.";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityReport {
    pub domain: String,
    pub entity: String,
    pub schema_version: u32,
    pub records: usize,
    pub completeness: Option<f64>,
    pub validity: Option<f64>,
    pub mean_freshness_days: Option<f64>,
    pub max_freshness_days: Option<f64>,
    pub provenance_coverage: Option<f64>,
    pub formulas: BTreeMap<String, String>,
    pub missing_data: String,
    pub limitations: String,
}

pub fn completeness(schema: &EntitySchema, r: &Record) -> f64 {
    if schema.fields.is_empty() {
        return 1.0;
    }
    let present = schema.fields.iter().filter(|f| r.attributes.get(&f.name).is_some_and(|v| !v.is_null())).count();
    present as f64 / schema.fields.len() as f64
}

/// `valid` holds one flag per record (same order) from validation reports.
pub fn aggregate(schema: &EntitySchema, records: &[&Record], valid: &[bool], now: OffsetDateTime) -> QualityReport {
    let n = records.len();
    let mean = |sum: f64| if n == 0 { None } else { Some(sum / n as f64) };
    let ages: Vec<f64> = records.iter().map(|r| ((now - r.updated_at).as_seconds_f64() / 86_400.0).max(0.0)).collect();
    QualityReport {
        domain: schema.domain.clone(),
        entity: schema.entity.clone(),
        schema_version: schema.version,
        records: n,
        completeness: mean(records.iter().map(|r| completeness(schema, r)).sum()),
        validity: mean(valid.iter().filter(|v| **v).count() as f64),
        mean_freshness_days: mean(ages.iter().sum()),
        max_freshness_days: ages.iter().copied().reduce(f64::max),
        provenance_coverage: mean(records.iter().filter(|r| r.source.is_some() || r.provenance.iter().any(|p| p.transformation.is_some())).count() as f64),
        formulas: FORMULAS.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
        missing_data: MISSING_DATA.into(),
        limitations: LIMITATIONS.into(),
    }
}
