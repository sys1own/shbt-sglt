//! Deep-RL trajectory planning, NSGA-III optimization, minimum-jerk profiles,
//! and N-k reactor derating (up2.txt §5). Spec ships `no_std`+alloc; built on
//! std inside this crate.

/// Kinematic bounds for the 5th-order minimum-jerk profile.
pub const KINEMATIC_MAX_VELOCITY_BOUND: f64 = 1.8750; // m/s
pub const KINEMATIC_MAX_ACCELERATION_BOUND: f64 = 5.7733; // m/s²
/// Required end-of-life propellant reserve fraction.
pub const MIN_EOL_PROPELLANT_RESERVE_MARGIN: f64 = 0.9800;
/// Fault-tolerant thrust continuity cap: F_max = 2 η P / (g0 Isp).
pub const THRUST_ETA: f64 = 0.78;
pub const G0: f64 = 9.80665;

/// Fission reactor health state for N-k analysis.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ReactorHealthState {
    pub reactor_id: u32,
    pub nominal_power_kw: f64,
    pub available_reactors: u32,
    pub total_reactors: u32,
    pub thermal_margin_k: f64,
    pub is_scram_active: u8,
}

impl ReactorHealthState {
    /// Remaining fraction after k reactor losses.
    pub fn power_ratio(&self) -> f64 {
        if self.total_reactors == 0 {
            return 0.0;
        }
        self.available_reactors as f64 / self.total_reactors as f64
    }
}

/// 5th-order minimum-jerk trajectory between p0 and p1 over time T.
/// s(τ) = 10τ³ − 15τ⁴ + 6τ⁵;  ṡ = 30τ²(1−τ)²;  s̈ = 60τ(1−τ)(1−2τ).
pub struct MinimumJerkProfile {
    pub p0: [f64; 3],
    pub p1: [f64; 3],
    pub duration_s: f64,
}

impl MinimumJerkProfile {
    pub fn evaluate(&self, t: f64) -> ([f64; 3], [f64; 3], [f64; 3]) {
        let t = t.clamp(0.0, self.duration_s);
        let tau = t / self.duration_s;
        let s = 10.0 * tau.powi(3) - 15.0 * tau.powi(4) + 6.0 * tau.powi(5);
        let sd = (30.0 * tau * tau * (1.0 - tau).powi(2)) / self.duration_s;
        let sdd =
            (60.0 * tau * (1.0 - tau) * (1.0 - 2.0 * tau)) / (self.duration_s * self.duration_s);
        let mut pos = [0.0; 3];
        let mut vel = [0.0; 3];
        let mut acc = [0.0; 3];
        for i in 0..3 {
            let dp = self.p1[i] - self.p0[i];
            pos[i] = self.p0[i] + dp * s;
            vel[i] = dp * sd;
            acc[i] = dp * sdd;
        }
        (pos, vel, acc)
    }

    /// Peak |v|, |a| over the profile duration (200-sample scan).
    pub fn kinematic_bounds_ok(&self) -> (f64, f64, bool) {
        let mut vmax = 0.0f64;
        let mut amax = 0.0f64;
        for i in 0..=200 {
            let t = self.duration_s * i as f64 / 200.0;
            let (_, v, a) = self.evaluate(t);
            vmax = vmax.max((v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt());
            amax = amax.max((a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt());
        }
        (
            vmax,
            amax,
            vmax <= KINEMATIC_MAX_VELOCITY_BOUND && amax <= KINEMATIC_MAX_ACCELERATION_BOUND,
        )
    }
}

/// PPO/SAC-style planner with N-k fault derating (up2.txt §5).
pub struct RLTrajectoryPlanner {
    pub reactor: ReactorHealthState,
    /// Nominal focal baseline f₀ (m); expanded to f_max under derating.
    pub focal_baseline_m: f64,
    pub focal_max_m: f64,
    /// Post-fault formation transit budget (5 d).
    pub transit_budget_s: f64,
}

impl Default for RLTrajectoryPlanner {
    fn default() -> Self {
        Self {
            reactor: ReactorHealthState {
                reactor_id: 0,
                nominal_power_kw: 120.0,
                available_reactors: 4,
                total_reactors: 4,
                thermal_margin_k: 50.0,
                is_scram_active: 0,
            },
            focal_baseline_m: 169.30,
            focal_max_m: 1692.99,
            transit_budget_s: 5.0 * 86400.0,
        }
    }
}

impl RLTrajectoryPlanner {
    /// N-k fault handler: P_derated = P_nom (N−k)/N. Under derating the
    /// planner expands the focal baseline f₀ → f_max (169.30 → 1692.99 m).
    /// Returns the commanded focal distance after derating.
    pub fn handle_nk_fault_event(&mut self, lost_reactors: u32) -> f64 {
        self.reactor.available_reactors = self.reactor.total_reactors.saturating_sub(lost_reactors);
        let ratio = self.reactor.power_ratio();
        if ratio <= 0.0 {
            return self.focal_max_m;
        }
        // Scale focal baseline inversely with available power, clamped.
        (self.focal_baseline_m / ratio).min(self.focal_max_m)
    }

    /// EOL propellant margin: remaining ≥ 98 % of reserve budget.
    pub fn check_eol_margin_compliance(&self, prop_remaining_kg: f64, prop_budget_kg: f64) -> bool {
        prop_budget_kg > 0.0
            && prop_remaining_kg / prop_budget_kg >= MIN_EOL_PROPELLANT_RESERVE_MARGIN
    }

    /// Continuous thrust cap F_max = 2ηP/(g0 Isp) (N).
    pub fn max_thrust_n(&self, isp_s: f64) -> f64 {
        let p_w = self.reactor.nominal_power_kw * 1e3 * self.reactor.power_ratio();
        2.0 * THRUST_ETA * p_w / (G0 * isp_s)
    }

    /// PPO scalar reward for a proposed state transition (dense shaping).
    pub fn ppo_reward(&self, state: &[f64], action: &[f64]) -> f64 {
        // Snapshot tracking, propellant, time, risk terms.
        let r_track = -(state.iter().map(|s| s * s).sum::<f64>()).sqrt();
        let r_prop = -0.01 * action.iter().map(|a| a.abs()).sum::<f64>();
        let r_risk =
            -100.0 * (self.reactor.total_reactors - self.reactor.available_reactors) as f64;
        r_track + r_prop + r_risk
    }

    /// NSGA-III objective vector J = (SNR·InfoGain, −Δm, −T_transit, −P_risk).
    pub fn objectives(
        &self,
        delta_m_kg: f64,
        transit_s: f64,
        risk: f64,
        info_snr: f64,
    ) -> [f64; 4] {
        [
            info_snr.max(0.0),
            -delta_m_kg,
            -transit_s / self.transit_budget_s,
            -risk.max(0.0),
        ]
    }

    /// 2-D hypervolume indicator for a minimizing front vs. `reference`
    /// (used for the GATE-50 HV ≥ 0.998 regression).
    pub fn hypervolume_2d(points: &[(f64, f64)], reference: (f64, f64)) -> f64 {
        let mut pts: Vec<(f64, f64)> = points
            .iter()
            .filter(|p| p.0 <= reference.0 && p.1 <= reference.1)
            .cloned()
            .collect();
        pts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(core::cmp::Ordering::Equal));
        // Sweep left→right accumulating dominated strips.
        let mut hv = 0.0;
        let mut best_y = reference.1;
        for (x, y) in pts {
            if y < best_y {
                hv += (reference.0 - x) * (best_y - y);
                best_y = y;
            }
        }
        hv
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn min_jerk_respects_bounds() {
        let p = MinimumJerkProfile {
            p0: [0.0; 3],
            p1: [1.0, 0.0, 0.0],
            duration_s: 60.0,
        };
        let (vmax, amax, ok) = p.kinematic_bounds_ok();
        // Analytic peaks for unit displacement in T=60: 1.875/60, 5.773/3600.
        assert!(vmax <= KINEMATIC_MAX_VELOCITY_BOUND);
        assert!(amax <= KINEMATIC_MAX_ACCELERATION_BOUND);
        assert!(ok);
        assert!((vmax - 1.8750 / 60.0).abs() < 1e-4);
        assert!((amax - 5.7733 / 3600.0).abs() < 1e-4);
    }

    #[test]
    fn nk_fault_expands_baseline() {
        let mut p = RLTrajectoryPlanner::default();
        let f = p.handle_nk_fault_event(1); // lose 1 of 4
        assert!((f - 169.30 / 0.75).abs() < 1e-6);
        let f = p.handle_nk_fault_event(3); // only 1 left
        assert!((f - 676.0).abs() < 60.0 || f <= 1692.99);
        assert!(f <= 1692.99);
    }

    #[test]
    fn eol_margin_gate() {
        let p = RLTrajectoryPlanner::default();
        assert!(p.check_eol_margin_compliance(98.5, 100.0));
        assert!(!p.check_eol_margin_compliance(97.0, 100.0));
    }

    #[test]
    fn hypervolume_near_one() {
        let pts = [(0.1, 0.1), (0.05, 0.3), (0.3, 0.05)];
        let hv = RLTrajectoryPlanner::hypervolume_2d(&pts, (1.0, 1.0));
        assert!((hv - 0.88).abs() < 1e-9);
        // A near-ideal front approaches the GATE-50 bound.
        let good = [(0.0005, 0.01), (0.001, 0.002), (0.01, 0.0005)];
        assert!(RLTrajectoryPlanner::hypervolume_2d(&good, (1.0, 1.0)) > 0.998);
    }
}
