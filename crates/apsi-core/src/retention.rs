//! Retention and archival policy types ("Retention policy" family).
//!
//! Semantics:
//! * `archive_after_days`: active/draft records not updated for N days are archived.
//! * `purge_archived_after_days`: archived records are permanently deleted N days
//!   after archival. Deletion removes the record and its version history; the audit
//!   log keeps the fact that it existed and who deleted it.
//! * Records under legal hold are never archived or purged by policy, and cannot be
//!   deleted manually until the hold is released.

use crate::error::{ApsiError, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RetentionPolicy {
    #[serde(default)]
    pub archive_after_days: Option<u32>,
    #[serde(default)]
    pub purge_archived_after_days: Option<u32>,
}

impl RetentionPolicy {
    pub fn check(&self) -> Result<()> {
        let bad = |d: Option<u32>| d.is_some_and(|d| d == 0 || d > 36_500);
        if bad(self.archive_after_days) || bad(self.purge_archived_after_days) {
            return Err(ApsiError::InvalidInput("retention periods must be 1..=36500 days".into()));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RetentionReport {
    pub dry_run: bool,
    pub archived: Vec<String>,
    pub purged: Vec<String>,
    /// Records that matched a policy but were skipped because of a legal hold.
    pub held: Vec<String>,
}
