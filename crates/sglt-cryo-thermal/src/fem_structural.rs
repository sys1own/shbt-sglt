//! Thermo-mechanical C/SiC optical-bench FEM deformation (up1.txt §4):
//!   ε = α_CTE ΔT I + C⁻¹ : σ,  ΔL(t) = ∫₀^Lb α_CTE(T) ΔT(s,t) ds
//! Stability limit: ΔL ≤ λ_min/20 = 200 nm/20 = 10 nm (GATE-15).

/// OPD stability limit, nm.
pub const OPD_LIMIT_NM: f64 = 10.0;
/// C/SiC coefficient of thermal expansion at 4.2 K, K⁻¹.
pub const CSIC_CTE: f64 = 1.2e-7;
/// Optical baseline length, m.
pub const BASELINE_M: f64 = 2.50;
/// Bench thickness, m.
pub const BENCH_THICKNESS_M: f64 = 3.75e-3;

/// 1D thermo-elastic bench model discretized along the optical baseline.
pub struct BenchFea {
    pub elements: usize,
}

impl Default for BenchFea {
    fn default() -> Self {
        Self { elements: 64 }
    }
}

impl BenchFea {
    /// OPD perturbation (nm) for a uniform temperature gradient ΔT applied
    /// across the baseline: ΔL = α_CTE · ΔT · L_b.
    pub fn opd_nm(&self, delta_t_k: f64) -> f64 {
        CSIC_CTE * delta_t_k.abs() * BASELINE_M * 1e9
    }

    /// OPD for a linear gradient between `t_a` and `t_b` (endpoint temps).
    pub fn opd_gradient_nm(&self, t_a: f64, t_b: f64) -> f64 {
        self.opd_nm((t_b - t_a).abs() * 0.5)
    }

    /// Check the GATE-15 stability bound ΔL ≤ 10 nm.
    pub fn within_stability(&self, delta_t_k: f64) -> bool {
        self.opd_nm(delta_t_k) <= OPD_LIMIT_NM
    }

    /// Strain tensor trace ε = α ΔT I (isotropic component).
    pub fn thermal_strain(&self, delta_t_k: f64) -> f64 {
        CSIC_CTE * delta_t_k
    }
}
