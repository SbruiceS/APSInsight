//! Reproducible workflows with typed input/output contracts ("Workflow integration").
//!
//! A workflow is an ordered list of steps. Each step's output type is known, and
//! every reference to an earlier step is type-checked when the workflow is
//! registered — not when it runs.

use crate::error::{ApsiError, Result};
use crate::ids::validate_slug;
use crate::search::Query;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataType {
    RecordSet,
    QualityReport,
    Json,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Binding {
    Literal { value: Value },
    Step { step: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum StepOp {
    /// Output: RecordSet.
    Search { query: Query },
    /// Output: QualityReport.
    Quality { domain: String, entity: String },
    /// Input: RecordSet. Output: Json array of numbers (records lacking the field are skipped).
    ExtractNumbers { from: String, field: String },
    /// Input: Json (literal or step). Output: Json. `function` must exist in the compute registry.
    Compute { function: String, input: Binding },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Step {
    pub id: String,
    #[serde(flatten)]
    pub op: StepOp,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub version: u32,
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionSpec {
    pub name: String,
    pub description: String,
    pub input: String,
    pub output: String,
}

/// Pure functions available to `compute` steps (implemented by apsi-science).
pub trait ComputeRegistry: Send + Sync {
    fn functions(&self) -> Vec<FunctionSpec>;
    fn call(&self, name: &str, input: &Value) -> std::result::Result<Value, String>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRun {
    pub id: String,
    pub workflow_id: String,
    pub workflow_version: u32,
    pub status: RunStatus,
    /// sha256 of the workflow definition.
    pub definition_hash: String,
    /// sha256 over (record id, version) of every record read; equal fingerprints
    /// plus equal definition hash imply identical outputs.
    pub data_fingerprint: String,
    pub outputs: BTreeMap<String, Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub actor: String,
    #[serde(with = "time::serde::rfc3339")]
    pub started_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub finished_at: OffsetDateTime,
}

impl WorkflowDefinition {
    /// Type-checks step references and function names.
    pub fn check(&self, registry: Option<&dyn ComputeRegistry>) -> Result<()> {
        validate_slug("workflow id", &self.id)?;
        if self.steps.is_empty() || self.steps.len() > 64 {
            return Err(ApsiError::InvalidInput("workflows need 1..=64 steps".into()));
        }
        let functions: Vec<String> = registry.map(|r| r.functions().into_iter().map(|f| f.name).collect()).unwrap_or_default();
        let mut types: HashMap<&str, DataType> = HashMap::new();
        for step in &self.steps {
            validate_slug("step id", &step.id)?;
            if types.contains_key(step.id.as_str()) {
                return Err(ApsiError::InvalidInput(format!("duplicate step id {}", step.id)));
            }
            let expect = |from: &str, want: DataType| -> Result<()> {
                match types.get(from) {
                    None => Err(ApsiError::InvalidInput(format!("step {} references unknown or later step {from}", step.id))),
                    Some(t) if *t != want => Err(ApsiError::InvalidInput(format!("step {} expects {want:?} from {from}, got {t:?}", step.id))),
                    Some(_) => Ok(()),
                }
            };
            let out = match &step.op {
                StepOp::Search { query } => {
                    query.check()?;
                    DataType::RecordSet
                }
                StepOp::Quality { .. } => DataType::QualityReport,
                StepOp::ExtractNumbers { from, field } => {
                    crate::ids::validate_field_name(field)?;
                    expect(from, DataType::RecordSet)?;
                    DataType::Json
                }
                StepOp::Compute { function, input } => {
                    if !functions.iter().any(|f| f == function) {
                        return Err(ApsiError::InvalidInput(format!("unknown compute function {function}")));
                    }
                    if let Binding::Step { step: from } = input {
                        expect(from, DataType::Json)?;
                    }
                    DataType::Json
                }
            };
            types.insert(step.id.as_str(), out);
        }
        Ok(())
    }
}
