//! Dashboard summaries ("Dashboard visibility" family). Every number links back to
//! a query the UI can drill into (domain + entity + lifecycle) and to the formula
//! definitions in [`crate::quality::FORMULAS`].

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EntitySummary {
    pub entity: String,
    pub name: String,
    pub schema_version: u32,
    pub total: usize,
    pub draft: usize,
    pub active: usize,
    pub archived: usize,
    pub legal_hold: usize,
    /// Share of non-archived records passing validation; null when there are none.
    pub validity: Option<f64>,
    pub completeness: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_updated: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DomainDashboard {
    pub domain: String,
    pub name: String,
    pub total: usize,
    pub entities: Vec<EntitySummary>,
    pub notifications: usize,
    pub recent_activity: Vec<crate::audit::AuditEvent>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DomainCount {
    pub domain: String,
    pub name: String,
    pub entity_kinds: usize,
    pub records: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Overview {
    pub tenant: String,
    pub domains: Vec<DomainCount>,
    pub records: usize,
    pub audit_events: usize,
    pub audit_chain_valid: bool,
}
