//! # APSI Compute Engine
//!
//! Turns an agricultural analytical request into an optimized, costed,
//! profiled execution, independent of the hardware underneath:
//!
//! ```text
//! workflow JSON ──► validation ──► schema + unit inference ──► optimizer
//!      ──► cost model (rows, bytes, backend per node) ──► scheduler
//!      ──► CPU (Rayon) | native C++ (apsi-native) | accelerator (AgriCUDA port, CPU fallback)
//!      ──► results + profile + validation ──► JSON / Arrow / Parquet
//! ```
//!
//! SQL over the same data runs on Apache DataFusion with Apache Arrow
//! ([`sql`]). Everything is functional at the edges: plans are immutable
//! values, the optimizer returns a new plan, expressions and reducers are pure,
//! and execution is deterministic for any thread count.
//!
//! | Component | Module |
//! |---|---|
//! | Workflow graph, logical IR | [`plan`] |
//! | Type and unit system | [`expr`], [`types`], `apsi_science::units` |
//! | Schema registry | [`source::DataSource`] (backed by `apsi-core` schemas) |
//! | Optimizer | [`optimizer`] |
//! | Cost model | [`cost`] |
//! | Scheduler, dataflow runtime | [`exec`] |
//! | CPU / native / accelerator backends | [`backend`], `apsi-native`, `apsi_contracts::agricuda` |
//! | Memory manager, checkpoint manager | [`runtime`] |
//! | Execution profiler, result validator | [`exec::NodeProfile`], [`exec::CheckResult`] |
//! | SQL engine, Arrow, Parquet | [`sql`] |

pub mod backend;
pub mod cost;
pub mod exec;
pub mod expr;
pub mod optimizer;
pub mod plan;
pub mod runtime;
pub mod source;
pub mod sql;
pub mod types;

use backend::{BackendInfo, Backends};
use cost::NodeCost;
use exec::{CheckResult, NodeProfile};
use optimizer::Rewrite;
use plan::Workflow;
use runtime::CheckpointStore;
use serde::Serialize;
use source::DataSource;
use std::collections::BTreeMap;
use std::sync::Arc;
use types::{Batch, Schema, Table};

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum EngineError {
    #[error("invalid plan: {0}")]
    InvalidPlan(String),
    #[error("type error: {0}")]
    Type(String),
    #[error("unit error: {0}")]
    Unit(String),
    #[error("data source: {0}")]
    Source(String),
    #[error("permission denied: {0}")]
    Denied(String),
    #[error("memory budget exceeded at `{what}`: {needed} bytes needed, limit {limit}")]
    MemoryExceeded { what: String, needed: u64, limit: u64 },
    #[error("kernel: {0}")]
    Kernel(String),
    #[error("sql: {0}")]
    Sql(String),
    #[error("timed out after {0} ms")]
    Timeout(u64),
    #[error("internal error: {0}")]
    Internal(String),
    #[error("node `{node}`: {error}")]
    AtNode { node: String, error: Box<EngineError> },
}

impl EngineError {
    /// Adds the node id, once.
    pub fn at(self, node: &str) -> Self {
        match self {
            Self::AtNode { .. } => self,
            e => Self::AtNode { node: node.into(), error: Box::new(e) },
        }
    }

    /// The underlying error, without node context.
    pub fn root(&self) -> &EngineError {
        match self {
            Self::AtNode { error, .. } => error.root(),
            e => e,
        }
    }
}

pub type Result<T> = std::result::Result<T, EngineError>;

#[derive(Debug, Clone, Serialize)]
pub struct Capabilities {
    pub engine_version: &'static str,
    pub backends: Vec<BackendInfo>,
    pub operators: Vec<&'static str>,
    pub kernels: Vec<&'static str>,
    pub aggregates: Vec<&'static str>,
    pub sql: SqlCapabilities,
    pub limits: Limits,
}

#[derive(Debug, Clone, Serialize)]
pub struct SqlCapabilities {
    pub engine: &'static str,
    pub columnar_format: &'static str,
    pub export: Vec<&'static str>,
    pub read_only: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Limits {
    pub max_nodes: usize,
    pub max_memory_mb: u64,
    pub max_output_rows: usize,
    pub checkpoint_capacity_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlanReport {
    pub optimized: Workflow,
    /// Original output id -> node that produces it after optimization.
    pub outputs: BTreeMap<String, String>,
    pub rewrites: Vec<Rewrite>,
    pub schemas: BTreeMap<String, Schema>,
    pub levels: Vec<Vec<String>>,
    pub costs: Vec<NodeCost>,
    /// Things the type system could not track (e.g. derived units).
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunReport {
    pub plan: PlanReport,
    pub outputs: BTreeMap<String, Table>,
    pub profile: Vec<NodeProfile>,
    pub checks: Vec<CheckResult>,
    pub memory: MemoryReport,
    pub elapsed_us: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryReport {
    pub limit_bytes: u64,
    pub peak_bytes: u64,
    pub checkpoint_entries: usize,
    pub checkpoint_bytes: u64,
}

pub const CHECKPOINT_CAPACITY: u64 = 256 * 1024 * 1024;

/// The engine. Cheap to clone; checkpoints are shared between clones.
#[derive(Clone)]
pub struct ComputeEngine {
    pub backends: Backends,
    checkpoints: Arc<CheckpointStore>,
}

impl Default for ComputeEngine {
    fn default() -> Self {
        Self::new(Backends::default())
    }
}

impl ComputeEngine {
    pub fn new(backends: Backends) -> Self {
        Self { backends, checkpoints: Arc::new(CheckpointStore::new(CHECKPOINT_CAPACITY)) }
    }

    pub fn capabilities(&self) -> Capabilities {
        Capabilities {
            engine_version: env!("CARGO_PKG_VERSION"),
            backends: self.backends.describe(),
            operators: vec!["scan", "values", "filter", "project", "aggregate", "join", "sort", "limit", "kernel"],
            kernels: vec!["ndvi", "moving_mean", "fft_power"],
            aggregates: vec!["count", "sum", "mean", "min", "max", "sd", "median"],
            sql: SqlCapabilities { engine: "Apache DataFusion 54", columnar_format: "Apache Arrow 58", export: vec!["json", "parquet"], read_only: true },
            limits: Limits { max_nodes: plan::MAX_NODES, max_memory_mb: 16 * 1024, max_output_rows: 100_000, checkpoint_capacity_bytes: CHECKPOINT_CAPACITY },
        }
    }

    /// Validates, type-checks, optimizes and costs a workflow without running it.
    pub fn plan(&self, wf: &Workflow, source: &dyn DataSource) -> Result<PlanReport> {
        wf.validate()?;
        // Type-check the plan as written, so errors refer to the user's nodes.
        let (_, mut notes) = wf.infer_schemas(source)?;
        let opt = optimizer::optimize(wf, source)?;
        let (schemas, more) = opt.workflow.infer_schemas(source)?;
        notes.extend(more);
        notes.sort();
        notes.dedup();
        let costs = cost::estimate(&opt.workflow, &schemas, source, &self.backends)?;
        let levels = opt.workflow.levels()?;
        Ok(PlanReport { optimized: opt.workflow, outputs: opt.outputs, rewrites: opt.rewrites, schemas, levels, costs, notes })
    }

    /// Plans and executes; returns the report and the full output batches.
    pub fn run_batches(&self, wf: &Workflow, source: &dyn DataSource) -> Result<(RunReport, BTreeMap<String, Arc<Batch>>)> {
        let report = self.plan(wf, source)?;
        let opt = optimizer::Optimized { workflow: report.optimized.clone(), outputs: report.outputs.clone(), rewrites: report.rewrites.clone() };
        let ex = exec::execute(&opt, &report.costs, source, &self.backends, Some(&self.checkpoints))?;
        let max = wf.options.max_output_rows;
        let outputs = ex.outputs.iter().map(|(k, b)| (k.clone(), b.to_table(max))).collect();
        let run = RunReport {
            plan: report,
            outputs,
            profile: ex.profile,
            checks: ex.checks,
            memory: MemoryReport { limit_bytes: ex.limit_bytes, peak_bytes: ex.peak_bytes, checkpoint_entries: self.checkpoints.len(), checkpoint_bytes: self.checkpoints.bytes() },
            elapsed_us: ex.elapsed_us,
        };
        Ok((run, ex.outputs))
    }

    pub fn run(&self, wf: &Workflow, source: &dyn DataSource) -> Result<RunReport> {
        Ok(self.run_batches(wf, source)?.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::MemorySource;
    use crate::types::{Column, DataType, Field};
    use backend::BackendKind;
    use runtime::CacheState;

    fn soil() -> MemorySource {
        let s = MemorySource::new();
        let schema = Schema::new(vec![
            Field::new("plot", DataType::Utf8, None),
            Field::new("treatment", DataType::Utf8, None),
            Field::new("ph", DataType::Float64, Some("1")),
            Field::new("yield", DataType::Float64, Some("t/ha")),
            Field::new("notes", DataType::Utf8, None),
        ])
        .unwrap();
        let rows = [("p1", "control", 6.1, 4.2), ("p2", "lime", 6.9, 5.1), ("p3", "control", 5.8, 3.9), ("p4", "lime", 7.1, 5.4), ("p5", "lime", 6.7, 4.8)];
        let b = Batch::new(
            schema,
            vec![
                Column::Utf8(rows.iter().map(|r| Some(r.0.into())).collect()),
                Column::Utf8(rows.iter().map(|r| Some(r.1.into())).collect()),
                Column::Float64(rows.iter().map(|r| Some(r.2)).collect()),
                Column::Float64(rows.iter().map(|r| Some(r.3)).collect()),
                Column::Utf8(vec![None; 5]),
            ],
        )
        .unwrap();
        s.put("soil", "plots", b);
        let sites = Batch::new(
            Schema::new(vec![Field::new("plot", DataType::Utf8, None), Field::new("rain", DataType::Float64, Some("mm"))]).unwrap(),
            vec![Column::Utf8(vec![Some("p1".into()), Some("p2".into()), Some("p9".into())]), Column::Float64(vec![Some(500.0), Some(620.0), Some(1.0)])],
        )
        .unwrap();
        s.put("weather", "sites", sites);
        s
    }

    fn wf(j: &str) -> Workflow {
        serde_json::from_str(j).unwrap()
    }

    #[test]
    fn end_to_end_with_pushdown_pruning_and_aggregation() {
        let src = soil();
        let w = wf(r#"{
          "nodes": [
            {"id": "plots", "op": "scan", "domain": "soil", "entity": "plots"},
            {"id": "proj", "op": "project", "input": "plots", "columns": [
              {"name": "treatment", "expr": {"col": "treatment"}},
              {"name": "ph", "expr": {"col": "ph"}},
              {"name": "yield_kg", "expr": {"convert": [{"col": "yield"}, "kg/ha"]}}]},
            {"id": "acid", "op": "filter", "input": "proj", "predicate": {"and": [{"lt": [{"col": "ph"}, {"lit": 7}]}, {"lt": [{"lit": 1}, {"lit": 2}]}]}},
            {"id": "unused", "op": "limit", "input": "plots", "n": 1},
            {"id": "by", "op": "aggregate", "input": "acid", "group_by": ["treatment"], "aggregates": [
              {"name": "n", "func": "count", "column": "ph"},
              {"name": "mean_yield", "func": "mean", "column": "yield_kg"},
              {"name": "sd_ph", "func": "sd", "column": "ph"}]}
          ],
          "outputs": ["by"],
          "checks": [{"node": "by", "column": "mean_yield", "min": 0, "max": 20000, "not_null": true}]
        }"#);
        let e = ComputeEngine::default();
        let r = e.run(&w, &src).unwrap();
        let rules: Vec<&str> = r.plan.rewrites.iter().map(|x| x.rule).collect();
        for want in ["constant_folding", "predicate_pushdown", "dead_node_elimination", "projection_pushdown"] {
            assert!(rules.contains(&want), "missing rewrite {want}: {rules:?}");
        }
        // The scan now filters and reads only what is used (not `plot`, `notes`).
        let scan = r.plan.optimized.nodes.iter().find(|n| matches!(n.op, plan::Op::Scan { .. })).unwrap();
        let plan::Op::Scan { columns, filter, .. } = &scan.op else { unreachable!() };
        assert_eq!(columns.as_deref(), Some(&["treatment".to_string(), "ph".to_string(), "yield".to_string()][..]));
        assert!(filter.is_some());
        let t = &r.outputs["by"];
        assert_eq!(t.rows.len(), 2);
        assert_eq!(t.columns[2].unit.as_deref(), Some("kg/ha"));
        // control: ph 6.1, 5.8 -> yields 4200, 3900; lime < 7: 6.9, 6.7 -> 5100, 4800.
        assert_eq!(t.rows[0][0], "control");
        assert_eq!(t.rows[0][1], 2);
        assert_eq!(t.rows[0][2].as_f64().unwrap(), 4050.0);
        assert_eq!(t.rows[1][2].as_f64().unwrap(), 4950.0);
        assert!((t.rows[0][3].as_f64().unwrap() - (0.045f64).sqrt()).abs() < 1e-12);
        assert!(r.checks.iter().all(|c| c.passed), "{:?}", r.checks);
        assert!(r.memory.peak_bytes > 0 && r.memory.peak_bytes <= r.memory.limit_bytes);
        assert!(r.profile.iter().all(|p| p.backend == BackendKind::Cpu));
    }

    #[test]
    fn joins_kernels_sort_limit_and_checks() {
        let src = soil();
        let w = wf(r#"{
          "nodes": [
            {"id": "p", "op": "scan", "domain": "soil", "entity": "plots"},
            {"id": "s", "op": "scan", "domain": "weather", "entity": "sites"},
            {"id": "j", "op": "join", "left": "p", "right": "s", "on": ["plot"], "kind": "left"},
            {"id": "o", "op": "sort", "input": "j", "by": [{"column": "rain", "descending": true}]},
            {"id": "top", "op": "limit", "input": "o", "n": 3},
            {"id": "px", "op": "values", "columns": [
              {"name": "red", "unit": "1", "values": [0.1, 0.0, -9999, 0.2]},
              {"name": "nir", "unit": "1", "values": [0.5, 0.0, 0.4, 0.6]}]},
            {"id": "veg", "op": "kernel", "input": "px", "kernel": "ndvi", "nodata": -9999},
            {"id": "mm", "op": "kernel", "input": "veg", "kernel": "moving_mean", "column": "nir", "window": 2, "output": "nir_ma"},
            {"id": "spec", "op": "kernel", "input": "px", "kernel": "fft_power", "column": "nir"}
          ],
          "outputs": ["top", "mm", "spec"],
          "checks": [{"node": "mm", "column": "ndvi", "min": -1, "max": 1, "not_null": true}]
        }"#);
        let r = ComputeEngine::default().run(&w, &src).unwrap();
        let top = &r.outputs["top"];
        assert_eq!(top.rows[0][0], "p2", "highest rain first");
        assert_eq!(top.rows[2][5], serde_json::Value::Null, "unmatched left rows have null rain, sorted last");
        let mm = &r.outputs["mm"];
        assert!((mm.rows[0][2].as_f64().unwrap() - 0.4 / 0.6).abs() < 1e-15);
        assert_eq!(mm.rows[1][2], serde_json::Value::Null, "zero denominator is undefined, not 0");
        assert_eq!(mm.rows[2][2], serde_json::Value::Null, "nodata pixel is undefined");
        assert_eq!(mm.rows[1][3].as_f64().unwrap(), 0.25);
        let check = r.checks.iter().find(|c| c.column == "ndvi" && c.rule.contains("not null")).unwrap();
        assert!(!check.passed && check.violations == 2 && check.examples == vec![1, 2], "{check:?}");
        assert_eq!(r.outputs["spec"].rows.len(), 3);
        assert!(r.profile.iter().any(|p| p.note.as_deref().is_some_and(|n| n.contains("undefined"))));
    }

    #[test]
    fn units_and_types_are_checked_before_anything_runs() {
        let src = soil();
        let e = ComputeEngine::default();
        let err = e
            .plan(&wf(r#"{"nodes": [{"id": "p", "op": "scan", "domain": "soil", "entity": "plots"},
                {"id": "bad", "op": "project", "input": "p", "columns": [{"name": "x", "expr": {"add": [{"col": "ph"}, {"col": "yield"}]}}]}],
                "outputs": ["bad"]}"#), &src)
            .unwrap_err();
        assert!(matches!(err, EngineError::AtNode { ref node, .. } if node == "bad"), "{err}");
        assert!(matches!(err.root(), EngineError::Unit(_)), "{err}");
        let err = e.plan(&wf(r#"{"nodes": [{"id": "p", "op": "scan", "domain": "soil", "entity": "plots"},
                {"id": "f", "op": "filter", "input": "p", "predicate": {"col": "ph"}}], "outputs": ["f"]}"#), &src).unwrap_err();
        assert!(err.to_string().contains("must be boolean"), "{err}");
        let err = e.plan(&wf(r#"{"nodes": [{"id": "p", "op": "scan", "domain": "soil", "entity": "nope"}], "outputs": ["p"]}"#), &src).unwrap_err();
        assert!(matches!(err.root(), EngineError::Source(_)));
    }

    #[test]
    fn checkpoints_are_reused_until_the_data_changes() {
        let src = soil();
        let e = ComputeEngine::default();
        let w = wf(r#"{"nodes": [{"id": "p", "op": "scan", "domain": "soil", "entity": "plots"},
            {"id": "a", "op": "aggregate", "input": "p", "aggregates": [{"name": "m", "func": "mean", "column": "ph"}]}], "outputs": ["a"]}"#);
        let cache = |r: &RunReport| r.profile.iter().find(|p| p.id == "a").unwrap().cache;
        assert_eq!(cache(&e.run(&w, &src).unwrap()), CacheState::Miss);
        let second = e.run(&w, &src).unwrap();
        assert_eq!(cache(&second), CacheState::Hit);
        // New data -> new fingerprint -> recomputed.
        let b = src.scan("soil", "plots", None).unwrap().slice(0, 2);
        src.put("soil", "plots", b);
        let third = e.run(&w, &src).unwrap();
        assert_eq!(cache(&third), CacheState::Miss);
        assert_ne!(third.outputs["a"].rows, second.outputs["a"].rows);
        let mut off = w.clone();
        off.options.checkpoint = false;
        assert_eq!(cache(&e.run(&off, &src).unwrap()), CacheState::Off);
    }

    #[test]
    fn memory_budget_is_enforced() {
        let src = MemorySource::new();
        let n = 200_000;
        src.put("big", "t", Batch::new(Schema::new(vec![Field::new("x", DataType::Float64, None)]).unwrap(), vec![Column::Float64((0..n).map(|i| Some(i as f64)).collect())]).unwrap());
        let mut w = wf(r#"{"nodes": [{"id": "t", "op": "scan", "domain": "big", "entity": "t"}], "outputs": ["t"]}"#);
        w.options.memory_limit_mb = 1;
        let err = ComputeEngine::default().run(&w, &src).unwrap_err();
        assert!(matches!(err.root(), EngineError::MemoryExceeded { .. }), "{err}");
    }

    #[test]
    fn results_do_not_depend_on_thread_count() {
        let src = MemorySource::new();
        let n = 100_003;
        let schema = Schema::new(vec![Field::new("g", DataType::Utf8, None), Field::new("x", DataType::Float64, Some("mm"))]).unwrap();
        src.put(
            "d",
            "t",
            Batch::new(schema, vec![Column::Utf8((0..n).map(|i| Some(format!("g{}", i % 7))).collect()), Column::Float64((0..n).map(|i| Some(((i * 7919) % 1000) as f64 * 0.137)).collect())]).unwrap(),
        );
        let w = wf(r#"{"nodes": [{"id": "t", "op": "scan", "domain": "d", "entity": "t"},
            {"id": "f", "op": "filter", "input": "t", "predicate": {"gt": [{"col": "x"}, {"lit": 10}]}},
            {"id": "a", "op": "aggregate", "input": "f", "group_by": ["g"], "aggregates": [{"name": "s", "func": "sum", "column": "x"}, {"name": "sd", "func": "sd", "column": "x"}]}],
            "outputs": ["a"], "options": {"checkpoint": false}}"#);
        let run = |threads| rayon::ThreadPoolBuilder::new().num_threads(threads).build().unwrap().install(|| ComputeEngine::default().run(&w, &src).unwrap().outputs["a"].clone());
        let one = run(1);
        assert_eq!(one, run(4));
        assert_eq!(one, run(8));
    }

    #[test]
    fn capabilities_are_honest() {
        let c = ComputeEngine::default().capabilities();
        assert_eq!(c.backends.len(), 3);
        assert!(!c.backends[2].available);
        assert!(c.sql.read_only);
    }
}
