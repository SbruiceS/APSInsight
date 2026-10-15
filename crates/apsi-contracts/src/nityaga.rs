//! Nityaga RTOS contract v1.0: device capabilities, firmware identity and the
//! command safety envelope.
//!
//! Commands to actuators are the highest-risk operation in the ecosystem, so the
//! envelope is strict: bounded duration, explicit expiry, interlocks, mandatory
//! acknowledgement, and a target actuator the device actually declares.

use crate::apsnetwork::CommandAck;
use crate::common::{Adapter, ContractError, Result, Unavailable, check_sha256, check_unit, non_empty, parse_time};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FirmwareIdentity {
    pub name: String,
    pub version: String,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SensorCapability {
    pub name: String,
    pub unit: String,
    pub min: f64,
    pub max: f64,
    pub resolution: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActuatorCapability {
    pub name: String,
    /// State the actuator returns to on fault, expiry or lost link.
    pub safe_state: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimeSync {
    pub source: String,
    pub max_drift_ms: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceCapability {
    pub device_id: String,
    pub firmware: FirmwareIdentity,
    pub sensors: Vec<SensorCapability>,
    pub actuators: Vec<ActuatorCapability>,
    pub time_sync: TimeSync,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandSafetyEnvelope {
    pub command_id: String,
    pub device_id: String,
    pub actuator: String,
    pub action: String,
    pub max_duration_ms: u32,
    pub issued_at: String,
    pub expires_at: String,
    pub requires_ack: bool,
    /// Conditions the device must verify before acting (e.g. `pressure_ok`).
    pub interlocks: Vec<String>,
    pub issued_by: String,
}

pub const MAX_COMMAND_DURATION_MS: u32 = 3_600_000;
pub const MAX_COMMAND_VALIDITY_S: i64 = 900;

impl DeviceCapability {
    pub fn validate(&self) -> Result<()> {
        non_empty("device_id", &self.device_id, 128)?;
        non_empty("firmware.name", &self.firmware.name, 64)?;
        non_empty("firmware.version", &self.firmware.version, 32)?;
        check_sha256("firmware.sha256", &self.firmware.sha256)?;
        for (i, s) in self.sensors.iter().enumerate() {
            let f = format!("sensors[{i}]");
            non_empty(&format!("{f}.name"), &s.name, 64)?;
            check_unit(&format!("{f}.unit"), &s.unit)?;
            if !(s.min.is_finite() && s.max.is_finite() && s.min < s.max && s.resolution > 0.0 && s.resolution.is_finite()) {
                return Err(ContractError::invalid(f, "requires finite min < max and resolution > 0"));
            }
        }
        for (i, a) in self.actuators.iter().enumerate() {
            non_empty(&format!("actuators[{i}].name"), &a.name, 64)?;
            non_empty(&format!("actuators[{i}].safe_state"), &a.safe_state, 64)?;
        }
        non_empty("time_sync.source", &self.time_sync.source, 64)?;
        Ok(())
    }
}

impl CommandSafetyEnvelope {
    pub fn validate_for(&self, device: &DeviceCapability) -> Result<()> {
        non_empty("command_id", &self.command_id, 128)?;
        non_empty("action", &self.action, 64)?;
        non_empty("issued_by", &self.issued_by, 128)?;
        if self.device_id != device.device_id {
            return Err(ContractError::invalid("device_id", "does not match the target device"));
        }
        if !device.actuators.iter().any(|a| a.name == self.actuator) {
            return Err(ContractError::invalid("actuator", format!("device does not declare actuator `{}`", self.actuator)));
        }
        if !self.requires_ack {
            return Err(ContractError::invalid("requires_ack", "actuator commands must be acknowledged"));
        }
        if self.interlocks.is_empty() || self.interlocks.iter().any(|s| s.trim().is_empty()) {
            return Err(ContractError::invalid("interlocks", "at least one named interlock is required"));
        }
        if !(1..=MAX_COMMAND_DURATION_MS).contains(&self.max_duration_ms) {
            return Err(ContractError::invalid("max_duration_ms", format!("must be within 1..={MAX_COMMAND_DURATION_MS}")));
        }
        let issued = parse_time("issued_at", &self.issued_at)?;
        let expires = parse_time("expires_at", &self.expires_at)?;
        let validity = (expires - issued).whole_seconds();
        if validity <= 0 || validity > MAX_COMMAND_VALIDITY_S {
            return Err(ContractError::invalid("expires_at", format!("must be after issued_at and at most {MAX_COMMAND_VALIDITY_S} s later")));
        }
        Ok(())
    }
}

pub trait DevicePort: Adapter {
    fn capabilities(&self, device_id: &str) -> Result<DeviceCapability>;
    fn command(&self, cmd: &CommandSafetyEnvelope) -> Result<CommandAck>;
}

impl DevicePort for Unavailable {
    fn capabilities(&self, device_id: &str) -> Result<DeviceCapability> {
        non_empty("device_id", device_id, 128)?;
        Err(self.error())
    }

    fn command(&self, _cmd: &CommandSafetyEnvelope) -> Result<CommandAck> {
        // Without the device's declared capabilities a command cannot be checked,
        // so nothing is ever sent.
        Err(self.error())
    }
}
