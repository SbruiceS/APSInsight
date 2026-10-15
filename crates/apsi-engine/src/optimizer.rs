//! Rule-based plan optimizer. Pure: takes a workflow, returns a new one plus a
//! log of every rewrite, so users can see why the executed plan differs from
//! what they wrote. Results are reported under the original output ids.

use crate::expr::{Expr, Literal};
use crate::plan::{Kernel, Op, Workflow, reachable};
use crate::source::DataSource;
use crate::{EngineError, Result};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Rewrite {
    pub rule: &'static str,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Optimized {
    pub workflow: Workflow,
    /// Original output id -> node that now produces it.
    pub outputs: BTreeMap<String, String>,
    pub rewrites: Vec<Rewrite>,
}

struct Ctx {
    wf: Workflow,
    alias: BTreeMap<String, String>,
    log: Vec<Rewrite>,
}

impl Ctx {
    fn note(&mut self, rule: &'static str, detail: String) {
        self.log.push(Rewrite { rule, detail });
    }

    fn consumers(&self, id: &str) -> usize {
        self.wf.nodes.iter().map(|n| n.op.inputs().iter().filter(|i| **i == id).count()).sum::<usize>() + usize::from(self.wf.outputs.iter().any(|o| o == id))
    }

    fn is_output(&self, id: &str) -> bool {
        self.wf.outputs.iter().any(|o| o == id)
    }

    /// Points every reference to `from` at `to` and drops `from`.
    fn replace(&mut self, from: &str, to: &str) {
        for n in &mut self.wf.nodes {
            for i in n.op.inputs_mut() {
                if i == from {
                    *i = to.to_string();
                }
            }
        }
        for o in &mut self.wf.outputs {
            if o == from {
                *o = to.to_string();
            }
        }
        for v in self.alias.values_mut() {
            if v == from {
                *v = to.to_string();
            }
        }
        self.wf.nodes.retain(|n| n.id != from);
    }

    fn op(&self, id: &str) -> Option<&Op> {
        self.wf.node(id).map(|n| &n.op)
    }

    fn op_mut(&mut self, id: &str) -> Option<&mut Op> {
        self.wf.nodes.iter_mut().find(|n| n.id == id).map(|n| &mut n.op)
    }
}

pub fn optimize(wf: &Workflow, source: &dyn DataSource) -> Result<Optimized> {
    wf.validate()?;
    let mut cx = Ctx { alias: wf.outputs.iter().map(|o| (o.clone(), o.clone())).collect(), wf: wf.clone(), log: Vec::new() };
    fold_constants(&mut cx);
    eliminate_common_subplans(&mut cx)?;
    for _ in 0..64 {
        let before = cx.log.len();
        remove_true_filters(&mut cx);
        merge_filters(&mut cx);
        push_filters_through_projects(&mut cx);
        push_filters_into_scans(&mut cx);
        if cx.log.len() == before {
            break;
        }
    }
    eliminate_dead_nodes(&mut cx);
    prune_scan_columns(&mut cx, source)?;
    let mut outputs: Vec<String> = Vec::new();
    for o in &cx.wf.outputs {
        if !outputs.contains(o) {
            outputs.push(o.clone());
        }
    }
    cx.wf.outputs = outputs;
    cx.wf.validate().map_err(|e| EngineError::Internal(format!("optimizer produced an invalid plan: {e}")))?;
    Ok(Optimized { workflow: cx.wf, outputs: cx.alias, rewrites: cx.log })
}

fn fold_constants(cx: &mut Ctx) {
    let mut changed = Vec::new();
    for n in &mut cx.wf.nodes {
        let mut touched = false;
        let mut fold = |e: &mut Expr| {
            let f = e.fold();
            if f != *e {
                *e = f;
                touched = true;
            }
        };
        match &mut n.op {
            Op::Filter { predicate, .. } => fold(predicate),
            Op::Scan { filter: Some(f), .. } => fold(f),
            Op::Project { columns, .. } => columns.iter_mut().for_each(|c| fold(&mut c.expr)),
            _ => {}
        }
        if touched {
            changed.push(n.id.clone());
        }
    }
    for id in changed {
        cx.note("constant_folding", format!("evaluated constant sub-expressions in `{id}`"));
    }
}

/// Identical operations over identical inputs are computed once.
fn eliminate_common_subplans(cx: &mut Ctx) -> Result<()> {
    let mut seen: BTreeMap<String, String> = BTreeMap::new();
    for id in cx.wf.topo_order()? {
        let Some(op) = cx.op(&id) else { continue };
        let key = serde_json::to_string(op).map_err(|e| EngineError::Internal(e.to_string()))?;
        match seen.get(&key) {
            Some(first) => {
                let first = first.clone();
                cx.replace(&id, &first);
                cx.note("common_subplan_elimination", format!("`{id}` is identical to `{first}` and reuses its result"));
            }
            None => {
                seen.insert(key, id);
            }
        }
    }
    Ok(())
}

fn remove_true_filters(cx: &mut Ctx) {
    let trivial: Vec<(String, String)> = cx
        .wf
        .nodes
        .iter()
        .filter_map(|n| match &n.op {
            Op::Filter { input, predicate: Expr::Lit(Literal::Bool(true)) } => Some((n.id.clone(), input.clone())),
            _ => None,
        })
        .collect();
    for (id, input) in trivial {
        cx.replace(&id, &input);
        cx.note("remove_trivial_filter", format!("`{id}` keeps every row and was removed"));
    }
}

/// filter(filter(x, a), b) -> filter(x, a and b) when the inner filter has no other reader.
fn merge_filters(cx: &mut Ctx) {
    loop {
        let candidate = cx.wf.nodes.iter().find_map(|n| match &n.op {
            Op::Filter { input, predicate } => match cx.op(input) {
                Some(Op::Filter { input: inner_input, predicate: inner }) if cx.consumers(input) == 1 && !cx.is_output(input) => {
                    Some((n.id.clone(), input.clone(), inner_input.clone(), Expr::and(inner.clone(), predicate.clone())))
                }
                _ => None,
            },
            _ => None,
        });
        let Some((outer, inner, inner_input, merged)) = candidate else { break };
        cx.wf.nodes.retain(|n| n.id != inner);
        if let Some(Op::Filter { input, predicate }) = cx.op_mut(&outer) {
            *input = inner_input;
            *predicate = merged;
        }
        cx.note("merge_filters", format!("merged `{inner}` into `{outer}`"));
    }
}

/// filter(project(x), p) -> project(filter(x, p')) when every column `p`
/// reads is passed through unchanged by the projection.
fn push_filters_through_projects(cx: &mut Ctx) {
    loop {
        let candidate = cx.wf.nodes.iter().find_map(|n| {
            let Op::Filter { input: pid, predicate } = &n.op else { return None };
            let Some(Op::Project { input: below, columns }) = cx.op(pid) else { return None };
            if cx.consumers(pid) != 1 || cx.is_output(pid) {
                return None;
            }
            let mut used = BTreeSet::new();
            predicate.columns(&mut used);
            let mut map = BTreeMap::new();
            for c in &used {
                match columns.iter().find(|nc| &nc.name == c).map(|nc| &nc.expr) {
                    Some(Expr::Col(src)) => {
                        map.insert(c.clone(), src.clone());
                    }
                    _ => return None,
                }
            }
            let renamed = predicate.rename(&|c: &str| map.get(c).cloned().unwrap_or_else(|| c.to_string()));
            Some((n.id.clone(), pid.clone(), below.clone(), columns.clone(), renamed))
        });
        let Some((fid, pid, below, columns, renamed)) = candidate else { break };
        // Node ids keep their positions: `fid` (read by others) becomes the
        // projection, `pid` becomes the filter beneath it.
        if let Some(op) = cx.op_mut(&pid) {
            *op = Op::Filter { input: below, predicate: renamed };
        }
        if let Some(op) = cx.op_mut(&fid) {
            *op = Op::Project { input: pid.clone(), columns };
        }
        cx.note("predicate_pushdown", format!("moved the predicate of `{fid}` below projection `{pid}`"));
    }
}

/// filter(scan) -> scan with a pushed-down filter, evaluated while scanning.
fn push_filters_into_scans(cx: &mut Ctx) {
    loop {
        let candidate = cx.wf.nodes.iter().find_map(|n| {
            let Op::Filter { input: sid, predicate } = &n.op else { return None };
            match cx.op(sid) {
                Some(Op::Scan { domain, entity, columns, filter }) if cx.consumers(sid) == 1 && !cx.is_output(sid) => {
                    let f = match filter {
                        Some(f) => Expr::and(f.clone(), predicate.clone()),
                        None => predicate.clone(),
                    };
                    Some((n.id.clone(), sid.clone(), Op::Scan { domain: domain.clone(), entity: entity.clone(), columns: columns.clone(), filter: Some(f) }))
                }
                _ => None,
            }
        });
        let Some((fid, sid, scan)) = candidate else { break };
        cx.wf.nodes.retain(|n| n.id != sid);
        if let Some(op) = cx.op_mut(&fid) {
            *op = scan;
        }
        cx.note("predicate_pushdown", format!("pushed the predicate of `{fid}` into scan `{sid}`"));
    }
}

fn eliminate_dead_nodes(cx: &mut Ctx) {
    let live = reachable(&cx.wf, &cx.wf.outputs);
    let dead: Vec<String> = cx.wf.nodes.iter().filter(|n| !live.contains(&n.id)).map(|n| n.id.clone()).collect();
    if !dead.is_empty() {
        cx.wf.nodes.retain(|n| live.contains(&n.id));
        cx.note("dead_node_elimination", format!("removed nodes no output depends on: {}", dead.join(", ")));
    }
}

/// Columns each node must deliver; `None` means all of them.
type Needs = BTreeMap<String, Option<BTreeSet<String>>>;

fn merge(acc: &mut Option<Option<BTreeSet<String>>>, add: Option<BTreeSet<String>>) {
    *acc = Some(match (acc.take(), add) {
        (None, a) => a,
        (Some(None), _) | (Some(_), None) => None,
        (Some(Some(mut a)), Some(b)) => {
            a.extend(b);
            Some(a)
        }
    });
}

fn with(base: &Option<BTreeSet<String>>, extra: impl IntoIterator<Item = String>) -> Option<BTreeSet<String>> {
    base.clone().map(|mut s| {
        s.extend(extra);
        s
    })
}

/// Projection pushdown: each scan reads only the columns something downstream uses.
fn prune_scan_columns(cx: &mut Ctx, source: &dyn DataSource) -> Result<()> {
    let mut order = cx.wf.topo_order()?;
    order.reverse();
    let mut acc: BTreeMap<String, Option<Option<BTreeSet<String>>>> = BTreeMap::new();
    for o in &cx.wf.outputs {
        acc.insert(o.clone(), Some(None));
    }
    let mut needs: Needs = BTreeMap::new();
    for id in &order {
        let need = acc.remove(id).flatten().unwrap_or_else(|| Some(BTreeSet::new()));
        needs.insert(id.clone(), need.clone());
        let Some(op) = cx.op(id) else { continue };
        let mut push = |input: &str, n: Option<BTreeSet<String>>| merge(acc.entry(input.to_string()).or_insert(None), n);
        match op {
            Op::Scan { .. } | Op::Values { .. } => {}
            Op::Filter { input, predicate } => {
                let mut c = BTreeSet::new();
                predicate.columns(&mut c);
                push(input, with(&need, c));
            }
            Op::Project { input, columns } => {
                let mut c = BTreeSet::new();
                for nc in columns {
                    nc.expr.columns(&mut c);
                }
                push(input, Some(c));
            }
            Op::Aggregate { input, group_by, aggregates } => {
                let c: BTreeSet<String> = group_by.iter().cloned().chain(aggregates.iter().map(|a| a.column.clone())).collect();
                push(input, Some(c));
            }
            Op::Join { left, right, on, .. } => {
                // Right-side collisions are renamed `<name>_right`; ask both
                // sides for the base name too.
                let base = need.clone().map(|s| s.into_iter().map(|c| c.strip_suffix("_right").map(str::to_owned).unwrap_or(c)).collect::<BTreeSet<_>>());
                push(left, with(&base, on.iter().cloned()));
                push(right, with(&base, on.iter().cloned()));
            }
            Op::Sort { input, by } => push(input, with(&need, by.iter().map(|k| k.column.clone()))),
            Op::Limit { input, .. } => push(input, need.clone()),
            Op::Kernel { input, kernel } => match kernel {
                Kernel::Ndvi { red, nir, output, .. } => {
                    let n = need.clone().map(|mut s| {
                        s.remove(output);
                        s
                    });
                    push(input, with(&n, [red.clone(), nir.clone()]));
                }
                Kernel::MovingMean { column, output, .. } => {
                    let n = need.clone().map(|mut s| {
                        s.remove(output);
                        s
                    });
                    push(input, with(&n, [column.clone()]));
                }
                Kernel::FftPower { column } => push(input, Some([column.clone()].into())),
            },
        }
    }
    let scans: Vec<String> = cx.wf.nodes.iter().filter(|n| matches!(n.op, Op::Scan { .. })).map(|n| n.id.clone()).collect();
    for id in scans {
        let Some(Some(need)) = needs.get(&id).cloned() else { continue };
        let Some(Op::Scan { domain, entity, columns, filter }) = cx.op(&id).cloned() else { continue };
        let full = source.schema(&domain, &entity).map_err(|e| e.at(&id))?;
        let mut keep = need;
        if let Some(f) = &filter {
            f.columns(&mut keep);
        }
        let current: Vec<String> = columns.clone().unwrap_or_else(|| full.names());
        let pruned: Vec<String> = current.iter().filter(|c| keep.contains(*c)).cloned().collect();
        if pruned.len() < current.len() && !pruned.is_empty() {
            let dropped = current.len() - pruned.len();
            if let Some(Op::Scan { columns, .. }) = cx.op_mut(&id) {
                *columns = Some(pruned.clone());
            }
            cx.note("projection_pushdown", format!("scan `{id}` reads {} of {} columns ({dropped} unused)", pruned.len(), current.len()));
        }
    }
    Ok(())
}
