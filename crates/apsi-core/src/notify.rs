//! Notification rules with deduplication ("Notification rules" family).
//!
//! Matching notifications are written to a per-tenant outbox. Webhook channels are
//! recorded with their target so a delivery worker can send them; the engine itself
//! never makes outbound network calls.

use crate::error::{ApsiError, Result};
use crate::search::Filter;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    Created,
    Updated,
    Archived,
    Restored,
    Deleted,
    /// Accepted with warning-severity validation issues.
    ValidationWarning,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Channel {
    InApp,
    Webhook { url: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationRule {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub tenant: String,
    pub domain: String,
    #[serde(default)]
    pub entity: Option<String>,
    pub events: Vec<EventKind>,
    /// Optional attribute condition evaluated against the record after the change.
    #[serde(default)]
    pub condition: Option<Filter>,
    pub channel: Channel,
    #[serde(default = "default_window")]
    pub dedup_window_secs: u64,
    #[serde(default)]
    pub created_by: String,
}

fn default_window() -> u64 {
    300
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub rule_id: String,
    pub tenant: String,
    pub event: EventKind,
    pub record_id: String,
    pub message: String,
    pub channel: Channel,
    #[serde(with = "time::serde::rfc3339")]
    pub at: OffsetDateTime,
}

impl NotificationRule {
    pub fn check(&self) -> Result<()> {
        if self.events.is_empty() {
            return Err(ApsiError::InvalidInput("notification rule needs at least one event".into()));
        }
        if self.dedup_window_secs > 30 * 86_400 {
            return Err(ApsiError::InvalidInput("dedup window is limited to 30 days".into()));
        }
        if let Channel::Webhook { url } = &self.channel
            && (!url.starts_with("https://") || url.len() > 2048 || url.chars().any(|c| c.is_control() || c == ' '))
        {
            return Err(ApsiError::InvalidInput("webhook url must be an https URL".into()));
        }
        if let Some(c) = &self.condition {
            crate::ids::validate_field_name(&c.field)?;
        }
        Ok(())
    }
}
