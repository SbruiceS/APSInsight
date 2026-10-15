//! Physical quantities with dimensional analysis.
//!
//! A [`Quantity`] stores its value in coherent SI units together with its
//! [`Dimension`] (exponents of length, mass, time, temperature, amount of
//! substance and electric current). Operations that are dimensionally invalid are
//! rejected with a typed error instead of producing a number:
//!
//! - adding or subtracting quantities of different dimension (`kg + m`),
//! - converting to a unit of a different dimension (`t/ha` to `mm`),
//! - adding two absolute temperatures (`20 Cel + 20 Cel`). Absolute temperature
//!   minus absolute temperature is a temperature *difference*; a difference may be
//!   added to an absolute temperature.
//!
//! Unit symbols are case-sensitive UCUM-style codes (`Cel`, `kg/ha`, `m3/m3`).
//! Currencies are deliberately not units here: converting between currencies
//! needs a dated exchange rate, which is data, not a constant.

use crate::{Result, ScienceError, finite};
use serde::Serialize;
use std::fmt;

/// Exponents of the base dimensions: [L, M, T, Θ, N, I].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct Dimension(pub [i8; 6]);

const NAMES: [&str; 6] = ["L", "M", "T", "Θ", "N", "I"];

impl Dimension {
    pub const NONE: Dimension = Dimension([0; 6]);
    pub const TEMPERATURE: Dimension = Dimension([0, 0, 0, 1, 0, 0]);

    fn combine(self, other: Dimension, sign: i8) -> Dimension {
        let mut d = self.0;
        for (a, b) in d.iter_mut().zip(other.0) {
            *a += sign * b;
        }
        Dimension(d)
    }
}

impl fmt::Display for Dimension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts: Vec<String> =
            self.0.iter().zip(NAMES).filter(|(e, _)| **e != 0).map(|(e, n)| if *e == 1 { n.to_string() } else { format!("{n}^{e}") }).collect();
        if parts.is_empty() { f.write_str("1") } else { f.write_str(&parts.join("·")) }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Unit {
    pub symbol: &'static str,
    pub name: &'static str,
    pub dimension: Dimension,
    /// `si = value * factor + offset`.
    pub factor: f64,
    pub offset: f64,
}

const fn u(symbol: &'static str, name: &'static str, d: [i8; 6], factor: f64) -> Unit {
    Unit { symbol, name, dimension: Dimension(d), factor, offset: 0.0 }
}

const L: [i8; 6] = [1, 0, 0, 0, 0, 0];
const A: [i8; 6] = [2, 0, 0, 0, 0, 0];
const V: [i8; 6] = [3, 0, 0, 0, 0, 0];
const M: [i8; 6] = [0, 1, 0, 0, 0, 0];
const T: [i8; 6] = [0, 0, 1, 0, 0, 0];
const ONE: [i8; 6] = [0; 6];

/// Exact factors from NIST SP 811 (2008) Appendix B where a definition exists.
pub const UNITS: &[Unit] = &[
    u("1", "dimensionless", ONE, 1.0),
    u("%", "percent", ONE, 0.01),
    u("g/kg", "gram per kilogram", ONE, 1e-3),
    u("mg/kg", "milligram per kilogram", ONE, 1e-6),
    u("m3/m3", "volumetric water content", ONE, 1.0),
    u("m", "metre", L, 1.0),
    u("km", "kilometre", L, 1e3),
    u("cm", "centimetre", L, 1e-2),
    u("mm", "millimetre", L, 1e-3),
    u("in", "inch", L, 0.0254),
    u("ft", "foot", L, 0.3048),
    u("m2", "square metre", A, 1.0),
    u("ha", "hectare", A, 1e4),
    u("ac", "acre (international)", A, 4_046.856_422_4),
    u("km2", "square kilometre", A, 1e6),
    u("m3", "cubic metre", V, 1.0),
    u("L", "litre", V, 1e-3),
    u("mL", "millilitre", V, 1e-6),
    u("kg", "kilogram", M, 1.0),
    u("g", "gram", M, 1e-3),
    u("mg", "milligram", M, 1e-6),
    u("t", "tonne", M, 1e3),
    u("lb", "pound (avoirdupois)", M, 0.453_592_37),
    u("s", "second", T, 1.0),
    u("min", "minute", T, 60.0),
    u("h", "hour", T, 3600.0),
    u("d", "day", T, 86_400.0),
    u("K", "kelvin", [0, 0, 0, 1, 0, 0], 1.0),
    Unit { symbol: "Cel", name: "degree Celsius", dimension: Dimension::TEMPERATURE, factor: 1.0, offset: 273.15 },
    Unit { symbol: "degF", name: "degree Fahrenheit", dimension: Dimension::TEMPERATURE, factor: 5.0 / 9.0, offset: 273.15 - 32.0 * 5.0 / 9.0 },
    u("m/s", "metre per second", [1, 0, -1, 0, 0, 0], 1.0),
    u("km/h", "kilometre per hour", [1, 0, -1, 0, 0, 0], 1000.0 / 3600.0),
    u("mm/h", "millimetre per hour", [1, 0, -1, 0, 0, 0], 1e-3 / 3600.0),
    u("mm/d", "millimetre per day", [1, 0, -1, 0, 0, 0], 1e-3 / 86_400.0),
    u("kg/m2", "kilogram per square metre", [-2, 1, 0, 0, 0, 0], 1.0),
    u("g/m2", "gram per square metre", [-2, 1, 0, 0, 0, 0], 1e-3),
    u("kg/ha", "kilogram per hectare", [-2, 1, 0, 0, 0, 0], 1e-4),
    u("t/ha", "tonne per hectare", [-2, 1, 0, 0, 0, 0], 0.1),
    u("lb/ac", "pound per acre", [-2, 1, 0, 0, 0, 0], 0.453_592_37 / 4_046.856_422_4),
    u("kg/m3", "kilogram per cubic metre", [-3, 1, 0, 0, 0, 0], 1.0),
    u("g/cm3", "gram per cubic centimetre", [-3, 1, 0, 0, 0, 0], 1e3),
    u("m3/s", "cubic metre per second", [3, 0, -1, 0, 0, 0], 1.0),
    u("m3/h", "cubic metre per hour", [3, 0, -1, 0, 0, 0], 1.0 / 3600.0),
    u("L/s", "litre per second", [3, 0, -1, 0, 0, 0], 1e-3),
    u("Pa", "pascal", [-1, 1, -2, 0, 0, 0], 1.0),
    u("kPa", "kilopascal", [-1, 1, -2, 0, 0, 0], 1e3),
    u("MPa", "megapascal", [-1, 1, -2, 0, 0, 0], 1e6),
    u("bar", "bar", [-1, 1, -2, 0, 0, 0], 1e5),
    u("J", "joule", [2, 1, -2, 0, 0, 0], 1.0),
    u("MJ", "megajoule", [2, 1, -2, 0, 0, 0], 1e6),
    u("kWh", "kilowatt hour", [2, 1, -2, 0, 0, 0], 3.6e6),
    u("W", "watt", [2, 1, -3, 0, 0, 0], 1.0),
    u("kW", "kilowatt", [2, 1, -3, 0, 0, 0], 1e3),
    u("W/m2", "watt per square metre", [0, 1, -3, 0, 0, 0], 1.0),
    u("MJ/m2/d", "megajoule per square metre per day", [0, 1, -3, 0, 0, 0], 1e6 / 86_400.0),
    u("S/m", "siemens per metre", [-3, -1, 3, 0, 0, 2], 1.0),
    u("dS/m", "decisiemens per metre", [-3, -1, 3, 0, 0, 2], 0.1),
    u("mol", "mole", [0, 0, 0, 0, 1, 0], 1.0),
    u("mol/kg", "mole per kilogram", [0, -1, 0, 0, 1, 0], 1.0),
    u("cmol/kg", "centimole (of charge) per kilogram", [0, -1, 0, 0, 1, 0], 0.01),
];

pub fn unit(symbol: &str) -> Result<&'static Unit> {
    UNITS.iter().find(|u| u.symbol == symbol).ok_or_else(|| ScienceError::Invalid(format!("unknown unit `{symbol}`")))
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Quantity {
    /// Value in coherent SI units (absolute kelvin for temperatures).
    pub si: f64,
    pub dimension: Dimension,
    /// True for temperature differences; only meaningful when the dimension is Θ.
    pub difference: bool,
}

impl Quantity {
    /// An absolute quantity, e.g. `Quantity::new(20.0, "Cel")`.
    pub fn new(value: f64, symbol: &str) -> Result<Self> {
        finite("value", value)?;
        let u = unit(symbol)?;
        Ok(Self { si: value * u.factor + u.offset, dimension: u.dimension, difference: false })
    }

    /// A difference, e.g. a 5 Cel rise. Offsets are not applied.
    pub fn difference(value: f64, symbol: &str) -> Result<Self> {
        finite("value", value)?;
        let u = unit(symbol)?;
        Ok(Self { si: value * u.factor, dimension: u.dimension, difference: true })
    }

    pub fn value_in(&self, symbol: &str) -> Result<f64> {
        let u = unit(symbol)?;
        if u.dimension != self.dimension {
            return Err(ScienceError::Invalid(format!("cannot express {} as `{symbol}` ({})", self.dimension, u.dimension)));
        }
        let offset = if self.difference { 0.0 } else { u.offset };
        Ok((self.si - offset) / u.factor)
    }

    fn same_dimension(&self, other: &Self, op: &str) -> Result<()> {
        if self.dimension == other.dimension { Ok(()) } else { Err(ScienceError::Invalid(format!("cannot {op} {} and {}", self.dimension, other.dimension))) }
    }

    fn is_temperature(&self) -> bool {
        self.dimension == Dimension::TEMPERATURE
    }

    pub fn try_add(self, other: Self) -> Result<Self> {
        self.same_dimension(&other, "add")?;
        if self.is_temperature() && !self.difference && !other.difference {
            return Err(ScienceError::Invalid("cannot add two absolute temperatures; add a temperature difference instead".into()));
        }
        Ok(Self { si: self.si + other.si, dimension: self.dimension, difference: self.difference && other.difference })
    }

    pub fn try_sub(self, other: Self) -> Result<Self> {
        self.same_dimension(&other, "subtract")?;
        if self.is_temperature() && self.difference && !other.difference {
            return Err(ScienceError::Invalid("cannot subtract an absolute temperature from a difference".into()));
        }
        // absolute - absolute = difference; absolute - difference = absolute.
        let difference = if self.is_temperature() { self.difference || !other.difference } else { false };
        Ok(Self { si: self.si - other.si, dimension: self.dimension, difference })
    }

    pub fn try_mul(self, other: Self) -> Result<Self> {
        if (self.is_temperature() && !self.difference) || (other.is_temperature() && !other.difference) {
            return Err(ScienceError::Invalid("absolute temperatures cannot be multiplied; use kelvin differences".into()));
        }
        Ok(Self { si: self.si * other.si, dimension: self.dimension.combine(other.dimension, 1), difference: false })
    }

    pub fn try_div(self, other: Self) -> Result<Self> {
        if other.si == 0.0 {
            return Err(ScienceError::Undefined("division by a zero quantity".into()));
        }
        if (self.is_temperature() && !self.difference) || (other.is_temperature() && !other.difference) {
            return Err(ScienceError::Invalid("absolute temperatures cannot be divided; use kelvin differences".into()));
        }
        Ok(Self { si: self.si / other.si, dimension: self.dimension.combine(other.dimension, -1), difference: false })
    }

    pub fn scale(self, k: f64) -> Result<Self> {
        finite("factor", k)?;
        if self.is_temperature() && !self.difference {
            return Err(ScienceError::Invalid("absolute temperatures cannot be scaled".into()));
        }
        Ok(Self { si: self.si * k, ..self })
    }
}

/// Converts `value` from one catalogued unit to another of the same dimension.
pub fn convert(value: f64, from: &str, to: &str) -> Result<f64> {
    Quantity::new(value, from)?.value_in(to)
}

/// The units catalogue as Markdown, published as `docs/UNITS.md`.
pub fn catalogue_markdown() -> String {
    let mut s = String::from(
        "# Units catalogue\n\n> Generated by `apsi docs generate` from `crates/apsi-science/src/units.rs`. Do not edit by hand.\n\n\
         Symbols are case-sensitive UCUM-style codes. `si = value × factor + offset`, in coherent SI units. \
         Dimension exponents use L (length), M (mass), T (time), Θ (temperature), N (amount of substance), I (electric current).\n\n\
         Rules enforced by `apsi_science::units::Quantity`: quantities of different dimension cannot be added, subtracted or converted; \
         absolute temperatures cannot be added, multiplied, divided or scaled (use differences); division by a zero quantity is undefined. \
         Currencies are not units: converting between currencies needs a dated exchange rate.\n\n\
         | Symbol | Name | Dimension | Factor | Offset |\n|---|---|---|---|---|\n",
    );
    for u in UNITS {
        s.push_str(&format!("| `{}` | {} | {} | {:e} | {} |\n", u.symbol, u.name, u.dimension, u.factor, u.offset));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-9 * b.abs().max(1.0)
    }

    #[test]
    fn invalid_dimensional_operations_are_rejected() {
        let kg = Quantity::new(1.0, "kg").unwrap();
        let m = Quantity::new(1.0, "m").unwrap();
        assert!(kg.try_add(m).is_err());
        assert!(kg.try_sub(m).is_err());
        assert!(kg.value_in("m").is_err());
        assert!(convert(1.0, "t/ha", "mm").is_err());
        assert!(convert(1.0, "kg", "furlong").is_err());
        assert!(Quantity::new(f64::NAN, "kg").is_err());
        assert!(kg.try_div(Quantity::new(0.0, "m").unwrap()).is_err());
    }

    #[test]
    fn derived_quantities_carry_dimensions() {
        let rate = Quantity::new(120.0, "kg/ha").unwrap();
        let area = Quantity::new(2.5, "ha").unwrap();
        let mass = rate.try_mul(area).unwrap();
        assert!(close(mass.value_in("kg").unwrap(), 300.0));
        assert!(mass.value_in("kg/ha").is_err());
        let depth = Quantity::new(25.0, "mm").unwrap();
        let volume = depth.try_mul(Quantity::new(1.0, "ha").unwrap()).unwrap();
        assert!(close(volume.value_in("m3").unwrap(), 250.0), "25 mm over 1 ha is 250 m3");
        let speed = Quantity::new(36.0, "km/h").unwrap();
        assert!(close(speed.value_in("m/s").unwrap(), 10.0));
        let ratio = Quantity::new(3.0, "t").unwrap().try_div(Quantity::new(1.5, "t").unwrap()).unwrap();
        assert_eq!(ratio.dimension, Dimension::NONE);
        assert!(close(ratio.value_in("1").unwrap(), 2.0));
    }

    #[test]
    fn temperature_semantics() {
        let a = Quantity::new(20.0, "Cel").unwrap();
        let b = Quantity::new(10.0, "Cel").unwrap();
        assert!(a.try_add(b).is_err(), "absolute + absolute");
        let diff = a.try_sub(b).unwrap();
        assert!(diff.difference);
        assert!(close(diff.value_in("K").unwrap(), 10.0));
        assert!(close(diff.value_in("Cel").unwrap(), 10.0), "a difference ignores offsets");
        let back = b.try_add(diff).unwrap();
        assert!(!back.difference && close(back.value_in("Cel").unwrap(), 20.0));
        assert!(close(convert(212.0, "degF", "Cel").unwrap(), 100.0));
        assert!(close(convert(0.0, "Cel", "K").unwrap(), 273.15));
        assert!(a.scale(2.0).is_err() && a.try_mul(a).is_err());
        assert!(diff.try_sub(a).is_err());
        assert!(close(Quantity::difference(9.0, "degF").unwrap().value_in("K").unwrap(), 5.0));
    }

    #[test]
    fn agrees_with_agronomy_conversions() {
        for (from, to) in [("lb/ac", "kg/ha"), ("kg/ha", "t/ha"), ("ac", "ha"), ("in", "mm"), ("degF", "Cel")] {
            for v in [0.0, 1.0, 37.5, 1234.0] {
                let a = convert(v, from, to).unwrap();
                let b = crate::agronomy::convert(v, from, to).unwrap();
                assert!(close(a, b), "{v} {from}->{to}: {a} vs {b}");
            }
        }
    }

    #[test]
    fn catalogue_is_consistent() {
        let mut seen = std::collections::HashSet::new();
        for u in UNITS {
            assert!(seen.insert(u.symbol), "duplicate {}", u.symbol);
            assert!(u.factor.is_finite() && u.factor > 0.0);
            assert!(u.offset == 0.0 || u.dimension == Dimension::TEMPERATURE);
            assert!(apsi_core_unit_code_ok(u.symbol), "{}", u.symbol);
        }
        assert!(catalogue_markdown().lines().count() > UNITS.len());
    }

    /// Mirrors the unit-code rule in the integration contracts.
    fn apsi_core_unit_code_ok(s: &str) -> bool {
        !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric() || b"/.^-_%[]()'".contains(&b))
    }

    proptest! {
        #[test]
        fn conversions_round_trip(v in -1e6f64..1e6, i in 0usize..UNITS.len(), j in 0usize..UNITS.len()) {
            let (a, b) = (UNITS[i], UNITS[j]);
            match convert(v, a.symbol, b.symbol) {
                Ok(x) => {
                    prop_assert_eq!(a.dimension, b.dimension);
                    prop_assert!(close(convert(x, b.symbol, a.symbol).unwrap(), v));
                }
                Err(_) => prop_assert_ne!(a.dimension, b.dimension),
            }
        }
    }
}
