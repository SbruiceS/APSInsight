//! The APSI Compute Engine over tenant records (used by apsi-server and apsi-cli).
//!
//! [`RecordSource`] implements the engine's `DataSource` for one request. It
//! reads through `apsi-core` with the caller's principal, so tenant isolation,
//! permissions and access metrics are exactly those of the record API. Each
//! entity kind is snapshotted at most once per request, so a workflow or query
//! sees one consistent version of the data.
//!
//! Columns of an entity-kind table: `id`, `name`, `lifecycle`, `version`,
//! `owner`, `tags` (comma-separated), `created_at`, `updated_at`, then one
//! column per schema field (geo points become `<field>_lat` / `<field>_lon`),
//! then any additional attributes. A numeric field gets a unit from its schema
//! definition, or, for the default `value` field, from the per-record `unit`
//! attribute when every record uses the same catalogued unit.

use apsi_core::auth::Principal;
use apsi_core::record::Record;
use apsi_core::schema::{EntitySchema, FieldType};
use apsi_core::{ApsiError, Engine};
use crate::source::{DataSource, TableInfo, sql_name};
use crate::types::{Batch, Column, DataType, Field, Schema};
use crate::{EngineError, Result};
use parking_lot::Mutex;
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::Arc;

const META: [(&str, DataType); 8] = [
    ("id", DataType::Utf8),
    ("name", DataType::Utf8),
    ("lifecycle", DataType::Utf8),
    ("version", DataType::Int64),
    ("owner", DataType::Utf8),
    ("tags", DataType::Utf8),
    ("created_at", DataType::Utf8),
    ("updated_at", DataType::Utf8),
];

pub struct RecordSource {
    engine: Arc<Engine>,
    principal: Principal,
    snapshots: Mutex<BTreeMap<(String, String), Arc<Snapshot>>>,
}

struct Snapshot {
    batch: Batch,
    schema_version: u32,
}

/// Maps core errors onto engine errors; `not found` is kept recognisable so
/// the API can answer 404.
pub fn from_core(e: ApsiError) -> EngineError {
    match e {
        ApsiError::Forbidden(m) => EngineError::Denied(m),
        ApsiError::NotFound(m) => EngineError::Source(format!("not found: {m}")),
        other => EngineError::Source(other.to_string()),
    }
}

/// Maps engine errors onto the API's error vocabulary.
pub fn to_core(e: EngineError) -> ApsiError {
    let message = e.to_string();
    match e.root() {
        EngineError::Denied(m) => ApsiError::Forbidden(m.clone()),
        EngineError::Source(m) if m.starts_with("not found") => ApsiError::NotFound(message),
        EngineError::Internal(_) => ApsiError::Internal(message),
        EngineError::MemoryExceeded { .. } => ApsiError::PayloadTooLarge(message),
        _ => ApsiError::InvalidInput(message),
    }
}

impl RecordSource {
    pub fn new(engine: Arc<Engine>, principal: Principal) -> Self {
        Self { engine, principal, snapshots: Mutex::new(BTreeMap::new()) }
    }

    fn snapshot(&self, domain: &str, entity: &str) -> Result<Arc<Snapshot>> {
        let key = (domain.to_string(), entity.to_string());
        if let Some(s) = self.snapshots.lock().get(&key) {
            return Ok(Arc::clone(s));
        }
        let (schema, records) = self.engine.analytics_snapshot(&self.principal, &self.principal.tenant, domain, entity).map_err(from_core)?;
        let snap = Arc::new(Snapshot { schema_version: schema.version, batch: to_batch(&schema, &records)? });
        self.snapshots.lock().insert(key, Arc::clone(&snap));
        Ok(snap)
    }
}

impl DataSource for RecordSource {
    fn schema(&self, domain: &str, entity: &str) -> Result<Schema> {
        Ok(self.snapshot(domain, entity)?.batch.schema.clone())
    }

    fn estimate_rows(&self, domain: &str, entity: &str) -> Result<u64> {
        Ok(self.snapshot(domain, entity)?.batch.rows() as u64)
    }

    fn fingerprint(&self, domain: &str, entity: &str) -> Result<String> {
        // The tenant is part of the key: checkpoints are shared by the server
        // and must never match across tenants.
        let tenant = &self.principal.tenant;
        let schema = self.snapshot(domain, entity)?.schema_version;
        let tables = self.engine.analytics_tables(&self.principal, tenant).map_err(from_core)?;
        Ok(match tables.iter().find(|t| t.domain == domain && t.entity == entity) {
            Some(t) => format!("{tenant}/{domain}/{entity}@s{schema}:{}:{}:{}", t.rows, t.version_sum, t.last_updated.unix_timestamp_nanos()),
            None => format!("{tenant}/{domain}/{entity}@s{schema}:empty"),
        })
    }

    fn scan(&self, domain: &str, entity: &str, columns: Option<&[String]>) -> Result<Batch> {
        let snap = self.snapshot(domain, entity)?;
        match columns {
            Some(c) => snap.batch.select(c),
            None => Ok(snap.batch.clone()),
        }
    }

    fn tables(&self) -> Result<Vec<TableInfo>> {
        Ok(self
            .engine
            .analytics_tables(&self.principal, &self.principal.tenant)
            .map_err(from_core)?
            .into_iter()
            .map(|t| TableInfo { table: sql_name(&t.domain, &t.entity), domain: t.domain, entity: t.entity, rows: t.rows as u64 })
            .collect())
    }
}

/// Builds the columnar table for one entity kind.
pub fn to_batch(schema: &EntitySchema, records: &[Record]) -> Result<Batch> {
    let mut fields: Vec<Field> = META.iter().map(|(n, t)| Field::new(*n, *t, None)).collect();
    let mut cols: Vec<Column> = vec![
        Column::Utf8(records.iter().map(|r| Some(r.id.clone())).collect()),
        Column::Utf8(records.iter().map(|r| Some(r.name.clone())).collect()),
        Column::Utf8(records.iter().map(|r| Some(r.lifecycle.as_str().to_string())).collect()),
        Column::Int64(records.iter().map(|r| Some(r.version as i64)).collect()),
        Column::Utf8(records.iter().map(|r| Some(r.owner.clone())).collect()),
        Column::Utf8(records.iter().map(|r| Some(r.tags.join(","))).collect()),
        Column::Utf8(records.iter().map(|r| r.created_at.format(&time::format_description::well_known::Rfc3339).ok()).collect()),
        Column::Utf8(records.iter().map(|r| r.updated_at.format(&time::format_description::well_known::Rfc3339).ok()).collect()),
    ];
    let taken = |fields: &[Field], name: &str| fields.iter().any(|f| f.name == name);
    let column_name = |fields: &[Field], name: &str| if taken(fields, name) { format!("attr_{name}") } else { name.to_string() };
    let attr = |r: &Record, name: &str| r.attributes.get(name).cloned().unwrap_or(Value::Null);

    for f in &schema.fields {
        let name = column_name(&fields, &f.name);
        match &f.ty {
            FieldType::Number | FieldType::Integer => {
                let unit = f.unit.clone().or_else(|| if f.name == "value" { uniform_unit(records) } else { None });
                if matches!(f.ty, FieldType::Integer) {
                    cols.push(Column::Int64(records.iter().map(|r| attr(r, &f.name).as_i64()).collect()));
                    fields.push(Field { name, dtype: DataType::Int64, unit });
                } else {
                    cols.push(Column::Float64(records.iter().map(|r| attr(r, &f.name).as_f64().filter(|v| v.is_finite())).collect()));
                    fields.push(Field { name, dtype: DataType::Float64, unit });
                }
            }
            FieldType::Boolean => {
                cols.push(Column::Boolean(records.iter().map(|r| attr(r, &f.name).as_bool()).collect()));
                fields.push(Field::new(name, DataType::Boolean, None));
            }
            FieldType::GeoPoint => {
                for (axis, key) in [("lat", "lat"), ("lon", "lon")] {
                    let n = column_name(&fields, &format!("{}_{axis}", f.name));
                    cols.push(Column::Float64(records.iter().map(|r| attr(r, &f.name).get(key).and_then(Value::as_f64)).collect()));
                    // Degrees are not in the unit catalogue, so no unit is claimed.
                    fields.push(Field::new(n, DataType::Float64, None));
                }
            }
            FieldType::String | FieldType::Timestamp | FieldType::Enum { .. } | FieldType::Array | FieldType::Object => {
                cols.push(Column::Utf8(records.iter().map(|r| text(&attr(r, &f.name))).collect()));
                fields.push(Field::new(name, DataType::Utf8, None));
            }
        }
    }

    // Attributes outside the schema (allowed when `additional_attributes`).
    let mut extra: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for r in records {
        for k in r.attributes.keys() {
            if schema.field(k).is_none() {
                extra.entry(k.clone()).or_default();
            }
        }
    }
    for name in extra.keys() {
        let values: Vec<Value> = records.iter().map(|r| attr(r, name)).collect();
        let present = values.iter().filter(|v| !v.is_null());
        let col_name = column_name(&fields, name);
        if present.clone().all(Value::is_number) {
            cols.push(Column::Float64(values.iter().map(|v| v.as_f64().filter(|x| x.is_finite())).collect()));
            fields.push(Field::new(col_name, DataType::Float64, None));
        } else if present.clone().all(Value::is_boolean) {
            cols.push(Column::Boolean(values.iter().map(Value::as_bool).collect()));
            fields.push(Field::new(col_name, DataType::Boolean, None));
        } else {
            cols.push(Column::Utf8(values.iter().map(text).collect()));
            fields.push(Field::new(col_name, DataType::Utf8, None));
        }
    }
    Batch::new(Schema::new(fields)?, cols)
}

fn text(v: &Value) -> Option<String> {
    match v {
        Value::Null => None,
        Value::String(s) => Some(s.clone()),
        other => Some(other.to_string()),
    }
}

/// The unit shared by every record's `unit` attribute, if it is one and catalogued.
fn uniform_unit(records: &[Record]) -> Option<String> {
    let mut found: Option<&str> = None;
    for r in records.iter().filter(|r| r.attributes.get("value").is_some_and(|v| !v.is_null())) {
        let u = r.attributes.get("unit").and_then(Value::as_str)?;
        match found {
            None => found = Some(u),
            Some(prev) if prev != u => return None,
            _ => {}
        }
    }
    let u = found?;
    apsi_science::units::unit(u).ok().map(|_| u.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use apsi_core::auth::{ActorKind, Role};
    use apsi_core::record::NewRecord;
    use serde_json::json;

    fn rec(name: &str, attrs: Value) -> NewRecord {
        NewRecord { name: name.into(), attributes: attrs.as_object().unwrap().clone(), ..Default::default() }
    }

    #[test]
    fn record_tables_have_typed_unit_aware_columns() {
        let e = Arc::new(Engine::in_memory());
        let p = Principal::new("eve", "farm-co", &[Role::Editor], ActorKind::User);
        let (d, k) = ("crop-yield-and-production", "yield-observations");
        e.create(&p, "farm-co", d, k, rec("A", json!({"value": 4.2, "unit": "t/ha", "location": {"lat": 28.6, "lon": 77.2}, "plot": "p1"}))).unwrap();
        e.create(&p, "farm-co", d, k, rec("B", json!({"value": 5.0, "unit": "t/ha", "plot": "p2", "irrigated": true}))).unwrap();
        let src = RecordSource::new(Arc::clone(&e), p.clone());
        let s = src.schema(d, k).unwrap();
        assert_eq!(s.field("value").unwrap().unit.as_deref(), Some("t/ha"), "uniform catalogued unit is adopted");
        assert_eq!(s.field("version").unwrap().dtype, DataType::Int64);
        assert_eq!(s.field("location_lat").unwrap().dtype, DataType::Float64);
        assert_eq!(s.field("plot").unwrap().dtype, DataType::Utf8);
        assert_eq!(s.field("irrigated").unwrap().dtype, DataType::Boolean);
        assert_eq!(src.scan(d, k, Some(&["value".to_string()])).unwrap().rows(), 2);
        let fp = src.fingerprint(d, k).unwrap();
        assert!(fp.starts_with("farm-co/"), "fingerprints are tenant-scoped: {fp}");
        assert_eq!(src.tables().unwrap()[0].table, "crop_yield_and_production__yield_observations");

        // Mixed units: no unit is claimed.
        e.create(&p, "farm-co", d, k, rec("C", json!({"value": 4100, "unit": "kg/ha"}))).unwrap();
        let fresh = RecordSource::new(Arc::clone(&e), p);
        assert_eq!(fresh.schema(d, k).unwrap().field("value").unwrap().unit, None);
        assert_ne!(fresh.fingerprint(d, k).unwrap(), fp);
    }

    #[test]
    fn other_tenants_and_missing_permissions_are_refused() {
        let e = Arc::new(Engine::in_memory());
        let p = Principal::new("eve", "farm-co", &[Role::Editor], ActorKind::User);
        e.create(&p, "farm-co", "soil-profile-and-classification", "ph", rec("A", json!({"value": 6.1}))).unwrap();
        let other = RecordSource::new(Arc::clone(&e), Principal::new("mallory", "other-co", &[Role::Admin], ActorKind::User));
        assert!(other.tables().unwrap().is_empty(), "another tenant sees nothing");
        let none = RecordSource::new(e, Principal::new("x", "farm-co", &[], ActorKind::User));
        assert!(matches!(none.scan("soil-profile-and-classification", "ph", None), Err(EngineError::Denied(_))));
        assert!(matches!(to_core(EngineError::Source("not found: entity kind a/b".into()).at("n")), ApsiError::NotFound(_)));
    }
}
