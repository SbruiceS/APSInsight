//! Geospatial foundations on WGS 84 (EPSG:4326) coordinates: validation, distance,
//! geodesic polygon area, point-in-polygon, bounding boxes and Web Mercator
//! reprojection.

use crate::{Result, ScienceError, finite};
use serde::{Deserialize, Serialize};

/// Mean Earth radius (IUGG), metres.
pub const EARTH_RADIUS_M: f64 = 6_371_008.8;
const WEB_MERCATOR_R: f64 = 6_378_137.0;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LonLat {
    pub lon: f64,
    pub lat: f64,
}

impl LonLat {
    pub fn check(&self) -> Result<()> {
        finite("lon", self.lon)?;
        finite("lat", self.lat)?;
        if !(-180.0..=180.0).contains(&self.lon) || !(-90.0..=90.0).contains(&self.lat) {
            return Err(ScienceError::Invalid(format!("coordinate ({}, {}) outside WGS 84 bounds", self.lon, self.lat)));
        }
        Ok(())
    }
}

/// Great-circle distance (haversine), metres.
pub fn distance_m(a: LonLat, b: LonLat) -> Result<f64> {
    a.check()?;
    b.check()?;
    let (p1, p2) = (a.lat.to_radians(), b.lat.to_radians());
    let dp = p2 - p1;
    let dl = (b.lon - a.lon).to_radians();
    let h = (dp / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dl / 2.0).sin().powi(2);
    Ok(2.0 * EARTH_RADIUS_M * h.sqrt().min(1.0).asin())
}

/// Validates a closed ring: >= 4 positions, first == last, valid coordinates and
/// no repeated consecutive vertices.
pub fn check_ring(ring: &[LonLat]) -> Result<()> {
    if ring.len() < 4 {
        return Err(ScienceError::Invalid("a ring needs at least 4 positions".into()));
    }
    if ring.first() != ring.last() {
        return Err(ScienceError::Invalid("ring is not closed (first != last)".into()));
    }
    for p in ring {
        p.check()?;
    }
    if ring.windows(2).any(|w| w[0] == w[1]) {
        return Err(ScienceError::Invalid("ring has repeated consecutive vertices".into()));
    }
    Ok(())
}

/// Geodesic area of a ring on the sphere (Chamberlain & Duquette 2007), square
/// metres. Orientation independent. Accuracy ~0.3% versus the ellipsoid.
pub fn ring_area_m2(ring: &[LonLat]) -> Result<f64> {
    check_ring(ring)?;
    let mut total = 0.0;
    for w in ring.windows(2) {
        let (a, b) = (w[0], w[1]);
        total += (b.lon - a.lon).to_radians() * (2.0 + a.lat.to_radians().sin() + b.lat.to_radians().sin());
    }
    Ok((total * EARTH_RADIUS_M * EARTH_RADIUS_M / 2.0).abs())
}

/// Ray-casting point-in-polygon test (planar in lon/lat; valid for field-scale
/// polygons not crossing the antimeridian). Points on the boundary count as inside.
pub fn contains(ring: &[LonLat], p: LonLat) -> Result<bool> {
    check_ring(ring)?;
    p.check()?;
    let mut inside = false;
    for w in ring.windows(2) {
        let (a, b) = (w[0], w[1]);
        let cross = (b.lon - a.lon) * (p.lat - a.lat) - (b.lat - a.lat) * (p.lon - a.lon);
        let within = p.lon >= a.lon.min(b.lon) && p.lon <= a.lon.max(b.lon) && p.lat >= a.lat.min(b.lat) && p.lat <= a.lat.max(b.lat);
        if cross.abs() < 1e-12 && within {
            return Ok(true);
        }
        if (a.lat > p.lat) != (b.lat > p.lat) && p.lon < (b.lon - a.lon) * (p.lat - a.lat) / (b.lat - a.lat) + a.lon {
            inside = !inside;
        }
    }
    Ok(inside)
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BBox {
    pub min_lon: f64,
    pub min_lat: f64,
    pub max_lon: f64,
    pub max_lat: f64,
}

pub fn bbox(points: &[LonLat]) -> Result<BBox> {
    let first = points.first().ok_or_else(|| ScienceError::Insufficient("no points".into()))?;
    let mut b = BBox { min_lon: first.lon, min_lat: first.lat, max_lon: first.lon, max_lat: first.lat };
    for p in points {
        p.check()?;
        b.min_lon = b.min_lon.min(p.lon);
        b.min_lat = b.min_lat.min(p.lat);
        b.max_lon = b.max_lon.max(p.lon);
        b.max_lat = b.max_lat.max(p.lat);
    }
    Ok(b)
}

/// WGS 84 -> Web Mercator (EPSG:3857) metres. Latitude is limited to ±85.051129°.
pub fn to_web_mercator(p: LonLat) -> Result<(f64, f64)> {
    p.check()?;
    if p.lat.abs() > 85.051_129 {
        return Err(ScienceError::Invalid("latitude outside Web Mercator range".into()));
    }
    let x = WEB_MERCATOR_R * p.lon.to_radians();
    let y = WEB_MERCATOR_R * (std::f64::consts::FRAC_PI_4 + p.lat.to_radians() / 2.0).tan().ln();
    Ok((x, y))
}

pub fn from_web_mercator(x: f64, y: f64) -> Result<LonLat> {
    finite("x", x)?;
    finite("y", y)?;
    let lon = (x / WEB_MERCATOR_R).to_degrees();
    let lat = (2.0 * (y / WEB_MERCATOR_R).exp().atan() - std::f64::consts::FRAC_PI_2).to_degrees();
    let p = LonLat { lon, lat };
    p.check()?;
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn ll(lon: f64, lat: f64) -> LonLat {
        LonLat { lon, lat }
    }

    #[test]
    fn distance_known_values() {
        // Delhi -> Mumbai, ~1,148 km great-circle
        let d = distance_m(ll(77.2090, 28.6139), ll(72.8777, 19.0760)).unwrap();
        assert!((d / 1000.0 - 1148.0).abs() < 10.0, "{d}");
        assert!(distance_m(ll(0.0, 0.0), ll(0.0, 0.0)).unwrap().abs() < 1e-9);
        assert!(distance_m(ll(200.0, 0.0), ll(0.0, 0.0)).is_err());
    }

    #[test]
    fn area_of_one_degree_cell_at_equator() {
        let ring = [ll(0.0, 0.0), ll(1.0, 0.0), ll(1.0, 1.0), ll(0.0, 1.0), ll(0.0, 0.0)];
        let km2 = ring_area_m2(&ring).unwrap() / 1e6;
        assert!((km2 - 12_364.0).abs() < 50.0, "{km2}");
        let mut rev = ring;
        rev.reverse();
        assert!((ring_area_m2(&rev).unwrap() - ring_area_m2(&ring).unwrap()).abs() < 1e-6);
    }

    #[test]
    fn ring_validation_and_containment() {
        assert!(check_ring(&[ll(0.0, 0.0), ll(1.0, 0.0), ll(0.0, 0.0)]).is_err());
        assert!(check_ring(&[ll(0.0, 0.0), ll(1.0, 0.0), ll(1.0, 1.0), ll(0.0, 1.0)]).is_err());
        let sq = [ll(0.0, 0.0), ll(2.0, 0.0), ll(2.0, 2.0), ll(0.0, 2.0), ll(0.0, 0.0)];
        assert!(contains(&sq, ll(1.0, 1.0)).unwrap());
        assert!(contains(&sq, ll(2.0, 1.0)).unwrap(), "boundary is inside");
        assert!(!contains(&sq, ll(3.0, 1.0)).unwrap());
        let b = bbox(&sq).unwrap();
        assert_eq!((b.min_lon, b.max_lat), (0.0, 2.0));
    }

    proptest! {
        #[test]
        fn web_mercator_roundtrip(lon in -180.0f64..180.0, lat in -85.0f64..85.0) {
            let (x, y) = to_web_mercator(ll(lon, lat)).unwrap();
            let back = from_web_mercator(x, y).unwrap();
            prop_assert!((back.lon - lon).abs() < 1e-9 && (back.lat - lat).abs() < 1e-9);
        }

        #[test]
        fn distance_is_symmetric_and_nonnegative(a in -180.0f64..180.0, b in -90.0f64..90.0, c in -180.0f64..180.0, d in -90.0f64..90.0) {
            let x = distance_m(ll(a, b), ll(c, d)).unwrap();
            let y = distance_m(ll(c, d), ll(a, b)).unwrap();
            prop_assert!(x >= 0.0 && (x - y).abs() < 1e-6);
        }
    }
}
