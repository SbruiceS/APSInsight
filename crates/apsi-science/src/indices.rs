//! Remote sensing vegetation and water indices from surface reflectance (0..1).
//!
//! Formula registry with the bands each index needs, so band compatibility can be
//! checked before computing. A pixel whose bands include the nodata value, or whose
//! denominator is zero, yields `None` (flagged), never NaN.

use crate::{Result, ScienceError, in_range};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Band {
    Blue,
    Green,
    Red,
    Nir,
    Swir1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Index {
    Ndvi,
    Evi,
    Savi,
    Ndwi,
    Ndmi,
    Gndvi,
}

#[derive(Debug, Clone, Serialize)]
pub struct IndexSpec {
    pub index: Index,
    pub formula: &'static str,
    pub bands: &'static [Band],
    pub valid_range: (f64, f64),
    pub reference: &'static str,
}

pub const REGISTRY: [IndexSpec; 6] = [
    IndexSpec {
        index: Index::Ndvi,
        formula: "(NIR - Red) / (NIR + Red)",
        bands: &[Band::Nir, Band::Red],
        valid_range: (-1.0, 1.0),
        reference: "Rouse et al. 1974",
    },
    IndexSpec {
        index: Index::Evi,
        formula: "2.5 * (NIR - Red) / (NIR + 6*Red - 7.5*Blue + 1)",
        bands: &[Band::Nir, Band::Red, Band::Blue],
        valid_range: (-1.0, 1.0),
        reference: "Huete et al. 2002",
    },
    IndexSpec {
        index: Index::Savi,
        formula: "(1 + L) * (NIR - Red) / (NIR + Red + L), L = 0.5",
        bands: &[Band::Nir, Band::Red],
        valid_range: (-1.5, 1.5),
        reference: "Huete 1988",
    },
    IndexSpec {
        index: Index::Ndwi,
        formula: "(Green - NIR) / (Green + NIR)",
        bands: &[Band::Green, Band::Nir],
        valid_range: (-1.0, 1.0),
        reference: "McFeeters 1996",
    },
    IndexSpec {
        index: Index::Ndmi,
        formula: "(NIR - SWIR1) / (NIR + SWIR1)",
        bands: &[Band::Nir, Band::Swir1],
        valid_range: (-1.0, 1.0),
        reference: "Gao 1996 (NDWI-Gao / NDMI)",
    },
    IndexSpec {
        index: Index::Gndvi,
        formula: "(NIR - Green) / (NIR + Green)",
        bands: &[Band::Nir, Band::Green],
        valid_range: (-1.0, 1.0),
        reference: "Gitelson et al. 1996",
    },
];

pub fn spec(index: Index) -> &'static IndexSpec {
    REGISTRY.iter().find(|s| s.index == index).expect("every index is registered")
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Pixel {
    #[serde(default)]
    pub blue: Option<f64>,
    #[serde(default)]
    pub green: Option<f64>,
    #[serde(default)]
    pub red: Option<f64>,
    #[serde(default)]
    pub nir: Option<f64>,
    #[serde(default)]
    pub swir1: Option<f64>,
}

impl Pixel {
    fn band(&self, b: Band) -> Option<f64> {
        match b {
            Band::Blue => self.blue,
            Band::Green => self.green,
            Band::Red => self.red,
            Band::Nir => self.nir,
            Band::Swir1 => self.swir1,
        }
    }
}

/// Checks a scene's available bands against the index requirements.
pub fn missing_bands(index: Index, available: &[Band]) -> Vec<Band> {
    spec(index).bands.iter().copied().filter(|b| !available.contains(b)).collect()
}

fn nd(a: f64, b: f64) -> Option<f64> {
    let d = a + b;
    (d.abs() > f64::EPSILON).then(|| (a - b) / d)
}

/// Computes one index for one pixel. Reflectances must be within [0, 1] (values
/// slightly outside are common from atmospheric correction; > 1.5 is rejected).
/// `Ok(None)` means nodata / undefined at this pixel.
pub fn compute(index: Index, px: &Pixel, nodata: Option<f64>) -> Result<Option<f64>> {
    let mut v = [0.0; 3];
    for (slot, b) in v.iter_mut().zip(spec(index).bands) {
        let Some(x) = px.band(*b) else {
            return Err(ScienceError::Invalid(format!("{b:?} band required for {index:?}")));
        };
        if nodata.is_some_and(|n| n == x) {
            return Ok(None);
        }
        *slot = in_range(&format!("{b:?} reflectance"), x, -0.5, 1.5)?;
    }
    let out = match index {
        Index::Ndvi | Index::Ndmi | Index::Gndvi => nd(v[0], v[1]),
        Index::Ndwi => nd(v[0], v[1]),
        Index::Savi => {
            let l = 0.5;
            let d = v[0] + v[1] + l;
            (d.abs() > f64::EPSILON).then(|| (1.0 + l) * (v[0] - v[1]) / d)
        }
        Index::Evi => {
            let d = v[0] + 6.0 * v[1] - 7.5 * v[2] + 1.0;
            (d.abs() > f64::EPSILON).then(|| 2.5 * (v[0] - v[1]) / d)
        }
    };
    Ok(out.filter(|x| x.is_finite()))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    pub valid_pixels: usize,
    pub flagged_pixels: usize,
    pub mean: Option<f64>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    /// Pixels outside the index's physical range (possible sensor/processing artefacts).
    pub out_of_range: usize,
}

pub fn summarize(index: Index, pixels: &[Pixel], nodata: Option<f64>) -> Result<IndexStats> {
    let (lo, hi) = spec(index).valid_range;
    let mut vals = Vec::with_capacity(pixels.len());
    let mut flagged = 0;
    for p in pixels {
        match compute(index, p, nodata)? {
            Some(v) => vals.push(v),
            None => flagged += 1,
        }
    }
    let n = vals.len();
    Ok(IndexStats {
        valid_pixels: n,
        flagged_pixels: flagged,
        mean: (n > 0).then(|| vals.iter().sum::<f64>() / n as f64),
        min: vals.iter().copied().reduce(f64::min),
        max: vals.iter().copied().reduce(f64::max),
        out_of_range: vals.iter().filter(|v| **v < lo || **v > hi).count(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn px(red: f64, nir: f64) -> Pixel {
        Pixel { red: Some(red), nir: Some(nir), blue: Some(0.04), green: Some(0.08), swir1: Some(0.2) }
    }

    #[test]
    fn known_values() {
        let p = px(0.08, 0.40);
        assert!((compute(Index::Ndvi, &p, None).unwrap().unwrap() - 0.666_666_666_7).abs() < 1e-9);
        let savi = compute(Index::Savi, &p, None).unwrap().unwrap();
        assert!((savi - 1.5 * 0.32 / 0.98).abs() < 1e-12);
        let evi = compute(Index::Evi, &p, None).unwrap().unwrap();
        assert!((evi - 2.5 * 0.32 / (0.40 + 0.48 - 0.3 + 1.0)).abs() < 1e-12);
        assert!(compute(Index::Ndwi, &p, None).unwrap().unwrap() < 0.0);
    }

    #[test]
    fn nodata_zero_denominator_and_missing_bands() {
        assert_eq!(compute(Index::Ndvi, &px(-9999.0, 0.3), Some(-9999.0)).unwrap(), None);
        assert_eq!(compute(Index::Ndvi, &px(0.0, 0.0), None).unwrap(), None);
        assert!(compute(Index::Ndvi, &Pixel { red: Some(0.1), ..Default::default() }, None).is_err());
        assert!(compute(Index::Ndvi, &px(f64::NAN, 0.3), None).is_err());
        assert_eq!(missing_bands(Index::Evi, &[Band::Red, Band::Nir]), vec![Band::Blue]);
        let s = summarize(Index::Ndvi, &[px(0.1, 0.5), px(0.0, 0.0), px(0.2, 0.2)], None).unwrap();
        assert_eq!((s.valid_pixels, s.flagged_pixels), (2, 1));
    }

    proptest! {
        #[test]
        fn normalized_difference_is_bounded(red in 0.0f64..1.0, nir in 0.0f64..1.0) {
            if let Some(v) = compute(Index::Ndvi, &px(red, nir), None).unwrap() {
                prop_assert!((-1.0..=1.0).contains(&v));
            }
        }
    }
}
