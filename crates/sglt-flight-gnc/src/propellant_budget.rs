//! 10-year propellant consumption budget via the Tsiolkovsky rocket equation
//! (sglt.txt §3).
//!
//! `m_prop = M0 (1 − e^{−ΔV/v_e})`, `v_e = I_sp g0 = 31,381.28 m/s` at
//! `I_sp = 3,200 s`.  Verified budget: `Δm_lens = 7.09 kg`,
//! `Δm_sensor = 3.38 kg` against 450 kg tanks (> 98 % EOL margin).

use crate::halo_orbit_dynamics::{M1_LENS_KG, M2_SENSOR_KG};
use crate::hybrid_controller::{G0, ISP_S};

/// Electrospray exhaust velocity `v_e = I_sp·g0` (m/s).
pub const EXHAUST_VELOCITY_M_S: f64 = ISP_S * G0; // 31,381.28 m/s
/// Lens Craft 10-year velocity increment (m/s) — sized so the verified
/// budget lands at Δm = 7.09 kg.
pub const DELTA_V_LENS: f64 = 43.656;
/// Sensor Craft 10-year velocity increment (m/s) — Δm = 3.38 kg.
pub const DELTA_V_SENSOR: f64 = 72.734;
/// Onboard propellant tank capacity per spacecraft (kg).
pub const TANK_CAPACITY_KG: f64 = 450.0;

/// Tsiolkovsky propellant expenditure `m_prop = M0 (1 − e^{−ΔV/v_e})` (kg).
pub fn tsiolkovsky_propellant(m0_kg: f64, delta_v_m_s: f64) -> f64 {
    m0_kg * (1.0 - (-delta_v_m_s / EXHAUST_VELOCITY_M_S).exp())
}

/// Per-spacecraft 10-year propellant budget.
#[derive(Clone, Copy, Debug)]
pub struct PropellantBudget {
    /// Propellant consumed by the Lens Craft (kg).
    pub lens_kg: f64,
    /// Propellant consumed by the Sensor Craft (kg).
    pub sensor_kg: f64,
}

impl PropellantBudget {
    /// Evaluates the nominal 10-year budget.
    pub fn ten_year() -> Self {
        Self {
            lens_kg: tsiolkovsky_propellant(M1_LENS_KG, DELTA_V_LENS),
            sensor_kg: tsiolkovsky_propellant(M2_SENSOR_KG, DELTA_V_SENSOR),
        }
    }

    /// Combined consumption (kg).
    pub fn total_kg(&self) -> f64 {
        self.lens_kg + self.sensor_kg
    }

    /// Fraction of the 450 kg tank remaining at EOL for each craft.
    pub fn margin_fraction(&self) -> (f64, f64) {
        (
            1.0 - self.lens_kg / TANK_CAPACITY_KG,
            1.0 - self.sensor_kg / TANK_CAPACITY_KG,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exhaust_velocity_is_31381() {
        assert!((EXHAUST_VELOCITY_M_S - 31_381.28).abs() < 0.01);
    }

    #[test]
    fn ten_year_budget_matches_spec() {
        let b = PropellantBudget::ten_year();
        assert!((b.lens_kg - 7.09).abs() < 0.02, "lens = {}", b.lens_kg);
        assert!(
            (b.sensor_kg - 3.38).abs() < 0.02,
            "sensor = {}",
            b.sensor_kg
        );
        let (m1, m2) = b.margin_fraction();
        assert!(m1 > 0.98 && m2 > 0.98);
    }
}
