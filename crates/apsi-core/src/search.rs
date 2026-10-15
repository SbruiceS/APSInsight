//! Search, filtering, facets and pagination ("Search and filtering" family).
//!
//! The engine keeps a partition index per (tenant, domain, entity) and an exact-match
//! index on scalar attribute values; `eq` filters use the index, other operators
//! scan only the candidate set.

use crate::error::{ApsiError, Result};
use crate::ids::validate_field_name;
use crate::record::{Lifecycle, Record};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const MAX_LIMIT: usize = 500;
pub const DEFAULT_LIMIT: usize = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Op {
    Eq,
    Ne,
    Gt,
    Gte,
    Lt,
    Lte,
    Contains,
    In,
    Exists,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Filter {
    /// Attribute name, or one of the record fields `name`, `version`, `schema_version`.
    pub field: String,
    pub op: Op,
    #[serde(default)]
    pub value: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortKey {
    #[default]
    CreatedAt,
    UpdatedAt,
    Name,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Query {
    /// Taken from the URL path by the API; required by the engine.
    #[serde(default)]
    pub domain: String,
    #[serde(default)]
    pub entity: Option<String>,
    /// Case-insensitive substring match on name and description.
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub lifecycle: Option<Lifecycle>,
    #[serde(default)]
    pub include_archived: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub filters: Vec<Filter>,
    #[serde(default)]
    pub facets: Vec<String>,
    #[serde(default)]
    pub sort: SortKey,
    #[serde(default)]
    pub descending: bool,
    #[serde(default)]
    pub offset: usize,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchPage {
    pub items: Vec<Record>,
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<usize>,
    pub facets: BTreeMap<String, BTreeMap<String, usize>>,
}

impl Query {
    pub fn check(&self) -> Result<()> {
        if self.limit.is_some_and(|l| l == 0 || l > MAX_LIMIT) {
            return Err(ApsiError::InvalidInput(format!("limit must be within 1..={MAX_LIMIT}")));
        }
        if self.filters.len() > 32 || self.facets.len() > 16 || self.tags.len() > 32 {
            return Err(ApsiError::InvalidInput("too many filters, facets or tags".into()));
        }
        if self.text.as_ref().is_some_and(|t| t.len() > 256) {
            return Err(ApsiError::InvalidInput("text query too long".into()));
        }
        for f in &self.filters {
            validate_field_name(&f.field)?;
            if matches!(f.op, Op::In) && !f.value.is_array() {
                return Err(ApsiError::InvalidInput("`in` filter needs an array value".into()));
            }
        }
        for f in &self.facets {
            if f != "lifecycle" && f != "tags" && f != "entity" {
                validate_field_name(f)?;
            }
        }
        Ok(())
    }

    pub fn effective_limit(&self) -> usize {
        self.limit.unwrap_or(DEFAULT_LIMIT)
    }
}

/// Canonical string key for the exact-match index; only scalars are indexed.
pub fn index_key(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(format!("s:{s}")),
        Value::Bool(b) => Some(format!("b:{b}")),
        Value::Number(n) => n.as_f64().map(|f| format!("n:{f}")),
        _ => None,
    }
}

pub fn field_value<'a>(r: &'a Record, field: &str) -> Option<std::borrow::Cow<'a, Value>> {
    use std::borrow::Cow;
    match field {
        "name" => Some(Cow::Owned(Value::from(r.name.clone()))),
        "version" => Some(Cow::Owned(Value::from(r.version))),
        "schema_version" => Some(Cow::Owned(Value::from(r.schema_version))),
        _ => r.attributes.get(field).map(Cow::Borrowed),
    }
}

fn compare(a: &Value, b: &Value) -> Option<std::cmp::Ordering> {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.as_f64()?.partial_cmp(&y.as_f64()?),
        (Value::String(x), Value::String(y)) => Some(x.cmp(y)),
        _ => None,
    }
}

pub fn matches_filter(r: &Record, f: &Filter) -> bool {
    use std::cmp::Ordering::*;
    let v = field_value(r, &f.field);
    match f.op {
        Op::Exists => v.is_some() == f.value.as_bool().unwrap_or(true),
        Op::Eq => v.is_some_and(|v| *v == f.value || compare(&v, &f.value) == Some(Equal)),
        Op::Ne => v.is_none_or(|v| *v != f.value && compare(&v, &f.value) != Some(Equal)),
        Op::Gt => v.is_some_and(|v| compare(&v, &f.value) == Some(Greater)),
        Op::Gte => v.is_some_and(|v| matches!(compare(&v, &f.value), Some(Greater | Equal))),
        Op::Lt => v.is_some_and(|v| compare(&v, &f.value) == Some(Less)),
        Op::Lte => v.is_some_and(|v| matches!(compare(&v, &f.value), Some(Less | Equal))),
        Op::Contains => v.is_some_and(|v| match (&*v, &f.value) {
            (Value::String(s), Value::String(n)) => s.to_lowercase().contains(&n.to_lowercase()),
            (Value::Array(items), needle) => items.contains(needle),
            _ => false,
        }),
        Op::In => v.is_some_and(|v| f.value.as_array().is_some_and(|a| a.contains(&v))),
    }
}

pub fn matches(r: &Record, q: &Query) -> bool {
    if q.entity.as_ref().is_some_and(|e| *e != r.entity) {
        return false;
    }
    match q.lifecycle {
        Some(l) if r.lifecycle != l => return false,
        None if !q.include_archived && r.lifecycle == Lifecycle::Archived => return false,
        _ => {}
    }
    if !q.tags.iter().all(|t| r.tags.contains(t)) {
        return false;
    }
    if let Some(text) = &q.text {
        let t = text.to_lowercase();
        let desc = r.attributes.get("description").and_then(Value::as_str).unwrap_or("");
        if !r.name.to_lowercase().contains(&t) && !desc.to_lowercase().contains(&t) {
            return false;
        }
    }
    q.filters.iter().all(|f| matches_filter(r, f))
}

pub fn facet_values(r: &Record, facet: &str) -> Vec<String> {
    match facet {
        "lifecycle" => vec![r.lifecycle.as_str().into()],
        "entity" => vec![r.entity.clone()],
        "tags" => r.tags.clone(),
        f => match r.attributes.get(f) {
            Some(Value::String(s)) => vec![s.clone()],
            Some(Value::Bool(b)) => vec![b.to_string()],
            Some(Value::Number(n)) => vec![n.to_string()],
            Some(Value::Array(a)) => a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect(),
            _ => vec!["(missing)".into()],
        },
    }
}

/// Sorts owned records or references; ties break on id for stable pagination.
pub fn sort<R: std::borrow::Borrow<Record>>(items: &mut [R], key: SortKey, descending: bool) {
    items.sort_by(|a, b| {
        let (a, b) = (a.borrow(), b.borrow());
        let o = match key {
            SortKey::CreatedAt => a.created_at.cmp(&b.created_at),
            SortKey::UpdatedAt => a.updated_at.cmp(&b.updated_at),
            SortKey::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
        .then_with(|| a.id.cmp(&b.id));
        if descending { o.reverse() } else { o }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn query_bounds() {
        let mut q = Query { domain: "d".into(), ..Default::default() };
        assert!(q.check().is_ok());
        q.limit = Some(0);
        assert!(q.check().is_err());
        q.limit = Some(MAX_LIMIT + 1);
        assert!(q.check().is_err());
        q.limit = None;
        q.filters.push(Filter { field: "x".into(), op: Op::In, value: json!(1) });
        assert!(q.check().is_err());
        q.filters[0] = Filter { field: "x;drop table".into(), op: Op::Eq, value: json!(1) };
        assert!(q.check().is_err());
    }

    #[test]
    fn index_keys_are_type_tagged() {
        assert_ne!(index_key(&json!("1")), index_key(&json!(1)));
        assert_eq!(index_key(&json!(1)), index_key(&json!(1.0)));
        assert_eq!(index_key(&json!([1])), None);
    }
}
