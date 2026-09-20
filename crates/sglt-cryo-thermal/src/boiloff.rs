//! Cryogen boil-off kinetics (up1.txt §4):
//!   dm_cryo/dt = −(Q_parasitic + Q_dissipated)
//!              / (ΔH_vap + ∫₄.₂^T_vent Cp(T) dT)
//! GATE-14: mass loss tracks enthalpy change ΔH_vap.

use crate::he4_fluid::HE4_LHV_J_KG;

/// Default vent temperature for sensible-heat credit, K.
pub const T_VENT_K: f64 = 77.0;
/// Helium gas mean Cp between 4.2 K and vent, J/kg·K.
pub const CP_GAS_J_KG_K: f64 = 5200.0;

pub struct BoiloffKinetics;

impl BoiloffKinetics {
    /// Effective enthalpy sink per kg boiled off, J/kg.
    pub fn effective_enthalpy(t_vent_k: f64) -> f64 {
        HE4_LHV_J_KG + CP_GAS_J_KG_K * (t_vent_k - 4.2).max(0.0)
    }

    /// dm/dt, kg/s, for a total absorbed heat load `q_w`.
    pub fn mass_rate_kg_s(q_w: f64, t_vent_k: f64) -> f64 {
        -q_w / Self::effective_enthalpy(t_vent_k)
    }

    /// Mass loss over `dt_s` seconds, kg.
    pub fn mass_loss_kg(q_w: f64, dt_s: f64) -> f64 {
        -Self::mass_rate_kg_s(q_w, T_VENT_K) * dt_s
    }
}
