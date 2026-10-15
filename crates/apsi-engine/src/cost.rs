//! Cost model: estimates rows, bytes and work per node, and picks a backend
//! for each kernel. Estimates are explicit heuristics (documented below), shown
//! to the user next to the measured profile so they can be checked.
//!
//! * A filter keeps 1/3 of rows per range conjunct and 1/10 per equality.
//! * A grouped aggregate yields about sqrt(rows) groups.
//! * Moving rows to an accelerator costs bytes / 12 GB/s each way; it is only
//!   worth it when the kernel is large (>= 1M rows) and an accelerator exists.
//! * Native C++ kernels win over the CPU path from 4096 rows, where call
//!   overhead stops mattering.

use crate::backend::{BackendKind, Backends};
use crate::expr::Expr;
use crate::plan::{BackendChoice, Op, Workflow};
use crate::source::DataSource;
use crate::types::Schema;
use crate::Result;
use serde::Serialize;
use std::collections::BTreeMap;

pub const NATIVE_MIN_ROWS: u64 = 4096;
pub const ACCELERATOR_MIN_ROWS: u64 = 1_000_000;
const TRANSFER_BYTES_PER_SEC: f64 = 12e9;
const CPU_OPS_PER_SEC: f64 = 2e9;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NodeCost {
    pub id: String,
    pub op: &'static str,
    pub rows: u64,
    pub bytes: u64,
    /// Abstract work units (≈ value-operations).
    pub work: u64,
    pub backend: BackendKind,
    pub reason: String,
}

fn selectivity(e: &Expr) -> f64 {
    e.conjuncts()
        .iter()
        .map(|c| match c {
            Expr::Eq(..) => 0.1,
            Expr::Ne(..) => 0.9,
            Expr::Lit(crate::expr::Literal::Bool(true)) => 1.0,
            _ => 1.0 / 3.0,
        })
        .product()
}

fn row_bytes(s: &Schema) -> u64 {
    s.fields.iter().map(|f| f.dtype.width()).sum::<u64>().max(1)
}

pub fn estimate(wf: &Workflow, schemas: &BTreeMap<String, Schema>, source: &dyn DataSource, backends: &Backends) -> Result<Vec<NodeCost>> {
    let mut rows: BTreeMap<String, u64> = BTreeMap::new();
    let mut out = Vec::new();
    for id in wf.topo_order()? {
        let Some(node) = wf.node(&id) else { continue };
        let r = |i: &str| -> u64 { rows.get(i).copied().unwrap_or(0) };
        let (n, work) = match &node.op {
            Op::Scan { domain, entity, filter, .. } => {
                let base = source.estimate_rows(domain, entity).map_err(|e| e.at(&id))?;
                let n = filter.as_ref().map_or(base, |f| (base as f64 * selectivity(f)).ceil() as u64);
                (n, base * filter.as_ref().map_or(1, Expr::weight))
            }
            Op::Values { columns } => (columns.first().map_or(0, |c| c.values.len() as u64), 0),
            Op::Filter { input, predicate } => ((r(input) as f64 * selectivity(predicate)).ceil() as u64, r(input) * predicate.weight()),
            Op::Project { input, columns } => (r(input), r(input) * columns.iter().map(|c| c.expr.weight()).sum::<u64>()),
            Op::Aggregate { input, group_by, aggregates } => {
                let n = if group_by.is_empty() { 1 } else { (r(input) as f64).sqrt().ceil() as u64 };
                (n, r(input) * (group_by.len() + aggregates.len()) as u64)
            }
            Op::Join { left, right, .. } => (r(left).max(r(right)), r(left) + r(right)),
            Op::Sort { input, .. } => {
                let n = r(input);
                (n, n * (n.max(2) as f64).log2().ceil() as u64)
            }
            Op::Limit { input, n } => (r(input).min(*n as u64), 0),
            Op::Kernel { input, kernel } => match kernel {
                crate::plan::Kernel::FftPower { .. } => {
                    let n = r(input);
                    (n / 2 + 1, n * (n.max(2) as f64).log2().ceil() as u64 * 5)
                }
                crate::plan::Kernel::MovingMean { window, .. } => (r(input), r(input) * *window as u64),
                crate::plan::Kernel::Ndvi { .. } => (r(input), r(input) * 4),
            },
        };
        let rows_in = r(node.op.inputs().first().copied().unwrap_or(""));
        rows.insert(id.clone(), n);
        let bytes = n * schemas.get(&id).map_or(16, row_bytes);
        let (backend, reason) = choose(&node.op, rows_in, bytes, work, wf.options.backend, backends);
        out.push(NodeCost { id, op: node.op.name(), rows: n, bytes, work, backend, reason });
    }
    Ok(out)
}

fn choose(op: &Op, rows_in: u64, bytes: u64, work: u64, pref: BackendChoice, b: &Backends) -> (BackendKind, String) {
    let Op::Kernel { kernel, .. } = op else {
        return (BackendKind::Cpu, "relational operator: Rust executor".into());
    };
    let accel_ok = matches!(kernel, crate::plan::Kernel::Ndvi { .. }) && b.accelerator_available();
    match pref {
        BackendChoice::Cpu => (BackendKind::Cpu, "requested cpu".into()),
        BackendChoice::Native => (BackendKind::Native, "requested native".into()),
        BackendChoice::Accelerator => (BackendKind::Accelerator, "requested accelerator".into()),
        BackendChoice::Auto => {
            let transfer = 2.0 * bytes as f64 / TRANSFER_BYTES_PER_SEC;
            let compute = work as f64 / CPU_OPS_PER_SEC;
            if accel_ok && rows_in >= ACCELERATOR_MIN_ROWS && transfer < compute {
                (BackendKind::Accelerator, format!("{rows_in} rows; transfer {transfer:.4}s < cpu {compute:.4}s"))
            } else if b.native_available() && rows_in >= NATIVE_MIN_ROWS {
                (BackendKind::Native, format!("{rows_in} rows >= {NATIVE_MIN_ROWS}: native C++ kernel"))
            } else if !b.native_available() {
                (BackendKind::Cpu, "native kernels not built".into())
            } else {
                (BackendKind::Cpu, format!("{rows_in} rows < {NATIVE_MIN_ROWS}: call overhead dominates"))
            }
        }
    }
}
