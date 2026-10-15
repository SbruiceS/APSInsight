//! Schema migrations with dry runs, all-or-nothing application and rollback
//! ("Migration support" family).

use crate::error::{ApsiError, Result};
use crate::ids::validate_field_name;
use crate::record::Attributes;
use crate::validation::ValidationReport;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum MigrationOp {
    Rename {
        from: String,
        to: String,
    },
    SetDefault {
        field: String,
        value: Value,
    },
    Remove {
        field: String,
    },
    /// Multiplies a numeric attribute, e.g. a unit conversion. Non-numeric values fail the migration.
    Scale {
        field: String,
        factor: f64,
    },
    SetUnit {
        unit: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationPlan {
    #[serde(default)]
    pub id: String,
    pub domain: String,
    pub entity: String,
    pub to_version: u32,
    pub ops: Vec<MigrationOp>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationFailure {
    pub record_id: String,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report: Option<ValidationReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationReport {
    pub id: String,
    pub dry_run: bool,
    pub applied: bool,
    pub total: usize,
    pub migratable: usize,
    pub failures: Vec<MigrationFailure>,
    /// record id -> (version before, version after). Used for rollback.
    pub touched: BTreeMap<String, (u64, u64)>,
}

impl MigrationOp {
    pub fn check(&self) -> Result<()> {
        match self {
            MigrationOp::Rename { from, to } => {
                validate_field_name(from)?;
                validate_field_name(to)
            }
            MigrationOp::SetDefault { field, .. } | MigrationOp::Remove { field } => validate_field_name(field),
            MigrationOp::Scale { field, factor } => {
                validate_field_name(field)?;
                if !factor.is_finite() || *factor == 0.0 {
                    return Err(ApsiError::InvalidInput("scale factor must be finite and non-zero".into()));
                }
                Ok(())
            }
            MigrationOp::SetUnit { unit } => {
                if unit.is_empty() || unit.len() > 32 {
                    return Err(ApsiError::InvalidInput("unit must be 1..32 characters".into()));
                }
                Ok(())
            }
        }
    }
}

pub fn apply_ops(attrs: &Attributes, ops: &[MigrationOp]) -> std::result::Result<Attributes, String> {
    let mut a = attrs.clone();
    for op in ops {
        match op {
            MigrationOp::Rename { from, to } => {
                if let Some(v) = a.remove(from) {
                    if a.contains_key(to) {
                        return Err(format!("rename {from} -> {to} would overwrite existing {to}"));
                    }
                    a.insert(to.clone(), v);
                }
            }
            MigrationOp::SetDefault { field, value } => {
                if a.get(field).is_none_or(Value::is_null) {
                    a.insert(field.clone(), value.clone());
                }
            }
            MigrationOp::Remove { field } => {
                a.remove(field);
            }
            MigrationOp::Scale { field, factor } => {
                if let Some(v) = a.get(field) {
                    let n = v.as_f64().ok_or_else(|| format!("{field} is not numeric"))?;
                    let scaled = serde_json::Number::from_f64(n * factor).ok_or_else(|| format!("{field} overflowed"))?;
                    a.insert(field.clone(), Value::Number(scaled));
                }
            }
            MigrationOp::SetUnit { unit } => {
                a.insert("unit".into(), Value::from(unit.clone()));
            }
        }
    }
    Ok(a)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn ops_apply_in_order() {
        let a = json!({"yield_lb_ac": 100.0, "unit": "lb/ac"}).as_object().unwrap().clone();
        let ops = vec![
            MigrationOp::Rename { from: "yield_lb_ac".into(), to: "value".into() },
            MigrationOp::Scale { field: "value".into(), factor: 1.120_851 },
            MigrationOp::SetUnit { unit: "kg/ha".into() },
            MigrationOp::SetDefault { field: "method".into(), value: json!("combine") },
        ];
        let out = apply_ops(&a, &ops).unwrap();
        assert!((out["value"].as_f64().unwrap() - 112.0851).abs() < 1e-9);
        assert_eq!(out["unit"], "kg/ha");
        assert_eq!(out["method"], "combine");
    }

    #[test]
    fn unsafe_ops_fail() {
        let a = json!({"a": "x", "b": 1}).as_object().unwrap().clone();
        assert!(apply_ops(&a, &[MigrationOp::Scale { field: "a".into(), factor: 2.0 }]).is_err());
        assert!(apply_ops(&a, &[MigrationOp::Rename { from: "a".into(), to: "b".into() }]).is_err());
        assert!(MigrationOp::Scale { field: "a".into(), factor: f64::NAN }.check().is_err());
    }
}
