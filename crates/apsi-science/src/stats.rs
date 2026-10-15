//! Statistics and inference: descriptive statistics, confidence intervals, Welch's
//! t-test, effect sizes, multiple-testing correction, simple linear regression with
//! diagnostics, and forecast error metrics. Assumptions are reported, not hidden.

use crate::{Result, ScienceError, finite};
use serde::{Deserialize, Serialize};

fn check(xs: &[f64], min_n: usize) -> Result<()> {
    if xs.len() < min_n {
        return Err(ScienceError::Insufficient(format!("need at least {min_n} observations, got {}", xs.len())));
    }
    for x in xs {
        finite("observation", *x)?;
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Descriptive {
    pub n: usize,
    pub mean: f64,
    pub sd: Option<f64>,
    pub min: f64,
    pub q1: f64,
    pub median: f64,
    pub q3: f64,
    pub max: f64,
}

/// Linear-interpolation quantile (type 7, the R/NumPy default).
pub fn quantile(sorted: &[f64], q: f64) -> f64 {
    let h = (sorted.len() - 1) as f64 * q;
    let lo = h.floor() as usize;
    let hi = h.ceil() as usize;
    sorted[lo] + (h - lo as f64) * (sorted[hi] - sorted[lo])
}

pub fn mean(xs: &[f64]) -> Result<f64> {
    check(xs, 1)?;
    Ok(xs.iter().sum::<f64>() / xs.len() as f64)
}

/// Sample variance (n - 1 denominator).
pub fn variance(xs: &[f64]) -> Result<f64> {
    check(xs, 2)?;
    let m = mean(xs)?;
    Ok(xs.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (xs.len() - 1) as f64)
}

pub fn describe(xs: &[f64]) -> Result<Descriptive> {
    check(xs, 1)?;
    let mut s = xs.to_vec();
    s.sort_by(f64::total_cmp);
    Ok(Descriptive {
        n: xs.len(),
        mean: mean(xs)?,
        sd: variance(xs).ok().map(f64::sqrt),
        min: s[0],
        q1: quantile(&s, 0.25),
        median: quantile(&s, 0.5),
        q3: quantile(&s, 0.75),
        max: s[s.len() - 1],
    })
}

fn ln_gamma(x: f64) -> f64 {
    // Lanczos approximation (g = 7, n = 9).
    const C: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    if x < 0.5 {
        return (std::f64::consts::PI / (std::f64::consts::PI * x).sin()).ln() - ln_gamma(1.0 - x);
    }
    let x = x - 1.0;
    let mut a = C[0];
    let t = x + 7.5;
    for (i, c) in C.iter().enumerate().skip(1) {
        a += c / (x + i as f64);
    }
    0.5 * (2.0 * std::f64::consts::PI).ln() + (x + 0.5) * t.ln() - t + a.ln()
}

/// Regularized incomplete beta I_x(a, b) via Lentz's continued fraction.
fn inc_beta(x: f64, a: f64, b: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let front = (ln_gamma(a + b) - ln_gamma(a) - ln_gamma(b) + a * x.ln() + b * (1.0 - x).ln()).exp();
    if x > (a + 1.0) / (a + b + 2.0) {
        return 1.0 - inc_beta(1.0 - x, b, a);
    }
    let tiny = 1e-300;
    let (mut c, mut d) = (1.0, 1.0 - (a + b) * x / (a + 1.0));
    if d.abs() < tiny {
        d = tiny;
    }
    d = 1.0 / d;
    let mut f = d;
    for m in 1..300 {
        let m = m as f64;
        for step in 0..2 {
            let num = if step == 0 {
                m * (b - m) * x / ((a + 2.0 * m - 1.0) * (a + 2.0 * m))
            } else {
                -(a + m) * (a + b + m) * x / ((a + 2.0 * m) * (a + 2.0 * m + 1.0))
            };
            d = 1.0 + num * d;
            if d.abs() < tiny {
                d = tiny;
            }
            c = 1.0 + num / c;
            if c.abs() < tiny {
                c = tiny;
            }
            d = 1.0 / d;
            f *= c * d;
        }
        if (c * d - 1.0).abs() < 1e-15 {
            break;
        }
    }
    front * f / a
}

/// Student t cumulative distribution function.
pub fn t_cdf(t: f64, df: f64) -> f64 {
    let x = df / (df + t * t);
    let tail = 0.5 * inc_beta(x, df / 2.0, 0.5);
    if t >= 0.0 { 1.0 - tail } else { tail }
}

/// Student t quantile by bisection on the CDF (monotone, so this always converges).
pub fn t_quantile(p: f64, df: f64) -> f64 {
    let (mut lo, mut hi) = (-1e3, 1e3);
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if t_cdf(mid, df) < p { lo = mid } else { hi = mid }
    }
    0.5 * (lo + hi)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interval {
    pub estimate: f64,
    pub lower: f64,
    pub upper: f64,
    pub confidence: f64,
    pub method: String,
}

/// t-based confidence interval for the mean. Assumes independent observations from
/// an approximately normal population (or n large enough for the CLT).
pub fn mean_ci(xs: &[f64], confidence: f64) -> Result<Interval> {
    check(xs, 2)?;
    if !(0.5..1.0).contains(&confidence) {
        return Err(ScienceError::Invalid("confidence must be in [0.5, 1)".into()));
    }
    let m = mean(xs)?;
    let se = (variance(xs)? / xs.len() as f64).sqrt();
    let t = t_quantile(1.0 - (1.0 - confidence) / 2.0, (xs.len() - 1) as f64);
    Ok(Interval { estimate: m, lower: m - t * se, upper: m + t * se, confidence, method: "Student t, n-1 df".into() })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TTest {
    pub t: f64,
    pub df: f64,
    pub p_value: f64,
    pub mean_difference: f64,
    pub cohens_d: f64,
    pub assumptions: Vec<String>,
}

/// Welch's two-sample t-test (unequal variances), two-sided.
pub fn welch_t_test(a: &[f64], b: &[f64]) -> Result<TTest> {
    check(a, 2)?;
    check(b, 2)?;
    let (ma, mb) = (mean(a)?, mean(b)?);
    let (va, vb) = (variance(a)?, variance(b)?);
    let (na, nb) = (a.len() as f64, b.len() as f64);
    let se2 = va / na + vb / nb;
    if se2 <= 0.0 {
        return Err(ScienceError::Undefined("both samples have zero variance".into()));
    }
    let t = (ma - mb) / se2.sqrt();
    let df = se2.powi(2) / ((va / na).powi(2) / (na - 1.0) + (vb / nb).powi(2) / (nb - 1.0));
    let pooled = (((na - 1.0) * va + (nb - 1.0) * vb) / (na + nb - 2.0)).sqrt();
    let mut assumptions =
        vec!["independent observations within and between groups".to_string(), "approximately normal group distributions or large samples".to_string()];
    if a.len() < 15 || b.len() < 15 {
        assumptions.push("small samples: normality matters; consider a rank-based test".into());
    }
    Ok(TTest {
        t,
        df,
        p_value: 2.0 * (1.0 - t_cdf(t.abs(), df)),
        mean_difference: ma - mb,
        cohens_d: if pooled > 0.0 { (ma - mb) / pooled } else { 0.0 },
        assumptions,
    })
}

/// Benjamini-Hochberg adjusted p-values (false discovery rate), input order preserved.
pub fn benjamini_hochberg(p: &[f64]) -> Result<Vec<f64>> {
    for v in p {
        crate::in_range("p-value", *v, 0.0, 1.0)?;
    }
    let n = p.len();
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&i, &j| p[i].total_cmp(&p[j]));
    let mut adj = vec![0.0; n];
    let mut running = 1.0f64;
    for (rank, &i) in order.iter().enumerate().rev() {
        running = running.min(p[i] * n as f64 / (rank + 1) as f64);
        adj[i] = running.min(1.0);
    }
    Ok(adj)
}

pub fn bonferroni(p: &[f64]) -> Result<Vec<f64>> {
    for v in p {
        crate::in_range("p-value", *v, 0.0, 1.0)?;
    }
    Ok(p.iter().map(|v| (v * p.len() as f64).min(1.0)).collect())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Regression {
    pub slope: f64,
    pub intercept: f64,
    pub r_squared: f64,
    pub residual_se: f64,
    pub n: usize,
    pub residuals: Vec<f64>,
    pub slope_p_value: f64,
}

/// Ordinary least squares y = intercept + slope * x.
pub fn linear_regression(x: &[f64], y: &[f64]) -> Result<Regression> {
    if x.len() != y.len() {
        return Err(ScienceError::Invalid("x and y lengths differ".into()));
    }
    check(x, 3)?;
    check(y, 3)?;
    let (mx, my) = (mean(x)?, mean(y)?);
    let sxx: f64 = x.iter().map(|v| (v - mx).powi(2)).sum();
    if sxx == 0.0 {
        return Err(ScienceError::Undefined("x has zero variance".into()));
    }
    let sxy: f64 = x.iter().zip(y).map(|(a, b)| (a - mx) * (b - my)).sum();
    let slope = sxy / sxx;
    let intercept = my - slope * mx;
    let residuals: Vec<f64> = x.iter().zip(y).map(|(a, b)| b - (intercept + slope * a)).collect();
    let sse: f64 = residuals.iter().map(|r| r * r).sum();
    let sst: f64 = y.iter().map(|v| (v - my).powi(2)).sum();
    let n = x.len();
    let residual_se = (sse / (n - 2) as f64).sqrt();
    let se_slope = residual_se / sxx.sqrt();
    let slope_p_value = if se_slope > 0.0 { 2.0 * (1.0 - t_cdf((slope / se_slope).abs(), (n - 2) as f64)) } else { 0.0 };
    Ok(Regression { slope, intercept, r_squared: if sst > 0.0 { 1.0 - sse / sst } else { 1.0 }, residual_se, n, residuals, slope_p_value })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastErrors {
    pub n: usize,
    pub mae: f64,
    pub rmse: f64,
    pub bias: f64,
    /// Mean absolute percentage error over observations with |actual| > 0; null if none.
    pub mape: Option<f64>,
}

pub fn forecast_errors(actual: &[f64], predicted: &[f64]) -> Result<ForecastErrors> {
    if actual.len() != predicted.len() {
        return Err(ScienceError::Invalid("actual and predicted lengths differ".into()));
    }
    check(actual, 1)?;
    check(predicted, 1)?;
    let n = actual.len() as f64;
    let e: Vec<f64> = predicted.iter().zip(actual).map(|(p, a)| p - a).collect();
    let pct: Vec<f64> = actual.iter().zip(&e).filter(|(a, _)| a.abs() > 0.0).map(|(a, e)| (e / a).abs()).collect();
    Ok(ForecastErrors {
        n: actual.len(),
        mae: e.iter().map(|v| v.abs()).sum::<f64>() / n,
        rmse: (e.iter().map(|v| v * v).sum::<f64>() / n).sqrt(),
        bias: e.iter().sum::<f64>() / n,
        mape: (!pct.is_empty()).then(|| 100.0 * pct.iter().sum::<f64>() / pct.len() as f64),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn descriptive() {
        let d = describe(&[1.0, 2.0, 3.0, 4.0]).unwrap();
        assert_eq!((d.mean, d.median, d.q1, d.q3), (2.5, 2.5, 1.75, 3.25));
        assert!((d.sd.unwrap() - 1.290_994_448_7).abs() < 1e-9);
        assert!(describe(&[]).is_err());
        assert!(describe(&[f64::NAN]).is_err());
        assert!(describe(&[5.0]).unwrap().sd.is_none());
    }

    #[test]
    fn t_distribution_matches_tables() {
        assert!((t_cdf(0.0, 10.0) - 0.5).abs() < 1e-12);
        assert!((t_quantile(0.975, 10.0) - 2.228_138_85).abs() < 1e-6);
        assert!((t_quantile(0.975, 1.0) - 12.706_204_7).abs() < 1e-4);
        assert!((t_quantile(0.95, 30.0) - 1.697_260_9).abs() < 1e-6);
    }

    #[test]
    fn welch_and_ci() {
        let a = [5.1, 4.9, 5.3, 5.0, 5.2, 4.8];
        let b = [5.9, 6.1, 6.0, 5.8, 6.2, 6.3];
        let r = welch_t_test(&a, &b).unwrap();
        assert!(r.p_value < 1e-5 && r.t < 0.0 && r.cohens_d < -3.0);
        assert!(r.assumptions.len() == 3);
        let ci = mean_ci(&a, 0.95).unwrap();
        assert!(ci.lower < 5.05 && ci.upper > 5.05);
        assert!(welch_t_test(&[1.0, 1.0], &[1.0, 1.0]).is_err());
    }

    #[test]
    fn multiple_testing() {
        let adj = benjamini_hochberg(&[0.01, 0.04, 0.03, 0.005]).unwrap();
        let expected = [0.02, 0.04, 0.04, 0.02];
        for (a, e) in adj.iter().zip(expected) {
            assert!((a - e).abs() < 1e-12, "{adj:?}");
        }
        assert_eq!(bonferroni(&[0.01, 0.5]).unwrap(), vec![0.02, 1.0]);
        assert!(benjamini_hochberg(&[1.5]).is_err());
    }

    #[test]
    fn regression_recovers_line() {
        let x = [1.0, 2.0, 3.0, 4.0, 5.0];
        let y: Vec<f64> = x.iter().map(|v| 2.0 * v + 1.0).collect();
        let r = linear_regression(&x, &y).unwrap();
        assert!((r.slope - 2.0).abs() < 1e-12 && (r.intercept - 1.0).abs() < 1e-12 && (r.r_squared - 1.0).abs() < 1e-12);
        assert!(linear_regression(&[1.0, 1.0, 1.0], &[1.0, 2.0, 3.0]).is_err());
    }

    #[test]
    fn forecast_metrics() {
        let f = forecast_errors(&[10.0, 0.0, 20.0], &[12.0, 1.0, 18.0]).unwrap();
        assert!((f.mae - 5.0 / 3.0).abs() < 1e-12);
        assert!((f.bias - 1.0 / 3.0).abs() < 1e-12);
        assert!((f.mape.unwrap() - 15.0).abs() < 1e-9, "zero actuals excluded");
    }

    proptest! {
        #[test]
        fn bh_adjusted_are_valid_probabilities(p in proptest::collection::vec(0.0f64..=1.0, 1..40)) {
            let adj = benjamini_hochberg(&p).unwrap();
            for (a, o) in adj.iter().zip(&p) {
                prop_assert!(*a >= *o - 1e-12 && *a <= 1.0);
            }
        }

        #[test]
        fn t_cdf_is_monotone(a in -20.0f64..20.0, d in 0.0f64..5.0, df in 1.0f64..200.0) {
            prop_assert!(t_cdf(a, df) <= t_cdf(a + d, df) + 1e-12);
        }
    }
}
