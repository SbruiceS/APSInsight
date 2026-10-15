//! Agronomy, soil and water computations: growing degree days, FAO-56 reference
//! evapotranspiration, soil water balance and irrigation need, USDA soil texture
//! classification, yield moisture correction and unit conversions.

use crate::{Result, ScienceError, finite, in_range};
use serde::{Deserialize, Serialize};

/// Growing degree days for one day, averaging method with cap/base:
/// GDD = max(0, (min(Tmax, cap) + max(Tmin, base)) / 2 - base). Units: °C·day.
pub fn growing_degree_days(tmin: f64, tmax: f64, base: f64, cap: Option<f64>) -> Result<f64> {
    in_range("tmin", tmin, -90.0, 60.0)?;
    in_range("tmax", tmax, -90.0, 60.0)?;
    finite("base", base)?;
    if tmin > tmax {
        return Err(ScienceError::Invalid("tmin > tmax".into()));
    }
    let hi = cap.map_or(tmax, |c| tmax.min(c));
    let lo = tmin.max(base);
    Ok(((hi.max(lo) + lo) / 2.0 - base).max(0.0))
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Et0Input {
    pub tmin_c: f64,
    pub tmax_c: f64,
    /// Mean relative humidity, %.
    pub rh_mean: f64,
    /// Wind speed at 2 m, m/s.
    pub wind_2m: f64,
    /// Net radiation at the crop surface, MJ m-2 day-1.
    pub net_radiation: f64,
    /// Station elevation, m.
    pub elevation_m: f64,
    /// Soil heat flux, MJ m-2 day-1 (≈ 0 for daily steps).
    #[serde(default)]
    pub soil_heat_flux: f64,
}

/// FAO-56 Penman-Monteith daily reference evapotranspiration (mm/day), Allen et al. 1998 eq. 6.
pub fn et0_penman_monteith(i: &Et0Input) -> Result<f64> {
    in_range("tmin_c", i.tmin_c, -60.0, 60.0)?;
    in_range("tmax_c", i.tmax_c, -60.0, 60.0)?;
    in_range("rh_mean", i.rh_mean, 0.0, 100.0)?;
    in_range("wind_2m", i.wind_2m, 0.0, 60.0)?;
    in_range("net_radiation", i.net_radiation, -10.0, 50.0)?;
    in_range("elevation_m", i.elevation_m, -500.0, 9000.0)?;
    finite("soil_heat_flux", i.soil_heat_flux)?;
    if i.tmin_c > i.tmax_c {
        return Err(ScienceError::Invalid("tmin > tmax".into()));
    }
    let t = (i.tmin_c + i.tmax_c) / 2.0;
    let e_s = |t: f64| 0.6108 * (17.27 * t / (t + 237.3)).exp();
    let es = (e_s(i.tmin_c) + e_s(i.tmax_c)) / 2.0;
    let ea = es * i.rh_mean / 100.0;
    let delta = 4098.0 * e_s(t) / (t + 237.3).powi(2);
    let p = 101.3 * ((293.0 - 0.0065 * i.elevation_m) / 293.0).powf(5.26);
    let gamma = 0.000_665 * p;
    let num = 0.408 * delta * (i.net_radiation - i.soil_heat_flux) + gamma * 900.0 / (t + 273.0) * i.wind_2m * (es - ea);
    let den = delta + gamma * (1.0 + 0.34 * i.wind_2m);
    Ok((num / den).max(0.0))
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct WaterBalanceDay {
    pub rain_mm: f64,
    pub irrigation_mm: f64,
    /// Crop evapotranspiration (ET0 x Kc), mm.
    pub etc_mm: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaterBalanceResult {
    /// Soil water in the root zone at the end of each day, mm.
    pub storage_mm: Vec<f64>,
    pub deep_percolation_mm: f64,
    /// ET demand not met because the profile was empty, mm.
    pub unmet_demand_mm: f64,
    /// Depth needed today to refill to field capacity, mm.
    pub irrigation_to_refill_mm: f64,
}

/// Single-bucket root-zone water balance: S = clamp(S + P + I - ETc, 0, capacity);
/// excess drains as deep percolation. Runoff is not modelled separately.
pub fn water_balance(capacity_mm: f64, initial_mm: f64, days: &[WaterBalanceDay]) -> Result<WaterBalanceResult> {
    in_range("capacity_mm", capacity_mm, 1.0, 2000.0)?;
    in_range("initial_mm", initial_mm, 0.0, capacity_mm)?;
    let mut s = initial_mm;
    let mut out =
        WaterBalanceResult { storage_mm: Vec::with_capacity(days.len()), deep_percolation_mm: 0.0, unmet_demand_mm: 0.0, irrigation_to_refill_mm: 0.0 };
    for d in days {
        for (k, v) in [("rain_mm", d.rain_mm), ("irrigation_mm", d.irrigation_mm), ("etc_mm", d.etc_mm)] {
            in_range(k, v, 0.0, 1000.0)?;
        }
        s += d.rain_mm + d.irrigation_mm - d.etc_mm;
        if s > capacity_mm {
            out.deep_percolation_mm += s - capacity_mm;
            s = capacity_mm;
        }
        if s < 0.0 {
            out.unmet_demand_mm -= s;
            s = 0.0;
        }
        out.storage_mm.push(s);
    }
    out.irrigation_to_refill_mm = capacity_mm - s;
    Ok(out)
}

/// Gross irrigation depth for a net requirement and application efficiency (0..1].
pub fn gross_irrigation_mm(net_mm: f64, efficiency: f64) -> Result<f64> {
    in_range("net_mm", net_mm, 0.0, 1000.0)?;
    in_range("efficiency", efficiency, 0.05, 1.0)?;
    Ok(net_mm / efficiency)
}

/// USDA soil texture class from sand/silt/clay percentages (must sum to 100 ± 0.5).
pub fn usda_texture(sand: f64, silt: f64, clay: f64) -> Result<&'static str> {
    for (k, v) in [("sand", sand), ("silt", silt), ("clay", clay)] {
        in_range(k, v, 0.0, 100.0)?;
    }
    if (sand + silt + clay - 100.0).abs() > 0.5 {
        return Err(ScienceError::Invalid("sand + silt + clay must equal 100%".into()));
    }
    Ok(if silt + 1.5 * clay < 15.0 {
        "sand"
    } else if silt + 2.0 * clay < 30.0 {
        "loamy sand"
    } else if (7.0..20.0).contains(&clay) && sand > 52.0 || clay < 7.0 && silt < 50.0 {
        "sandy loam"
    } else if (7.0..27.0).contains(&clay) && (28.0..50.0).contains(&silt) && sand <= 52.0 {
        "loam"
    } else if silt >= 50.0 && ((12.0..27.0).contains(&clay) || (50.0..80.0).contains(&silt) && clay < 12.0) {
        "silt loam"
    } else if silt >= 80.0 && clay < 12.0 {
        "silt"
    } else if (20.0..35.0).contains(&clay) && silt < 28.0 && sand > 45.0 {
        "sandy clay loam"
    } else if (27.0..40.0).contains(&clay) && sand > 20.0 && sand <= 45.0 {
        "clay loam"
    } else if (27.0..40.0).contains(&clay) && sand <= 20.0 {
        "silty clay loam"
    } else if clay >= 35.0 && sand > 45.0 {
        "sandy clay"
    } else if clay >= 40.0 && silt >= 40.0 {
        "silty clay"
    } else {
        "clay"
    })
}

/// Converts a harvested weight at `moisture_pct` to the standard moisture basis:
/// W_std = W * (100 - m) / (100 - m_std).
pub fn moisture_corrected_yield(yield_value: f64, moisture_pct: f64, standard_pct: f64) -> Result<f64> {
    in_range("yield", yield_value, 0.0, 1e9)?;
    in_range("moisture_pct", moisture_pct, 0.0, 95.0)?;
    in_range("standard_pct", standard_pct, 0.0, 95.0)?;
    Ok(yield_value * (100.0 - moisture_pct) / (100.0 - standard_pct))
}

/// Supported unit conversions (exact factors from NIST SP 811).
pub fn convert(value: f64, from: &str, to: &str) -> Result<f64> {
    finite("value", value)?;
    const LB_AC_TO_KG_HA: f64 = 0.453_592_37 / 0.404_685_642_24;
    let f = match (from, to) {
        (a, b) if a == b => 1.0,
        ("lb/ac", "kg/ha") => LB_AC_TO_KG_HA,
        ("kg/ha", "lb/ac") => 1.0 / LB_AC_TO_KG_HA,
        ("kg/ha", "t/ha") => 0.001,
        ("t/ha", "kg/ha") => 1000.0,
        ("ac", "ha") => 0.404_685_642_24,
        ("ha", "ac") => 1.0 / 0.404_685_642_24,
        ("in", "mm") => 25.4,
        ("mm", "in") => 1.0 / 25.4,
        ("degF", "Cel") => return Ok((value - 32.0) * 5.0 / 9.0),
        ("Cel", "degF") => return Ok(value * 9.0 / 5.0 + 32.0),
        _ => return Err(ScienceError::Invalid(format!("no conversion from {from} to {to}"))),
    };
    Ok(value * f)
}

/// Bushels per acre -> tonnes per hectare for a crop's standard bushel weight (lb/bu).
pub fn bushels_to_t_ha(bu_ac: f64, lb_per_bushel: f64) -> Result<f64> {
    in_range("bu_ac", bu_ac, 0.0, 1e6)?;
    in_range("lb_per_bushel", lb_per_bushel, 1.0, 100.0)?;
    convert(bu_ac * lb_per_bushel, "lb/ac", "kg/ha").map(|kg| kg / 1000.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gdd() {
        assert_eq!(growing_degree_days(10.0, 30.0, 10.0, Some(30.0)).unwrap(), 10.0);
        assert_eq!(growing_degree_days(5.0, 9.0, 10.0, None).unwrap(), 0.0);
        assert_eq!(growing_degree_days(12.0, 40.0, 10.0, Some(30.0)).unwrap(), 11.0);
        assert!(growing_degree_days(30.0, 10.0, 10.0, None).is_err());
    }

    #[test]
    fn fao56_example_17_order_of_magnitude() {
        // FAO-56 example 18 (Brussels, 6 July): ET0 ≈ 3.9 mm/day.
        let et0 = et0_penman_monteith(&Et0Input {
            tmin_c: 12.3,
            tmax_c: 21.5,
            rh_mean: 68.0,
            wind_2m: 2.078,
            net_radiation: 13.28,
            elevation_m: 100.0,
            soil_heat_flux: 0.0,
        })
        .unwrap();
        assert!((et0 - 3.9).abs() < 0.3, "{et0}");
        assert!(
            et0_penman_monteith(&Et0Input {
                tmin_c: 20.0,
                tmax_c: 10.0,
                rh_mean: 50.0,
                wind_2m: 2.0,
                net_radiation: 10.0,
                elevation_m: 0.0,
                soil_heat_flux: 0.0
            })
            .is_err()
        );
    }

    #[test]
    fn water_balance_drains_and_depletes() {
        let days = [
            WaterBalanceDay { rain_mm: 50.0, irrigation_mm: 0.0, etc_mm: 5.0 },
            WaterBalanceDay { rain_mm: 0.0, irrigation_mm: 0.0, etc_mm: 60.0 },
            WaterBalanceDay { rain_mm: 0.0, irrigation_mm: 0.0, etc_mm: 60.0 },
        ];
        let r = water_balance(100.0, 80.0, &days).unwrap();
        assert_eq!(r.storage_mm, vec![100.0, 40.0, 0.0]);
        assert_eq!((r.deep_percolation_mm, r.unmet_demand_mm, r.irrigation_to_refill_mm), (25.0, 20.0, 100.0));
        assert!((gross_irrigation_mm(40.0, 0.8).unwrap() - 50.0).abs() < 1e-12);
        assert!(water_balance(100.0, 120.0, &[]).is_err());
    }

    #[test]
    fn texture_classes() {
        assert_eq!(usda_texture(92.0, 5.0, 3.0).unwrap(), "sand");
        assert_eq!(usda_texture(40.0, 40.0, 20.0).unwrap(), "loam");
        assert_eq!(usda_texture(20.0, 65.0, 15.0).unwrap(), "silt loam");
        assert_eq!(usda_texture(5.0, 88.0, 7.0).unwrap(), "silt");
        assert_eq!(usda_texture(10.0, 30.0, 60.0).unwrap(), "clay");
        assert_eq!(usda_texture(65.0, 20.0, 15.0).unwrap(), "sandy loam");
        assert_eq!(usda_texture(30.0, 35.0, 35.0).unwrap(), "clay loam");
        assert_eq!(usda_texture(5.0, 50.0, 45.0).unwrap(), "silty clay");
        assert!(usda_texture(50.0, 50.0, 50.0).is_err());
    }

    #[test]
    fn conversions_and_moisture() {
        assert!((convert(100.0, "lb/ac", "kg/ha").unwrap() - 112.085_1).abs() < 1e-3);
        assert!((convert(convert(7.0, "kg/ha", "lb/ac").unwrap(), "lb/ac", "kg/ha").unwrap() - 7.0).abs() < 1e-12);
        assert!((convert(212.0, "degF", "Cel").unwrap() - 100.0).abs() < 1e-12);
        assert!(convert(1.0, "kg", "furlong").is_err());
        assert!((bushels_to_t_ha(150.0, 56.0).unwrap() - 9.415).abs() < 0.01, "corn");
        assert!((moisture_corrected_yield(1000.0, 20.0, 15.5).unwrap() - 946.745).abs() < 0.01);
    }
}
