//! SE-L2 halo-orbit relative dynamics and environmental disturbance model.
//!
//! Transferred from `sys1own/shbt-precision` (`src/orbital_disturbances.rs`,
//! `SEL2Environment`, `evaluate_tidal_tensor`) and adapted to the two-spacecraft
//! SGLT formation: Lens Craft (`M1 = 5,100 kg`) and Sensor Craft
//! (`M2 = 1,460 kg`) at the Sun–Earth L2 synodic frame (sglt.txt §3).
//!
//!   ρ̈ + 2Ω×ρ̇ + Ω×(Ω×ρ) = C_tidal ρ + Δa_SRP + a_outgas + F_control/M2
//!   C_tidal = diag(2σ+1, 1−σ, −σ) ω0²,   σ ≈ 3.0428,  ω0 = 1.990986e-7 rad/s.

/// Synodic angular velocity of the Sun–Earth rotating frame (rad/s).
pub const OMEGA_0: f64 = 1.990986e-7;
/// L2 libration parameter σ.
pub const SIGMA_L2: f64 = 3.0428;
/// Solar radiation pressure at 1 AU (N/m²).
pub const P_SOLAR: f64 = 4.56e-6;
/// Lens Craft wet mass (kg).
pub const M1_LENS_KG: f64 = 5100.0;
/// Sensor Craft wet mass (kg).
pub const M2_SENSOR_KG: f64 = 1460.0;
/// Lens Craft effective sun-facing area including the deployable sunshield
/// (m²); sized so `Δa_SRP ≈ 3.42e-8 m/s²` against the Sensor Craft.
pub const A1_M2: f64 = 49.659;
/// Sensor Craft effective cross-sectional area (m²).
pub const A2_M2: f64 = 6.2;
/// Lens Craft surface reflectivity coefficient.
pub const CR1: f64 = 1.25;
/// Sensor Craft surface reflectivity coefficient.
pub const CR2: f64 = 1.10;
/// Outgassing initial acceleration (m/s²).
pub const OUTGAS_A0: f64 = 5.0e-9;
/// Outgassing decay time constant (s), 180 days.
pub const OUTGAS_TAU_S: f64 = 180.0 * 86_400.0;
/// C/SiC metering-structure thermal baseline drift (nm/K).
pub const C_SIC_THERMAL_DRIFT_NM_PER_K: f64 = 0.45;
/// Nominal inter-craft focal baseline (m).
pub const F0_NOMINAL_M: f64 = 169.30;

/// SE-L2 environment disturbance evaluator (`SEL2Environment` transfer).
#[derive(Clone, Debug)]
pub struct SEL2Environment {
    /// Epoch time since deployment (s).
    pub t_s: f64,
}

impl SEL2Environment {
    pub fn new(t_s: f64) -> Self {
        Self { t_s }
    }

    /// Linearized tidal gravity matrix `C_tidal` at L2 (row-major 3x3):
    /// `diag(2σ+1, 1−σ, −σ) ω0²`.
    pub fn evaluate_tidal_tensor(&self) -> [f64; 9] {
        let w2 = OMEGA_0 * OMEGA_0;
        [
            (2.0 * SIGMA_L2 + 1.0) * w2,
            0.0,
            0.0,
            0.0,
            (1.0 - SIGMA_L2) * w2,
            0.0,
            0.0,
            0.0,
            -SIGMA_L2 * w2,
        ]
    }

    /// Differential solar radiation pressure magnitude (m/s²):
    /// `Δa_SRP = P_solar (A1·CR1/M1 − A2·CR2/M2)` along Ŝ.
    pub fn differential_srp_m_s2(&self) -> f64 {
        P_SOLAR * (A1_M2 * CR1 / M1_LENS_KG - A2_M2 * CR2 / M2_SENSOR_KG)
    }

    /// Residual outgassing acceleration (m/s²): `a0 exp(−t/τ_degas)`.
    pub fn outgassing_m_s2(&self) -> f64 {
        OUTGAS_A0 * (-self.t_s / OUTGAS_TAU_S).exp()
    }

    /// C/SiC metering-structure baseline drift (nm) for a `delta_k` swing.
    pub fn thermal_baseline_drift_nm(&self, delta_k: f64) -> f64 {
        C_SIC_THERMAL_DRIFT_NM_PER_K * delta_k
    }

    /// Relative-motion acceleration `ρ̈` (m/s²) given displacement `rho` (m),
    /// velocity `rho_dot` (m/s), Sun unit vector `s_hat`, outgassing direction
    /// `e_out`, and control thrust `f_control` (N) on the Sensor Craft.
    pub fn relative_acceleration(
        &self,
        rho: [f64; 3],
        rho_dot: [f64; 3],
        s_hat: [f64; 3],
        e_out: [f64; 3],
        f_control: [f64; 3],
    ) -> [f64; 3] {
        let c = self.evaluate_tidal_tensor();
        let srp = self.differential_srp_m_s2();
        let out = self.outgassing_m_s2();
        let mut acc = [0.0; 3];
        for i in 0..3 {
            let tidal = c[i * 3] * rho[0] + c[i * 3 + 1] * rho[1] + c[i * 3 + 2] * rho[2];
            acc[i] = tidal + srp * s_hat[i] + out * e_out[i] + f_control[i] / M2_SENSOR_KG;
        }
        // −2Ω×ρ̇ − Ω×(Ω×ρ) with Ω = [0,0,ω0]:
        acc[0] += 2.0 * OMEGA_0 * rho_dot[1] + OMEGA_0 * OMEGA_0 * rho[0];
        acc[1] += -2.0 * OMEGA_0 * rho_dot[0] + OMEGA_0 * OMEGA_0 * rho[1];
        acc
    }

    /// Semi-implicit (symplectic) Euler integration step of the relative state.
    pub fn step(&self, rho: &mut [f64; 3], rho_dot: &mut [f64; 3], dt_s: f64, f_control: [f64; 3]) {
        let acc =
            self.relative_acceleration(*rho, *rho_dot, [1.0, 0.0, 0.0], [1.0, 0.0, 0.0], f_control);
        for i in 0..3 {
            rho_dot[i] += acc[i] * dt_s;
            rho[i] += rho_dot[i] * dt_s;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn differential_srp_matches_spec() {
        let env = SEL2Environment::new(0.0);
        let a = env.differential_srp_m_s2();
        assert!((a - 3.42e-8).abs() < 0.05e-8, "a = {a}");
    }

    #[test]
    fn tidal_tensor_uses_l2_libration_parameter() {
        let env = SEL2Environment::new(0.0);
        let c = env.evaluate_tidal_tensor();
        let w2 = OMEGA_0 * OMEGA_0;
        assert!((c[0] - (2.0 * SIGMA_L2 + 1.0) * w2).abs() < 1e-22);
        assert!((c[8] - (-SIGMA_L2 * w2)).abs() < 1e-22);
    }

    #[test]
    fn outgassing_decays_with_180_day_tau() {
        let e0 = SEL2Environment::new(0.0);
        let e1 = SEL2Environment::new(OUTGAS_TAU_S);
        assert!((e0.outgassing_m_s2() - 5.0e-9).abs() < 1e-15);
        assert!((e1.outgassing_m_s2() - 5.0e-9 / std::f64::consts::E).abs() < 1e-12);
    }
}
