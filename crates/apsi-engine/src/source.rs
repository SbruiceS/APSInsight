//! Where scans read from. The server implements [`DataSource`] over the
//! record store (tenant-scoped, authorised and audited by `apsi-core`);
//! [`MemorySource`] serves tests and embedded use.

use crate::types::{Batch, Schema};
use crate::{EngineError, Result};
use parking_lot::RwLock;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TableInfo {
    pub domain: String,
    pub entity: String,
    /// SQL name: `domain__entity` with `-` replaced by `_`.
    pub table: String,
    pub rows: u64,
    pub schema: Schema,
}

pub fn sql_name(domain: &str, entity: &str) -> String {
    format!("{}__{}", domain.replace('-', "_"), entity.replace('-', "_"))
}

pub trait DataSource: Send + Sync {
    fn schema(&self, domain: &str, entity: &str) -> Result<Schema>;
    /// Row count used by the cost model; may be an estimate.
    fn estimate_rows(&self, domain: &str, entity: &str) -> Result<u64>;
    /// Changes whenever the data changes; part of every checkpoint key.
    fn fingerprint(&self, domain: &str, entity: &str) -> Result<String>;
    /// All rows, restricted to `columns` when given (projection pushdown).
    fn scan(&self, domain: &str, entity: &str, columns: Option<&[String]>) -> Result<Batch>;
    /// Tables that currently hold data, for SQL and discovery.
    fn tables(&self) -> Result<Vec<TableInfo>>;
}

/// An in-memory source. Each `put` bumps the table's fingerprint.
#[derive(Default)]
pub struct MemorySource {
    tables: RwLock<BTreeMap<(String, String), (Batch, u64)>>,
}

impl MemorySource {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn put(&self, domain: &str, entity: &str, batch: Batch) {
        let mut t = self.tables.write();
        let version = t.get(&(domain.into(), entity.into())).map_or(0, |(_, v)| v + 1);
        t.insert((domain.into(), entity.into()), (batch, version));
    }

    fn get(&self, domain: &str, entity: &str) -> Result<(Batch, u64)> {
        self.tables.read().get(&(domain.into(), entity.into())).cloned().ok_or_else(|| EngineError::Source(format!("no table {domain}/{entity}")))
    }
}

impl DataSource for MemorySource {
    fn schema(&self, domain: &str, entity: &str) -> Result<Schema> {
        Ok(self.get(domain, entity)?.0.schema)
    }

    fn estimate_rows(&self, domain: &str, entity: &str) -> Result<u64> {
        Ok(self.get(domain, entity)?.0.rows() as u64)
    }

    fn fingerprint(&self, domain: &str, entity: &str) -> Result<String> {
        Ok(format!("v{}", self.get(domain, entity)?.1))
    }

    fn scan(&self, domain: &str, entity: &str, columns: Option<&[String]>) -> Result<Batch> {
        let b = self.get(domain, entity)?.0;
        match columns {
            Some(c) => b.select(c),
            None => Ok(b),
        }
    }

    fn tables(&self) -> Result<Vec<TableInfo>> {
        Ok(self
            .tables
            .read()
            .iter()
            .map(|((d, e), (b, _))| TableInfo { domain: d.clone(), entity: e.clone(), table: sql_name(d, e), rows: b.rows() as u64, schema: b.schema.clone() })
            .collect())
    }
}
