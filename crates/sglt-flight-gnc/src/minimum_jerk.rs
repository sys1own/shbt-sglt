//! Sensor Craft 5th-order minimum-jerk repositioning profile (sglt.txt §3).
//!
//! `s(τ) = 10τ³ − 15τ⁴ + 6τ⁵`, `τ = t/T_slew ∈ [0,1]`.
//!
//! Kinematic peaks (exact):
//!   max(ṡ)  = 1.8750        at τ = 1/2
//!   max|s̈|  = 10/√3 ≈ 5.7735 (bounded at 5.7733)
//!   at τ = (3 ± √3)/6 ≈ 0.2113, 0.7887

/// Dimensionless displacement `s(τ)`.
pub fn s(tau: f64) -> f64 {
    tau.powi(3) * (10.0 - 15.0 * tau + 6.0 * tau * tau)
}

/// Normalized velocity `ds/dτ = 30τ²(1−τ)²`.
pub fn s_dot(tau: f64) -> f64 {
    30.0 * tau * tau * (1.0 - tau).powi(2)
}

/// Normalized acceleration `d²s/dτ² = 60τ(1−τ)(1−2τ)`.
pub fn s_ddot(tau: f64) -> f64 {
    60.0 * tau * (1.0 - tau) * (1.0 - 2.0 * tau)
}

/// Normalized jerk `d³s/dτ³ = 60(1 − 6τ + 6τ²)`.
pub fn s_dddot(tau: f64) -> f64 {
    60.0 * (1.0 - 6.0 * tau + 6.0 * tau * tau)
}

/// Exact peak normalized velocity at τ = 1/2: `30·(1/2)²·(1/2)² = 1.8750`.
pub const PEAK_VELOCITY: f64 = 1.8750;
/// Numerically-bounded peak |acceleration|: `10/√3 ≈ 5.7735 → 5.7733`.
pub const PEAK_ACCELERATION: f64 = 5.7733;
/// Inflection root `τ = (3 − √3)/6`.
pub const INFLECTION_LO: f64 = 0.2113248654051871;
/// Inflection root `τ = (3 + √3)/6`.
pub const INFLECTION_HI: f64 = 0.7886751345948129;

/// Dimensional trajectory generator for a `T_slew`-second repositioning over
/// `delta_f_m` metres of focal-baseline change.
#[derive(Clone, Copy, Debug)]
pub struct MinJerkTrajectory {
    /// Slew duration (s).
    pub t_slew_s: f64,
    /// Total displacement (m).
    pub delta_f_m: f64,
}

impl MinJerkTrajectory {
    pub fn new(t_slew_s: f64, delta_f_m: f64) -> Self {
        Self {
            t_slew_s,
            delta_f_m,
        }
    }

    /// Position (m) at time `t_s`.
    pub fn position_m(&self, t_s: f64) -> f64 {
        self.delta_f_m * s((t_s / self.t_slew_s).clamp(0.0, 1.0))
    }

    /// Velocity (m/s) at time `t_s`.
    pub fn velocity_m_s(&self, t_s: f64) -> f64 {
        self.delta_f_m / self.t_slew_s * s_dot((t_s / self.t_slew_s).clamp(0.0, 1.0))
    }

    /// Acceleration (m/s²) at time `t_s`.
    pub fn acceleration_m_s2(&self, t_s: f64) -> f64 {
        self.delta_f_m / self.t_slew_s.powi(2) * s_ddot((t_s / self.t_slew_s).clamp(0.0, 1.0))
    }

    /// Peak dimensional velocity (m/s).
    pub fn peak_velocity_m_s(&self) -> f64 {
        PEAK_VELOCITY * self.delta_f_m / self.t_slew_s
    }

    /// Peak dimensional acceleration magnitude (m/s²).
    pub fn peak_acceleration_m_s2(&self) -> f64 {
        PEAK_ACCELERATION * self.delta_f_m / self.t_slew_s.powi(2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_are_zero_and_one() {
        assert_eq!(s(0.0), 0.0);
        assert_eq!(s(1.0), 1.0);
        assert_eq!(s_dot(0.0), 0.0);
        assert_eq!(s_dot(1.0), 0.0);
        assert_eq!(s_ddot(0.0), 0.0);
        assert_eq!(s_ddot(1.0), 0.0);
    }

    #[test]
    fn peak_velocity_exact() {
        assert_eq!(s_dot(0.5), PEAK_VELOCITY);
    }

    #[test]
    fn peak_acceleration_at_inflection_roots() {
        let a = s_ddot(INFLECTION_LO).abs();
        let b = s_ddot(INFLECTION_HI).abs();
        let expect = 10.0 / 3.0f64.sqrt();
        assert!((a - expect).abs() < 1e-12, "a = {a}");
        assert!((b - expect).abs() < 1e-12);
        assert!(a - PEAK_ACCELERATION < 5e-4);
    }

    #[test]
    fn dimensional_profile_peaks() {
        let traj = MinJerkTrajectory::new(600.0, 1523.69);
        assert!(traj.peak_velocity_m_s() > 0.0);
        let v_mid = traj.velocity_m_s(300.0);
        assert!((v_mid - traj.peak_velocity_m_s()).abs() < 1e-12);
    }
}
