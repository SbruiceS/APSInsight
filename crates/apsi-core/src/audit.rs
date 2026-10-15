//! Append-only, hash-chained audit log. Every allowed, denied or failed operation
//! is recorded; `verify` detects tampering with any stored event.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Allowed,
    Denied,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditEvent {
    pub seq: u64,
    #[serde(with = "time::serde::rfc3339")]
    pub at: OffsetDateTime,
    pub tenant: String,
    pub actor: String,
    pub action: String,
    pub target: String,
    pub outcome: Outcome,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    pub prev_hash: String,
    pub hash: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditLog {
    events: Vec<AuditEvent>,
}

const GENESIS: &str = "0000000000000000000000000000000000000000000000000000000000000000";

fn event_hash(e: &AuditEvent) -> String {
    let material = serde_json::json!({
        "seq": e.seq, "at": e.at.unix_timestamp_nanos().to_string(), "tenant": e.tenant, "actor": e.actor,
        "action": e.action, "target": e.target, "outcome": e.outcome, "detail": e.detail, "prev": e.prev_hash,
    });
    hex::encode(Sha256::digest(material.to_string().as_bytes()))
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditInput<'a> {
    pub tenant: &'a str,
    pub actor: &'a str,
    pub action: &'a str,
    pub target: &'a str,
    pub outcome: Outcome,
    pub detail: Option<String>,
}

impl AuditLog {
    pub fn append(&mut self, at: OffsetDateTime, input: AuditInput<'_>) -> &AuditEvent {
        let prev_hash = self.events.last().map_or_else(|| GENESIS.to_string(), |e| e.hash.clone());
        let mut e = AuditEvent {
            seq: self.events.len() as u64 + 1,
            at,
            tenant: input.tenant.into(),
            actor: input.actor.into(),
            action: input.action.into(),
            target: input.target.into(),
            outcome: input.outcome,
            detail: input.detail.map(|d| crate::metrics::redact(&d)),
            prev_hash,
            hash: String::new(),
        };
        e.hash = event_hash(&e);
        self.events.push(e);
        self.events.last().expect("just pushed")
    }

    /// Re-appends a persisted event during journal replay. Events already present
    /// (sequence number not beyond the current length) are skipped, so replaying a
    /// journal over a newer snapshot is idempotent. `verify` checks the result.
    pub fn restore(&mut self, e: AuditEvent) {
        if e.seq as usize == self.events.len() + 1 {
            self.events.push(e);
        }
    }

    pub fn events(&self) -> &[AuditEvent] {
        &self.events
    }

    pub fn for_tenant<'a>(&'a self, tenant: &'a str) -> impl DoubleEndedIterator<Item = &'a AuditEvent> + 'a {
        self.events.iter().filter(move |e| e.tenant == tenant)
    }

    /// Returns the sequence number of the first broken link, if any.
    pub fn verify(&self) -> Result<(), u64> {
        let mut prev = GENESIS.to_string();
        for e in &self.events {
            if e.prev_hash != prev || event_hash(e) != e.hash {
                return Err(e.seq);
            }
            prev = e.hash.clone();
        }
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    #[cfg(test)]
    pub(crate) fn events_mut(&mut self) -> &mut Vec<AuditEvent> {
        &mut self.events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_detects_tampering() {
        let mut log = AuditLog::default();
        let now = OffsetDateTime::UNIX_EPOCH;
        for i in 0..5 {
            log.append(now, AuditInput { tenant: "t", actor: "a", action: "create", target: &format!("r{i}"), outcome: Outcome::Allowed, detail: None });
        }
        assert!(log.verify().is_ok());
        log.events_mut()[2].target = "forged".into();
        assert_eq!(log.verify(), Err(3));
    }

    #[test]
    fn details_are_redacted() {
        let mut log = AuditLog::default();
        let e = log.append(
            OffsetDateTime::UNIX_EPOCH,
            AuditInput { tenant: "t", actor: "a", action: "x", target: "y", outcome: Outcome::Failed, detail: Some("token=abc123 failed".into()) },
        );
        assert!(!e.detail.as_ref().unwrap().contains("abc123"));
    }
}
