//! Columnar data model: typed, unit-annotated schemas and column batches.
//!
//! `None` in a column is a *missing* value. Computations that are undefined
//! (e.g. NDVI with a zero denominator) also produce `None`, never a fake 0 or a
//! silent NaN, so downstream aggregates can count what was skipped.

use crate::{EngineError, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataType {
    Float64,
    Int64,
    Utf8,
    Boolean,
}

impl DataType {
    pub fn is_numeric(self) -> bool {
        matches!(self, Self::Float64 | Self::Int64)
    }

    /// Approximate in-memory bytes per value, used by the cost model and the
    /// memory budget (the `Option` tag plus payload; strings estimated).
    pub fn width(self) -> u64 {
        match self {
            Self::Float64 | Self::Int64 => 16,
            Self::Boolean => 2,
            Self::Utf8 => 40,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    #[serde(rename = "type")]
    pub dtype: DataType,
    /// Unit of measure (catalogued code such as `kg/ha`); `None` when unknown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}

impl Field {
    pub fn new(name: impl Into<String>, dtype: DataType, unit: Option<&str>) -> Self {
        Self { name: name.into(), dtype, unit: unit.map(str::to_owned) }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Schema {
    pub fields: Vec<Field>,
}

impl Schema {
    pub fn new(fields: Vec<Field>) -> Result<Self> {
        let mut seen = std::collections::BTreeSet::new();
        for f in &fields {
            if !seen.insert(f.name.as_str()) {
                return Err(EngineError::Type(format!("duplicate column `{}`", f.name)));
            }
        }
        Ok(Self { fields })
    }

    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.fields.iter().position(|f| f.name == name)
    }

    pub fn field(&self, name: &str) -> Result<&Field> {
        self.fields.iter().find(|f| f.name == name).ok_or_else(|| {
            let known: Vec<&str> = self.fields.iter().map(|f| f.name.as_str()).collect();
            EngineError::Type(format!("unknown column `{name}` (available: {})", known.join(", ")))
        })
    }

    pub fn names(&self) -> Vec<String> {
        self.fields.iter().map(|f| f.name.clone()).collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Column {
    Float64(Vec<Option<f64>>),
    Int64(Vec<Option<i64>>),
    Utf8(Vec<Option<String>>),
    Boolean(Vec<Option<bool>>),
}

impl Column {
    pub fn len(&self) -> usize {
        match self {
            Self::Float64(v) => v.len(),
            Self::Int64(v) => v.len(),
            Self::Utf8(v) => v.len(),
            Self::Boolean(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn dtype(&self) -> DataType {
        match self {
            Self::Float64(_) => DataType::Float64,
            Self::Int64(_) => DataType::Int64,
            Self::Utf8(_) => DataType::Utf8,
            Self::Boolean(_) => DataType::Boolean,
        }
    }

    pub fn empty(dtype: DataType) -> Self {
        match dtype {
            DataType::Float64 => Self::Float64(Vec::new()),
            DataType::Int64 => Self::Int64(Vec::new()),
            DataType::Utf8 => Self::Utf8(Vec::new()),
            DataType::Boolean => Self::Boolean(Vec::new()),
        }
    }

    pub fn nulls(dtype: DataType, n: usize) -> Self {
        match dtype {
            DataType::Float64 => Self::Float64(vec![None; n]),
            DataType::Int64 => Self::Int64(vec![None; n]),
            DataType::Utf8 => Self::Utf8(vec![None; n]),
            DataType::Boolean => Self::Boolean(vec![None; n]),
        }
    }

    /// Numeric view (integers widened to f64).
    pub fn as_f64(&self) -> Option<Vec<Option<f64>>> {
        match self {
            Self::Float64(v) => Some(v.clone()),
            Self::Int64(v) => Some(v.iter().map(|x| x.map(|i| i as f64)).collect()),
            _ => None,
        }
    }

    /// Rows selected by `idx`, in that order.
    pub fn take(&self, idx: &[usize]) -> Self {
        match self {
            Self::Float64(v) => Self::Float64(idx.iter().map(|&i| v[i]).collect()),
            Self::Int64(v) => Self::Int64(idx.iter().map(|&i| v[i]).collect()),
            Self::Utf8(v) => Self::Utf8(idx.iter().map(|&i| v[i].clone()).collect()),
            Self::Boolean(v) => Self::Boolean(idx.iter().map(|&i| v[i]).collect()),
        }
    }

    /// Rows `start..start + len`.
    pub fn slice(&self, start: usize, len: usize) -> Self {
        let r = start..start + len;
        match self {
            Self::Float64(v) => Self::Float64(v[r].to_vec()),
            Self::Int64(v) => Self::Int64(v[r].to_vec()),
            Self::Utf8(v) => Self::Utf8(v[r].to_vec()),
            Self::Boolean(v) => Self::Boolean(v[r].to_vec()),
        }
    }

    /// Like [`take`](Self::take) but `None` indices produce nulls (outer joins).
    pub fn take_opt(&self, idx: &[Option<usize>]) -> Self {
        match self {
            Self::Float64(v) => Self::Float64(idx.iter().map(|i| i.and_then(|i| v[i])).collect()),
            Self::Int64(v) => Self::Int64(idx.iter().map(|i| i.and_then(|i| v[i])).collect()),
            Self::Utf8(v) => Self::Utf8(idx.iter().map(|i| i.and_then(|i| v[i].clone())).collect()),
            Self::Boolean(v) => Self::Boolean(idx.iter().map(|i| i.and_then(|i| v[i])).collect()),
        }
    }

    pub fn value(&self, i: usize) -> Value {
        match self {
            Self::Float64(v) => v[i].and_then(serde_json::Number::from_f64).map_or(Value::Null, Value::Number),
            Self::Int64(v) => v[i].map_or(Value::Null, Value::from),
            Self::Utf8(v) => v[i].clone().map_or(Value::Null, Value::String),
            Self::Boolean(v) => v[i].map_or(Value::Null, Value::Bool),
        }
    }

    pub fn is_null(&self, i: usize) -> bool {
        match self {
            Self::Float64(v) => v[i].is_none(),
            Self::Int64(v) => v[i].is_none(),
            Self::Utf8(v) => v[i].is_none(),
            Self::Boolean(v) => v[i].is_none(),
        }
    }

    pub fn bytes(&self) -> u64 {
        match self {
            Self::Utf8(v) => v.iter().map(|s| 24 + s.as_ref().map_or(0, |s| s.len() as u64)).sum(),
            other => other.len() as u64 * other.dtype().width(),
        }
    }

    /// Appends `other` (same type) to `self`.
    pub fn extend(&mut self, other: Column) -> Result<()> {
        match (self, other) {
            (Self::Float64(a), Self::Float64(b)) => a.extend(b),
            (Self::Int64(a), Self::Int64(b)) => a.extend(b),
            (Self::Utf8(a), Self::Utf8(b)) => a.extend(b),
            (Self::Boolean(a), Self::Boolean(b)) => a.extend(b),
            _ => return Err(EngineError::Internal("column type mismatch while concatenating".into())),
        }
        Ok(())
    }
}

/// A set of equally long columns with their schema.
#[derive(Debug, Clone, PartialEq)]
pub struct Batch {
    pub schema: Schema,
    pub columns: Vec<Column>,
}

impl Batch {
    pub fn new(schema: Schema, columns: Vec<Column>) -> Result<Self> {
        if schema.fields.len() != columns.len() {
            return Err(EngineError::Internal(format!("{} fields but {} columns", schema.fields.len(), columns.len())));
        }
        let rows = columns.first().map_or(0, Column::len);
        for (f, c) in schema.fields.iter().zip(&columns) {
            if c.len() != rows {
                return Err(EngineError::Internal(format!("column `{}` has {} rows, expected {rows}", f.name, c.len())));
            }
            if c.dtype() != f.dtype {
                return Err(EngineError::Internal(format!("column `{}` is {:?}, schema says {:?}", f.name, c.dtype(), f.dtype)));
            }
        }
        Ok(Self { schema, columns })
    }

    pub fn empty(schema: Schema) -> Self {
        let columns = schema.fields.iter().map(|f| Column::empty(f.dtype)).collect();
        Self { schema, columns }
    }

    pub fn rows(&self) -> usize {
        self.columns.first().map_or(0, Column::len)
    }

    pub fn column(&self, name: &str) -> Result<&Column> {
        let i = self.schema.index_of(name).ok_or_else(|| EngineError::Type(format!("unknown column `{name}`")))?;
        Ok(&self.columns[i])
    }

    pub fn bytes(&self) -> u64 {
        self.columns.iter().map(Column::bytes).sum()
    }

    pub fn take(&self, idx: &[usize]) -> Self {
        Self { schema: self.schema.clone(), columns: self.columns.iter().map(|c| c.take(idx)).collect() }
    }

    pub fn slice(&self, start: usize, len: usize) -> Self {
        Self { schema: self.schema.clone(), columns: self.columns.iter().map(|c| c.slice(start, len)).collect() }
    }

    /// Only the named columns, in the given order.
    pub fn select(&self, names: &[String]) -> Result<Self> {
        let mut fields = Vec::with_capacity(names.len());
        let mut columns = Vec::with_capacity(names.len());
        for n in names {
            let i = self.schema.index_of(n).ok_or_else(|| EngineError::Type(format!("unknown column `{n}`")))?;
            fields.push(self.schema.fields[i].clone());
            columns.push(self.columns[i].clone());
        }
        Self::new(Schema::new(fields)?, columns)
    }

    /// JSON table for API responses, capped at `max_rows`.
    pub fn to_table(&self, max_rows: usize) -> Table {
        let shown = self.rows().min(max_rows);
        Table {
            columns: self.schema.fields.clone(),
            rows: (0..shown).map(|r| self.columns.iter().map(|c| c.value(r)).collect()).collect(),
            total_rows: self.rows(),
            truncated: shown < self.rows(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Table {
    pub columns: Vec<Field>,
    pub rows: Vec<Vec<Value>>,
    pub total_rows: usize,
    pub truncated: bool,
}
