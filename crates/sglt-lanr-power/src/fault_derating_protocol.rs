//! N-k fault-tolerant seed-mass derating protocol (sglt.txt §3) built on the
//! GUM Supplement 1 dual-number engine transferred from `sys1own/shbt-cf`
//! (`crates/shbt-metrology-gum/src/{dual,engine}.rs`).
//!
//! When `k` LANR modules fail, the net power loss converts to a 512-bit
//! register bit decrement
//!
//!   ΔN(k) = ⌊k · 507.32 / P_bit⌋,  P_bit = 8.9506e-4 W/bit
//!
//! which scales the boundary seed mass `M_seed(N−k)` from `1e-6 M_☉`
//! (906.00 kW) down to `1e-7 M_☉` (90.60 kW) and extends the focal baseline
//!
//!   f(M_seed) = f0 · M_seed,nominal / M_seed(N−k)
//!
//! from `f0 = 169.30 m` to `f_max = 1,692.99 m` at `k = 1,607`.

use crate::module_ledger::{DEMAND_W, K_MAX_SURVIVABLE, MODULE_NET_W};

/// Power-to-bit decrement conversion (W/bit).
pub const P_BIT_W: f64 = 8.9506e-4;
/// Nominal boundary seed mass (M_☉) at 906.00 kW net entropy balance.
pub const M_SEED_NOMINAL_MSUN: f64 = 1.0e-6;
/// Degraded seed-mass floor (M_☉) at the 90.60 kW limit (k = 1,607).
pub const M_SEED_MIN_MSUN: f64 = 1.0e-7;
/// Nominal inter-craft focal baseline (m).
pub const F0_M: f64 = 169.30;
/// Maximum focal baseline (m) at `k = 1,607`.
pub const F_MAX_M: f64 = 1692.99;

// ---------------------------------------------------------------------------
// GUM Supplement 1 dual-number engine (transferred from shbt-metrology-gum)
// ---------------------------------------------------------------------------

/// A scalar value with its gradient w.r.t. the independent inputs.
#[derive(Clone, Debug, PartialEq)]
pub struct DualNum {
    /// Nominal value.
    pub value: f64,
    /// Partial derivatives in input order.
    pub derivatives: Vec<f64>,
}

impl DualNum {
    /// Independent input variable.
    pub fn variable(value: f64, index: usize, dimension: usize) -> Self {
        let mut derivatives = vec![0.0; dimension];
        derivatives[index] = 1.0;
        Self { value, derivatives }
    }

    /// Constant in a gradient space.
    pub fn constant(value: f64, dimension: usize) -> Self {
        Self {
            value,
            derivatives: vec![0.0; dimension],
        }
    }

    /// `exp(self)` via the chain rule.
    pub fn exp(self) -> Self {
        let scale = self.value.exp();
        Self {
            value: scale,
            derivatives: self.derivatives.into_iter().map(|d| scale * d).collect(),
        }
    }

    /// `ln(self)` via the chain rule.
    pub fn ln(self) -> Self {
        Self {
            value: self.value.ln(),
            derivatives: self
                .derivatives
                .into_iter()
                .map(|d| d / self.value)
                .collect(),
        }
    }

    /// `self^c` for constant `c`.
    pub fn powf(self, exponent: f64) -> Self {
        let value = self.value.powf(exponent);
        let scale = exponent * self.value.powf(exponent - 1.0);
        Self {
            value,
            derivatives: self.derivatives.into_iter().map(|d| scale * d).collect(),
        }
    }
}

macro_rules! dual_binop {
    ($trait:ident, $method:ident, $vf:expr, $df:expr) => {
        impl std::ops::$trait for DualNum {
            type Output = Self;
            fn $method(self, rhs: Self) -> Self {
                debug_assert_eq!(self.derivatives.len(), rhs.derivatives.len());
                let lhs_d = self.derivatives.clone();
                let rhs_d = rhs.derivatives.clone();
                Self {
                    value: $vf(self.value, rhs.value),
                    derivatives: lhs_d
                        .into_iter()
                        .zip(rhs_d)
                        .map(|(a, b)| $df(a, b, self.value, rhs.value))
                        .collect(),
                }
            }
        }
    };
}

dual_binop!(
    Add,
    add,
    |a: f64, b: f64| a + b,
    |a, b, _x: f64, _y: f64| a + b
);
dual_binop!(
    Sub,
    sub,
    |a: f64, b: f64| a - b,
    |a, b, _x: f64, _y: f64| a - b
);
dual_binop!(Mul, mul, |a: f64, b: f64| a * b, |a, b, x: f64, y: f64| a
    * y
    + b * x);
dual_binop!(Div, div, |a: f64, b: f64| a / b, |a, b, x: f64, y: f64| (a
    * y
    - b * x)
    / (y * y));

/// GUM covariance-propagation report.
#[derive(Clone, Debug, PartialEq)]
pub struct GumReport {
    /// Output nominal values.
    pub values: Vec<f64>,
    /// Jacobian (output × input).
    pub jacobian: Vec<Vec<f64>>,
    /// Output covariance `J Σ Jᵀ`.
    pub covariance: Vec<Vec<f64>>,
}

/// First-order GUM propagation of input covariance through a dual model.
pub fn propagate_uncertainty_gum<F>(means: &[f64], covariance: &[Vec<f64>], model: F) -> GumReport
where
    F: Fn(&[DualNum]) -> Vec<DualNum>,
{
    let n = means.len();
    assert_eq!(covariance.len(), n, "covariance dimension mismatch");
    assert!(
        covariance.iter().all(|row| row.len() == n),
        "covariance must be square"
    );
    let inputs: Vec<_> = means
        .iter()
        .enumerate()
        .map(|(i, &x)| DualNum::variable(x, i, n))
        .collect();
    let outputs = model(&inputs);
    let values: Vec<f64> = outputs.iter().map(|o| o.value).collect();
    let jacobian: Vec<Vec<f64>> = outputs.iter().map(|o| o.derivatives.clone()).collect();
    let m = outputs.len();
    let mut cov = vec![vec![0.0; m]; m];
    for i in 0..m {
        for j in 0..m {
            cov[i][j] = jacobian[i]
                .iter()
                .enumerate()
                .map(|(a, &da)| {
                    da * jacobian[j]
                        .iter()
                        .enumerate()
                        .map(|(b, &db)| covariance[a][b] * db)
                        .sum::<f64>()
                })
                .sum();
        }
    }
    GumReport {
        values,
        jacobian,
        covariance: cov,
    }
}

/// SplitMix64 PRNG for the GUM-S1 Monte Carlo loop (deterministic seed).
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Next uniform deviate in [0, 1).
    pub fn next_f64(&mut self) -> f64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        (z >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Standard normal deviate (Box–Muller).
    pub fn next_gaussian(&mut self) -> f64 {
        let u1 = (1.0 - self.next_f64()).max(f64::MIN_POSITIVE);
        let u2 = self.next_f64();
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }
}

/// GUM-S1 Monte Carlo summary of a scalar measurand.
#[derive(Clone, Debug)]
pub struct McReport {
    /// Model evaluations performed.
    pub evaluations: usize,
    /// Best estimate `ȳ`.
    pub mean: f64,
    /// Standard uncertainty `u(y)`.
    pub std_dev: f64,
    /// 95 % coverage interval.
    pub coverage_95: (f64, f64),
}

/// Runs `n` Monte Carlo evaluations of `model(x)` with `x ~ N(mean, std)`
/// scalar input — the SGLT derating use case (`N ≥ 1e6` supported).
pub fn run_gum_monte_carlo<F>(mean: f64, std: f64, model: F, n: usize, seed: u64) -> McReport
where
    F: Fn(f64) -> f64,
{
    let mut rng = SplitMix64::new(seed);
    let mut ys = Vec::with_capacity(n);
    for _ in 0..n {
        ys.push(model(mean + std * rng.next_gaussian()));
    }
    let mean_y = ys.iter().sum::<f64>() / n as f64;
    let var = ys.iter().map(|y| (y - mean_y).powi(2)).sum::<f64>() / (n - 1) as f64;
    ys.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let lo = ys[(0.025 * n as f64) as usize];
    let hi = ys[((0.975 * n as f64) as usize).min(n - 1)];
    McReport {
        evaluations: n,
        mean: mean_y,
        std_dev: var.sqrt(),
        coverage_95: (lo, hi),
    }
}

// ---------------------------------------------------------------------------
// N-k derating chain
// ---------------------------------------------------------------------------

/// Bit decrement for `k` failed modules: `⌊k·507.32 / P_bit⌋`.
pub fn delta_n_bits(k: u32) -> u64 {
    (k as f64 * MODULE_NET_W / P_BIT_W).floor() as u64
}

/// Derated seed mass `M_seed(N−k)` (M_☉).  Scales linearly in lost power from
/// `1e-6 M_☉` at k = 0 to `1e-7 M_☉` at k = 1,607.
pub fn seed_mass_msun(k: u32) -> f64 {
    let frac = (k.min(K_MAX_SURVIVABLE) as f64) / (K_MAX_SURVIVABLE as f64);
    M_SEED_NOMINAL_MSUN + (M_SEED_MIN_MSUN - M_SEED_NOMINAL_MSUN) * frac
}

/// Derated focal baseline `f(M_seed) = f0 · M_nominal / M_seed(N−k)` (m).
pub fn focal_baseline_m(k: u32) -> f64 {
    F0_M * M_SEED_NOMINAL_MSUN / seed_mass_msun(k)
}

/// Remaining net power after `k` failures (W).
pub fn net_power_w(k: u32) -> f64 {
    DEMAND_W - k as f64 * MODULE_NET_W
}

/// Full derating state for `k` failed modules.
#[derive(Clone, Copy, Debug)]
pub struct DeratingState {
    /// Failed modules.
    pub k: u32,
    /// 512-bit register bit decrement ΔN(k).
    pub delta_n_bits: u64,
    /// Derated seed mass (M_☉).
    pub m_seed_msun: f64,
    /// Extended focal baseline (m).
    pub focal_baseline_m: f64,
    /// Remaining net power margin vs the 906 kW demand (W).
    pub net_power_margin_w: f64,
    /// True while `k ≤ 1,607`.
    pub survivable: bool,
}

/// Evaluates the complete N-k derating chain.
pub fn derate(k: u32) -> DeratingState {
    DeratingState {
        k,
        delta_n_bits: delta_n_bits(k),
        m_seed_msun: seed_mass_msun(k),
        focal_baseline_m: focal_baseline_m(k),
        net_power_margin_w: net_power_w(k),
        survivable: k <= K_MAX_SURVIVABLE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn k12_bit_decrement_matches_spec() {
        // sglt.txt §4: k = 12 → ΔN ≈ 6.80e6 bits (spec lists 6,801,666).
        assert!((delta_n_bits(12) as f64 - 6.8016e6).abs() < 200.0);
    }

    #[test]
    fn k1607_reaches_min_mass_and_max_baseline() {
        assert!((seed_mass_msun(K_MAX_SURVIVABLE) - 1e-7).abs() < 1e-16);
        assert!((focal_baseline_m(K_MAX_SURVIVABLE) - F_MAX_M).abs() < 0.05);
    }

    #[test]
    fn k12_baseline_matches_spec() {
        // sglt.txt §4: k = 12 → f ≈ 170.43 m.
        let f = focal_baseline_m(12);
        assert!((f - 170.43).abs() < 0.1, "f = {f}");
    }

    #[test]
    fn monte_carlo_derating_is_reproducible() {
        let r1 = run_gum_monte_carlo(12.0, 0.5, |k| k * MODULE_NET_W, 10_000, 42);
        let r2 = run_gum_monte_carlo(12.0, 0.5, |k| k * MODULE_NET_W, 10_000, 42);
        assert_eq!(r1.mean, r2.mean);
        assert!((r1.mean - 12.0 * MODULE_NET_W).abs() < 5.0);
    }

    #[test]
    fn gum_jacobian_is_analytic() {
        let report =
            propagate_uncertainty_gum(&[2.0, 3.0], &[vec![0.25, 0.1], vec![0.1, 0.36]], |x| {
                vec![x[0].clone() * x[1].clone(), x[0].clone() + x[1].clone()]
            });
        assert_eq!(report.values, vec![6.0, 5.0]);
        assert_eq!(report.jacobian, vec![vec![3.0, 2.0], vec![1.0, 1.0]]);
    }
}
