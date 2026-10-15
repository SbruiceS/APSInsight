//! Farm finance and sustainability: gross margin, break-even analysis and activity-
//! based emission estimates with uncertainty propagation.

use crate::{Result, ScienceError, in_range};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostLine {
    pub item: String,
    /// Cost per hectare in the report currency.
    pub per_ha: f64,
    #[serde(default)]
    pub fixed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarginInput {
    pub area_ha: f64,
    /// Yield per hectare in the unit the price is quoted for (e.g. t/ha).
    pub yield_per_ha: f64,
    pub price_per_unit: f64,
    pub costs: Vec<CostLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarginResult {
    pub revenue: f64,
    pub variable_costs: f64,
    pub fixed_costs: f64,
    pub gross_margin: f64,
    pub net_margin: f64,
    pub gross_margin_per_ha: f64,
    /// Yield per ha needed to cover all costs at the given price; null if price is 0.
    pub break_even_yield_per_ha: Option<f64>,
    /// Price needed to cover all costs at the given yield; null if yield is 0.
    pub break_even_price: Option<f64>,
}

pub fn margin(i: &MarginInput) -> Result<MarginResult> {
    in_range("area_ha", i.area_ha, 1e-6, 1e7)?;
    in_range("yield_per_ha", i.yield_per_ha, 0.0, 1e6)?;
    in_range("price_per_unit", i.price_per_unit, 0.0, 1e9)?;
    if i.costs.len() > 500 {
        return Err(ScienceError::Invalid("at most 500 cost lines".into()));
    }
    for c in &i.costs {
        in_range(&format!("cost {}", c.item), c.per_ha, 0.0, 1e9)?;
    }
    let sum = |fixed: bool| i.costs.iter().filter(|c| c.fixed == fixed).map(|c| c.per_ha).sum::<f64>() * i.area_ha;
    let revenue = i.area_ha * i.yield_per_ha * i.price_per_unit;
    let (variable, fixed) = (sum(false), sum(true));
    let total_per_ha = (variable + fixed) / i.area_ha;
    Ok(MarginResult {
        revenue,
        variable_costs: variable,
        fixed_costs: fixed,
        gross_margin: revenue - variable,
        net_margin: revenue - variable - fixed,
        gross_margin_per_ha: (revenue - variable) / i.area_ha,
        break_even_yield_per_ha: (i.price_per_unit > 0.0).then(|| total_per_ha / i.price_per_unit),
        break_even_price: (i.yield_per_ha > 0.0).then(|| total_per_ha / i.yield_per_ha),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    pub name: String,
    pub amount: f64,
    /// kg CO2e per unit of activity.
    pub factor: f64,
    /// Relative standard uncertainty of the factor (e.g. 0.3 = ±30%).
    #[serde(default)]
    pub factor_uncertainty: f64,
    #[serde(default)]
    pub amount_uncertainty: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmissionResult {
    pub total_kg_co2e: f64,
    /// One standard uncertainty, combined in quadrature assuming independent errors (IPCC Approach 1).
    pub uncertainty_kg_co2e: f64,
    pub by_activity: Vec<(String, f64)>,
    pub method: String,
}

pub fn emissions(activities: &[Activity]) -> Result<EmissionResult> {
    if activities.is_empty() {
        return Err(ScienceError::Insufficient("no activities".into()));
    }
    let mut total = 0.0;
    let mut var = 0.0;
    let mut by = Vec::new();
    for a in activities {
        in_range("amount", a.amount, 0.0, 1e12)?;
        in_range("factor", a.factor, -1e6, 1e6)?;
        in_range("factor_uncertainty", a.factor_uncertainty, 0.0, 10.0)?;
        in_range("amount_uncertainty", a.amount_uncertainty, 0.0, 10.0)?;
        let e = a.amount * a.factor;
        let rel = (a.factor_uncertainty.powi(2) + a.amount_uncertainty.powi(2)).sqrt();
        var += (e * rel).powi(2);
        total += e;
        by.push((a.name.clone(), e));
    }
    Ok(EmissionResult {
        total_kg_co2e: total,
        uncertainty_kg_co2e: var.sqrt(),
        by_activity: by,
        method: "activity x emission factor; IPCC 2006 Vol.1 Ch.3 Approach 1 error propagation".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn margins_and_break_even() {
        let m = margin(&MarginInput {
            area_ha: 10.0,
            yield_per_ha: 5.0,
            price_per_unit: 200.0,
            costs: vec![CostLine { item: "seed".into(), per_ha: 150.0, fixed: false }, CostLine { item: "rent".into(), per_ha: 250.0, fixed: true }],
        })
        .unwrap();
        assert_eq!((m.revenue, m.gross_margin, m.net_margin), (10_000.0, 8_500.0, 6_000.0));
        assert_eq!((m.break_even_yield_per_ha, m.break_even_price), (Some(2.0), Some(80.0)));
        let zero = margin(&MarginInput { area_ha: 1.0, yield_per_ha: 0.0, price_per_unit: 0.0, costs: vec![] }).unwrap();
        assert_eq!((zero.break_even_price, zero.break_even_yield_per_ha), (None, None));
        assert!(margin(&MarginInput { area_ha: 0.0, yield_per_ha: 1.0, price_per_unit: 1.0, costs: vec![] }).is_err());
    }

    #[test]
    fn emission_uncertainty_in_quadrature() {
        let r = emissions(&[
            Activity { name: "diesel".into(), amount: 100.0, factor: 2.68, factor_uncertainty: 0.05, amount_uncertainty: 0.0 },
            Activity { name: "N fertilizer".into(), amount: 1000.0, factor: 0.0, factor_uncertainty: 0.5, amount_uncertainty: 0.1 },
        ])
        .unwrap();
        assert!((r.total_kg_co2e - 268.0).abs() < 1e-9);
        assert!((r.uncertainty_kg_co2e - 13.4).abs() < 1e-9);
        assert!(emissions(&[]).is_err());
    }
}
