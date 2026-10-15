//! Export and import contracts ("Export contract", "Import contract", "Interoperability").
//!
//! Export formats:
//! * `json`    — one document: `{ "header": {...}, "records": [...] }`.
//! * `ndjson`  — first line `{"header": {...}}`, then one record per line.
//! * `csv`     — RFC 4180, UTF-8, fixed columns; `attributes` and `provenance` are
//!   JSON-encoded cells so the export is lossless.
//! * `geojson` — RFC 7946 FeatureCollection; geometry from the `location` attribute
//!   (null geometry when absent); all other data in `properties`.

use crate::error::{ApsiError, Result};
use crate::record::{Attributes, Lifecycle, NewRecord, Record, SourceRef};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use time::OffsetDateTime;

pub const EXPORT_FORMAT: &str = "apsinsight.export";
pub const EXPORT_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    Json,
    Ndjson,
    Csv,
    Geojson,
}

impl Format {
    pub fn parse(s: &str) -> Result<Format> {
        match s {
            "json" => Ok(Format::Json),
            "ndjson" => Ok(Format::Ndjson),
            "csv" => Ok(Format::Csv),
            "geojson" => Ok(Format::Geojson),
            other => Err(ApsiError::InvalidInput(format!("unknown format {other}"))),
        }
    }

    pub fn content_type(self) -> &'static str {
        match self {
            Format::Json => "application/json",
            Format::Ndjson => "application/x-ndjson",
            Format::Csv => "text/csv; charset=utf-8",
            Format::Geojson => "application/geo+json",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportHeader {
    pub format: String,
    pub format_version: u32,
    pub encoding: String,
    #[serde(with = "time::serde::rfc3339")]
    pub exported_at: OffsetDateTime,
    pub tenant: String,
    pub domain: String,
    pub entity: String,
    pub schema_version: u32,
    pub count: usize,
    pub provenance_included: bool,
}

pub const CSV_COLUMNS: [&str; 14] = [
    "id",
    "domain",
    "entity",
    "schema_version",
    "version",
    "lifecycle",
    "name",
    "tags",
    "source_system",
    "source_id",
    "created_at",
    "updated_at",
    "attributes",
    "provenance",
];

fn rfc3339(t: OffsetDateTime) -> String {
    t.format(&time::format_description::well_known::Rfc3339).unwrap_or_default()
}

fn csv_cell(s: &str) -> String {
    // A leading BOM would be mistaken for a file BOM on re-import, so it is quoted too.
    if s.contains([',', '"', '\n', '\r']) || s.starts_with('\u{feff}') { format!("\"{}\"", s.replace('"', "\"\"")) } else { s.to_string() }
}

pub fn render(format: Format, header: &ExportHeader, records: &[Record]) -> Result<String> {
    Ok(match format {
        Format::Json => serde_json::to_string_pretty(&json!({ "header": header, "records": records })).map_err(|e| ApsiError::Internal(e.to_string()))?,
        Format::Ndjson => {
            let mut out = ser(&json!({ "header": header })) + "\n";
            for r in records {
                out.push_str(&ser(r));
                out.push('\n');
            }
            out
        }
        Format::Csv => {
            let mut out = CSV_COLUMNS.join(",") + "\r\n";
            for r in records {
                let cells = [
                    r.id.clone(),
                    r.domain.clone(),
                    r.entity.clone(),
                    r.schema_version.to_string(),
                    r.version.to_string(),
                    r.lifecycle.as_str().into(),
                    r.name.clone(),
                    r.tags.join("|"),
                    r.source.as_ref().map(|s| s.system.clone()).unwrap_or_default(),
                    r.source.as_ref().map(|s| s.id.clone()).unwrap_or_default(),
                    rfc3339(r.created_at),
                    rfc3339(r.updated_at),
                    ser(&r.attributes),
                    ser(&r.provenance),
                ];
                out.push_str(&cells.iter().map(|c| csv_cell(c)).collect::<Vec<_>>().join(","));
                out.push_str("\r\n");
            }
            out
        }
        Format::Geojson => {
            let features: Vec<Value> = records
                .iter()
                .map(|r| {
                    let geometry = r
                        .attributes
                        .get("location")
                        .and_then(|l| Some(json!({ "type": "Point", "coordinates": [l.get("lon")?.as_f64()?, l.get("lat")?.as_f64()?] })))
                        .unwrap_or(Value::Null);
                    let mut props: Map<String, Value> = r.attributes.iter().filter(|(k, _)| *k != "location").map(|(k, v)| (k.clone(), v.clone())).collect();
                    props.insert("name".into(), Value::from(r.name.clone()));
                    props.insert("lifecycle".into(), Value::from(r.lifecycle.as_str()));
                    props.insert("version".into(), Value::from(r.version));
                    props.insert("tags".into(), Value::from(r.tags.clone()));
                    json!({ "type": "Feature", "id": r.id, "geometry": geometry, "properties": props })
                })
                .collect();
            serde_json::to_string_pretty(&json!({ "type": "FeatureCollection", "apsinsight": header, "features": features }))
                .map_err(|e| ApsiError::Internal(e.to_string()))?
        }
    })
}

fn ser<T: Serialize + ?Sized>(v: &T) -> String {
    serde_json::to_string(v).unwrap_or_else(|_| "null".into())
}

/// RFC 4180 parser: quoted fields, doubled quotes, embedded newlines, CRLF or LF.
pub fn parse_csv(text: &str) -> Result<Vec<Vec<String>>> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut cell = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    let mut any = false;
    while let Some(c) = chars.next() {
        any = true;
        if quoted {
            match c {
                '"' if chars.peek() == Some(&'"') => {
                    chars.next();
                    cell.push('"');
                }
                '"' => quoted = false,
                _ => cell.push(c),
            }
            continue;
        }
        match c {
            '"' if cell.is_empty() => quoted = true,
            '"' => return Err(ApsiError::InvalidInput(format!("unexpected quote in row {}", rows.len() + 1))),
            ',' => row.push(std::mem::take(&mut cell)),
            '\r' if chars.peek() == Some(&'\n') => {}
            '\n' | '\r' => {
                row.push(std::mem::take(&mut cell));
                rows.push(std::mem::take(&mut row));
                any = false;
            }
            _ => cell.push(c),
        }
    }
    if quoted {
        return Err(ApsiError::InvalidInput("unterminated quoted field".into()));
    }
    if any {
        row.push(cell);
        rows.push(row);
    }
    rows.retain(|r| !(r.len() == 1 && r[0].is_empty()));
    Ok(rows)
}

/// Field mapping applied to imported attributes before validation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FieldMapping {
    /// source attribute name -> target attribute name
    #[serde(default)]
    pub rename: BTreeMap<String, String>,
    #[serde(default)]
    pub defaults: Attributes,
    #[serde(default)]
    pub drop: Vec<String>,
}

impl FieldMapping {
    pub fn check(&self) -> Result<()> {
        for (a, b) in &self.rename {
            crate::ids::validate_field_name(b).map_err(|_| ApsiError::InvalidInput(format!("mapping {a} -> {b}: invalid target")))?;
        }
        for k in self.defaults.keys() {
            crate::ids::validate_field_name(k)?;
        }
        Ok(())
    }

    pub fn apply(&self, attrs: &Attributes) -> Attributes {
        let mut out = Attributes::new();
        for (k, v) in attrs {
            if self.drop.contains(k) {
                continue;
            }
            out.insert(self.rename.get(k).cloned().unwrap_or_else(|| k.clone()), v.clone());
        }
        for (k, v) in &self.defaults {
            if out.get(k).is_none_or(Value::is_null) {
                out.insert(k.clone(), v.clone());
            }
        }
        out
    }
}

fn scalar(cell: &str) -> Value {
    if cell.is_empty() {
        return Value::Null;
    }
    if let Ok(i) = cell.parse::<i64>() {
        return Value::from(i);
    }
    if let Ok(f) = cell.parse::<f64>()
        && f.is_finite()
    {
        return Value::from(f);
    }
    match cell {
        "true" => Value::Bool(true),
        "false" => Value::Bool(false),
        _ => Value::from(cell),
    }
}

/// Converts CSV rows into new records. Recognised columns: `name` (required),
/// `tags` (`|`-separated), `source_system`, `source_id`, `lifecycle`, `attributes`
/// (JSON object, as produced by export). Every other column becomes an attribute;
/// numeric-looking cells become numbers, empty cells are omitted. Export-only
/// columns (`id`, `version`, timestamps, `provenance`...) are ignored.
pub fn csv_to_records(text: &str) -> Result<Vec<std::result::Result<NewRecord, String>>> {
    let rows = parse_csv(text)?;
    let Some((header, body)) = rows.split_first() else { return Ok(vec![]) };
    let col = |name: &str| header.iter().position(|h| h == name);
    let name_col = col("name").ok_or_else(|| ApsiError::InvalidInput("CSV needs a `name` column".into()))?;
    const IGNORED: [&str; 8] = ["id", "domain", "entity", "schema_version", "version", "created_at", "updated_at", "provenance"];
    Ok(body
        .iter()
        .enumerate()
        .map(|(i, row)| {
            if row.len() != header.len() {
                return Err(format!("row {}: expected {} cells, found {}", i + 2, header.len(), row.len()));
            }
            let mut attributes = match col("attributes").map(|c| row[c].as_str()) {
                Some(s) if !s.is_empty() => match serde_json::from_str::<Value>(s) {
                    Ok(Value::Object(m)) => m,
                    _ => return Err(format!("row {}: attributes cell is not a JSON object", i + 2)),
                },
                _ => Map::new(),
            };
            for (c, h) in header.iter().enumerate() {
                let reserved = ["name", "tags", "source_system", "source_id", "lifecycle", "attributes"].contains(&h.as_str()) || IGNORED.contains(&h.as_str());
                if !reserved && !row[c].is_empty() {
                    attributes.insert(h.clone(), scalar(&row[c]));
                }
            }
            let source = match (col("source_system").map(|c| &row[c]), col("source_id").map(|c| &row[c])) {
                (Some(s), Some(id)) if !s.is_empty() && !id.is_empty() => Some(SourceRef { system: s.clone(), id: id.clone() }),
                _ => None,
            };
            let lifecycle = match col("lifecycle").map(|c| row[c].as_str()) {
                Some("") | None => None,
                Some(l) => Some(Lifecycle::parse(l).map_err(|e| format!("row {}: {e}", i + 2))?),
            };
            let tags = col("tags").map(|c| row[c].split('|').filter(|t| !t.is_empty()).map(str::to_string).collect()).unwrap_or_default();
            Ok(NewRecord { name: row[name_col].clone(), attributes, tags, source, lifecycle, transformation: Some("csv import".into()), inputs: vec![] })
        })
        .collect())
}

/// Accepts either an array of new records or an APSInsight JSON export document.
pub fn json_to_records(value: &Value) -> Result<Vec<std::result::Result<NewRecord, String>>> {
    let items = match value {
        Value::Array(a) => a.clone(),
        Value::Object(o) if o.contains_key("records") => o["records"].as_array().cloned().unwrap_or_default(),
        _ => return Err(ApsiError::InvalidInput("expected an array of records or an export document".into())),
    };
    Ok(items
        .into_iter()
        .enumerate()
        .map(|(i, v)| {
            let mut r: NewRecord = serde_json::from_value(v.clone()).map_err(|e| format!("item {i}: {e}"))?;
            if r.lifecycle == Some(Lifecycle::Archived) {
                r.lifecycle = None;
            }
            Ok(r)
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn csv_roundtrip_with_quotes_and_newlines() {
        let rows = parse_csv("name,notes\r\n\"a, b\",\"line1\nline2 \"\"q\"\"\"\r\nplain,x\n").unwrap();
        assert_eq!(rows, vec![vec!["name", "notes"], vec!["a, b", "line1\nline2 \"q\""], vec!["plain", "x"]]);
        assert!(parse_csv("a,\"b").is_err());
        assert!(parse_csv("a,b\"c\"").is_err());
    }

    #[test]
    fn csv_to_records_maps_columns() {
        let out = csv_to_records("name,tags,ph,source_system,source_id\nS1,a|b,6.5,lims,42\nS2,,,,\nbad\n").unwrap();
        let r = out[0].as_ref().unwrap();
        assert_eq!(r.tags, vec!["a", "b"]);
        assert_eq!(r.attributes["ph"], 6.5);
        assert_eq!(r.source.as_ref().unwrap().id, "42");
        assert!(out[1].as_ref().unwrap().attributes.is_empty());
        assert!(out[2].is_err());
        assert!(csv_to_records("x\n1\n").is_err());
    }

    #[test]
    fn mapping_renames_drops_and_defaults() {
        let m = FieldMapping {
            rename: [("pH".to_string(), "ph".to_string())].into(),
            defaults: json!({"unit": "pH"}).as_object().unwrap().clone(),
            drop: vec!["junk".into()],
        };
        let out = m.apply(json!({"pH": 7, "junk": 1}).as_object().unwrap());
        assert_eq!(Value::Object(out), json!({"ph": 7, "unit": "pH"}));
    }

    proptest! {
        #[test]
        fn csv_cells_roundtrip(cells in proptest::collection::vec("[^\r]{0,20}", 1..6)) {
            let line = cells.iter().map(|c| csv_cell(c)).collect::<Vec<_>>().join(",");
            let parsed = parse_csv(&format!("{line}\r\n")).unwrap();
            if cells.len() == 1 && cells[0].is_empty() {
                prop_assert!(parsed.is_empty());
            } else {
                prop_assert_eq!(&parsed[0], &cells);
            }
        }
    }
}
