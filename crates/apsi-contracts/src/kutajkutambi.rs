//! KutajKutambi AI contract v1.0: inference and agent tasks.
//!
//! The contract makes three things structural rather than advisory:
//! - outputs can be predictions, interpretations or recommendations, never
//!   observations (there is no `observation` variant);
//! - every output carries model identity, an evaluation reference and uncertainty;
//! - tools may be granted read or propose scope only; there is no write scope, and
//!   recommendations always require human approval.

use crate::common::{Adapter, ContractError, Result, Unavailable, check_sha256, non_empty};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InferenceRequest {
    pub tenant: String,
    pub task: TaskKind,
    /// Natural-language or structured instruction.
    pub instruction: String,
    /// Records the model may use, pinned to versions and content hashes.
    pub evidence: Vec<EvidenceRef>,
    #[serde(default)]
    pub tools: Vec<ToolPermission>,
    pub deadline_ms: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    Classification,
    Forecast,
    Interpretation,
    Recommendation,
    AgentTask,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceRef {
    pub record_id: String,
    pub version: u64,
    pub content_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolPermission {
    pub tool: String,
    pub scope: ToolScope,
}

/// There is intentionally no write scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolScope {
    Read,
    Propose,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputKind {
    Prediction,
    AiInterpretation,
    Recommendation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Completed,
    Partial,
    Refused,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelMeta {
    pub name: String,
    pub version: String,
    /// Points to the evaluation evidence (model card, metrics) for this version.
    pub evaluation_ref: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Uncertainty {
    pub method: String,
    #[serde(default)]
    pub confidence: Option<f64>,
    #[serde(default)]
    pub interval: Option<[f64; 2]>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InferenceResponse {
    pub status: ExecutionStatus,
    pub kind: OutputKind,
    pub output: serde_json::Value,
    pub model: ModelMeta,
    pub uncertainty: Uncertainty,
    /// The subset of request evidence actually used.
    pub evidence_used: Vec<EvidenceRef>,
    pub requires_human_approval: bool,
}

impl InferenceRequest {
    pub fn validate(&self) -> Result<()> {
        non_empty("tenant", &self.tenant, 64)?;
        non_empty("instruction", &self.instruction, 32 * 1024)?;
        if self.evidence.is_empty() {
            return Err(ContractError::invalid("evidence", "at least one evidence record is required"));
        }
        for (i, e) in self.evidence.iter().enumerate() {
            non_empty(&format!("evidence[{i}].record_id"), &e.record_id, 128)?;
            check_sha256(&format!("evidence[{i}].content_sha256"), &e.content_sha256)?;
        }
        for (i, t) in self.tools.iter().enumerate() {
            non_empty(&format!("tools[{i}].tool"), &t.tool, 64)?;
        }
        if !(1..=600_000).contains(&self.deadline_ms) {
            return Err(ContractError::invalid("deadline_ms", "must be within 1..=600000"));
        }
        Ok(())
    }
}

impl InferenceResponse {
    pub fn validate_for(&self, req: &InferenceRequest) -> Result<()> {
        non_empty("model.name", &self.model.name, 128)?;
        non_empty("model.version", &self.model.version, 64)?;
        non_empty("model.evaluation_ref", &self.model.evaluation_ref, 512)?;
        non_empty("uncertainty.method", &self.uncertainty.method, 128)?;
        if let Some(c) = self.uncertainty.confidence
            && !(0.0..=1.0).contains(&c)
        {
            return Err(ContractError::invalid("uncertainty.confidence", "must be within [0, 1]"));
        }
        if let Some([lo, hi]) = self.uncertainty.interval
            && !(lo.is_finite() && hi.is_finite() && lo <= hi)
        {
            return Err(ContractError::invalid("uncertainty.interval", "must be finite with lo <= hi"));
        }
        if let Some(e) = self.evidence_used.iter().find(|e| !req.evidence.contains(e)) {
            return Err(ContractError::invalid("evidence_used", format!("{} was not supplied in the request", e.record_id)));
        }
        let recommends = self.kind == OutputKind::Recommendation || req.task == TaskKind::Recommendation;
        if recommends && !self.requires_human_approval {
            return Err(ContractError::invalid("requires_human_approval", "recommendations always require human approval"));
        }
        if self.status != ExecutionStatus::Refused && self.evidence_used.is_empty() {
            return Err(ContractError::invalid("evidence_used", "a non-refused answer must cite evidence"));
        }
        Ok(())
    }
}

pub trait InferencePort: Adapter {
    fn infer(&self, req: &InferenceRequest) -> Result<InferenceResponse>;
}

impl InferencePort for Unavailable {
    fn infer(&self, req: &InferenceRequest) -> Result<InferenceResponse> {
        req.validate()?;
        Err(self.error())
    }
}
