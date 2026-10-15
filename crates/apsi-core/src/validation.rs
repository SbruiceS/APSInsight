//! Configurable validation rules with severities and machine-readable reports
//! ("Validation rules" family). Schema checks always run; rules are layered on top.

use crate::error::{ApsiError, Result};
use crate::ids::{validate_field_name, validate_slug};
use crate::schema::EntitySchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RuleKind {
    Required {
        field: String,
    },
    Range {
        field: String,
        min: Option<f64>,
        max: Option<f64>,
    },
    MaxLength {
        field: String,
        max: usize,
    },
    OneOf {
        field: String,
        values: Vec<Value>,
    },
    /// When `field` is present the record must also carry a non-empty `unit` attribute.
    RequiresUnit {
        field: String,
    },
    MinTags {
        min: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationRule {
    pub id: String,
    #[serde(flatten)]
    pub kind: RuleKind,
    pub severity: Severity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(default = "enabled_default")]
    pub enabled: bool,
}

fn enabled_default() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub rule: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    pub severity: Severity,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationReport {
    /// False when at least one issue has severity `error`.
    pub valid: bool,
    pub schema_version: u32,
    pub errors: usize,
    pub warnings: usize,
    pub infos: usize,
    pub issues: Vec<ValidationIssue>,
}

impl ValidationReport {
    pub fn from_issues(schema_version: u32, mut issues: Vec<ValidationIssue>) -> Self {
        issues.sort_by(|a, b| b.severity.cmp(&a.severity).then_with(|| a.rule.cmp(&b.rule)));
        let count = |s| issues.iter().filter(|i| i.severity == s).count();
        let errors = count(Severity::Error);
        Self { valid: errors == 0, schema_version, errors, warnings: count(Severity::Warning), infos: count(Severity::Info), issues }
    }

    pub fn into_result(self) -> Result<ValidationReport> {
        if self.valid { Ok(self) } else { Err(ApsiError::Validation(Box::new(self))) }
    }
}

impl ValidationRule {
    pub fn check_well_formed(&self) -> Result<()> {
        validate_slug("rule id", &self.id)?;
        match &self.kind {
            RuleKind::Required { field } | RuleKind::RequiresUnit { field } | RuleKind::MaxLength { field, .. } => validate_field_name(field),
            RuleKind::Range { field, min, max } => {
                validate_field_name(field)?;
                if min.is_none() && max.is_none() {
                    return Err(ApsiError::InvalidInput("range rule needs min or max".into()));
                }
                if let (Some(a), Some(b)) = (min, max)
                    && a > b
                {
                    return Err(ApsiError::InvalidInput("range rule min > max".into()));
                }
                Ok(())
            }
            RuleKind::OneOf { field, values } => {
                validate_field_name(field)?;
                if values.is_empty() {
                    return Err(ApsiError::InvalidInput("one_of rule needs values".into()));
                }
                Ok(())
            }
            RuleKind::MinTags { .. } => Ok(()),
        }
    }

    fn evaluate(&self, attributes: &Map<String, Value>, tags: &[String]) -> Option<ValidationIssue> {
        let present = |f: &str| attributes.get(f).filter(|v| !v.is_null());
        let (field, failed, default_msg): (Option<&str>, bool, String) = match &self.kind {
            RuleKind::Required { field } => (Some(field), present(field).is_none(), format!("{field} is required")),
            RuleKind::Range { field, min, max } => {
                let n = present(field).and_then(Value::as_f64);
                let bad = n.is_some_and(|n| min.is_some_and(|m| n < m) || max.is_some_and(|m| n > m));
                (Some(field), bad, format!("{field} must be within [{min:?}, {max:?}]"))
            }
            RuleKind::MaxLength { field, max } => {
                let bad = present(field).and_then(Value::as_str).is_some_and(|s| s.chars().count() > *max);
                (Some(field), bad, format!("{field} must be at most {max} characters"))
            }
            RuleKind::OneOf { field, values } => {
                let bad = present(field).is_some_and(|v| !values.contains(v));
                (Some(field), bad, format!("{field} must be one of the allowed values"))
            }
            RuleKind::RequiresUnit { field } => {
                let has_unit = present("unit").and_then(Value::as_str).is_some_and(|u| !u.trim().is_empty());
                (Some(field), present(field).is_some() && !has_unit, format!("{field} requires a unit"))
            }
            RuleKind::MinTags { min } => (None, tags.len() < *min, format!("at least {min} tags are required")),
        };
        failed.then(|| ValidationIssue {
            rule: self.id.clone(),
            field: field.map(str::to_string),
            severity: self.severity,
            code: kind_code(&self.kind).into(),
            message: self.message.clone().unwrap_or(default_msg),
        })
    }
}

fn kind_code(k: &RuleKind) -> &'static str {
    match k {
        RuleKind::Required { .. } => "required",
        RuleKind::Range { .. } => "range",
        RuleKind::MaxLength { .. } => "max_length",
        RuleKind::OneOf { .. } => "one_of",
        RuleKind::RequiresUnit { .. } => "requires_unit",
        RuleKind::MinTags { .. } => "min_tags",
    }
}

/// Built-in record checks plus schema checks plus configured rules.
pub fn validate(name: &str, attributes: &Map<String, Value>, tags: &[String], schema: &EntitySchema, rules: &[ValidationRule]) -> ValidationReport {
    let mut issues = Vec::new();
    let builtin = |code: &str, field: &str, message: &str| ValidationIssue {
        rule: format!("builtin.{code}"),
        field: Some(field.into()),
        severity: Severity::Error,
        code: code.into(),
        message: message.into(),
    };
    if name.trim().is_empty() {
        issues.push(builtin("name_required", "name", "name must not be empty"));
    }
    if name.chars().count() > 256 {
        issues.push(builtin("name_length", "name", "name must be at most 256 characters"));
    }
    if name.chars().any(char::is_control) {
        issues.push(builtin("name_control_chars", "name", "name must not contain control characters"));
    }
    if tags.len() > 64 || tags.iter().any(|t| t.is_empty() || t.len() > 64 || t.chars().any(char::is_control)) {
        issues.push(builtin("tags", "tags", "at most 64 tags of 1..64 printable characters"));
    }
    issues.extend(schema.check_attributes(attributes));
    issues.extend(rules.iter().filter(|r| r.enabled).filter_map(|r| r.evaluate(attributes, tags)));
    ValidationReport::from_issues(schema.version, issues)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use serde_json::json;

    fn schema() -> EntitySchema {
        EntitySchema::default_for("d", "e")
    }

    #[test]
    fn severities_drive_validity() {
        let rules = vec![
            ValidationRule {
                id: "ph-range".into(),
                kind: RuleKind::Range { field: "value".into(), min: Some(0.0), max: Some(14.0) },
                severity: Severity::Error,
                message: None,
                enabled: true,
            },
            ValidationRule {
                id: "unit".into(),
                kind: RuleKind::RequiresUnit { field: "value".into() },
                severity: Severity::Warning,
                message: Some("add a unit".into()),
                enabled: true,
            },
            ValidationRule { id: "tags".into(), kind: RuleKind::MinTags { min: 1 }, severity: Severity::Info, message: None, enabled: true },
        ];
        let a = json!({"value": 7.0}).as_object().unwrap().clone();
        let r = validate("sample", &a, &[], &schema(), &rules);
        assert!(r.valid);
        assert_eq!((r.errors, r.warnings, r.infos), (0, 1, 1));
        assert_eq!(r.issues[0].message, "add a unit");
        let a = json!({"value": 15.0, "unit": "pH"}).as_object().unwrap().clone();
        let r = validate("sample", &a, &["x".into()], &schema(), &rules);
        assert!(!r.valid);
        assert_eq!(r.issues[0].rule, "ph-range");
        assert!(r.clone().into_result().is_err());
    }

    #[test]
    fn disabled_rules_are_skipped_and_malformed_rules_rejected() {
        let mut rule =
            ValidationRule { id: "req".into(), kind: RuleKind::Required { field: "value".into() }, severity: Severity::Error, message: None, enabled: false };
        assert!(validate("n", &Map::new(), &[], &schema(), std::slice::from_ref(&rule)).valid);
        rule.enabled = true;
        assert!(!validate("n", &Map::new(), &[], &schema(), &[rule]).valid);
        let bad = ValidationRule {
            id: "r".into(),
            kind: RuleKind::Range { field: "v".into(), min: None, max: None },
            severity: Severity::Error,
            message: None,
            enabled: true,
        };
        assert!(bad.check_well_formed().is_err());
    }

    #[test]
    fn builtin_name_checks() {
        assert!(!validate("", &Map::new(), &[], &schema(), &[]).valid);
        assert!(!validate("a\u{0007}b", &Map::new(), &[], &schema(), &[]).valid);
        assert!(!validate(&"n".repeat(257), &Map::new(), &[], &schema(), &[]).valid);
    }

    proptest! {
        #[test]
        fn validation_is_total_on_arbitrary_input(name in ".{0,300}", n in proptest::num::f64::ANY, s in ".{0,50}") {
            let a = json!({"value": n, "unit": s, "observed_at": s}).as_object().unwrap().clone();
            let r = validate(&name, &a, std::slice::from_ref(&s), &schema(), &[]);
            prop_assert_eq!(r.valid, r.errors == 0);
        }
    }
}
