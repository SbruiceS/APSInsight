//! Time-series utilities on (unix seconds, value) observations: ordering checks,
//! gap detection, late-arrival detection and interval aggregation (downsampling).

use crate::{Result, ScienceError, finite};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Obs {
    pub t: i64,
    pub v: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Gap {
    pub from: i64,
    pub to: i64,
    pub missing_intervals: i64,
}

/// Gaps longer than `tolerance` x `interval` between consecutive observations.
pub fn gaps(obs: &[Obs], interval: i64, tolerance: f64) -> Result<Vec<Gap>> {
    if interval <= 0 || !(1.0..=100.0).contains(&tolerance) {
        return Err(ScienceError::Invalid("interval must be > 0 and tolerance in [1, 100]".into()));
    }
    let mut s = obs.to_vec();
    s.sort_by_key(|o| o.t);
    Ok(s.windows(2)
        .filter(|w| (w[1].t - w[0].t) as f64 > interval as f64 * tolerance)
        .map(|w| Gap { from: w[0].t, to: w[1].t, missing_intervals: (w[1].t - w[0].t) / interval - 1 })
        .collect())
}

/// Indices of observations that arrived out of order (timestamp earlier than a
/// previously received one) — late arrivals in a stream.
pub fn late_arrivals(received: &[Obs]) -> Vec<usize> {
    let mut max = i64::MIN;
    let mut late = Vec::new();
    for (i, o) in received.iter().enumerate() {
        if o.t < max {
            late.push(i);
        }
        max = max.max(o.t);
    }
    late
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Agg {
    Mean,
    Sum,
    Min,
    Max,
    Count,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bucket {
    pub start: i64,
    pub value: f64,
    pub count: usize,
}

/// Aggregates observations into fixed buckets aligned to multiples of `width`
/// seconds since the epoch. Empty buckets are omitted (not zero-filled).
pub fn downsample(obs: &[Obs], width: i64, agg: Agg) -> Result<Vec<Bucket>> {
    if width <= 0 {
        return Err(ScienceError::Invalid("bucket width must be > 0".into()));
    }
    let mut groups: BTreeMap<i64, Vec<f64>> = BTreeMap::new();
    for o in obs {
        finite("value", o.v)?;
        groups.entry(o.t.div_euclid(width) * width).or_default().push(o.v);
    }
    Ok(groups
        .into_iter()
        .map(|(start, vs)| {
            let value = match agg {
                Agg::Mean => vs.iter().sum::<f64>() / vs.len() as f64,
                Agg::Sum => vs.iter().sum(),
                Agg::Min => vs.iter().copied().fold(f64::INFINITY, f64::min),
                Agg::Max => vs.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                Agg::Count => vs.len() as f64,
            };
            Bucket { start, value, count: vs.len() }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn o(t: i64, v: f64) -> Obs {
        Obs { t, v }
    }

    #[test]
    fn gap_and_late_detection() {
        let s = [o(0, 1.0), o(60, 1.0), o(300, 1.0), o(360, 1.0)];
        let g = gaps(&s, 60, 1.5).unwrap();
        assert_eq!(g, vec![Gap { from: 60, to: 300, missing_intervals: 3 }]);
        assert_eq!(late_arrivals(&[o(10, 0.0), o(20, 0.0), o(15, 0.0), o(30, 0.0)]), vec![2]);
        assert!(gaps(&s, 0, 1.5).is_err());
    }

    #[test]
    fn downsampling() {
        let s = [o(0, 1.0), o(30, 3.0), o(3600, 10.0), o(-1, 7.0)];
        let b = downsample(&s, 3600, Agg::Mean).unwrap();
        assert_eq!(b.len(), 3);
        assert_eq!((b[0].start, b[0].value), (-3600, 7.0), "negative times bucket correctly");
        assert_eq!((b[1].value, b[1].count), (2.0, 2));
        assert_eq!(downsample(&s, 3600, Agg::Count).unwrap()[2].value, 1.0);
        assert!(downsample(&[o(0, f64::NAN)], 60, Agg::Sum).is_err());
    }
}
