//! Liquid helium-4 reservoir properties (up1.txt §4).

/// Latent heat of vaporization ΔH_vap = 20.9 J/g = 20 900 J/kg at 4.2 K.
pub const HE4_LHV_J_KG: f64 = 20_900.0;
/// Bath operating point.
pub const HE4_BATH_K: f64 = 4.2;
/// Saturated liquid density, kg/m³.
pub const HE4_RHO_KG_M3: f64 = 125.0;

/// Superfluid He-4 bath state.
pub struct He4Bath {
    pub temp_k: f64,
    pub mass_kg: f64,
}

impl He4Bath {
    /// Temperature-dependent heat capacity of He-II near the lambda point
    /// (approximate, J/kg·K).
    pub fn cp(&self) -> f64 {
        let t = self.temp_k.max(0.5);
        4.6e3 * (t / 4.2).powi(6) * (-5.6 * (1.0 - t / 2.17).abs()).exp()
    }

    /// Vent-gas sensible enthalpy ∫₄.₂^T_vent Cp dT, J/kg (Cp ≈ gas value).
    pub fn vent_sensible_enthalpy(&self, t_vent_k: f64) -> f64 {
        let cp_gas = 5200.0; // J/kg·K helium gas
        cp_gas * (t_vent_k - HE4_BATH_K).max(0.0)
    }
}
