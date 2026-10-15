//! JSON-in/JSON-out registry of science functions, used by workflow `compute`
//! steps, the `/api/v1/compute/{function}` endpoint and `apsi compute`.

use crate::{agronomy, economics, geo, indices, stats, timeseries, units};
use apsi_core::workflow::{ComputeRegistry, FunctionSpec};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

type Handler = fn(&Value) -> Result<Value, String>;

struct Entry {
    name: &'static str,
    description: &'static str,
    input: &'static str,
    output: &'static str,
    handler: Handler,
}

fn arg<T: DeserializeOwned>(v: &Value) -> Result<T, String> {
    serde_json::from_value(v.clone()).map_err(|e| e.to_string())
}

fn out<T: serde::Serialize>(r: crate::Result<T>) -> Result<Value, String> {
    // Callers wrap failures as ApsiError::InvalidInput, which adds the "invalid input" prefix itself.
    r.map_err(|e| match e {
        crate::ScienceError::Invalid(m) => m,
        other => other.to_string(),
    })
    .and_then(|v| serde_json::to_value(v).map_err(|e| e.to_string()))
}

/// Accepts either a bare array of numbers or `{ "values": [...] }`.
fn numbers(v: &Value) -> Result<Vec<f64>, String> {
    let arr = v.as_array().or_else(|| v.get("values").and_then(Value::as_array)).ok_or("expected an array of numbers")?;
    arr.iter().map(|x| x.as_f64().ok_or_else(|| "array must contain only numbers".to_string())).collect()
}

#[derive(Deserialize)]
struct IndexArgs {
    index: indices::Index,
    #[serde(default)]
    pixel: Option<indices::Pixel>,
    #[serde(default)]
    pixels: Vec<indices::Pixel>,
    #[serde(default)]
    nodata: Option<f64>,
}

#[derive(Deserialize)]
struct TwoPoints {
    a: geo::LonLat,
    b: geo::LonLat,
}

#[derive(Deserialize)]
struct RingArgs {
    ring: Vec<geo::LonLat>,
    #[serde(default)]
    point: Option<geo::LonLat>,
}

#[derive(Deserialize)]
struct CiArgs {
    values: Vec<f64>,
    #[serde(default = "ninety_five")]
    confidence: f64,
}

fn ninety_five() -> f64 {
    0.95
}

#[derive(Deserialize)]
struct AB {
    a: Vec<f64>,
    b: Vec<f64>,
}

#[derive(Deserialize)]
struct XY {
    x: Vec<f64>,
    y: Vec<f64>,
}

#[derive(Deserialize)]
struct PValues {
    p_values: Vec<f64>,
}

#[derive(Deserialize)]
struct Forecast {
    actual: Vec<f64>,
    predicted: Vec<f64>,
}

#[derive(Deserialize)]
struct GapArgs {
    observations: Vec<timeseries::Obs>,
    interval: i64,
    #[serde(default = "one_and_half")]
    tolerance: f64,
}

fn one_and_half() -> f64 {
    1.5
}

#[derive(Deserialize)]
struct DownArgs {
    observations: Vec<timeseries::Obs>,
    width: i64,
    agg: timeseries::Agg,
}

#[derive(Deserialize)]
struct Gdd {
    tmin: f64,
    tmax: f64,
    base: f64,
    #[serde(default)]
    cap: Option<f64>,
}

#[derive(Deserialize)]
struct Balance {
    capacity_mm: f64,
    initial_mm: f64,
    days: Vec<agronomy::WaterBalanceDay>,
}

#[derive(Deserialize)]
struct Texture {
    sand: f64,
    silt: f64,
    clay: f64,
}

#[derive(Deserialize)]
struct Moisture {
    #[serde(rename = "yield")]
    yield_value: f64,
    moisture_pct: f64,
    standard_pct: f64,
}

#[derive(Deserialize)]
struct Convert {
    value: f64,
    from: String,
    to: String,
}

#[derive(Deserialize)]
struct Bushels {
    bu_ac: f64,
    lb_per_bushel: f64,
}

#[derive(Deserialize)]
struct Activities {
    activities: Vec<economics::Activity>,
}

const ENTRIES: &[Entry] = &[
    Entry {
        name: "indices.registry",
        description: "Index formulas, required bands and valid ranges",
        input: "{}",
        output: "IndexSpec[]",
        handler: |_| Ok(json!(indices::REGISTRY)),
    },
    Entry {
        name: "indices.compute",
        description: "One vegetation/water index for one pixel",
        input: "{index, pixel, nodata?}",
        output: "number|null",
        handler: |v| {
            let a: IndexArgs = arg(v)?;
            out(indices::compute(a.index, &a.pixel.ok_or("pixel is required")?, a.nodata))
        },
    },
    Entry {
        name: "indices.summarize",
        description: "Index statistics over pixels with nodata flags",
        input: "{index, pixels[], nodata?}",
        output: "IndexStats",
        handler: |v| {
            let a: IndexArgs = arg(v)?;
            out(indices::summarize(a.index, &a.pixels, a.nodata))
        },
    },
    Entry {
        name: "geo.distance",
        description: "Great-circle distance in metres",
        input: "{a:{lon,lat}, b:{lon,lat}}",
        output: "number",
        handler: |v| {
            let a: TwoPoints = arg(v)?;
            out(geo::distance_m(a.a, a.b))
        },
    },
    Entry {
        name: "geo.area",
        description: "Geodesic area of a closed ring",
        input: "{ring:[{lon,lat}]}",
        output: "{m2, ha}",
        handler: |v| {
            let a: RingArgs = arg(v)?;
            let m2 = geo::ring_area_m2(&a.ring).map_err(|e| e.to_string())?;
            Ok(json!({ "m2": m2, "ha": m2 / 10_000.0 }))
        },
    },
    Entry {
        name: "geo.contains",
        description: "Point-in-polygon (boundary counts as inside)",
        input: "{ring, point}",
        output: "boolean",
        handler: |v| {
            let a: RingArgs = arg(v)?;
            out(geo::contains(&a.ring, a.point.ok_or("point is required")?))
        },
    },
    Entry {
        name: "geo.bbox",
        description: "Bounding box of a ring",
        input: "{ring}",
        output: "BBox",
        handler: |v| {
            let a: RingArgs = arg(v)?;
            out(geo::bbox(&a.ring))
        },
    },
    Entry {
        name: "geo.web_mercator",
        description: "Reproject WGS 84 to EPSG:3857",
        input: "{lon, lat}",
        output: "{x, y}",
        handler: |v| {
            let p: geo::LonLat = arg(v)?;
            let (x, y) = geo::to_web_mercator(p).map_err(|e| e.to_string())?;
            Ok(json!({ "x": x, "y": y }))
        },
    },
    Entry { name: "stats.mean", description: "Arithmetic mean", input: "number[]", output: "number", handler: |v| out(stats::mean(&numbers(v)?)) },
    Entry {
        name: "stats.describe",
        description: "n, mean, sd, quartiles",
        input: "number[]",
        output: "Descriptive",
        handler: |v| out(stats::describe(&numbers(v)?)),
    },
    Entry {
        name: "stats.mean_ci",
        description: "t confidence interval for the mean",
        input: "{values, confidence?}",
        output: "Interval",
        handler: |v| {
            let a: CiArgs = arg(v)?;
            out(stats::mean_ci(&a.values, a.confidence))
        },
    },
    Entry {
        name: "stats.welch_t_test",
        description: "Welch two-sample t-test with Cohen's d",
        input: "{a[], b[]}",
        output: "TTest",
        handler: |v| {
            let a: AB = arg(v)?;
            out(stats::welch_t_test(&a.a, &a.b))
        },
    },
    Entry {
        name: "stats.benjamini_hochberg",
        description: "FDR-adjusted p-values",
        input: "{p_values[]}",
        output: "number[]",
        handler: |v| {
            let a: PValues = arg(v)?;
            out(stats::benjamini_hochberg(&a.p_values))
        },
    },
    Entry {
        name: "stats.bonferroni",
        description: "Bonferroni-adjusted p-values",
        input: "{p_values[]}",
        output: "number[]",
        handler: |v| {
            let a: PValues = arg(v)?;
            out(stats::bonferroni(&a.p_values))
        },
    },
    Entry {
        name: "stats.linear_regression",
        description: "OLS with R², residuals, slope p-value",
        input: "{x[], y[]}",
        output: "Regression",
        handler: |v| {
            let a: XY = arg(v)?;
            out(stats::linear_regression(&a.x, &a.y))
        },
    },
    Entry {
        name: "stats.forecast_errors",
        description: "MAE, RMSE, bias, MAPE",
        input: "{actual[], predicted[]}",
        output: "ForecastErrors",
        handler: |v| {
            let a: Forecast = arg(v)?;
            out(stats::forecast_errors(&a.actual, &a.predicted))
        },
    },
    Entry {
        name: "ts.gaps",
        description: "Gaps longer than tolerance x interval",
        input: "{observations[{t,v}], interval, tolerance?}",
        output: "Gap[]",
        handler: |v| {
            let a: GapArgs = arg(v)?;
            out(timeseries::gaps(&a.observations, a.interval, a.tolerance))
        },
    },
    Entry {
        name: "ts.late_arrivals",
        description: "Indices of out-of-order observations",
        input: "{observations[]}",
        output: "number[]",
        handler: |v| {
            let a: GapArgs = arg(&json!({ "observations": v.get("observations").cloned().unwrap_or(Value::Null), "interval": 1 }))?;
            Ok(json!(timeseries::late_arrivals(&a.observations)))
        },
    },
    Entry {
        name: "ts.downsample",
        description: "Fixed-width bucket aggregation",
        input: "{observations[], width, agg}",
        output: "Bucket[]",
        handler: |v| {
            let a: DownArgs = arg(v)?;
            out(timeseries::downsample(&a.observations, a.width, a.agg))
        },
    },
    Entry {
        name: "agro.gdd",
        description: "Growing degree days (°C·day)",
        input: "{tmin, tmax, base, cap?}",
        output: "number",
        handler: |v| {
            let a: Gdd = arg(v)?;
            out(agronomy::growing_degree_days(a.tmin, a.tmax, a.base, a.cap))
        },
    },
    Entry {
        name: "agro.et0",
        description: "FAO-56 Penman-Monteith ET0 (mm/day)",
        input: "Et0Input",
        output: "number",
        handler: |v| {
            let a: agronomy::Et0Input = arg(v)?;
            out(agronomy::et0_penman_monteith(&a))
        },
    },
    Entry {
        name: "agro.water_balance",
        description: "Root-zone bucket water balance",
        input: "{capacity_mm, initial_mm, days[]}",
        output: "WaterBalanceResult",
        handler: |v| {
            let a: Balance = arg(v)?;
            out(agronomy::water_balance(a.capacity_mm, a.initial_mm, &a.days))
        },
    },
    Entry {
        name: "agro.usda_texture",
        description: "USDA soil texture class",
        input: "{sand, silt, clay}",
        output: "string",
        handler: |v| {
            let a: Texture = arg(v)?;
            out(agronomy::usda_texture(a.sand, a.silt, a.clay))
        },
    },
    Entry {
        name: "agro.moisture_correction",
        description: "Yield at standard moisture",
        input: "{yield, moisture_pct, standard_pct}",
        output: "number",
        handler: |v| {
            let a: Moisture = arg(v)?;
            out(agronomy::moisture_corrected_yield(a.yield_value, a.moisture_pct, a.standard_pct))
        },
    },
    Entry {
        name: "agro.convert",
        description: "Unit conversion (lb/ac, kg/ha, t/ha, ac, ha, in, mm, degF, Cel)",
        input: "{value, from, to}",
        output: "number",
        handler: |v| {
            let a: Convert = arg(v)?;
            out(agronomy::convert(a.value, &a.from, &a.to))
        },
    },
    Entry {
        name: "units.convert",
        description: "Dimension-checked conversion between any two catalogued units (see docs/UNITS.md); mismatched dimensions are rejected",
        input: "{value, from, to}",
        output: "number",
        handler: |v| {
            let a: Convert = arg(v)?;
            out(units::convert(a.value, &a.from, &a.to))
        },
    },
    Entry {
        name: "agro.bushels_to_t_ha",
        description: "Bushels/acre to t/ha",
        input: "{bu_ac, lb_per_bushel}",
        output: "number",
        handler: |v| {
            let a: Bushels = arg(v)?;
            out(agronomy::bushels_to_t_ha(a.bu_ac, a.lb_per_bushel))
        },
    },
    Entry {
        name: "econ.margin",
        description: "Gross/net margin and break-even",
        input: "MarginInput",
        output: "MarginResult",
        handler: |v| {
            let a: economics::MarginInput = arg(v)?;
            out(economics::margin(&a))
        },
    },
    Entry {
        name: "econ.emissions",
        description: "Activity-based CO2e with uncertainty",
        input: "{activities[]}",
        output: "EmissionResult",
        handler: |v| {
            let a: Activities = arg(v)?;
            out(economics::emissions(&a.activities))
        },
    },
];

#[derive(Debug, Default, Clone, Copy)]
pub struct ScienceRegistry;

impl ComputeRegistry for ScienceRegistry {
    fn functions(&self) -> Vec<FunctionSpec> {
        ENTRIES
            .iter()
            .map(|e| FunctionSpec { name: e.name.into(), description: e.description.into(), input: e.input.into(), output: e.output.into() })
            .collect()
    }

    fn call(&self, name: &str, input: &Value) -> Result<Value, String> {
        let e = ENTRIES.iter().find(|e| e.name == name).ok_or_else(|| format!("unknown function {name}"))?;
        if serde_json::to_vec(input).map(|b| b.len()).unwrap_or(0) > 8 * 1024 * 1024 {
            return Err("input exceeds 8 MiB".into());
        }
        (e.handler)(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_function_rejects_garbage_without_panicking() {
        let r = ScienceRegistry;
        for f in r.functions() {
            for bad in [Value::Null, json!("x"), json!([1, "a"]), json!({"a": {}})] {
                let _ = r.call(&f.name, &bad);
            }
        }
        assert!(r.call("nope", &json!({})).is_err());
    }

    #[test]
    fn calls_dispatch() {
        let r = ScienceRegistry;
        assert_eq!(r.call("stats.mean", &json!([1, 2, 3])).unwrap(), json!(2.0));
        assert_eq!(r.call("agro.usda_texture", &json!({"sand": 40, "silt": 40, "clay": 20})).unwrap(), json!("loam"));
        let ndvi = r.call("indices.compute", &json!({"index": "ndvi", "pixel": {"red": 0.1, "nir": 0.5}})).unwrap();
        assert!((ndvi.as_f64().unwrap() - 0.666_666_666).abs() < 1e-6);
        let area = r.call("geo.area", &json!({"ring": [{"lon":0,"lat":0},{"lon":1,"lat":0},{"lon":1,"lat":1},{"lon":0,"lat":1},{"lon":0,"lat":0}]})).unwrap();
        assert!(area["ha"].as_f64().unwrap() > 1.2e6);
        assert!(r.functions().len() >= 25);
    }
}
