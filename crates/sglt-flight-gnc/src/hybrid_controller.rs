//! 2-tier hybrid GNC controller for SGLT formation keeping.
//!
//! Transferred from `sys1own/shbt-precision` (`src/gnc_hybrid_controller.rs`,
//! `HybridGNCController`, `execute_dual_tier_control`) and adapted to the SGLT
//! actuator suite:
//!
//! * Coarse tier (f < 0.1 Hz): EMI-BF4 ionic-liquid electrospray thrusters,
//!   continuous thrust 0.1–150.0 µN, `I_sp = 3,200 s`.
//! * Fine tier (0.1 Hz ≤ f ≤ 100 kHz): 8×8 InP electro-optic phase-shifter
//!   array, drive 3.8–7.4 V, for micro-vibration and phase-jitter rejection.

/// Crossover frequency between coarse and fine control tiers (Hz).
pub const TIER_CROSSOVER_HZ: f64 = 0.1;
/// Fine-tier upper bandwidth (Hz).
pub const FINE_TIER_MAX_HZ: f64 = 100.0e3;
/// Minimum electrospray thrust (µN).
pub const THRUST_MIN_UN: f64 = 0.1;
/// Maximum electrospray thrust (µN).
pub const THRUST_MAX_UN: f64 = 150.0;
/// Electrospray specific impulse (s).
pub const ISP_S: f64 = 3200.0;
/// Phase-shifter minimum drive voltage (V).
pub const PHASE_SHIFTER_MIN_V: f64 = 3.8;
/// Phase-shifter maximum drive voltage (V).
pub const PHASE_SHIFTER_MAX_V: f64 = 7.4;
/// Phase-shifter array edge dimension (8×8).
pub const PHASE_SHIFTER_DIM: usize = 8;
/// Standard gravity (m/s²) for Isp ↔ exhaust-velocity conversion.
pub const G0: f64 = 9.80665;

/// Per-tier actuator command produced by [`HybridGNCController`].
#[derive(Clone, Debug)]
pub struct DualTierCommand {
    /// Coarse-tier thrust vector (µN), applied to electrospray thrusters.
    pub thrust_un: [f64; 3],
    /// Fine-tier phase-shifter voltage map (8×8, V).
    pub phase_shifter_v: [[f64; PHASE_SHIFTER_DIM]; PHASE_SHIFTER_DIM],
    /// True when the coarse tier saturated and the residual was handed off.
    pub coarse_saturated: bool,
}

/// 2-tier hybrid controller (`HybridGNCController` transfer).
#[derive(Clone, Debug)]
pub struct HybridGNCController {
    /// Coarse-tier proportional gain (µN per m of displacement error).
    pub kp_coarse_un_per_m: f64,
    /// Coarse-tier derivative gain (µN per m/s).
    pub kd_coarse_un_per_m_s: f64,
    /// Fine-tier phase gain (V per rad of wavefront error).
    pub kp_fine_v_per_rad: f64,
}

impl Default for HybridGNCController {
    fn default() -> Self {
        Self {
            kp_coarse_un_per_m: 4.0e6,
            kd_coarse_un_per_m_s: 8.0e7,
            kp_fine_v_per_rad: 0.6,
        }
    }
}

impl HybridGNCController {
    pub fn new() -> Self {
        Self::default()
    }

    /// Electrospray exhaust velocity `v_e = I_sp g0` (m/s).
    pub fn exhaust_velocity_m_s(&self) -> f64 {
        ISP_S * G0
    }

    /// Execute one dual-tier control step.
    ///
    /// `pos_err_m`/`vel_err_m_s` drive the coarse loop; `phase_err_rad` is the
    /// per-element wavefront error map (flattened row-major, length 64) for
    /// the fine loop.  Coarse thrust is clamped to [0.1, 150.0] µN per axis;
    /// shifter voltages are clamped to [3.8, 7.4] V around the 5.6 V midpoint.
    pub fn execute_dual_tier_control(
        &self,
        pos_err_m: [f64; 3],
        vel_err_m_s: [f64; 3],
        phase_err_rad: &[f64; PHASE_SHIFTER_DIM * PHASE_SHIFTER_DIM],
    ) -> DualTierCommand {
        let mid_v = 0.5 * (PHASE_SHIFTER_MIN_V + PHASE_SHIFTER_MAX_V);
        let half_span_v = 0.5 * (PHASE_SHIFTER_MAX_V - PHASE_SHIFTER_MIN_V);
        let mut thrust_un = [0.0; 3];
        let mut saturated = false;
        for axis in 0..3 {
            let raw = -(self.kp_coarse_un_per_m * pos_err_m[axis]
                + self.kd_coarse_un_per_m_s * vel_err_m_s[axis]);
            let cmd = raw.clamp(THRUST_MIN_UN, THRUST_MAX_UN);
            saturated |= (raw - cmd).abs() > 1e-9;
            thrust_un[axis] = cmd;
        }
        let mut phase_shifter_v = [[mid_v; PHASE_SHIFTER_DIM]; PHASE_SHIFTER_DIM];
        for (i, row) in phase_shifter_v.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate() {
                let err = phase_err_rad[i * PHASE_SHIFTER_DIM + j];
                *cell = (mid_v + self.kp_fine_v_per_rad * err)
                    .clamp(mid_v - half_span_v, mid_v + half_span_v);
            }
        }
        DualTierCommand {
            thrust_un,
            phase_shifter_v,
            coarse_saturated: saturated,
        }
    }

    /// True when a disturbance frequency is owned by the coarse tier.
    pub fn is_coarse_band(&self, f_hz: f64) -> bool {
        f_hz < TIER_CROSSOVER_HZ
    }

    /// True when a disturbance frequency is owned by the fine tier.
    pub fn is_fine_band(&self, f_hz: f64) -> bool {
        (TIER_CROSSOVER_HZ..=FINE_TIER_MAX_HZ).contains(&f_hz)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bands_split_at_crossover() {
        let c = HybridGNCController::new();
        assert!(c.is_coarse_band(0.01));
        assert!(!c.is_fine_band(0.01));
        assert!(c.is_fine_band(50e3));
        assert!(!c.is_fine_band(200e3));
    }

    #[test]
    fn coarse_thrust_is_clamped() {
        let c = HybridGNCController::new();
        let out = c.execute_dual_tier_control([1.0, 0.0, 0.0], [0.0; 3], &[0.0; 64]);
        assert!(out.coarse_saturated);
        assert_eq!(out.thrust_un[0], THRUST_MIN_UN); // sign: raw negative → clamped low
        for row in &out.phase_shifter_v {
            for v in row {
                assert!((PHASE_SHIFTER_MIN_V..=PHASE_SHIFTER_MAX_V).contains(v));
            }
        }
    }

    #[test]
    fn fine_loop_centres_on_midpoint_voltage() {
        let c = HybridGNCController::new();
        let out = c.execute_dual_tier_control([0.0; 3], [0.0; 3], &[0.0; 64]);
        assert_eq!(out.phase_shifter_v[0][0], 5.6);
    }
}
