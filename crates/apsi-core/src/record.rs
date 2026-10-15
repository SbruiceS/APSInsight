//! Records, lifecycle, provenance and version history
//! ("Registration and identity", "CRUD and lifecycle", "Versioning", "Provenance").

use crate::auth::ActorKind;
use crate::error::{ApsiError, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use time::OffsetDateTime;

pub type Attributes = Map<String, Value>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Lifecycle {
    Draft,
    Active,
    Archived,
}

impl Lifecycle {
    pub fn as_str(self) -> &'static str {
        match self {
            Lifecycle::Draft => "draft",
            Lifecycle::Active => "active",
            Lifecycle::Archived => "archived",
        }
    }

    pub fn parse(s: &str) -> Result<Lifecycle> {
        match s {
            "draft" => Ok(Lifecycle::Draft),
            "active" => Ok(Lifecycle::Active),
            "archived" => Ok(Lifecycle::Archived),
            other => Err(ApsiError::InvalidInput(format!("unknown lifecycle state {other}"))),
        }
    }

    /// Allowed transitions. Restoring from `archived` is an administrative action
    /// and is checked separately by the engine.
    pub fn can_transition(self, to: Lifecycle) -> bool {
        use Lifecycle::*;
        matches!((self, to), (Draft, Active) | (Draft, Archived) | (Active, Archived) | (Archived, Active))
    }
}

/// Identity of the record in the system it came from. Unique per entity kind and
/// tenant, which makes re-imports idempotent.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceRef {
    pub system: String,
    pub id: String,
}

impl SourceRef {
    pub fn check(&self) -> Result<()> {
        for (k, v) in [("source.system", &self.system), ("source.id", &self.id)] {
            if v.is_empty() || v.len() > 256 || v.chars().any(char::is_control) {
                return Err(ApsiError::InvalidInput(format!("{k} must be 1..256 printable characters")));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProvenanceEvent {
    #[serde(with = "time::serde::rfc3339")]
    pub at: OffsetDateTime,
    pub actor: String,
    pub actor_kind: ActorKind,
    pub action: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceRef>,
    /// Free-text description of the transformation applied (e.g. `unit conversion lb/ac -> kg/ha`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transformation: Option<String>,
    /// Records this change was derived from.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Record {
    pub id: String,
    pub tenant: String,
    pub domain: String,
    pub entity: String,
    pub schema_version: u32,
    /// Monotonic version, starting at 1; every accepted change increments it.
    pub version: u64,
    pub lifecycle: Lifecycle,
    pub name: String,
    pub attributes: Attributes,
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceRef>,
    pub owner: String,
    pub legal_hold: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    pub created_by: String,
    pub updated_by: String,
    #[serde(default, with = "time::serde::rfc3339::option", skip_serializing_if = "Option::is_none")]
    pub archived_at: Option<OffsetDateTime>,
    pub provenance: Vec<ProvenanceEvent>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NewRecord {
    pub name: String,
    #[serde(default)]
    pub attributes: Attributes,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub source: Option<SourceRef>,
    /// Initial lifecycle state; `active` when omitted. `archived` is not allowed.
    #[serde(default)]
    pub lifecycle: Option<Lifecycle>,
    #[serde(default)]
    pub transformation: Option<String>,
    #[serde(default)]
    pub inputs: Vec<String>,
}

/// Partial update. In `attributes`, a `null` value removes the attribute; other
/// keys are set. `replace_attributes` swaps the whole map instead of merging.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecordPatch {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub attributes: Option<Attributes>,
    #[serde(default)]
    pub replace_attributes: bool,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub transformation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordVersion {
    pub version: u64,
    #[serde(with = "time::serde::rfc3339")]
    pub at: OffsetDateTime,
    pub actor: String,
    pub change: String,
    pub schema_version: u32,
    pub lifecycle: Lifecycle,
    pub name: String,
    pub attributes: Attributes,
    pub tags: Vec<String>,
}

impl RecordVersion {
    pub fn of(record: &Record, change: &str) -> Self {
        Self {
            version: record.version,
            at: record.updated_at,
            actor: record.updated_by.clone(),
            change: change.into(),
            schema_version: record.schema_version,
            lifecycle: record.lifecycle,
            name: record.name.clone(),
            attributes: record.attributes.clone(),
            tags: record.tags.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldChange {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<Value>,
}

/// Field-level comparison between two versions.
pub fn diff(a: &RecordVersion, b: &RecordVersion) -> Vec<FieldChange> {
    let mut out = Vec::new();
    let mut push = |path: String, before: Option<Value>, after: Option<Value>| {
        if before != after {
            out.push(FieldChange { path, before, after });
        }
    };
    push("name".into(), Some(Value::from(a.name.clone())), Some(Value::from(b.name.clone())));
    push("lifecycle".into(), Some(Value::from(a.lifecycle.as_str())), Some(Value::from(b.lifecycle.as_str())));
    push("tags".into(), Some(Value::from(a.tags.clone())), Some(Value::from(b.tags.clone())));
    push("schema_version".into(), Some(Value::from(a.schema_version)), Some(Value::from(b.schema_version)));
    let mut keys: Vec<&String> = a.attributes.keys().chain(b.attributes.keys()).collect();
    keys.sort();
    keys.dedup();
    for k in keys {
        push(format!("attributes.{k}"), a.attributes.get(k).cloned(), b.attributes.get(k).cloned());
    }
    out
}

pub fn apply_patch(attributes: &mut Attributes, patch: &Attributes, replace: bool) {
    if replace {
        *attributes = patch.iter().filter(|(_, v)| !v.is_null()).map(|(k, v)| (k.clone(), v.clone())).collect();
        return;
    }
    for (k, v) in patch {
        if v.is_null() {
            attributes.remove(k);
        } else {
            attributes.insert(k.clone(), v.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn lifecycle_transitions() {
        use Lifecycle::*;
        assert!(Draft.can_transition(Active));
        assert!(Active.can_transition(Archived));
        assert!(!Active.can_transition(Draft));
        assert!(!Archived.can_transition(Draft));
        assert_eq!(Lifecycle::parse("archived").unwrap(), Archived);
        assert!(Lifecycle::parse("deleted").is_err());
    }

    #[test]
    fn patch_merges_and_removes() {
        let mut a = json!({"x": 1, "y": 2}).as_object().unwrap().clone();
        apply_patch(&mut a, json!({"y": null, "z": 3}).as_object().unwrap(), false);
        assert_eq!(Value::Object(a.clone()), json!({"x": 1, "z": 3}));
        apply_patch(&mut a, json!({"q": 1, "r": null}).as_object().unwrap(), true);
        assert_eq!(Value::Object(a), json!({"q": 1}));
    }

    #[test]
    fn source_ref_rejects_control_chars() {
        assert!(SourceRef { system: "lims".into(), id: "A-1".into() }.check().is_ok());
        assert!(SourceRef { system: "".into(), id: "A-1".into() }.check().is_err());
        assert!(SourceRef { system: "x".into(), id: "a\nb".into() }.check().is_err());
    }
}
