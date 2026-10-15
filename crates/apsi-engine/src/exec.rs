//! The dataflow runtime: executes an optimized plan level by level. Nodes in
//! one level run concurrently on Rayon; inside a node, rows are processed in
//! fixed [`CHUNK`]-sized pieces combined in order, so results are identical
//! for any thread count. Intermediate results are released as soon as their
//! last reader has run.

use crate::backend::{BackendKind, Backends, CHUNK, sum};
use crate::cost::NodeCost;
use crate::expr::{Expr, Literal};
use crate::optimizer::Optimized;
use crate::plan::{AggFunc, JoinKind, Kernel, Op, SortKey};
use crate::runtime::{CacheState, CheckpointStore, MemoryBudget, Reservation};
use crate::source::DataSource;
use crate::types::{Batch, Column, DataType, Field, Schema};
use crate::{EngineError, Result};
use parking_lot::Mutex;
use rayon::prelude::*;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NodeProfile {
    pub id: String,
    pub op: &'static str,
    pub backend: BackendKind,
    pub rows: usize,
    pub bytes: u64,
    pub elapsed_us: u64,
    pub cache: CacheState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CheckResult {
    pub output: String,
    pub column: String,
    pub rule: String,
    pub violations: usize,
    /// Row numbers (0-based) of the first violations.
    pub examples: Vec<usize>,
    pub passed: bool,
}

pub struct Execution {
    /// Keyed by the user's output ids.
    pub outputs: BTreeMap<String, Arc<Batch>>,
    pub profile: Vec<NodeProfile>,
    pub checks: Vec<CheckResult>,
    pub peak_bytes: u64,
    pub limit_bytes: u64,
    pub elapsed_us: u64,
}

struct Done {
    batch: Arc<Batch>,
    _reservation: Reservation,
    profile: NodeProfile,
}

pub fn execute(opt: &Optimized, costs: &[NodeCost], source: &dyn DataSource, backends: &Backends, checkpoints: Option<&CheckpointStore>) -> Result<Execution> {
    let started = Instant::now();
    let wf = &opt.workflow;
    let budget = MemoryBudget::new(wf.options.memory_limit_mb * 1024 * 1024);
    let backend_of: BTreeMap<&str, BackendKind> = costs.iter().map(|c| (c.id.as_str(), c.backend)).collect();
    let keys = if wf.options.checkpoint && checkpoints.is_some() { checkpoint_keys(opt, source)? } else { BTreeMap::new() };
    let mut remaining = wf.consumer_counts();
    let results: Mutex<BTreeMap<String, Done>> = Mutex::new(BTreeMap::new());
    let mut profile = Vec::new();

    for level in wf.levels()? {
        let done: Vec<Result<(String, Done)>> = level
            .par_iter()
            .map(|id| {
                let node = wf.node(id).ok_or_else(|| EngineError::Internal(format!("missing node {id}")))?;
                let inputs: Vec<Arc<Batch>> = {
                    let r = results.lock();
                    node.op.inputs().iter().map(|i| r.get(*i).map(|d| Arc::clone(&d.batch)).ok_or_else(|| EngineError::Internal(format!("input {i} not ready")))).collect::<Result<_>>()?
                };
                let t = Instant::now();
                let want = backend_of.get(id.as_str()).copied().unwrap_or(BackendKind::Cpu);
                let key = keys.get(id).filter(|_| eligible(&node.op));
                let (batch, used, note, cache) = match (key, checkpoints) {
                    (Some(k), Some(store)) => match store.get(k) {
                        Some(b) => (b, want, Some("reused checkpoint".to_string()), CacheState::Hit),
                        None => {
                            let (b, used, note) = run_node(&node.op, &inputs, source, backends, want).map_err(|e| e.at(id))?;
                            let b = Arc::new(b);
                            store.put(k.clone(), Arc::clone(&b));
                            (b, used, note, CacheState::Miss)
                        }
                    },
                    _ => {
                        let (b, used, note) = run_node(&node.op, &inputs, source, backends, want).map_err(|e| e.at(id))?;
                        (Arc::new(b), used, note, CacheState::Off)
                    }
                };
                let bytes = batch.bytes();
                let reservation = budget.reserve(bytes, id)?;
                let profile = NodeProfile { id: id.clone(), op: node.op.name(), backend: used, rows: batch.rows(), bytes, elapsed_us: t.elapsed().as_micros() as u64, cache, note };
                Ok((id.clone(), Done { batch, _reservation: reservation, profile }))
            })
            .collect();
        let mut finished = Vec::new();
        for d in done {
            let (id, d) = d?;
            profile.push(d.profile.clone());
            finished.push(id.clone());
            results.lock().insert(id, d);
        }
        // Release inputs whose readers have all run.
        for id in &finished {
            let Some(node) = wf.node(id) else { continue };
            for i in node.op.inputs() {
                if let Some(n) = remaining.get_mut(i) {
                    *n = n.saturating_sub(1);
                    if *n == 0 && !wf.outputs.iter().any(|o| o == i) {
                        results.lock().remove(i);
                    }
                }
            }
        }
    }

    let r = results.lock();
    let mut outputs = BTreeMap::new();
    for (user, node) in &opt.outputs {
        let d = r.get(node).ok_or_else(|| EngineError::Internal(format!("output {node} missing")))?;
        outputs.insert(user.clone(), Arc::clone(&d.batch));
    }
    drop(r);
    let checks = validate(wf, &outputs);
    Ok(Execution { outputs, profile, checks, peak_bytes: budget.peak(), limit_bytes: budget.limit(), elapsed_us: started.elapsed().as_micros() as u64 })
}

/// Sub-plans worth caching: more expensive than re-reading their inputs.
fn eligible(op: &Op) -> bool {
    matches!(op, Op::Aggregate { .. } | Op::Join { .. } | Op::Sort { .. } | Op::Kernel { .. })
}

/// Key = hash(operation, keys of its inputs); scans also hash the data
/// fingerprint, so a key changes whenever anything upstream changes.
fn checkpoint_keys(opt: &Optimized, source: &dyn DataSource) -> Result<BTreeMap<String, String>> {
    let mut keys: BTreeMap<String, String> = BTreeMap::new();
    for id in opt.workflow.topo_order()? {
        let Some(n) = opt.workflow.node(&id) else { continue };
        let mut h = Sha256::new();
        h.update(n.op.name());
        let mut op = n.op.clone();
        for i in op.inputs_mut() {
            *i = keys.get(i.as_str()).cloned().unwrap_or_default();
        }
        h.update(serde_json::to_vec(&op).map_err(|e| EngineError::Internal(e.to_string()))?);
        if let Op::Scan { domain, entity, .. } = &n.op {
            h.update(source.fingerprint(domain, entity).map_err(|e| e.at(&id))?);
        }
        keys.insert(id, hex::encode(h.finalize()));
    }
    Ok(keys)
}

fn run_node(op: &Op, inputs: &[Arc<Batch>], source: &dyn DataSource, backends: &Backends, want: BackendKind) -> Result<(Batch, BackendKind, Option<String>)> {
    let cpu = |b: Batch| Ok((b, BackendKind::Cpu, None));
    match op {
        Op::Scan { domain, entity, columns, filter } => {
            let b = source.scan(domain, entity, columns.as_deref())?;
            match filter {
                Some(f) => cpu(filter_batch(&b, f)?),
                None => cpu(b),
            }
        }
        Op::Values { columns } => {
            let mut fields = Vec::new();
            let mut cols = Vec::new();
            for c in columns {
                let ty = c.values.iter().find_map(|v| match v {
                    Literal::Bool(_) => Some(DataType::Boolean),
                    Literal::Number(_) => Some(DataType::Float64),
                    Literal::Text(_) => Some(DataType::Utf8),
                    Literal::Null(()) => None,
                });
                let ty = ty.unwrap_or(DataType::Float64);
                cols.push(match ty {
                    DataType::Boolean => Column::Boolean(c.values.iter().map(|v| if let Literal::Bool(b) = v { Some(*b) } else { None }).collect()),
                    DataType::Utf8 => Column::Utf8(c.values.iter().map(|v| if let Literal::Text(s) = v { Some(s.clone()) } else { None }).collect()),
                    _ => Column::Float64(c.values.iter().map(|v| if let Literal::Number(n) = v { Some(*n) } else { None }).collect()),
                });
                fields.push(Field { name: c.name.clone(), dtype: ty, unit: c.unit.clone() });
            }
            cpu(Batch::new(Schema::new(fields)?, cols)?)
        }
        Op::Filter { predicate, .. } => cpu(filter_batch(&inputs[0], predicate)?),
        Op::Project { columns, .. } => {
            let b = &inputs[0];
            let mut notes = Vec::new();
            let mut fields = Vec::new();
            let mut cols = Vec::new();
            for c in columns {
                let t = c.expr.infer(&b.schema, &mut notes)?;
                let col = par_eval(&c.expr, b)?;
                // Literal-only expressions evaluate as Float64; keep the inferred type.
                fields.push(Field { name: c.name.clone(), dtype: col.dtype(), unit: t.unit });
                cols.push(col);
            }
            cpu(Batch::new(Schema::new(fields)?, cols)?)
        }
        Op::Aggregate { group_by, aggregates, .. } => cpu(aggregate(&inputs[0], group_by, aggregates)?),
        Op::Join { on, kind, .. } => cpu(join(&inputs[0], &inputs[1], on, *kind)?),
        Op::Sort { by, .. } => cpu(sort(&inputs[0], by)?),
        Op::Limit { n, .. } => {
            let b = &inputs[0];
            cpu(b.slice(0, b.rows().min(*n)))
        }
        Op::Kernel { kernel, .. } => run_kernel(&inputs[0], kernel, backends, want),
    }
}

/// Chunked evaluation; chunks are concatenated in order (deterministic).
fn par_eval(e: &Expr, b: &Batch) -> Result<Column> {
    let n = b.rows();
    if n <= CHUNK {
        return e.eval(b);
    }
    let parts: Vec<Result<Column>> = (0..n.div_ceil(CHUNK)).into_par_iter().map(|c| e.eval(&b.slice(c * CHUNK, CHUNK.min(n - c * CHUNK)))).collect();
    let mut it = parts.into_iter();
    let mut out = it.next().ok_or_else(|| EngineError::Internal("no chunks".into()))??;
    for p in it {
        out.extend(p?)?;
    }
    Ok(out)
}

fn filter_batch(b: &Batch, predicate: &Expr) -> Result<Batch> {
    match par_eval(predicate, b)? {
        Column::Boolean(mask) => {
            let idx: Vec<usize> = mask.iter().enumerate().filter(|(_, m)| **m == Some(true)).map(|(i, _)| i).collect();
            Ok(b.take(&idx))
        }
        Column::Float64(v) if v.iter().all(Option::is_none) => Ok(b.take(&[])),
        other => Err(EngineError::Type(format!("a predicate must be boolean, got {:?}", other.dtype()))),
    }
}

/// Group/join key part with a total order (numbers by IEEE total order).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum KeyPart {
    Null,
    Bool(bool),
    Num(OrdF64),
    Str(String),
}

#[derive(Debug, Clone, Copy)]
struct OrdF64(f64);

impl PartialEq for OrdF64 {
    fn eq(&self, o: &Self) -> bool {
        self.0.total_cmp(&o.0) == Ordering::Equal
    }
}
impl Eq for OrdF64 {}
impl PartialOrd for OrdF64 {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for OrdF64 {
    fn cmp(&self, o: &Self) -> Ordering {
        self.0.total_cmp(&o.0)
    }
}
impl std::hash::Hash for OrdF64 {
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        // -0.0 and 0.0 compare unequal under total order, matching this hash.
        self.0.to_bits().hash(h);
    }
}

fn key_part(c: &Column, i: usize) -> KeyPart {
    match c {
        Column::Float64(v) => v[i].map_or(KeyPart::Null, |x| KeyPart::Num(OrdF64(x))),
        Column::Int64(v) => v[i].map_or(KeyPart::Null, |x| KeyPart::Num(OrdF64(x as f64))),
        Column::Utf8(v) => v[i].clone().map_or(KeyPart::Null, KeyPart::Str),
        Column::Boolean(v) => v[i].map_or(KeyPart::Null, KeyPart::Bool),
    }
}

fn aggregate(b: &Batch, group_by: &[String], aggs: &[crate::plan::Aggregate]) -> Result<Batch> {
    let keys: Vec<&Column> = group_by.iter().map(|g| b.column(g)).collect::<Result<_>>()?;
    // BTreeMap: groups come out sorted by key, independent of input order.
    let mut groups: BTreeMap<Vec<KeyPart>, Vec<usize>> = BTreeMap::new();
    for i in 0..b.rows() {
        groups.entry(keys.iter().map(|c| key_part(c, i)).collect()).or_default().push(i);
    }
    if group_by.is_empty() && groups.is_empty() {
        groups.insert(Vec::new(), Vec::new());
    }
    let groups: Vec<(Vec<KeyPart>, Vec<usize>)> = groups.into_iter().collect();
    let firsts: Vec<usize> = groups.iter().map(|(_, rows)| rows.first().copied().unwrap_or(0)).collect();
    let mut fields: Vec<Field> = Vec::new();
    let mut cols: Vec<Column> = Vec::new();
    for (g, c) in group_by.iter().zip(&keys) {
        fields.push(b.schema.field(g)?.clone());
        cols.push(c.take(&firsts));
    }
    for a in aggs {
        let col = b.column(&a.column)?;
        let unit = b.schema.field(&a.column)?.unit.clone();
        if a.func == AggFunc::Count {
            let counts: Vec<Option<i64>> = groups.iter().map(|(_, rows)| Some(rows.iter().filter(|&&i| !col.is_null(i)).count() as i64)).collect();
            fields.push(Field::new(a.name.clone(), DataType::Int64, None));
            cols.push(Column::Int64(counts));
            continue;
        }
        let num = col.as_f64().ok_or_else(|| EngineError::Type(format!("`{}` is not numeric", a.column)))?;
        let values: Vec<Option<f64>> = groups
            .par_iter()
            .map(|(_, rows)| {
                let mut v: Vec<f64> = rows.iter().filter_map(|&i| num[i]).collect();
                reduce(a.func, &mut v)
            })
            .collect();
        fields.push(Field { name: a.name.clone(), dtype: DataType::Float64, unit });
        cols.push(Column::Float64(values));
    }
    Batch::new(Schema::new(fields)?, cols)
}

/// Pure reducers over the non-null values of one group. Empty input gives
/// `None` (no data), never 0.
fn reduce(func: AggFunc, v: &mut [f64]) -> Option<f64> {
    let n = v.len();
    match func {
        AggFunc::Count => Some(n as f64),
        AggFunc::Sum => (n > 0).then(|| sum(v)),
        AggFunc::Mean => (n > 0).then(|| sum(v) / n as f64),
        AggFunc::Min => v.iter().copied().reduce(f64::min),
        AggFunc::Max => v.iter().copied().reduce(f64::max),
        AggFunc::Sd => {
            if n < 2 {
                return None;
            }
            let mean = sum(v) / n as f64;
            let sq: Vec<f64> = v.iter().map(|x| (x - mean) * (x - mean)).collect();
            Some((sum(&sq) / (n - 1) as f64).sqrt())
        }
        AggFunc::Median => {
            if n == 0 {
                return None;
            }
            v.sort_by(f64::total_cmp);
            Some(if n % 2 == 1 { v[n / 2] } else { (v[n / 2 - 1] + v[n / 2]) / 2.0 })
        }
    }
}

fn join(l: &Batch, r: &Batch, on: &[String], kind: JoinKind) -> Result<Batch> {
    let lk: Vec<&Column> = on.iter().map(|k| l.column(k)).collect::<Result<_>>()?;
    let rk: Vec<&Column> = on.iter().map(|k| r.column(k)).collect::<Result<_>>()?;
    let mut table: HashMap<Vec<KeyPart>, Vec<usize>> = HashMap::new();
    for i in 0..r.rows() {
        let key: Vec<KeyPart> = rk.iter().map(|c| key_part(c, i)).collect();
        // SQL semantics: a null key never matches.
        if !key.contains(&KeyPart::Null) {
            table.entry(key).or_default().push(i);
        }
    }
    let mut li = Vec::new();
    let mut ri: Vec<Option<usize>> = Vec::new();
    for i in 0..l.rows() {
        let key: Vec<KeyPart> = lk.iter().map(|c| key_part(c, i)).collect();
        match table.get(&key).filter(|_| !key.contains(&KeyPart::Null)) {
            Some(matches) => {
                for &j in matches {
                    li.push(i);
                    ri.push(Some(j));
                }
            }
            None if kind == JoinKind::Left => {
                li.push(i);
                ri.push(None);
            }
            None => {}
        }
    }
    let mut fields = l.schema.fields.clone();
    let mut cols: Vec<Column> = l.columns.iter().map(|c| c.take(&li)).collect();
    for (f, c) in r.schema.fields.iter().zip(&r.columns) {
        if on.contains(&f.name) {
            continue;
        }
        let mut f = f.clone();
        if l.schema.index_of(&f.name).is_some() {
            f.name = format!("{}_right", f.name);
        }
        fields.push(f);
        cols.push(c.take_opt(&ri));
    }
    Batch::new(Schema::new(fields)?, cols)
}

fn sort(b: &Batch, by: &[SortKey]) -> Result<Batch> {
    let keys: Vec<(&Column, bool)> = by.iter().map(|k| Ok((b.column(&k.column)?, k.descending))).collect::<Result<_>>()?;
    let mut idx: Vec<usize> = (0..b.rows()).collect();
    // Stable; nulls sort last in either direction.
    idx.sort_by(|&x, &y| {
        for (c, desc) in &keys {
            let (a, z) = (key_part(c, x), key_part(c, y));
            let o = match (&a, &z) {
                (KeyPart::Null, KeyPart::Null) => Ordering::Equal,
                (KeyPart::Null, _) => Ordering::Greater,
                (_, KeyPart::Null) => Ordering::Less,
                _ if *desc => z.cmp(&a),
                _ => a.cmp(&z),
            };
            if o != Ordering::Equal {
                return o;
            }
        }
        Ordering::Equal
    });
    Ok(b.take(&idx))
}

fn nan_to_none(v: Vec<f64>) -> Vec<Option<f64>> {
    v.into_iter().map(|x| x.is_finite().then_some(x)).collect()
}

fn run_kernel(b: &Batch, kernel: &Kernel, backends: &Backends, want: BackendKind) -> Result<(Batch, BackendKind, Option<String>)> {
    let dense = |c: &str| -> Result<Vec<f64>> { Ok(b.column(c)?.as_f64().ok_or_else(|| EngineError::Type(format!("`{c}` is not numeric")))?.into_iter().map(|v| v.unwrap_or(f64::NAN)).collect()) };
    match kernel {
        Kernel::Ndvi { red, nir, nodata, output } => {
            let (values, used, note) = backends.ndvi(want, &dense(red)?, &dense(nir)?, *nodata)?;
            let values = nan_to_none(values);
            let undefined = values.iter().filter(|v| v.is_none()).count();
            let note = match (note, undefined) {
                (n, 0) => n,
                (Some(n), u) => Some(format!("{n}; {u} pixel(s) undefined (nodata, missing band or zero sum)")),
                (None, u) => Some(format!("{u} pixel(s) undefined (nodata, missing band or zero sum)")),
            };
            let mut out = b.clone();
            out.schema.fields.push(Field::new(output.clone(), DataType::Float64, Some("1")));
            out.columns.push(Column::Float64(values));
            Ok((Batch::new(out.schema, out.columns)?, used, note))
        }
        Kernel::MovingMean { column, window, output } => {
            let (values, used) = backends.moving_mean(want, &dense(column)?, *window)?;
            let unit = b.schema.field(column)?.unit.clone();
            let mut out = b.clone();
            out.schema.fields.push(Field { name: output.clone(), dtype: DataType::Float64, unit });
            out.columns.push(Column::Float64(nan_to_none(values)));
            Ok((Batch::new(out.schema, out.columns)?, used, Some(format!("first {} row(s) have no full window", window.saturating_sub(1).min(b.rows())))))
        }
        Kernel::FftPower { column } => {
            let col = b.column(column)?;
            if (0..col.len()).any(|i| col.is_null(i)) {
                return Err(EngineError::Kernel(format!("fft_power: `{column}` has missing values; fill or filter them first")));
            }
            let (power, used) = backends.fft_power(want, &dense(column)?)?;
            let unit = b.schema.field(column)?.unit.clone().map(|u| format!("({u})^2"));
            let bins = Column::Int64((0..power.len() as i64).map(Some).collect());
            let schema = Schema::new(vec![Field::new("bin", DataType::Int64, None), Field { name: "power".into(), dtype: DataType::Float64, unit }])?;
            Ok((Batch::new(schema, vec![bins, Column::Float64(power.into_iter().map(Some).collect())])?, used, None))
        }
    }
}

fn validate(wf: &crate::plan::Workflow, outputs: &BTreeMap<String, Arc<Batch>>) -> Vec<CheckResult> {
    let mut out = Vec::new();
    for c in &wf.checks {
        let Some(b) = outputs.get(&c.node) else { continue };
        let Ok(col) = b.column(&c.column) else {
            out.push(CheckResult { output: c.node.clone(), column: c.column.clone(), rule: "column exists".into(), violations: 1, examples: vec![], passed: false });
            continue;
        };
        let num = col.as_f64();
        let mut rule = Vec::new();
        if let Some(m) = c.min {
            rule.push(format!(">= {m}"));
        }
        if let Some(m) = c.max {
            rule.push(format!("<= {m}"));
        }
        if c.not_null {
            rule.push("not null".into());
        }
        let bad: Vec<usize> = (0..col.len())
            .filter(|&i| {
                let v = num.as_ref().and_then(|n| n[i]);
                (c.not_null && col.is_null(i)) || c.min.zip(v).is_some_and(|(m, v)| v < m) || c.max.zip(v).is_some_and(|(m, v)| v > m)
            })
            .collect();
        out.push(CheckResult { output: c.node.clone(), column: c.column.clone(), rule: rule.join(", "), violations: bad.len(), examples: bad.iter().take(5).copied().collect(), passed: bad.is_empty() });
    }
    // Always: computed numbers are finite (undefined values must be null).
    for (name, b) in outputs {
        for (f, c) in b.schema.fields.iter().zip(&b.columns) {
            if let Column::Float64(v) = c {
                let bad: Vec<usize> = v.iter().enumerate().filter(|(_, x)| x.is_some_and(|x| !x.is_finite())).map(|(i, _)| i).collect();
                out.push(CheckResult { output: name.clone(), column: f.name.clone(), rule: "finite".into(), violations: bad.len(), examples: bad.iter().take(5).copied().collect(), passed: bad.is_empty() });
            }
        }
    }
    out
}
