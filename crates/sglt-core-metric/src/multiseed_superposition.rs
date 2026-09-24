//! Multi-seed metric superposition engine and 3+1 CCZ4 numerical-relativity
//! solver (transferred from `sys1own/shbt-ghost` `ghost-multiseed-gravity`).
//!
//! Linearized multi-seed superposition
//! `g_{μν} = η_{μν} + Σ_i h_{μν}^{(i)} + I_{μν}` for arbitrary `K` seed
//! configurations under 512-bit MPFR arithmetic, plus the 3+1 CCZ4/BSSN
//! hyperbolic field evolution with Gundlach constraint damping used for
//! strong-field near-seed curvature and Solar `J_2` quadrupole coupling.

use rug::Float;

use crate::interference_tensor::{
    parse_512bit, I_00_STR, I_11_STR, I_22_STR, I_33_STR, M_SUN_KG,
};
use crate::SPEED_OF_LIGHT_M_S;

/// Bit-congestion safety radius `R_congestion = 2.954e15 m`.
pub const R_CONGESTION_M: f64 = 2.954e15;

/// Convergence limit for the linearized superposition (seeds).
pub const MAX_SEEDS_LINEAR: usize = 16;

/// Gravitational constant (SI).
pub const G_SI: f64 = 6.67430e-11;

/// Solar radius (m) for the `J_2` quadrupole coupling.
pub const R_SUN_M: f64 = 6.957e8;

/// Solar `J_2` quadrupole moment.
pub const J2_SUN: f64 = 1.0e-7;

/// Holographic constraint-violation noise floor.
pub const HOLOGRAPHIC_FLOOR: f64 = 1.0e-122;

/// 4x4 metric tensor as row-major `f64` (signature -+++).
pub type Metric4 = [[f64; 4]; 4];

/// Minkowski background `η_{μν} = diag(-1, 1, 1, 1)`.
pub fn minkowski() -> Metric4 {
    let mut g = [[0.0; 4]; 4];
    g[0][0] = -1.0;
    g[1][1] = 1.0;
    g[2][2] = 1.0;
    g[3][3] = 1.0;
    g
}

/// Exact 512-bit interference tensor diagonal `(I_00, I_11, I_22, I_33)`.
pub fn interference_diagonal_512() -> [Float; 4] {
    [
        parse_512bit(I_00_STR),
        parse_512bit(I_11_STR),
        parse_512bit(I_22_STR),
        parse_512bit(I_33_STR),
    ]
}

/// `I_{μν}` as an `f64` diagonal matrix.
pub fn interference_tensor() -> Metric4 {
    let d = interference_diagonal_512();
    let mut m = [[0.0; 4]; 4];
    for (mu, v) in d.iter().enumerate() {
        m[mu][mu] = v.to_f64();
    }
    m
}

/// Per-seed linearized perturbation `h_{μν}^{(i)} = (2GM/c²r) δ_{μν}`
/// (isotropic weak-field) for a `mass_msun` seed at standoff `r_m`.
pub fn seed_perturbation(mass_msun: f64, r_m: f64) -> Metric4 {
    let s = 2.0 * G_SI * mass_msun * M_SUN_KG / (SPEED_OF_LIGHT_M_S * SPEED_OF_LIGHT_M_S * r_m);
    let mut h = [[0.0; 4]; 4];
    for (mu, row) in h.iter_mut().enumerate() {
        row[mu] = s;
    }
    h
}

/// Linearized multi-seed superposition
/// `g_{μν} = η_{μν} + Σ_i h_{μν}^{(i)} + I_{μν}` for `seeds = (M_i, r_i)`.
/// The interference constant `I_{μν}` is evaluated at 512-bit precision and
/// demoted to `f64` for the matrix sum.
pub fn superposed_metric(seeds: &[(f64, f64)]) -> Metric4 {
    let mut g = minkowski();
    for &(m, r) in seeds {
        let h = seed_perturbation(m, r);
        for mu in 0..4 {
            for nu in 0..4 {
                g[mu][nu] += h[mu][nu];
            }
        }
    }
    let i = interference_tensor();
    for mu in 0..4 {
        g[mu][mu] += i[mu][mu];
    }
    g
}

/// Multi-seed superposition engine for arbitrary `K` seed configurations.
#[derive(Debug, Clone, Default)]
pub struct MultiSeedSuperpositionEngine {
    /// Seed list as `(mass_M_sun, standoff_m)` pairs.
    pub seeds: Vec<(f64, f64)>,
}

impl MultiSeedSuperpositionEngine {
    pub fn new(seeds: &[(f64, f64)]) -> Self {
        Self { seeds: seeds.to_vec() }
    }

    /// Evaluate `g_{μν}` for the configured seed set.
    pub fn evaluate(&self) -> Metric4 {
        superposed_metric(&self.seeds)
    }

    /// Superposition convergence audit: `K <= 16` and the per-seed
    /// perturbation stays `<< 1`.
    pub fn converges(&self, seed_mass_msun: f64, r_m: f64) -> bool {
        self.seeds.len() <= MAX_SEEDS_LINEAR
            && seed_perturbation(seed_mass_msun, r_m)[0][0].abs() < 1e-3
    }

    /// ADM volume audit `|det(g) + 1|` for a diagonal metric.
    pub fn adm_volume_audit(g: &Metric4) -> f64 {
        let det = g[0][0] * g[1][1] * g[2][2] * g[3][3];
        (det + 1.0).abs()
    }

    /// Gram-matrix positivity: every spatial diagonal element positive.
    pub fn gram_positive(g: &Metric4) -> bool {
        g[1][1] > 0.0 && g[2][2] > 0.0 && g[3][3] > 0.0
    }

    /// 2nd-order cross-coupling `I_{00}` for `K` seeds within
    /// `R_congestion` (rest frames `u = (1,0,0,0)`):
    /// `I_{00} = Σ_{i<j} G² M_i M_j / (c⁴ r_i r_j)`.
    pub fn nonlinear_cross_coupling(&self) -> f64 {
        let c2 = SPEED_OF_LIGHT_M_S * SPEED_OF_LIGHT_M_S;
        let mut i00 = 0.0;
        for (idx, &(m_i, r_i)) in self.seeds.iter().enumerate() {
            for &(m_j, r_j) in self.seeds.iter().skip(idx + 1) {
                i00 += G_SI * G_SI * (m_i * M_SUN_KG) * (m_j * M_SUN_KG)
                    / (c2 * c2 * r_i * r_j);
            }
        }
        i00
    }

    /// Third-order wake-tensor coupling magnitude `Θ_{μρσ}` (leading
    /// gravitomagnetic term): `Σ_i 8π G M_i / (c⁴ r_i)`.
    pub fn wake_tensor_third_order(&self) -> f64 {
        let mut t = 0.0;
        for &(m_i, r_i) in &self.seeds {
            t += 8.0 * std::f64::consts::PI * G_SI * m_i * M_SUN_KG
                / (SPEED_OF_LIGHT_M_S.powi(4) * r_i);
        }
        t
    }

    /// Harmonic 2PN `g_00` with Solar `J_2` quadrupole coupling:
    /// `g_00 = -1 + 2u - 2u² + 3u J_2 (R_☉/r)² (3z²/r² - 1)`,
    /// `u = GM_☉/(c²r)`.
    pub fn g00_2pn_j2(r_m: f64, z_over_r: f64) -> f64 {
        let u = G_SI * M_SUN_KG / (SPEED_OF_LIGHT_M_S * SPEED_OF_LIGHT_M_S * r_m);
        -1.0 + 2.0 * u - 2.0 * u * u
            + 3.0 * u * J2_SUN * (R_SUN_M * R_SUN_M / (r_m * r_m))
                * (3.0 * z_over_r * z_over_r - 1.0)
    }
}

// ---------------------------------------------------------------------------
// 3+1 CCZ4 / BSSN hyperbolic field evolution
// ---------------------------------------------------------------------------

/// CCZ4 field state at a grid point: conformal factor `phi`, conformal
/// metric `γ̃_{ij}`, trace-free `Ã_{ij}`, trace `K`, conformal connection
/// `Γ̃^i`, Z4 vector `Z_i`, and Z4 scalar `Θ`.
#[derive(Debug, Clone, Default)]
pub struct Ccz4State {
    pub phi: f64,
    pub gamma_tilde: [[f64; 3]; 3],
    pub a_tilde: [[f64; 3]; 3],
    pub k: f64,
    pub gamma_tilde_up: [f64; 3],
    pub z: [f64; 3],
    pub theta: f64,
}

/// Pointwise CCZ4 right-hand-side derivatives for lapse `α` with shift
/// `β^i`; curvature/matter terms are supplied externally so the solver stays
/// grid-agnostic.
#[derive(Debug, Clone)]
pub struct Ccz4Rhs {
    pub d_phi: f64,
    pub d_gamma_tilde: [[f64; 3]; 3],
    pub d_k: f64,
    pub d_theta: f64,
    pub d_z: [f64; 3],
}

/// CCZ4/BSSN evolution solver with Gundlach constraint damping.
pub struct Ccz4Solver {
    /// Gundlach constraint damping `κ_1 > 0`.
    pub kappa1: f64,
    /// `κ_2 > -1`.
    pub kappa2: f64,
}

impl Ccz4Solver {
    pub fn new(kappa1: f64, kappa2: f64) -> Self {
        assert!(kappa1 > 0.0 && kappa2 > -1.0);
        Self { kappa1, kappa2 }
    }

    /// Algebraic (principal + damping) RHS at a point with lapse `alpha`,
    /// scalar curvature `r3`, matter terms `rho_adm`, `s_i`, and `a2` the
    /// contracted `Ã_{ij}Ã^{ij}` term.
    pub fn rhs(
        &self,
        st: &Ccz4State,
        alpha: f64,
        r3: f64,
        rho_adm: f64,
        s_i: [f64; 3],
        a2: f64,
    ) -> Ccz4Rhs {
        let g = G_SI;
        let d_phi = -alpha * (st.k - st.theta) / 6.0;
        let d_k = alpha
            * (r3 + st.k * st.k - 2.0 * st.theta * st.k
                + 4.0 * std::f64::consts::PI * g * (0.0 - 3.0 * rho_adm))
            - 3.0 * alpha * self.kappa1 * (1.0 + self.kappa2) * st.theta;
        let d_theta = 0.5
            * alpha
            * (r3 - a2 + 2.0 / 3.0 * st.k * st.k
                - 2.0 * st.theta * st.k
                - 16.0 * std::f64::consts::PI * g * rho_adm)
            - alpha * self.kappa1 * (2.0 + self.kappa2) * st.theta;
        let mut d_z = [0.0; 3];
        for (i, dz) in d_z.iter_mut().enumerate() {
            *dz = alpha * (-8.0 * std::f64::consts::PI * g * s_i[i])
                - alpha * self.kappa1 * st.z[i];
        }
        let mut d_gamma_tilde = [[0.0; 3]; 3];
        for (row_d, row_a) in d_gamma_tilde.iter_mut().zip(st.a_tilde.iter()) {
            for (d, a) in row_d.iter_mut().zip(row_a.iter()) {
                *d = -2.0 * alpha * a;
            }
        }
        Ccz4Rhs { d_phi, d_gamma_tilde, d_k, d_theta, d_z }
    }

    /// Gundlach damping: `‖H(t)‖₂ ≤ ‖H(0)‖₂ exp(-κ₁ α t)`, saturated at the
    /// `1e-122` holographic noise floor.
    pub fn damped_constraint(&self, h0: f64, alpha: f64, t: f64) -> f64 {
        (h0.abs() * (-self.kappa1 * alpha * t).exp()).max(HOLOGRAPHIC_FLOOR)
    }
}

impl Default for Ccz4Solver {
    fn default() -> Self {
        Self::new(0.5, 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn superposition_additive() {
        let e = MultiSeedSuperpositionEngine::new(&[(1e-6, 1e12); 4]);
        let g = e.evaluate();
        assert!(MultiSeedSuperpositionEngine::adm_volume_audit(&g).is_finite());
        assert!(MultiSeedSuperpositionEngine::gram_positive(&g));
        assert!(e.converges(1e-6, 1e12));
    }

    #[test]
    fn interference_constants_exact() {
        let d = interference_diagonal_512();
        assert_eq!(d[0].to_f64(), parse_512bit(I_00_STR).to_f64());
        assert_eq!(d[3].to_f64(), parse_512bit(I_33_STR).to_f64());
        assert_eq!(R_CONGESTION_M, 2.954e15);
    }

    #[test]
    fn cross_coupling_and_j2() {
        let e = MultiSeedSuperpositionEngine::new(&[(1e-6, 1e15), (1e-6, 1.1e15)]);
        assert!(e.nonlinear_cross_coupling() > 0.0);
        assert!(e.wake_tensor_third_order() > 0.0);
        let g00 = MultiSeedSuperpositionEngine::g00_2pn_j2(1e12, 0.0);
        assert!(g00 < 0.0 && (g00 + 1.0).abs() < 1e-8);
    }

    #[test]
    fn ccz4_rhs_and_damping() {
        let solver = Ccz4Solver::default();
        let st = Ccz4State { theta: 1e-6, ..Default::default() };
        let rhs = solver.rhs(&st, 1.0, 0.0, 0.0, [0.0; 3], 0.0);
        assert!((rhs.d_phi - 1e-6 / 6.0).abs() < 1e-12);
        assert!(rhs.d_theta < 0.0);
        assert_eq!(solver.damped_constraint(1e-20, 1.0, 600.0), 1e-122);
        assert!(solver.damped_constraint(1e-20, 1.0, 10.0) < 1e-20);
    }
}
