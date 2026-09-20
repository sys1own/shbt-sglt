//! Semiconductor displacement-damage dose tracking (up1.txt §3):
//!   D_DDD = ∫ Φ(E)·S_NIEL(E) dE   [MeV·g⁻¹]
//!   Δ(1/β) = K_g · D_DDD        (Messenger–Spratt)

/// NIEL damage curve for InP (piecewise-log fit, MeV·cm²/g over 1–100 MeV
/// proton spectrum).
pub fn niel_inp(energy_mev: f64) -> f64 {
    // Approximate NIEL scaling ~ E^-0.5 · ln-tail for proton-induced
    // displacement in InP between 1–100 MeV.
    1.2e-3 * energy_mev.powf(-0.35)
}

/// GCR/SPE proton differential fluence model Φ(E) [protons/cm²/MeV].
/// Power-law GCR spectrum with SPE rollover.
pub fn proton_fluence(energy_mev: f64, mission_years: f64) -> f64 {
    let gcr = 2.0e4 * energy_mev.powf(-2.5);
    let spe = 8.0e5 * (-energy_mev / 30.0).exp();
    (gcr + spe) * mission_years
}

/// Accumulates displacement-damage dose for InP/InGaAs HBT electronics.
pub struct DddTracker {
    pub dose_mev_g: f64,
    /// Damage constant, g·MeV⁻¹ (3.4e-11 for InP HBT @ 4.2 K).
    pub k_g: f64,
    beta0: f64,
}

impl Default for DddTracker {
    fn default() -> Self {
        Self {
            dose_mev_g: 0.0,
            k_g: 3.4e-11,
            beta0: 100.0,
        }
    }
}

impl DddTracker {
    /// Integrate D_DDD over [e_min, e_max] MeV with midpoint quadrature.
    pub fn accumulate(&mut self, e_min: f64, e_max: f64, mission_years: f64) {
        let n = 512;
        let de = (e_max - e_min) / n as f64;
        let mut dose = 0.0;
        for i in 0..n {
            let e = e_min + (i as f64 + 0.5) * de;
            dose += proton_fluence(e, mission_years) * niel_inp(e) * de;
        }
        self.dose_mev_g += dose;
    }

    /// Messenger–Spratt current-gain degradation Δ(1/β) = K_g·D_DDD.
    pub fn gain_degradation(&self) -> f64 {
        self.k_g * self.dose_mev_g
    }

    /// Forward current gain under accumulated dose.
    pub fn beta(&self) -> f64 {
        1.0 / (1.0 / self.beta0 + self.gain_degradation())
    }
}
