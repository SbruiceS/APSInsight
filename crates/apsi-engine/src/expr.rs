//! Expressions: a small, pure, typed language for filters and projections.
//!
//! JSON form, e.g. `{"gt": [{"col": "value"}, {"lit": 6.5}]}`. Units follow
//! the catalogue in `apsi_science::units`:
//!
//! * `+`, `-` and comparisons need operands in the *same* unit (or a bare
//!   literal, which takes the other side's unit). Same dimension but a
//!   different unit is an error that asks for an explicit `convert`.
//! * Two absolute temperatures cannot be added.
//! * `*` and `/` keep the unit when one side is a bare literal; otherwise the
//!   result unit is not tracked and the type checker reports a note.
//! * `ln`, `exp` and `sqrt` need a dimensionless (or unknown) input.
//!
//! Null semantics follow SQL: arithmetic and comparisons with a null give null,
//! `and`/`or` use three-valued logic, and a filter keeps only `true` rows.

use crate::types::{Batch, Column, DataType, Schema};
use crate::{EngineError, Result};
use apsi_science::units::{self, Dimension};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Literal {
    Bool(bool),
    Number(f64),
    Text(String),
    Null(()),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Expr {
    Col(String),
    Lit(Literal),
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    Eq(Box<Expr>, Box<Expr>),
    Ne(Box<Expr>, Box<Expr>),
    Lt(Box<Expr>, Box<Expr>),
    Le(Box<Expr>, Box<Expr>),
    Gt(Box<Expr>, Box<Expr>),
    Ge(Box<Expr>, Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Not(Box<Expr>),
    IsNull(Box<Expr>),
    Abs(Box<Expr>),
    Sqrt(Box<Expr>),
    Ln(Box<Expr>),
    Exp(Box<Expr>),
    /// Unit conversion, e.g. `{"convert": [{"col": "yield"}, "kg/ha"]}`.
    Convert(Box<Expr>, String),
}

/// Inferred type of an expression.
#[derive(Debug, Clone, PartialEq)]
pub struct ExprType {
    pub dtype: DataType,
    pub unit: Option<String>,
    /// True for bare literals, whose unit adapts to the other operand.
    pub literal: bool,
}

#[derive(Clone, Copy)]
enum Arith {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Clone, Copy)]
enum Cmp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Clone, Copy)]
enum Math {
    Abs,
    Sqrt,
    Ln,
    Exp,
}

impl Expr {
    pub fn col(name: &str) -> Self {
        Self::Col(name.into())
    }

    pub fn num(v: f64) -> Self {
        Self::Lit(Literal::Number(v))
    }

    pub fn and(a: Expr, b: Expr) -> Self {
        Self::And(Box::new(a), Box::new(b))
    }

    /// Every column the expression reads.
    pub fn columns(&self, out: &mut BTreeSet<String>) {
        match self {
            Self::Col(c) => {
                out.insert(c.clone());
            }
            Self::Lit(_) => {}
            Self::Add(a, b)
            | Self::Sub(a, b)
            | Self::Mul(a, b)
            | Self::Div(a, b)
            | Self::Eq(a, b)
            | Self::Ne(a, b)
            | Self::Lt(a, b)
            | Self::Le(a, b)
            | Self::Gt(a, b)
            | Self::Ge(a, b)
            | Self::And(a, b)
            | Self::Or(a, b) => {
                a.columns(out);
                b.columns(out);
            }
            Self::Not(a) | Self::IsNull(a) | Self::Abs(a) | Self::Sqrt(a) | Self::Ln(a) | Self::Exp(a) | Self::Convert(a, _) => a.columns(out),
        }
    }

    /// Number of operators; used by the cost model.
    pub fn weight(&self) -> u64 {
        match self {
            Self::Col(_) | Self::Lit(_) => 1,
            Self::Add(a, b)
            | Self::Sub(a, b)
            | Self::Mul(a, b)
            | Self::Div(a, b)
            | Self::Eq(a, b)
            | Self::Ne(a, b)
            | Self::Lt(a, b)
            | Self::Le(a, b)
            | Self::Gt(a, b)
            | Self::Ge(a, b)
            | Self::And(a, b)
            | Self::Or(a, b) => 1 + a.weight() + b.weight(),
            Self::Not(a) | Self::IsNull(a) | Self::Abs(a) | Self::Sqrt(a) | Self::Ln(a) | Self::Exp(a) | Self::Convert(a, _) => 1 + a.weight(),
        }
    }

    /// Renames columns (used when pushing a filter below a projection).
    pub fn rename(&self, f: &dyn Fn(&str) -> String) -> Expr {
        let b = |e: &Expr| Box::new(e.rename(f));
        match self {
            Self::Col(c) => Self::Col(f(c)),
            Self::Lit(l) => Self::Lit(l.clone()),
            Self::Add(x, y) => Self::Add(b(x), b(y)),
            Self::Sub(x, y) => Self::Sub(b(x), b(y)),
            Self::Mul(x, y) => Self::Mul(b(x), b(y)),
            Self::Div(x, y) => Self::Div(b(x), b(y)),
            Self::Eq(x, y) => Self::Eq(b(x), b(y)),
            Self::Ne(x, y) => Self::Ne(b(x), b(y)),
            Self::Lt(x, y) => Self::Lt(b(x), b(y)),
            Self::Le(x, y) => Self::Le(b(x), b(y)),
            Self::Gt(x, y) => Self::Gt(b(x), b(y)),
            Self::Ge(x, y) => Self::Ge(b(x), b(y)),
            Self::And(x, y) => Self::And(b(x), b(y)),
            Self::Or(x, y) => Self::Or(b(x), b(y)),
            Self::Not(x) => Self::Not(b(x)),
            Self::IsNull(x) => Self::IsNull(b(x)),
            Self::Abs(x) => Self::Abs(b(x)),
            Self::Sqrt(x) => Self::Sqrt(b(x)),
            Self::Ln(x) => Self::Ln(b(x)),
            Self::Exp(x) => Self::Exp(b(x)),
            Self::Convert(x, u) => Self::Convert(b(x), u.clone()),
        }
    }

    /// Splits a conjunction into its parts (for filter pushdown and costing).
    pub fn conjuncts(&self) -> Vec<&Expr> {
        match self {
            Self::And(a, b) => {
                let mut v = a.conjuncts();
                v.extend(b.conjuncts());
                v
            }
            other => vec![other],
        }
    }

    // ------------------------------------------------------------- typing

    pub fn infer(&self, schema: &Schema, notes: &mut Vec<String>) -> Result<ExprType> {
        let t = |dtype, unit: Option<String>| Ok(ExprType { dtype, unit, literal: false });
        match self {
            Self::Col(c) => {
                let f = schema.field(c)?;
                t(f.dtype, f.unit.clone())
            }
            Self::Lit(l) => Ok(ExprType {
                dtype: match l {
                    Literal::Bool(_) => DataType::Boolean,
                    Literal::Number(_) | Literal::Null(()) => DataType::Float64,
                    Literal::Text(_) => DataType::Utf8,
                },
                unit: None,
                literal: true,
            }),
            Self::Add(a, b) => arith_type(Arith::Add, a.infer(schema, notes)?, b.infer(schema, notes)?, notes),
            Self::Sub(a, b) => arith_type(Arith::Sub, a.infer(schema, notes)?, b.infer(schema, notes)?, notes),
            Self::Mul(a, b) => arith_type(Arith::Mul, a.infer(schema, notes)?, b.infer(schema, notes)?, notes),
            Self::Div(a, b) => arith_type(Arith::Div, a.infer(schema, notes)?, b.infer(schema, notes)?, notes),
            Self::Eq(a, b) | Self::Ne(a, b) | Self::Lt(a, b) | Self::Le(a, b) | Self::Gt(a, b) | Self::Ge(a, b) => {
                let (x, y) = (a.infer(schema, notes)?, b.infer(schema, notes)?);
                let ordered = matches!(self, Self::Lt(..) | Self::Le(..) | Self::Gt(..) | Self::Ge(..));
                if x.dtype.is_numeric() && y.dtype.is_numeric() {
                    same_unit("compare", &x, &y)?;
                } else if x.dtype != y.dtype && !(x.literal && matches!(**a, Self::Lit(Literal::Null(())))) && !(y.literal && matches!(**b, Self::Lit(Literal::Null(())))) {
                    return Err(EngineError::Type(format!("cannot compare {:?} with {:?}", x.dtype, y.dtype)));
                } else if ordered && x.dtype == DataType::Boolean {
                    return Err(EngineError::Type("booleans have no order; use eq/ne".into()));
                }
                t(DataType::Boolean, None)
            }
            Self::And(a, b) | Self::Or(a, b) => {
                for e in [a, b] {
                    let x = e.infer(schema, notes)?;
                    if x.dtype != DataType::Boolean {
                        return Err(EngineError::Type(format!("and/or need booleans, got {:?}", x.dtype)));
                    }
                }
                t(DataType::Boolean, None)
            }
            Self::Not(a) => {
                if a.infer(schema, notes)?.dtype != DataType::Boolean {
                    return Err(EngineError::Type("not needs a boolean".into()));
                }
                t(DataType::Boolean, None)
            }
            Self::IsNull(a) => {
                a.infer(schema, notes)?;
                t(DataType::Boolean, None)
            }
            Self::Abs(a) => {
                let x = numeric(a.infer(schema, notes)?, "abs")?;
                t(DataType::Float64, x.unit)
            }
            Self::Sqrt(a) | Self::Ln(a) | Self::Exp(a) => {
                let name = match self {
                    Self::Sqrt(_) => "sqrt",
                    Self::Ln(_) => "ln",
                    _ => "exp",
                };
                let x = numeric(a.infer(schema, notes)?, name)?;
                if let Some(u) = &x.unit {
                    if dimension(u)? != Dimension::NONE {
                        return Err(EngineError::Unit(format!("{name} needs a dimensionless input, got `{u}`")));
                    }
                }
                t(DataType::Float64, if name == "sqrt" { None } else { Some("1".into()) })
            }
            Self::Convert(a, to) => {
                let x = numeric(a.infer(schema, notes)?, "convert")?;
                let from = x.unit.ok_or_else(|| EngineError::Unit(format!("cannot convert to `{to}`: the input has no unit")))?;
                let (df, dt) = (dimension(&from)?, dimension(to)?);
                if df != dt {
                    return Err(EngineError::Unit(format!("cannot convert `{from}` ({df}) to `{to}` ({dt})")));
                }
                t(DataType::Float64, Some(to.clone()))
            }
        }
    }

    // ---------------------------------------------------------- folding

    /// Evaluates constant sub-expressions. Pure: returns a new expression.
    pub fn fold(&self) -> Expr {
        let bin = |a: &Expr, b: &Expr, mk: fn(Box<Expr>, Box<Expr>) -> Expr| -> Expr { mk(Box::new(a.fold()), Box::new(b.fold())) };
        let folded = match self {
            Self::Add(a, b) => bin(a, b, Self::Add),
            Self::Sub(a, b) => bin(a, b, Self::Sub),
            Self::Mul(a, b) => bin(a, b, Self::Mul),
            Self::Div(a, b) => bin(a, b, Self::Div),
            Self::Eq(a, b) => bin(a, b, Self::Eq),
            Self::Ne(a, b) => bin(a, b, Self::Ne),
            Self::Lt(a, b) => bin(a, b, Self::Lt),
            Self::Le(a, b) => bin(a, b, Self::Le),
            Self::Gt(a, b) => bin(a, b, Self::Gt),
            Self::Ge(a, b) => bin(a, b, Self::Ge),
            Self::And(a, b) => {
                let (a, b) = (a.fold(), b.fold());
                return match (&a, &b) {
                    (Self::Lit(Literal::Bool(true)), x) | (x, Self::Lit(Literal::Bool(true))) => x.clone(),
                    (Self::Lit(Literal::Bool(false)), _) | (_, Self::Lit(Literal::Bool(false))) => Self::Lit(Literal::Bool(false)),
                    _ => Self::And(Box::new(a), Box::new(b)),
                };
            }
            Self::Or(a, b) => {
                let (a, b) = (a.fold(), b.fold());
                return match (&a, &b) {
                    (Self::Lit(Literal::Bool(false)), x) | (x, Self::Lit(Literal::Bool(false))) => x.clone(),
                    (Self::Lit(Literal::Bool(true)), _) | (_, Self::Lit(Literal::Bool(true))) => Self::Lit(Literal::Bool(true)),
                    _ => Self::Or(Box::new(a), Box::new(b)),
                };
            }
            Self::Not(a) => Self::Not(Box::new(a.fold())),
            Self::IsNull(a) => Self::IsNull(Box::new(a.fold())),
            Self::Abs(a) => Self::Abs(Box::new(a.fold())),
            Self::Sqrt(a) => Self::Sqrt(Box::new(a.fold())),
            Self::Ln(a) => Self::Ln(Box::new(a.fold())),
            Self::Exp(a) => Self::Exp(Box::new(a.fold())),
            Self::Convert(a, u) => Self::Convert(Box::new(a.fold()), u.clone()),
            Self::Col(_) | Self::Lit(_) => return self.clone(),
        };
        // A node whose children are all literals is evaluated once, here.
        // `convert` is excluded: a bare literal has no unit to convert from.
        let mut cols = BTreeSet::new();
        folded.columns(&mut cols);
        if cols.is_empty() && !matches!(folded, Self::Convert(..)) && !folded.has_convert() {
            let one = Batch::empty(Schema::default());
            if let Ok(c) = folded.eval_n(&one, 1) {
                if let Some(l) = literal_of(&c) {
                    return Self::Lit(l);
                }
            }
        }
        folded
    }

    fn has_convert(&self) -> bool {
        match self {
            Self::Convert(..) => true,
            Self::Col(_) | Self::Lit(_) => false,
            Self::Add(a, b)
            | Self::Sub(a, b)
            | Self::Mul(a, b)
            | Self::Div(a, b)
            | Self::Eq(a, b)
            | Self::Ne(a, b)
            | Self::Lt(a, b)
            | Self::Le(a, b)
            | Self::Gt(a, b)
            | Self::Ge(a, b)
            | Self::And(a, b)
            | Self::Or(a, b) => a.has_convert() || b.has_convert(),
            Self::Not(a) | Self::IsNull(a) | Self::Abs(a) | Self::Sqrt(a) | Self::Ln(a) | Self::Exp(a) => a.has_convert(),
        }
    }

    // -------------------------------------------------------- evaluation

    /// Evaluates over every row of `batch`. `schema_units` supplies the input
    /// units needed by `convert`.
    pub fn eval(&self, batch: &Batch) -> Result<Column> {
        self.eval_n(batch, batch.rows())
    }

    fn eval_n(&self, batch: &Batch, n: usize) -> Result<Column> {
        match self {
            Self::Col(c) => Ok(batch.column(c)?.clone()),
            Self::Lit(l) => Ok(match l {
                Literal::Bool(b) => Column::Boolean(vec![Some(*b); n]),
                Literal::Number(v) => Column::Float64(vec![Some(*v); n]),
                Literal::Text(s) => Column::Utf8(vec![Some(s.clone()); n]),
                Literal::Null(()) => Column::Float64(vec![None; n]),
            }),
            Self::Add(a, b) => arith(Arith::Add, &a.eval_n(batch, n)?, &b.eval_n(batch, n)?),
            Self::Sub(a, b) => arith(Arith::Sub, &a.eval_n(batch, n)?, &b.eval_n(batch, n)?),
            Self::Mul(a, b) => arith(Arith::Mul, &a.eval_n(batch, n)?, &b.eval_n(batch, n)?),
            Self::Div(a, b) => arith(Arith::Div, &a.eval_n(batch, n)?, &b.eval_n(batch, n)?),
            Self::Eq(a, b) => compare(Cmp::Eq, &a.eval_n(batch, n)?, &b.eval_n(batch, n)?),
            Self::Ne(a, b) => compare(Cmp::Ne, &a.eval_n(batch, n)?, &b.eval_n(batch, n)?),
            Self::Lt(a, b) => compare(Cmp::Lt, &a.eval_n(batch, n)?, &b.eval_n(batch, n)?),
            Self::Le(a, b) => compare(Cmp::Le, &a.eval_n(batch, n)?, &b.eval_n(batch, n)?),
            Self::Gt(a, b) => compare(Cmp::Gt, &a.eval_n(batch, n)?, &b.eval_n(batch, n)?),
            Self::Ge(a, b) => compare(Cmp::Ge, &a.eval_n(batch, n)?, &b.eval_n(batch, n)?),
            Self::And(a, b) | Self::Or(a, b) => {
                let and = matches!(self, Self::And(..));
                let (x, y) = (bools(&a.eval_n(batch, n)?)?, bools(&b.eval_n(batch, n)?)?);
                Ok(Column::Boolean(
                    x.iter()
                        .zip(&y)
                        .map(|(p, q)| match (and, p, q) {
                            (true, Some(false), _) | (true, _, Some(false)) => Some(false),
                            (true, Some(true), Some(true)) => Some(true),
                            (false, Some(true), _) | (false, _, Some(true)) => Some(true),
                            (false, Some(false), Some(false)) => Some(false),
                            _ => None,
                        })
                        .collect(),
                ))
            }
            Self::Not(a) => Ok(Column::Boolean(bools(&a.eval_n(batch, n)?)?.iter().map(|v| v.map(|b| !b)).collect())),
            Self::IsNull(a) => {
                let c = a.eval_n(batch, n)?;
                Ok(Column::Boolean((0..c.len()).map(|i| Some(c.is_null(i))).collect()))
            }
            Self::Abs(a) => math(Math::Abs, &a.eval_n(batch, n)?),
            Self::Sqrt(a) => math(Math::Sqrt, &a.eval_n(batch, n)?),
            Self::Ln(a) => math(Math::Ln, &a.eval_n(batch, n)?),
            Self::Exp(a) => math(Math::Exp, &a.eval_n(batch, n)?),
            Self::Convert(a, to) => {
                let mut notes = Vec::new();
                let from = a.infer(&batch.schema, &mut notes)?.unit.ok_or_else(|| EngineError::Unit(format!("cannot convert to `{to}`: the input has no unit")))?;
                let v = a.eval_n(batch, n)?.as_f64().ok_or_else(|| EngineError::Type("convert needs a number".into()))?;
                // Linear conversion; computed once from the catalogue factors.
                let (f, t) = (units::unit(&from).map_err(unit_err)?, units::unit(to).map_err(unit_err)?);
                Ok(Column::Float64(v.iter().map(|x| x.map(|x| (x * f.factor + f.offset - t.offset) / t.factor)).collect()))
            }
        }
    }
}

fn unit_err(e: apsi_science::ScienceError) -> EngineError {
    EngineError::Unit(e.to_string())
}

pub(crate) fn dimension(symbol: &str) -> Result<Dimension> {
    Ok(units::unit(symbol).map_err(unit_err)?.dimension)
}

fn numeric(x: ExprType, op: &str) -> Result<ExprType> {
    if x.dtype.is_numeric() { Ok(x) } else { Err(EngineError::Type(format!("{op} needs a number, got {:?}", x.dtype))) }
}

fn same_unit(op: &str, x: &ExprType, y: &ExprType) -> Result<()> {
    match (&x.unit, &y.unit) {
        (Some(a), Some(b)) if a != b => {
            let (da, db) = (dimension(a)?, dimension(b)?);
            if da == db {
                Err(EngineError::Unit(format!("cannot {op} `{a}` and `{b}` directly; convert one side first, e.g. {{\"convert\": [expr, \"{a}\"]}}")))
            } else {
                Err(EngineError::Unit(format!("cannot {op} `{a}` ({da}) and `{b}` ({db})")))
            }
        }
        _ => Ok(()),
    }
}

fn arith_type(op: Arith, x: ExprType, y: ExprType, notes: &mut Vec<String>) -> Result<ExprType> {
    let name = match op {
        Arith::Add => "add",
        Arith::Sub => "subtract",
        Arith::Mul => "multiply",
        Arith::Div => "divide",
    };
    let x = numeric(x, name)?;
    let y = numeric(y, name)?;
    let unit = match op {
        Arith::Add | Arith::Sub => {
            same_unit(name, &x, &y)?;
            if matches!(op, Arith::Add) && !x.literal && !y.literal {
                if let Some(u) = &x.unit {
                    if dimension(u)? == Dimension::TEMPERATURE {
                        return Err(EngineError::Unit("cannot add two absolute temperatures; add a temperature difference instead".into()));
                    }
                }
            }
            x.unit.or(y.unit)
        }
        Arith::Mul | Arith::Div => match (&x.unit, &y.unit, x.literal, y.literal) {
            (u, _, false, true) => u.clone(),
            (_, u, true, false) if matches!(op, Arith::Mul) => u.clone(),
            (None, None, ..) => None,
            (a, b, ..) => {
                notes.push(format!(
                    "unit of {} {name} {} is not tracked; the result has no unit",
                    a.as_deref().unwrap_or("(unknown)"),
                    b.as_deref().unwrap_or("(unknown)")
                ));
                None
            }
        },
    };
    Ok(ExprType { dtype: DataType::Float64, unit, literal: x.literal && y.literal })
}

fn literal_of(c: &Column) -> Option<Literal> {
    Some(match c {
        Column::Float64(v) => v.first()?.map_or(Literal::Null(()), Literal::Number),
        Column::Int64(v) => v.first()?.map_or(Literal::Null(()), |i| Literal::Number(i as f64)),
        Column::Utf8(v) => v.first()?.clone().map_or(Literal::Null(()), Literal::Text),
        Column::Boolean(v) => v.first()?.map_or(Literal::Null(()), Literal::Bool),
    })
}

fn bools(c: &Column) -> Result<Vec<Option<bool>>> {
    match c {
        Column::Boolean(v) => Ok(v.clone()),
        // An all-null literal column typed as number acts as boolean null.
        Column::Float64(v) if v.iter().all(Option::is_none) => Ok(vec![None; v.len()]),
        other => Err(EngineError::Type(format!("expected booleans, got {:?}", other.dtype()))),
    }
}

fn arith(op: Arith, a: &Column, b: &Column) -> Result<Column> {
    let (x, y) = (a.as_f64().ok_or_else(|| EngineError::Type("arithmetic needs numbers".into()))?, b.as_f64().ok_or_else(|| EngineError::Type("arithmetic needs numbers".into()))?);
    Ok(Column::Float64(
        x.iter()
            .zip(&y)
            .map(|(p, q)| {
                let (p, q) = ((*p)?, (*q)?);
                let r = match op {
                    Arith::Add => p + q,
                    Arith::Sub => p - q,
                    Arith::Mul => p * q,
                    // Division by zero is undefined: missing, not infinity.
                    Arith::Div if q == 0.0 => return None,
                    Arith::Div => p / q,
                };
                r.is_finite().then_some(r)
            })
            .collect(),
    ))
}

fn ord<T: PartialOrd>(op: Cmp, p: &T, q: &T) -> bool {
    match op {
        Cmp::Eq => p == q,
        Cmp::Ne => p != q,
        Cmp::Lt => p < q,
        Cmp::Le => p <= q,
        Cmp::Gt => p > q,
        Cmp::Ge => p >= q,
    }
}

fn compare(op: Cmp, a: &Column, b: &Column) -> Result<Column> {
    let out: Vec<Option<bool>> = match (a, b) {
        (Column::Utf8(x), Column::Utf8(y)) => x.iter().zip(y).map(|(p, q)| Some(ord(op, p.as_ref()?, q.as_ref()?))).collect(),
        (Column::Boolean(x), Column::Boolean(y)) => x.iter().zip(y).map(|(p, q)| Some(ord(op, &(*p)?, &(*q)?))).collect(),
        _ => match (a.as_f64(), b.as_f64()) {
            (Some(x), Some(y)) => x.iter().zip(&y).map(|(p, q)| Some(ord(op, &(*p)?, &(*q)?))).collect(),
            // Comparing anything with an all-null literal gives null.
            _ if [a, b].iter().any(|c| matches!(c, Column::Float64(v) if v.iter().all(Option::is_none))) => vec![None; a.len()],
            _ => return Err(EngineError::Type(format!("cannot compare {:?} with {:?}", a.dtype(), b.dtype()))),
        },
    };
    Ok(Column::Boolean(out))
}

fn math(op: Math, a: &Column) -> Result<Column> {
    let x = a.as_f64().ok_or_else(|| EngineError::Type("math functions need numbers".into()))?;
    Ok(Column::Float64(
        x.iter()
            .map(|v| {
                let v = (*v)?;
                let r = match op {
                    Math::Abs => v.abs(),
                    Math::Sqrt if v < 0.0 => return None,
                    Math::Sqrt => v.sqrt(),
                    Math::Ln if v <= 0.0 => return None,
                    Math::Ln => v.ln(),
                    Math::Exp => v.exp(),
                };
                r.is_finite().then_some(r)
            })
            .collect(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Field;

    fn batch() -> Batch {
        let schema = Schema::new(vec![
            Field::new("yield", DataType::Float64, Some("t/ha")),
            Field::new("rate", DataType::Float64, Some("kg/ha")),
            Field::new("rain", DataType::Float64, Some("mm")),
            Field::new("tmax", DataType::Float64, Some("Cel")),
            Field::new("site", DataType::Utf8, None),
        ])
        .unwrap();
        Batch::new(
            schema,
            vec![
                Column::Float64(vec![Some(4.0), Some(6.0), None]),
                Column::Float64(vec![Some(4000.0), Some(1.0), Some(2.0)]),
                Column::Float64(vec![Some(10.0), Some(0.0), Some(5.0)]),
                Column::Float64(vec![Some(20.0), Some(30.0), Some(25.0)]),
                Column::Utf8(vec![Some("north".into()), Some("south".into()), None]),
            ],
        )
        .unwrap()
    }

    fn parse(json: &str) -> Expr {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn json_form_round_trips() {
        let e = parse(r#"{"and": [{"gt": [{"col": "yield"}, {"lit": 5}]}, {"eq": [{"col": "site"}, {"lit": "south"}]}]}"#);
        assert_eq!(serde_json::from_value::<Expr>(serde_json::to_value(&e).unwrap()).unwrap(), e);
        assert_eq!(e.eval(&batch()).unwrap(), Column::Boolean(vec![Some(false), Some(true), Some(false)]));
    }

    #[test]
    fn units_are_enforced() {
        let s = batch().schema;
        let mut n = Vec::new();
        let err = |j: &str| parse(j).infer(&s, &mut Vec::new()).unwrap_err().to_string();
        assert!(err(r#"{"add": [{"col": "yield"}, {"col": "rate"}]}"#).contains("convert one side first"));
        assert!(err(r#"{"lt": [{"col": "yield"}, {"col": "rain"}]}"#).contains("cannot compare"));
        assert!(err(r#"{"add": [{"col": "tmax"}, {"col": "tmax"}]}"#).contains("absolute temperatures"));
        assert!(err(r#"{"ln": {"col": "rain"}}"#).contains("dimensionless"));
        assert!(err(r#"{"convert": [{"col": "rain"}, "kg/ha"]}"#).contains("cannot convert"));
        let ok = parse(r#"{"add": [{"convert": [{"col": "yield"}, "kg/ha"]}, {"col": "rate"}]}"#).infer(&s, &mut n).unwrap();
        assert_eq!(ok.unit.as_deref(), Some("kg/ha"));
        assert_eq!(parse(r#"{"mul": [{"col": "rain"}, {"lit": 2}]}"#).infer(&s, &mut n).unwrap().unit.as_deref(), Some("mm"));
        assert!(n.is_empty());
        assert_eq!(parse(r#"{"mul": [{"col": "rain"}, {"col": "rate"}]}"#).infer(&s, &mut n).unwrap().unit, None);
        assert_eq!(n.len(), 1, "untracked unit is reported");
    }

    #[test]
    fn evaluation_follows_sql_null_semantics() {
        let b = batch();
        assert_eq!(parse(r#"{"div": [{"col": "yield"}, {"col": "rain"}]}"#).eval(&b).unwrap(), Column::Float64(vec![Some(0.4), None, None]));
        assert_eq!(parse(r#"{"is_null": {"col": "site"}}"#).eval(&b).unwrap(), Column::Boolean(vec![Some(false), Some(false), Some(true)]));
        // null AND false = false; null OR true = true; null AND true = null.
        let or = parse(r#"{"or": [{"gt": [{"col": "yield"}, {"lit": 100}]}, {"lit": true}]}"#);
        assert_eq!(or.eval(&b).unwrap(), Column::Boolean(vec![Some(true); 3]));
        let and = parse(r#"{"and": [{"gt": [{"col": "yield"}, {"lit": 1}]}, {"gt": [{"col": "rain"}, {"lit": -1}]}]}"#);
        assert_eq!(and.eval(&b).unwrap(), Column::Boolean(vec![Some(true), Some(true), None]));
        let conv = parse(r#"{"convert": [{"col": "yield"}, "kg/ha"]}"#).eval(&b).unwrap();
        assert_eq!(conv, Column::Float64(vec![Some(4000.0), Some(6000.0), None]));
        assert_eq!(parse(r#"{"sqrt": {"lit": -1}}"#).eval(&b).unwrap(), Column::Float64(vec![None; 3]));
    }

    #[test]
    fn constant_folding_is_pure_and_complete() {
        let e = parse(r#"{"and": [{"gt": [{"col": "yield"}, {"add": [{"lit": 2}, {"lit": 3}]}]}, {"lt": [{"lit": 1}, {"lit": 2}]}]}"#);
        assert_eq!(e.fold(), parse(r#"{"gt": [{"col": "yield"}, {"lit": 5.0}]}"#));
        assert_eq!(parse(r#"{"or": [{"col": "x"}, {"lit": true}]}"#).fold(), Expr::Lit(Literal::Bool(true)));
        let mut cols = BTreeSet::new();
        e.columns(&mut cols);
        assert_eq!(cols.into_iter().collect::<Vec<_>>(), vec!["yield".to_string()]);
    }
}
