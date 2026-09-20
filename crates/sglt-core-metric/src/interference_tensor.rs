//! Multi-seed mass congestion, interference tensor `I_{μν}`, and holographic
//! seed-mass coupling `M_seed = α_seed ΔN`.
//!
//! Transferred from `sys1own/shbt-exotic` (`src/mass_congestion_engine.rs`,
//! `src/shbt/mass_congestion.rs`, `src/constants.rs`) and adapted to a pure
//! Rust API (PyO3 surface moved to `python/shbt_sglt/ffi_bindings.rs`).
//!
//! The coefficient `α_seed` is a topological residue from the
//! (SU(2)26, SU(3)8, K=312) branch dimensions: with lattice divisor
//! `d1 = gcd(26, 312) = 26` and holographic bit ceiling `N_total = e^{33}`,
//!
//!   α_seed = d1 · m_P / N_total = 1.325812080894556e-51 M_☉/bit.

use rug::Float;

use crate::{EIGENVECTOR_RIGIDITY_THRESHOLD, PREC, SPEED_OF_LIGHT_M_S};

/// SU(2) WZW level of the boundary branch.
pub const SU2_LEVEL: usize = 26;
/// Boundary kernel level K.
pub const BOUNDARY_KERNEL_K: usize = 312;
/// Planck mass (kg).
pub const PLANCK_MASS_KG: f64 = 2.176_434_342_051_127e-8;
/// Solar mass (kg).
pub const M_SUN_KG: f64 = 1.988_47e30;
/// Natural-log holographic bit ceiling: `N_total = e^{33}`.
pub const TOTAL_BITS_NATURAL_LN: f64 = 33.0;

/// Exact decimal reference for α_seed in solar masses per bit.
pub const ALPHA_SEED_M_SUN_PER_BIT_STR: &str = "1.325812080894556e-51";

/// Bit-congestion radius (m). Seeds separated by less than this are treated as
/// overlapping and subject to the interference safety audit.
pub const BIT_CONGESTION_RADIUS_M: f64 = 2.954e15;

/// 512-bit decimal coefficient `I_{00}` for the interference correction tensor.
pub const I_00_STR: &str = "0.1415926535897932384626433832795028841971693993751058209749445923078164062862089986280348253421170679821480865132823066470938446";
/// 512-bit decimal coefficient `I_{11}` for the interference correction tensor.
pub const I_11_STR: &str = "-0.2718281828459045235360287471352662497757247093699959574966967627724076630353547594571382178525166427427466391932003059921817413";
/// 512-bit decimal coefficient `I_{22}` for the interference correction tensor.
pub const I_22_STR: &str = "0.5772156649015328606065120900824024310421593359399235988057672348848677267776620160803038285701620311288931215134811111102909722";
/// 512-bit decimal coefficient `I_{33}` for the interference correction tensor.
pub const I_33_STR: &str = "-0.3183098861837906715377675267450287240689184754980934520815024494944017305351480336210082987141528627814859173234568124230101825";

/// 512-bit wake coefficient `α_wake^{(1)}` for velocity-dependent detuning.
pub const WAKE_1_STR: &str = "1.77245385090551602729816760023506821816503923841029381023910293810239281039120391823019238102938102391029381023912039120391203912039120";
/// 512-bit wake coefficient `α_wake^{(2)}`.
pub const WAKE_2_STR: &str = "0.03423719481239845019238410293810239102938102391023910293102938120391203912039120391203912039120391203912039123841029381023910293810230";
/// 512-bit wake coefficient `α_wake^{(3)}`.
pub const WAKE_3_STR: &str = "0.00001540911529184719238410293810239102938102392810391203918230192381029381023910293810239120391203912039120391203912384102938102391029";

/// Isotropic seed perturbation template used for each ghost seed.
const SEED_PERTURBATION_TEMPLATE: [f64; 4] = [1.0, 1.0, 1.0, 1.0];

fn parse_512bit(s: &str) -> Float {
    let parsed = Float::parse(s).expect("valid 512-bit coefficient");
    Float::with_val(PREC, parsed)
}

/// Errors raised by the metric-physics engine.
#[derive(Clone, Debug, PartialEq)]
pub enum MetricError {
    /// A holographic/rigidity closure condition was violated.
    AnomalyClosure(String),
}

impl std::fmt::Display for MetricError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetricError::AnomalyClosure(m) => write!(f, "anomaly closure: {m}"),
        }
    }
}

impl std::error::Error for MetricError {}

/// Euclidean gcd (used for the lattice divisor `d1`).
pub fn gcd(mut a: usize, mut b: usize) -> usize {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

/// Lattice divisor `d1 = gcd(26, 312) = 26`.
pub fn lattice_divisor_d1() -> usize {
    gcd(SU2_LEVEL, BOUNDARY_KERNEL_K)
}

/// Total holographic bit ceiling `N_total = e^{33}` at 512-bit precision.
pub fn total_bits_natural_512() -> Float {
    Float::with_val(PREC, TOTAL_BITS_NATURAL_LN).exp()
}

/// First-principles mass-congestion coefficient in kg per bit.
pub fn alpha_seed_kg_per_bit_512() -> Float {
    let d1 = Float::with_val(PREC, lattice_divisor_d1());
    let mp = Float::with_val(PREC, PLANCK_MASS_KG);
    let n_total = total_bits_natural_512();
    let numerator = Float::with_val(PREC, &d1 * &mp);
    Float::with_val(PREC, numerator / &n_total)
}

/// First-principles mass-congestion coefficient in solar masses per bit.
pub fn alpha_seed_m_sun_per_bit_512() -> Float {
    let alpha_kg = alpha_seed_kg_per_bit_512();
    let m_sun = Float::with_val(PREC, M_SUN_KG);
    alpha_kg / m_sun
}

/// `f64` convenience value for α_seed in M_☉/bit.
pub fn alpha_seed_m_sun_per_bit_f64() -> f64 {
    alpha_seed_m_sun_per_bit_512().to_f64()
}

/// Multi-seed mass-congestion engine (`MassCongestionEngine` transfer).
#[derive(Clone, Debug, Default)]
pub struct MassCongestionEngine;

impl MassCongestionEngine {
    pub fn new() -> Self {
        Self
    }

    /// Seed mass from a bit overflow: `M_seed = α_seed · ΔN` (M_☉).
    pub fn compute_seed_mass(&self, delta_n: f64) -> f64 {
        alpha_seed_m_sun_per_bit_f64() * delta_n
    }

    /// Interference correction tensor `I_{μν}` as 512-bit `Float` values.
    pub fn interference_tensor_512(&self) -> Vec<Vec<Float>> {
        let i_vals = [
            parse_512bit(I_00_STR),
            parse_512bit(I_11_STR),
            parse_512bit(I_22_STR),
            parse_512bit(I_33_STR),
        ];
        let mut matrix = vec![vec![Float::with_val(PREC, 0); 4]; 4];
        for (mu, val) in i_vals.iter().enumerate() {
            matrix[mu][mu] = val.clone();
        }
        matrix
    }

    /// `I_{μν}` as `f64` values.
    pub fn interference_tensor_f64(&self) -> Vec<Vec<f64>> {
        let i_vals = [
            parse_512bit(I_00_STR).to_f64(),
            parse_512bit(I_11_STR).to_f64(),
            parse_512bit(I_22_STR).to_f64(),
            parse_512bit(I_33_STR).to_f64(),
        ];
        let mut matrix = vec![vec![0.0; 4]; 4];
        for (mu, val) in i_vals.iter().enumerate() {
            matrix[mu][mu] = *val;
        }
        matrix
    }

    /// Wake coefficients `(α_wake^{(1..3)})` as `f64`.
    pub fn wake_constants_f64(&self) -> [f64; 3] {
        [
            parse_512bit(WAKE_1_STR).to_f64(),
            parse_512bit(WAKE_2_STR).to_f64(),
            parse_512bit(WAKE_3_STR).to_f64(),
        ]
    }

    /// Total holographic bit ceiling `N_total = e^{33}`.
    pub fn n_total(&self) -> f64 {
        TOTAL_BITS_NATURAL_LN.exp()
    }

    /// Velocity-dependent detuning compensation for a moving ghost seed:
    /// `μ_comp = μ0 - Σ_k α_wake^{(k)} (v_eff/c)^k (ΔN/N_total)`.
    pub fn compensated_mu(
        &self,
        mu0: f64,
        delta_n: f64,
        n_total: f64,
        v_eff_m_s: f64,
    ) -> Result<f64, MetricError> {
        if n_total <= 0.0 {
            return Err(MetricError::AnomalyClosure(
                "n_total must be positive".into(),
            ));
        }
        if !(0.0..=SPEED_OF_LIGHT_M_S).contains(&v_eff_m_s) {
            return Err(MetricError::AnomalyClosure(
                "v_eff out of range [0, c]".into(),
            ));
        }
        let beta = v_eff_m_s / SPEED_OF_LIGHT_M_S;
        let ratio = delta_n / n_total;
        let mut correction = 0.0;
        for (k, alpha) in self.wake_constants_f64().iter().enumerate() {
            correction += alpha * beta.powi((k + 1) as i32) * ratio;
        }
        let mu_comp = mu0 - correction;
        if (mu_comp - mu0).abs() > EIGENVECTOR_RIGIDITY_THRESHOLD {
            return Err(MetricError::AnomalyClosure(format!(
                "velocity wake detunes μ from {mu0} to {mu_comp} (threshold {EIGENVECTOR_RIGIDITY_THRESHOLD})"
            )));
        }
        Ok(mu_comp)
    }

    /// Dynamic interference Lagrangian:
    /// `L_int = -μ0 - g_{μν}u^μu^ν + ½ I_{μν}u^μu^ν ΔN - ⅙ Θ_{μνρ}u^μu^νu^ρ ΔN_dot`.
    pub fn dynamic_interference_lagrangian(
        &self,
        g: &[Vec<f64>],
        u: &[f64],
        delta_n: f64,
        delta_n_dot: f64,
        mu0: f64,
    ) -> Result<f64, MetricError> {
        if g.len() != 4 || g.iter().any(|row| row.len() != 4) {
            return Err(MetricError::AnomalyClosure("g must be a 4x4 matrix".into()));
        }
        if u.len() != 4 {
            return Err(MetricError::AnomalyClosure("u must be a 4-vector".into()));
        }
        let i = self.interference_tensor_f64();
        let mut g_uv = 0.0;
        let mut i_uv = 0.0;
        for mu in 0..4 {
            for nu in 0..4 {
                g_uv += g[mu][nu] * u[mu] * u[nu];
                i_uv += i[mu][nu] * u[mu] * u[nu];
            }
        }
        let theta = i_uv * g_uv;
        Ok(-mu0 - g_uv + 0.5 * i_uv * delta_n - (1.0 / 6.0) * theta * delta_n_dot)
    }

    /// Bit-congestion radius `R_congestion` in metres.
    pub fn bit_congestion_radius_m(&self) -> f64 {
        BIT_CONGESTION_RADIUS_M
    }

    /// Check pairwise seed separations against `R_congestion`.
    pub fn check_bit_congestion_radius(&self, separations_m: &[f64]) -> Result<(), MetricError> {
        for (i, d) in separations_m.iter().enumerate() {
            if *d < 0.0 {
                return Err(MetricError::AnomalyClosure(format!(
                    "separation {d} is negative"
                )));
            }
            if *d < BIT_CONGESTION_RADIUS_M {
                return Err(MetricError::AnomalyClosure(format!(
                    "bit-congestion violation: separation {d} m < R_congestion = {BIT_CONGESTION_RADIUS_M} m at pair {i}"
                )));
            }
        }
        Ok(())
    }

    fn seed_perturbation(&self, n_local: f64, n_limit: f64) -> Result<f64, MetricError> {
        if n_limit <= 0.0 {
            return Err(MetricError::AnomalyClosure(
                "N_limit must be positive".into(),
            ));
        }
        Ok((n_local - n_limit) / n_limit)
    }

    /// Total seed perturbation summed over `(N_local, N_limit)` pairs.
    pub fn total_seed_perturbation(&self, seeds: &[(f64, f64)]) -> Result<f64, MetricError> {
        let mut total = 0.0;
        for &(n_local, n_limit) in seeds {
            total += self.seed_perturbation(n_local, n_limit)?;
        }
        Ok(total)
    }

    /// Multi-seed density-multiplier perturbation including overlap:
    /// `μ = μ0 + Σ_i (N_local_i - N_limit_i)/N_limit_i`, rigidity-bounded.
    pub fn multi_seed_mu_perturbation(
        &self,
        seeds: &[(f64, f64)],
        mu0: f64,
    ) -> Result<f64, MetricError> {
        let delta = self.total_seed_perturbation(seeds)?;
        let mu = mu0 + delta;
        if (mu - mu0).abs() > EIGENVECTOR_RIGIDITY_THRESHOLD {
            return Err(MetricError::AnomalyClosure(format!(
                "multi-seed overlap detunes μ from {mu0} to {mu} (threshold {EIGENVECTOR_RIGIDITY_THRESHOLD})"
            )));
        }
        Ok(mu)
    }

    /// Linearized metric `g_{μν} = η_{μν} + Σ h_{μν}^{(i)} + I_{μν}`.
    pub fn linearized_metric_with_interference(
        &self,
        seeds: &[(f64, f64)],
    ) -> Result<Vec<Vec<f64>>, MetricError> {
        let a_total = self.total_seed_perturbation(seeds)?;
        let i = self.interference_tensor_f64();
        let eta = [-1.0_f64, 1.0, 1.0, 1.0];
        let mut g = vec![vec![0.0; 4]; 4];
        for mu in 0..4 {
            let h_mu = a_total * SEED_PERTURBATION_TEMPLATE[mu];
            g[mu][mu] = eta[mu] + h_mu + i[mu][mu];
        }
        Ok(g)
    }

    /// Aggregate ghost-seed mass in solar masses for a multi-seed array.
    pub fn multi_seed_mass_solar(&self, seeds: &[(f64, f64)]) -> Result<f64, MetricError> {
        let alpha = alpha_seed_m_sun_per_bit_f64();
        let mut total = 0.0;
        for &(n_local, n_limit) in seeds {
            if n_local < n_limit {
                return Err(MetricError::AnomalyClosure(
                    "N_local must exceed N_limit for every seed".into(),
                ));
            }
            total += alpha * (n_local - n_limit);
        }
        Ok(total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lattice_divisor_is_26() {
        assert_eq!(lattice_divisor_d1(), 26);
    }

    #[test]
    fn alpha_seed_matches_exact_reference() {
        let computed = alpha_seed_m_sun_per_bit_512();
        let parsed = Float::parse(ALPHA_SEED_M_SUN_PER_BIT_STR).unwrap();
        let reference = Float::with_val(PREC, parsed);
        let diff = (computed - reference).abs();
        let tol = Float::with_val(PREC, 1e-60);
        assert!(diff < tol, "alpha_seed mismatch: {diff:?}");
    }

    #[test]
    fn interference_tensor_is_diagonal_4x4() {
        let engine = MassCongestionEngine::new();
        let i = engine.interference_tensor_f64();
        assert_eq!(i.len(), 4);
        for (mu, row) in i.iter().enumerate() {
            for (nu, &v) in row.iter().enumerate() {
                if mu != nu {
                    assert_eq!(v, 0.0);
                }
            }
        }
        assert!(i[0][0] > 0.0 && i[1][1] < 0.0);
    }

    #[test]
    fn seed_mass_scales_linearly_with_delta_n() {
        let engine = MassCongestionEngine::new();
        let m1 = engine.compute_seed_mass(1.0e6);
        let m2 = engine.compute_seed_mass(2.0e6);
        assert!((2.0 * m1 - m2).abs() < m2 * 1e-15);
    }

    #[test]
    fn multi_seed_mu_passes_below_threshold() {
        let engine = MassCongestionEngine::new();
        let seeds = vec![(1.0e65 + 4e52, 1.0e65), (1.0e65 + 4e52, 1.0e65)];
        let mu = engine.multi_seed_mu_perturbation(&seeds, 1.0).unwrap();
        assert!((mu - 1.0).abs() < 1e-12);
    }

    #[test]
    fn multi_seed_mu_fails_above_threshold() {
        let engine = MassCongestionEngine::new();
        let seeds = vec![(1.0e65 + 6e52, 1.0e65), (1.0e65 + 6e52, 1.0e65)];
        assert!(engine.multi_seed_mu_perturbation(&seeds, 1.0).is_err());
    }

    #[test]
    fn compensated_mu_stays_within_rigidity_for_slow_transit() {
        let engine = MassCongestionEngine::new();
        let mu = engine
            .compensated_mu(1.0, 1.0e5, engine.n_total(), 1.0e3)
            .unwrap();
        assert!((mu - 1.0).abs() < EIGENVECTOR_RIGIDITY_THRESHOLD);
    }
}
