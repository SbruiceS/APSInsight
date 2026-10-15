//! The logical intermediate representation: a workflow is an immutable DAG of
//! typed nodes, independent of the hardware that will execute it.
//!
//! ```json
//! {
//!   "nodes": [
//!     {"id": "ph", "op": "scan", "domain": "soil-profile-and-classification", "entity": "ph"},
//!     {"id": "acid", "op": "filter", "input": "ph", "predicate": {"lt": [{"col": "value"}, {"lit": 6.5}]}},
//!     {"id": "by_tag", "op": "aggregate", "input": "acid", "group_by": ["lifecycle"],
//!      "aggregates": [{"name": "mean_ph", "func": "mean", "column": "value"}]}
//!   ],
//!   "outputs": ["by_tag"]
//! }
//! ```

use crate::expr::{Expr, Literal, dimension};
use crate::source::DataSource;
use crate::types::{DataType, Field, Schema};
use crate::{EngineError, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const MAX_NODES: usize = 256;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Workflow {
    pub nodes: Vec<Node>,
    pub outputs: Vec<String>,
    #[serde(default)]
    pub options: Options,
    /// Scientific constraints checked on the results.
    #[serde(default)]
    pub checks: Vec<Check>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    #[serde(flatten)]
    pub op: Op,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NamedExpr {
    pub name: String,
    pub expr: Expr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AggFunc {
    Count,
    Sum,
    Mean,
    Min,
    Max,
    /// Sample standard deviation (n - 1).
    Sd,
    Median,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Aggregate {
    pub name: String,
    pub func: AggFunc,
    pub column: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SortKey {
    pub column: String,
    #[serde(default)]
    pub descending: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum JoinKind {
    #[default]
    Inner,
    Left,
}

/// Numerical kernels, dispatched to the CPU, native C++ or accelerator backend.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kernel", rename_all = "snake_case")]
pub enum Kernel {
    /// Adds `output` = NDVI(red, nir).
    Ndvi {
        red: String,
        nir: String,
        #[serde(default)]
        nodata: Option<f64>,
        #[serde(default = "default_ndvi")]
        output: String,
    },
    /// Adds `output` = trailing moving mean of `column` (rows in input order).
    MovingMean { column: String, window: usize, output: String },
    /// Replaces the batch with the power spectrum of `column`: columns `bin`
    /// and `power`. The row count must be a power of two.
    FftPower { column: String },
}

fn default_ndvi() -> String {
    "ndvi".into()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Op {
    /// Records of one entity kind. `columns` and `filter` are usually filled in
    /// by the optimizer (projection pruning and predicate pushdown).
    Scan {
        domain: String,
        entity: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        columns: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        filter: Option<Expr>,
    },
    /// Inline data, e.g. for experiments or tests.
    Values { columns: Vec<ValueColumn> },
    Filter { input: String, predicate: Expr },
    Project { input: String, columns: Vec<NamedExpr> },
    Aggregate {
        input: String,
        #[serde(default)]
        group_by: Vec<String>,
        aggregates: Vec<Aggregate>,
    },
    Join {
        left: String,
        right: String,
        on: Vec<String>,
        #[serde(default)]
        kind: JoinKind,
    },
    Sort { input: String, by: Vec<SortKey> },
    Limit { input: String, n: usize },
    Kernel {
        input: String,
        #[serde(flatten)]
        kernel: Kernel,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValueColumn {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    pub values: Vec<Literal>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BackendChoice {
    /// Chosen per node by the cost model.
    #[default]
    Auto,
    Cpu,
    Native,
    Accelerator,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Options {
    pub backend: BackendChoice,
    /// Memory budget for intermediate results, MiB.
    pub memory_limit_mb: u64,
    /// Reuse checkpointed results of identical sub-plans over unchanged data.
    pub checkpoint: bool,
    /// Rows returned per output in the JSON response.
    pub max_output_rows: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self { backend: BackendChoice::Auto, memory_limit_mb: 512, checkpoint: true, max_output_rows: 1000 }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Check {
    pub node: String,
    pub column: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(default)]
    pub not_null: bool,
}

impl Op {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Scan { .. } => "scan",
            Self::Values { .. } => "values",
            Self::Filter { .. } => "filter",
            Self::Project { .. } => "project",
            Self::Aggregate { .. } => "aggregate",
            Self::Join { .. } => "join",
            Self::Sort { .. } => "sort",
            Self::Limit { .. } => "limit",
            Self::Kernel { .. } => "kernel",
        }
    }

    pub fn inputs(&self) -> Vec<&str> {
        match self {
            Self::Scan { .. } | Self::Values { .. } => vec![],
            Self::Filter { input, .. } | Self::Project { input, .. } | Self::Aggregate { input, .. } | Self::Sort { input, .. } | Self::Limit { input, .. } | Self::Kernel { input, .. } => {
                vec![input.as_str()]
            }
            Self::Join { left, right, .. } => vec![left.as_str(), right.as_str()],
        }
    }

    pub fn inputs_mut(&mut self) -> Vec<&mut String> {
        match self {
            Self::Scan { .. } | Self::Values { .. } => vec![],
            Self::Filter { input, .. } | Self::Project { input, .. } | Self::Aggregate { input, .. } | Self::Sort { input, .. } | Self::Limit { input, .. } | Self::Kernel { input, .. } => vec![input],
            Self::Join { left, right, .. } => vec![left, right],
        }
    }
}

fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

impl Workflow {
    pub fn node(&self, id: &str) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// Structural validation: ids, references, acyclicity and limits.
    pub fn validate(&self) -> Result<()> {
        if self.nodes.is_empty() {
            return Err(EngineError::InvalidPlan("a workflow needs at least one node".into()));
        }
        if self.nodes.len() > MAX_NODES {
            return Err(EngineError::InvalidPlan(format!("at most {MAX_NODES} nodes are allowed")));
        }
        let mut ids = BTreeSet::new();
        for n in &self.nodes {
            if !valid_id(&n.id) {
                return Err(EngineError::InvalidPlan(format!("node id `{}` must be 1-64 characters of [A-Za-z0-9_-]", n.id)));
            }
            if !ids.insert(n.id.as_str()) {
                return Err(EngineError::InvalidPlan(format!("duplicate node id `{}`", n.id)));
            }
        }
        for n in &self.nodes {
            for i in n.op.inputs() {
                if !ids.contains(i) {
                    return Err(EngineError::InvalidPlan(format!("node `{}` reads unknown node `{i}`", n.id)));
                }
                if i == n.id {
                    return Err(EngineError::InvalidPlan(format!("node `{}` reads itself", n.id)));
                }
            }
        }
        if self.outputs.is_empty() {
            return Err(EngineError::InvalidPlan("list at least one output node".into()));
        }
        for o in &self.outputs {
            if !ids.contains(o.as_str()) {
                return Err(EngineError::InvalidPlan(format!("output `{o}` is not a node")));
            }
        }
        for c in &self.checks {
            if !self.outputs.contains(&c.node) {
                return Err(EngineError::InvalidPlan(format!("check on `{}`: checks apply to output nodes", c.node)));
            }
        }
        if self.options.memory_limit_mb == 0 || self.options.memory_limit_mb > 16 * 1024 {
            return Err(EngineError::InvalidPlan("memory_limit_mb must be within 1..=16384".into()));
        }
        if self.options.max_output_rows == 0 || self.options.max_output_rows > 100_000 {
            return Err(EngineError::InvalidPlan("max_output_rows must be within 1..=100000".into()));
        }
        self.topo_order().map(|_| ())
    }

    /// Kahn's algorithm, ties broken by declaration order: deterministic.
    pub fn topo_order(&self) -> Result<Vec<String>> {
        let index: BTreeMap<&str, usize> = self.nodes.iter().enumerate().map(|(i, n)| (n.id.as_str(), i)).collect();
        let mut indegree = vec![0usize; self.nodes.len()];
        let mut consumers: Vec<Vec<usize>> = vec![Vec::new(); self.nodes.len()];
        for (i, n) in self.nodes.iter().enumerate() {
            for input in n.op.inputs() {
                let j = index[input];
                indegree[i] += 1;
                consumers[j].push(i);
            }
        }
        let mut ready: BTreeSet<usize> = indegree.iter().enumerate().filter(|(_, d)| **d == 0).map(|(i, _)| i).collect();
        let mut order = Vec::with_capacity(self.nodes.len());
        while let Some(i) = ready.pop_first() {
            order.push(self.nodes[i].id.clone());
            for &c in &consumers[i] {
                indegree[c] -= 1;
                if indegree[c] == 0 {
                    ready.insert(c);
                }
            }
        }
        if order.len() != self.nodes.len() {
            let stuck: Vec<&str> = self.nodes.iter().filter(|n| !order.contains(&n.id)).map(|n| n.id.as_str()).collect();
            return Err(EngineError::InvalidPlan(format!("the workflow has a cycle through {}", stuck.join(", "))));
        }
        Ok(order)
    }

    /// Nodes grouped into levels: every node depends only on earlier levels,
    /// so the nodes of one level can run concurrently.
    pub fn levels(&self) -> Result<Vec<Vec<String>>> {
        let order = self.topo_order()?;
        let mut level: BTreeMap<String, usize> = BTreeMap::new();
        let mut out: Vec<Vec<String>> = Vec::new();
        for id in order {
            let node = self.node(&id).ok_or_else(|| EngineError::Internal(format!("missing node {id}")))?;
            let l = node.op.inputs().iter().map(|i| level[*i] + 1).max().unwrap_or(0);
            level.insert(id.clone(), l);
            if out.len() <= l {
                out.resize(l + 1, Vec::new());
            }
            out[l].push(id);
        }
        Ok(out)
    }

    /// Number of nodes that read each node.
    pub fn consumer_counts(&self) -> BTreeMap<String, usize> {
        let mut m: BTreeMap<String, usize> = self.nodes.iter().map(|n| (n.id.clone(), 0)).collect();
        for n in &self.nodes {
            for i in n.op.inputs() {
                *m.entry(i.to_string()).or_default() += 1;
            }
        }
        m
    }

    /// Infers every node's output schema (types and units), returning notes
    /// for anything the type system could not track.
    pub fn infer_schemas(&self, source: &dyn DataSource) -> Result<(BTreeMap<String, Schema>, Vec<String>)> {
        let mut schemas: BTreeMap<String, Schema> = BTreeMap::new();
        let mut notes = Vec::new();
        for id in self.topo_order()? {
            let node = self.node(&id).ok_or_else(|| EngineError::Internal(format!("missing node {id}")))?;
            let at = |e: EngineError| e.at(&id);
            let schema = infer_node(&node.op, &schemas, source, &mut notes).map_err(at)?;
            schemas.insert(id, schema);
        }
        Ok((schemas, notes))
    }
}

fn infer_node(op: &Op, schemas: &BTreeMap<String, Schema>, source: &dyn DataSource, notes: &mut Vec<String>) -> Result<Schema> {
    let input = |id: &String| schemas.get(id).cloned().ok_or_else(|| EngineError::Internal(format!("schema of {id} not inferred yet")));
    match op {
        Op::Scan { domain, entity, columns, filter } => {
            let full = source.schema(domain, entity)?;
            if let Some(f) = filter {
                predicate(f, &full, notes)?;
            }
            match columns {
                None => Ok(full),
                Some(cols) => Schema::new(cols.iter().map(|c| full.field(c).cloned()).collect::<Result<_>>()?),
            }
        }
        Op::Values { columns } => {
            if columns.is_empty() {
                return Err(EngineError::InvalidPlan("values needs at least one column".into()));
            }
            let rows = columns[0].values.len();
            let mut fields = Vec::new();
            for c in columns {
                if c.values.len() != rows {
                    return Err(EngineError::InvalidPlan(format!("values column `{}` has {} rows, expected {rows}", c.name, c.values.len())));
                }
                if let Some(u) = &c.unit {
                    dimension(u)?;
                }
                fields.push(Field { name: c.name.clone(), dtype: value_type(&c.name, &c.values)?, unit: c.unit.clone() });
            }
            Schema::new(fields)
        }
        Op::Filter { input: i, predicate: p } => {
            let s = input(i)?;
            predicate(p, &s, notes)?;
            Ok(s)
        }
        Op::Project { input: i, columns } => {
            let s = input(i)?;
            if columns.is_empty() {
                return Err(EngineError::InvalidPlan("project needs at least one column".into()));
            }
            let fields = columns
                .iter()
                .map(|c| {
                    let t = c.expr.infer(&s, notes)?;
                    Ok(Field { name: c.name.clone(), dtype: t.dtype, unit: t.unit })
                })
                .collect::<Result<_>>()?;
            Schema::new(fields)
        }
        Op::Aggregate { input: i, group_by, aggregates } => {
            let s = input(i)?;
            if aggregates.is_empty() {
                return Err(EngineError::InvalidPlan("aggregate needs at least one aggregate".into()));
            }
            let mut fields: Vec<Field> = group_by.iter().map(|g| s.field(g).cloned()).collect::<Result<_>>()?;
            for a in aggregates {
                let f = s.field(&a.column)?;
                if a.func != AggFunc::Count && !f.dtype.is_numeric() {
                    return Err(EngineError::Type(format!("{:?} of `{}` needs a numeric column", a.func, a.column)));
                }
                let (dtype, unit) = match a.func {
                    AggFunc::Count => (DataType::Int64, None),
                    _ => (DataType::Float64, f.unit.clone()),
                };
                fields.push(Field { name: a.name.clone(), dtype, unit });
            }
            Schema::new(fields)
        }
        Op::Join { left, right, on, .. } => {
            let (l, r) = (input(left)?, input(right)?);
            if on.is_empty() {
                return Err(EngineError::InvalidPlan("join needs at least one key column".into()));
            }
            for k in on {
                let (a, b) = (l.field(k)?, r.field(k)?);
                if a.dtype != b.dtype && !(a.dtype.is_numeric() && b.dtype.is_numeric()) {
                    return Err(EngineError::Type(format!("join key `{k}` is {:?} on the left and {:?} on the right", a.dtype, b.dtype)));
                }
                if a.unit != b.unit {
                    return Err(EngineError::Unit(format!("join key `{k}` has unit {:?} on the left and {:?} on the right", a.unit, b.unit)));
                }
            }
            let mut fields = l.fields.clone();
            for f in r.fields.iter().filter(|f| !on.contains(&f.name)) {
                let mut f = f.clone();
                if l.index_of(&f.name).is_some() {
                    f.name = format!("{}_right", f.name);
                }
                fields.push(f);
            }
            Schema::new(fields)
        }
        Op::Sort { input: i, by } => {
            let s = input(i)?;
            for k in by {
                s.field(&k.column)?;
            }
            Ok(s)
        }
        Op::Limit { input: i, .. } => input(i),
        Op::Kernel { input: i, kernel } => {
            let s = input(i)?;
            let num = |c: &str| -> Result<Field> {
                let f = s.field(c)?.clone();
                if f.dtype.is_numeric() { Ok(f) } else { Err(EngineError::Type(format!("kernel input `{c}` must be numeric"))) }
            };
            match kernel {
                Kernel::Ndvi { red, nir, output, .. } => {
                    let (r, n) = (num(red)?, num(nir)?);
                    if r.unit != n.unit {
                        return Err(EngineError::Unit(format!("NDVI bands must share a unit (reflectance); got {:?} and {:?}", r.unit, n.unit)));
                    }
                    let mut fields = s.fields.clone();
                    fields.push(Field::new(output.clone(), DataType::Float64, Some("1")));
                    Schema::new(fields)
                }
                Kernel::MovingMean { column, window, output } => {
                    let f = num(column)?;
                    if *window == 0 {
                        return Err(EngineError::InvalidPlan("moving_mean window must be at least 1".into()));
                    }
                    let mut fields = s.fields.clone();
                    fields.push(Field { name: output.clone(), dtype: DataType::Float64, unit: f.unit });
                    Schema::new(fields)
                }
                Kernel::FftPower { column } => {
                    let f = num(column)?;
                    let unit = f.unit.map(|u| format!("({u})^2"));
                    if unit.is_some() {
                        notes.push(format!("power spectrum of `{column}` is in squared input units; not a catalogued unit"));
                    }
                    Schema::new(vec![Field::new("bin", DataType::Int64, None), Field { name: "power".into(), dtype: DataType::Float64, unit }])
                }
            }
        }
    }
}

fn predicate(p: &Expr, s: &Schema, notes: &mut Vec<String>) -> Result<()> {
    let t = p.infer(s, notes)?;
    if t.dtype != DataType::Boolean {
        return Err(EngineError::Type(format!("a predicate must be boolean, got {:?}", t.dtype)));
    }
    Ok(())
}

fn value_type(name: &str, values: &[Literal]) -> Result<DataType> {
    let mut ty: Option<DataType> = None;
    for v in values {
        let t = match v {
            Literal::Null(()) => continue,
            Literal::Bool(_) => DataType::Boolean,
            Literal::Number(_) => DataType::Float64,
            Literal::Text(_) => DataType::Utf8,
        };
        match ty {
            None => ty = Some(t),
            Some(prev) if prev != t => return Err(EngineError::Type(format!("values column `{name}` mixes {prev:?} and {t:?}"))),
            _ => {}
        }
    }
    Ok(ty.unwrap_or(DataType::Float64))
}

/// Ids reachable backwards from `roots`.
pub fn reachable(wf: &Workflow, roots: &[String]) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    let mut queue: VecDeque<String> = roots.iter().cloned().collect();
    while let Some(id) = queue.pop_front() {
        if !seen.insert(id.clone()) {
            continue;
        }
        if let Some(n) = wf.node(&id) {
            queue.extend(n.op.inputs().into_iter().map(str::to_owned));
        }
    }
    seen
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn wf(json: &str) -> Workflow {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn structure_is_validated() {
        let bad = |j: &str| wf(j).validate().unwrap_err().to_string();
        assert!(bad(r#"{"nodes": [], "outputs": []}"#).contains("at least one node"));
        assert!(bad(r#"{"nodes": [{"id": "a b", "op": "limit", "input": "a", "n": 1}], "outputs": ["a b"]}"#).contains("[A-Za-z0-9_-]"));
        assert!(bad(r#"{"nodes": [{"id": "a", "op": "limit", "input": "zzz", "n": 1}], "outputs": ["a"]}"#).contains("unknown node"));
        assert!(
            bad(r#"{"nodes": [{"id": "a", "op": "limit", "input": "b", "n": 1}, {"id": "b", "op": "limit", "input": "a", "n": 1}], "outputs": ["a"]}"#)
                .contains("cycle")
        );
        assert!(bad(r#"{"nodes": [{"id": "a", "op": "values", "columns": []}], "outputs": ["x"]}"#).contains("not a node"));
        assert!(bad(r#"{"nodes": [{"id": "a", "op": "values", "columns": []}], "outputs": ["a"], "options": {"memory_limit_mb": 0}}"#).contains("memory_limit_mb"));
    }

    #[test]
    fn order_and_levels_are_deterministic() {
        let w = wf(r#"{"nodes": [
            {"id": "j", "op": "join", "left": "a", "right": "b", "on": ["k"]},
            {"id": "b", "op": "values", "columns": [{"name": "k", "values": [1]}]},
            {"id": "a", "op": "values", "columns": [{"name": "k", "values": [1]}]}
        ], "outputs": ["j"]}"#);
        assert_eq!(w.topo_order().unwrap(), vec!["b", "a", "j"]);
        assert_eq!(w.levels().unwrap(), vec![vec!["b".to_string(), "a".to_string()], vec!["j".to_string()]]);
        assert_eq!(w.consumer_counts()["a"], 1);
    }
}
