//! Pure-Rust reference kernels. Each mirrors the C++ algorithm exactly (same
//! pairwise block size, same loop order) so results can be compared bit for
//! bit. Callers must pass already-validated arguments; see the crate root.

const PAIRWISE_BLOCK: usize = 128;
const TILE: usize = 64;

pub fn sum(x: &[f64]) -> f64 {
    if x.len() <= PAIRWISE_BLOCK {
        let mut s = 0.0;
        for v in x {
            s += v;
        }
        return s;
    }
    let (a, b) = x.split_at(x.len() / 2);
    sum(a) + sum(b)
}

pub fn dot(a: &[f64], b: &[f64]) -> f64 {
    if a.len() <= PAIRWISE_BLOCK {
        let mut s = 0.0;
        for (x, y) in a.iter().zip(b) {
            s += x * y;
        }
        return s;
    }
    let h = a.len() / 2;
    dot(&a[..h], &b[..h]) + dot(&a[h..], &b[h..])
}

pub fn gemm(m: usize, k: usize, n: usize, a: &[f64], b: &[f64]) -> Vec<f64> {
    let mut c = vec![0.0; m * n];
    for kk in (0..k).step_by(TILE) {
        let k_end = k.min(kk + TILE);
        for jj in (0..n).step_by(TILE) {
            let j_end = n.min(jj + TILE);
            for i in 0..m {
                for p in kk..k_end {
                    let aip = a[i * k + p];
                    for j in jj..j_end {
                        c[i * n + j] += aip * b[p * n + j];
                    }
                }
            }
        }
    }
    c
}

pub fn ndvi(red: &[f64], nir: &[f64], nodata: Option<f64>) -> Vec<f64> {
    red.iter()
        .zip(nir)
        .map(|(&r, &v)| {
            let missing = r.is_nan() || v.is_nan() || nodata.is_some_and(|nd| r == nd || v == nd);
            let den = v + r;
            if missing || den == 0.0 { f64::NAN } else { (v - r) / den }
        })
        .collect()
}

pub fn moving_mean(x: &[f64], window: usize) -> Vec<f64> {
    (0..x.len()).map(|i| if i + 1 < window { f64::NAN } else { sum(&x[i + 1 - window..=i]) / window as f64 }).collect()
}

pub fn fft_power(x: &[f64]) -> Vec<f64> {
    let n = x.len();
    let mut re = x.to_vec();
    let mut im = vec![0.0; n];
    let mut j = 0usize;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let angle = -2.0 * std::f64::consts::PI / len as f64;
        let (wr, wi) = (angle.cos(), angle.sin());
        for start in (0..n).step_by(len) {
            let (mut cr, mut ci) = (1.0, 0.0);
            for t in 0..len / 2 {
                let (a, b) = (start + t, start + t + len / 2);
                let (vr, vi) = (re[b] * cr - im[b] * ci, re[b] * ci + im[b] * cr);
                re[b] = re[a] - vr;
                im[b] = im[a] - vi;
                re[a] += vr;
                im[a] += vi;
                let next = cr * wr - ci * wi;
                ci = cr * wi + ci * wr;
                cr = next;
            }
        }
        len <<= 1;
    }
    (0..=n / 2).map(|k| (re[k] * re[k] + im[k] * im[k]) / n as f64).collect()
}
