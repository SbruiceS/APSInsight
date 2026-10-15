//! Bulk import/update types ("Bulk operations" family).
//!
//! Bulk requests are processed item by item: valid items are applied, invalid items
//! are reported with their index and error, so one bad row never blocks the batch
//! (partial-failure reporting). `dry_run` evaluates everything without persisting.
//! An `idempotency_key` makes retries safe: the same key with the same payload
//! returns the original report; the same key with a different payload is rejected.

use crate::exchange::FieldMapping;
use crate::record::NewRecord;
use serde::{Deserialize, Serialize};

pub const MAX_BULK_ITEMS: usize = 10_000;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BulkRequest {
    pub items: Vec<NewRecord>,
    #[serde(default)]
    pub dry_run: bool,
    /// When an item's source reference already exists, update that record instead of failing.
    #[serde(default)]
    pub upsert: bool,
    #[serde(default)]
    pub idempotency_key: Option<String>,
    #[serde(default)]
    pub mapping: Option<FieldMapping>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemStatus {
    Created,
    Updated,
    Unchanged,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemResult {
    pub index: usize,
    pub status: ItemStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<crate::error::ErrorBodyOwned>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BulkReport {
    pub dry_run: bool,
    pub replayed: bool,
    pub total: usize,
    pub created: usize,
    pub updated: usize,
    pub unchanged: usize,
    pub failed: usize,
    pub results: Vec<ItemResult>,
}

impl BulkReport {
    pub fn push(&mut self, r: ItemResult) {
        match r.status {
            ItemStatus::Created => self.created += 1,
            ItemStatus::Updated => self.updated += 1,
            ItemStatus::Unchanged => self.unchanged += 1,
            ItemStatus::Failed => self.failed += 1,
        }
        self.total += 1;
        self.results.push(r);
    }
}
