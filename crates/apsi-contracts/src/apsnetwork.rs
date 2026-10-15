//! APSNetwork contract v1.0: telemetry exchange and edge job delivery.
//!
//! Event time and receive time are kept separately so late data can be detected;
//! every measurement carries a unit and an explicit quality flag.

use crate::common::{Adapter, ContractError, ContractVersion, Quantity, Result, Unavailable, non_empty, parse_time};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TelemetryEnvelope {
    pub contract: ContractVersion,
    pub device_id: String,
    /// Monotonic per device; used for deduplication and gap detection.
    pub sequence: u64,
    pub event_time: String,
    pub received_time: String,
    pub measurements: BTreeMap<String, Quantity>,
    #[serde(default)]
    pub link: Option<LinkQuality>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LinkQuality {
    pub rssi_dbm: f64,
    pub snr_db: f64,
    pub packet_loss_ratio: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeJob {
    pub job_id: String,
    pub target_device: String,
    pub kind: String,
    pub payload: serde_json::Value,
    pub expires_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AckStatus {
    Accepted,
    Rejected,
    Executed,
    TimedOut,
    Queued,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandAck {
    pub command_id: String,
    pub status: AckStatus,
    pub at: String,
    #[serde(default)]
    pub detail: Option<String>,
}

/// Maximum tolerated clock skew where receive time precedes event time.
pub const MAX_CLOCK_SKEW_S: i64 = 300;

impl TelemetryEnvelope {
    pub fn validate(&self, expected: ContractVersion) -> Result<()> {
        if !self.contract.satisfies(expected) {
            return Err(ContractError::Incompatible { required: expected, offered: self.contract });
        }
        non_empty("device_id", &self.device_id, 128)?;
        let ev = parse_time("event_time", &self.event_time)?;
        let rx = parse_time("received_time", &self.received_time)?;
        if (ev - rx).whole_seconds() > MAX_CLOCK_SKEW_S {
            return Err(ContractError::invalid("event_time", "more than 300 s after received_time; device clock is unsynchronised"));
        }
        if self.measurements.is_empty() {
            return Err(ContractError::invalid("measurements", "must not be empty"));
        }
        for (k, q) in &self.measurements {
            non_empty("measurements", k, 64)?;
            q.validate(&format!("measurements.{k}"))?;
        }
        if let Some(l) = &self.link
            && (!(0.0..=1.0).contains(&l.packet_loss_ratio) || !l.rssi_dbm.is_finite() || !l.snr_db.is_finite())
        {
            return Err(ContractError::invalid("link", "rssi/snr must be finite and packet_loss_ratio within [0, 1]"));
        }
        Ok(())
    }

    /// Seconds between event and receipt; > 0 means late arrival.
    pub fn lateness_s(&self) -> Result<i64> {
        Ok((parse_time("received_time", &self.received_time)? - parse_time("event_time", &self.event_time)?).whole_seconds())
    }
}

impl EdgeJob {
    pub fn validate(&self) -> Result<()> {
        non_empty("job_id", &self.job_id, 128)?;
        non_empty("target_device", &self.target_device, 128)?;
        non_empty("kind", &self.kind, 64)?;
        parse_time("expires_at", &self.expires_at)?;
        Ok(())
    }
}

pub trait EdgeNetworkPort: Adapter {
    fn deliver(&self, job: &EdgeJob) -> Result<CommandAck>;
}

impl EdgeNetworkPort for Unavailable {
    fn deliver(&self, job: &EdgeJob) -> Result<CommandAck> {
        job.validate()?;
        Err(self.error())
    }
}
