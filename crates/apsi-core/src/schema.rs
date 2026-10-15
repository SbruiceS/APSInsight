//! Versioned entity schemas ("Schema contract" and "Metadata model" families).
//!
//! Every one of the 800 entity kinds starts with a default v1 schema. Tenants'
//! administrators may publish new versions; each publication is checked for
//! backward compatibility against the previous version.

use crate::error::{ApsiError, Result};
use crate::ids::validate_field_name;
use crate::validation::{Severity, ValidationIssue};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FieldType {
    String,
    Integer,
    Number,
    Boolean,
    /// RFC 3339 timestamp string.
    Timestamp,
    Enum {
        values: Vec<String>,
    },
    /// `{ "lat": f64, "lon": f64 }` in WGS 84 (EPSG:4326).
    GeoPoint,
    Array,
    Object,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldDef {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: FieldType,
    #[serde(default)]
    pub required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Unit of measure (UCUM-style code such as `kg/ha`, `mm`, `Cel`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_length: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntitySchema {
    pub domain: String,
    pub entity: String,
    pub version: u32,
    pub fields: Vec<FieldDef>,
    /// When true, attributes not declared in `fields` are accepted and preserved.
    #[serde(default = "default_true")]
    pub additional_attributes: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilityLevel {
    Identical,
    BackwardCompatible,
    Breaking,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Compatibility {
    pub level: CompatibilityLevel,
    pub changes: Vec<String>,
}

fn field(name: &str, ty: FieldType, description: &str) -> FieldDef {
    FieldDef { name: name.into(), ty, required: false, description: Some(description.into()), unit: None, min: None, max: None, max_length: None }
}

impl EntitySchema {
    /// Default v1 schema shared by all entity kinds. Domain-specific fields are
    /// published as later versions; unknown attributes are preserved meanwhile.
    pub fn default_for(domain: &str, entity: &str) -> Self {
        let mut description = field("description", FieldType::String, "Human-readable description");
        description.max_length = Some(4000);
        let mut unit = field("unit", FieldType::String, "Unit of measure for the primary value");
        unit.max_length = Some(32);
        Self {
            domain: domain.into(),
            entity: entity.into(),
            version: 1,
            fields: vec![
                description,
                unit,
                field("value", FieldType::Number, "Primary numeric value, expressed in `unit`"),
                field("observed_at", FieldType::Timestamp, "When the value was observed or became valid"),
                field("location", FieldType::GeoPoint, "WGS 84 location"),
            ],
            additional_attributes: true,
        }
    }

    pub fn field(&self, name: &str) -> Option<&FieldDef> {
        self.fields.iter().find(|f| f.name == name)
    }

    /// Structural self-check performed before a schema can be published.
    pub fn check_well_formed(&self) -> Result<()> {
        let mut seen = std::collections::HashSet::new();
        for f in &self.fields {
            validate_field_name(&f.name)?;
            if !seen.insert(f.name.as_str()) {
                return Err(ApsiError::InvalidInput(format!("duplicate field {}", f.name)));
            }
            if let (Some(min), Some(max)) = (f.min, f.max)
                && min > max
            {
                return Err(ApsiError::InvalidInput(format!("field {}: min > max", f.name)));
            }
            if let FieldType::Enum { values } = &f.ty
                && values.is_empty()
            {
                return Err(ApsiError::InvalidInput(format!("field {}: enum needs values", f.name)));
            }
        }
        if self.fields.len() > 256 {
            return Err(ApsiError::InvalidInput("schemas are limited to 256 fields".into()));
        }
        Ok(())
    }

    /// Compares `next` against `self`. Breaking changes are anything that could make
    /// a previously valid record invalid.
    pub fn compatibility(&self, next: &EntitySchema) -> Compatibility {
        let mut changes = Vec::new();
        let mut breaking = false;
        for old in &self.fields {
            match next.field(&old.name) {
                None => {
                    changes.push(format!("removed field {}", old.name));
                    breaking |= !next.additional_attributes;
                }
                Some(new) => {
                    if new.ty != old.ty {
                        let widened = matches!((&old.ty, &new.ty), (FieldType::Integer, FieldType::Number))
                            || matches!((&old.ty, &new.ty), (FieldType::Enum { values: a }, FieldType::Enum { values: b }) if a.iter().all(|v| b.contains(v)));
                        changes.push(format!("type of {} changed", old.name));
                        breaking |= !widened;
                    }
                    if new.required && !old.required {
                        changes.push(format!("{} became required", old.name));
                        breaking = true;
                    }
                    if tighter_lower(old.min, new.min) || tighter_upper(old.max, new.max) {
                        changes.push(format!("range of {} narrowed", old.name));
                        breaking = true;
                    }
                    if let Some(n) = new.max_length
                        && old.max_length.is_none_or(|o| n < o)
                    {
                        changes.push(format!("max_length of {} reduced", old.name));
                        breaking = true;
                    }
                    if new.unit != old.unit {
                        changes.push(format!("unit of {} changed", old.name));
                        breaking = true;
                    }
                    if new.description != old.description {
                        changes.push(format!("description of {} changed", old.name));
                    }
                }
            }
        }
        for new in &next.fields {
            if self.field(&new.name).is_none() {
                changes.push(format!("added {} field {}", if new.required { "required" } else { "optional" }, new.name));
                breaking |= new.required;
            }
        }
        if self.additional_attributes && !next.additional_attributes {
            changes.push("additional attributes no longer accepted".into());
            breaking = true;
        }
        let level = if changes.is_empty() {
            CompatibilityLevel::Identical
        } else if breaking {
            CompatibilityLevel::Breaking
        } else {
            CompatibilityLevel::BackwardCompatible
        };
        Compatibility { level, changes }
    }

    /// Type and constraint checks for attributes. Issues use rule ids prefixed
    /// with `schema.` so reports distinguish them from configurable rules.
    pub fn check_attributes(&self, attributes: &Map<String, Value>) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
        let issue = |field: &str, code: &str, message: String| ValidationIssue {
            rule: format!("schema.v{}.{}", self.version, code),
            field: Some(field.to_string()),
            severity: Severity::Error,
            code: code.to_string(),
            message,
        };
        for f in &self.fields {
            match attributes.get(&f.name) {
                None | Some(Value::Null) => {
                    if f.required {
                        issues.push(issue(&f.name, "required", format!("{} is required", f.name)));
                    }
                }
                Some(v) => {
                    if let Some(msg) = type_error(&f.ty, v) {
                        issues.push(issue(&f.name, "type", format!("{}: {msg}", f.name)));
                        continue;
                    }
                    if let Some(n) = v.as_f64()
                        && (f.min.is_some_and(|m| n < m) || f.max.is_some_and(|m| n > m))
                    {
                        issues.push(issue(&f.name, "range", format!("{} = {n} outside [{:?}, {:?}]", f.name, f.min, f.max)));
                    }
                    if let (Some(s), Some(max)) = (v.as_str(), f.max_length)
                        && s.chars().count() > max
                    {
                        issues.push(issue(&f.name, "max_length", format!("{} longer than {max} characters", f.name)));
                    }
                }
            }
        }
        if !self.additional_attributes {
            for key in attributes.keys() {
                if self.field(key).is_none() {
                    issues.push(issue(key, "undeclared", format!("{key} is not declared in schema v{}", self.version)));
                }
            }
        }
        issues
    }
}

fn tighter_lower(old: Option<f64>, new: Option<f64>) -> bool {
    match (old, new) {
        (_, None) => false,
        (None, Some(_)) => true,
        (Some(o), Some(n)) => n > o,
    }
}

fn tighter_upper(old: Option<f64>, new: Option<f64>) -> bool {
    match (old, new) {
        (_, None) => false,
        (None, Some(_)) => true,
        (Some(o), Some(n)) => n < o,
    }
}

fn type_error(ty: &FieldType, v: &Value) -> Option<String> {
    let ok = match ty {
        FieldType::String => v.is_string(),
        FieldType::Integer => v.is_i64() || v.is_u64(),
        FieldType::Number => v.is_number(),
        FieldType::Boolean => v.is_boolean(),
        FieldType::Timestamp => v.as_str().is_some_and(|s| time::OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339).is_ok()),
        FieldType::Enum { values } => v.as_str().is_some_and(|s| values.iter().any(|x| x == s)),
        FieldType::GeoPoint => {
            let lat = v.get("lat").and_then(Value::as_f64);
            let lon = v.get("lon").and_then(Value::as_f64);
            matches!((lat, lon), (Some(a), Some(o)) if (-90.0..=90.0).contains(&a) && (-180.0..=180.0).contains(&o))
        }
        FieldType::Array => v.is_array(),
        FieldType::Object => v.is_object(),
    };
    if ok { None } else { Some(format!("expected {}", type_name(ty))) }
}

fn type_name(ty: &FieldType) -> String {
    match ty {
        FieldType::Enum { values } => format!("one of {}", values.join("|")),
        FieldType::GeoPoint => "geo point {lat in [-90,90], lon in [-180,180]}".into(),
        FieldType::Timestamp => "RFC 3339 timestamp".into(),
        other => format!("{other:?}").to_lowercase(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn attrs(v: Value) -> Map<String, Value> {
        v.as_object().unwrap().clone()
    }

    #[test]
    fn default_schema_accepts_valid_and_rejects_malformed() {
        let s = EntitySchema::default_for("soil-profile-and-classification", "ph");
        s.check_well_formed().unwrap();
        let ok = attrs(json!({"value": 6.5, "unit": "pH", "observed_at": "2026-05-01T10:00:00Z", "location": {"lat": 28.6, "lon": 77.2}, "extra": [1]}));
        assert!(s.check_attributes(&ok).is_empty());
        let bad = attrs(json!({"value": "x", "observed_at": "yesterday", "location": {"lat": 120, "lon": 0}}));
        let issues = s.check_attributes(&bad);
        assert_eq!(issues.len(), 3, "{issues:?}");
        assert!(issues.iter().all(|i| i.code == "type"));
    }

    #[test]
    fn compatibility_rules() {
        let v1 = EntitySchema::default_for("d", "e");
        let mut v2 = v1.clone();
        v2.version = 2;
        assert_eq!(v1.compatibility(&v2).level, CompatibilityLevel::Identical);
        v2.fields.push(field("depth_cm", FieldType::Number, "Depth"));
        assert_eq!(v1.compatibility(&v2).level, CompatibilityLevel::BackwardCompatible);
        let mut v3 = v2.clone();
        v3.fields.last_mut().unwrap().required = true;
        assert_eq!(v2.compatibility(&v3).level, CompatibilityLevel::Breaking);
        let mut v4 = v2.clone();
        v4.fields.last_mut().unwrap().max = Some(10.0);
        assert_eq!(v2.compatibility(&v4).level, CompatibilityLevel::Breaking);
        let mut v5 = v1.clone();
        v5.fields[2].ty = FieldType::String;
        assert_eq!(v1.compatibility(&v5).level, CompatibilityLevel::Breaking);
    }

    #[test]
    fn malformed_schemas_are_rejected() {
        let mut s = EntitySchema::default_for("d", "e");
        s.fields.push(s.fields[0].clone());
        assert!(s.check_well_formed().is_err());
        let mut s = EntitySchema::default_for("d", "e");
        s.fields[2].min = Some(5.0);
        s.fields[2].max = Some(1.0);
        assert!(s.check_well_formed().is_err());
        let mut s = EntitySchema::default_for("d", "e");
        s.fields[0].name = "Bad Name".into();
        assert!(s.check_well_formed().is_err());
    }

    #[test]
    fn closed_schema_reports_undeclared_attributes() {
        let mut s = EntitySchema::default_for("d", "e");
        s.additional_attributes = false;
        let issues = s.check_attributes(&attrs(json!({"surprise": 1})));
        assert_eq!(issues[0].code, "undeclared");
    }
}
