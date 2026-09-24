//! 12 Gbps 1550 nm optical downlink over 600 AU (up1.txt §5):
//!   P_r = P_t G_t G_r (λ/4πd)² η_trans η_atm exp(−2σ_j²/θ_beam²)
//!   σ_R² = 1.23 Cn² k^{7/6} L^{11/6}

use std::f64::consts::PI;

pub const LAMBDA_COMM_M: f64 = 1550e-9;
pub const AU_M: f64 = 1.495978707e11;
pub const H_PLANCK: f64 = 6.62607015e-34;
pub const C_LIGHT: f64 = 2.99792458e8;

/// Primary optical telecom payload.
pub struct OpticalLink {
    pub tx_power_w: f64,   // 100 W
    pub tx_diameter_m: f64, // 1.0 m
    pub rx_diameter_m: f64, // 10.0 m
    pub beam_div_rad: f64, // 1.85 µrad
    pub eta_trans: f64,
    pub eta_atm: f64,
    pub cn2_nominal: f64,    // 1.5e-15 m^-2/3
    pub path_length_m: f64,  // atmospheric propagation path
}

impl Default for OpticalLink {
    fn default() -> Self {
        Self {
            tx_power_w: 100.0,
            tx_diameter_m: 1.0,
            rx_diameter_m: 10.0,
            beam_div_rad: 1.85e-6,
            eta_trans: 0.85,
            eta_atm: 0.90,
            cn2_nominal: 1.5e-15,
            path_length_m: 20e3,
        }
    }
}

impl OpticalLink {
    /// Received photon flux power, W, at `distance_au` with pointing jitter.
    pub fn received_power_w(&self, distance_au: f64, jitter_rad: f64) -> f64 {
        let d = distance_au * AU_M;
        let gt = (PI * self.tx_diameter_m / LAMBDA_COMM_M).powi(2);
        let gr = (PI * self.rx_diameter_m / LAMBDA_COMM_M).powi(2);
        let fs = (LAMBDA_COMM_M / (4.0 * PI * d)).powi(2);
        let jitter = (-2.0 * jitter_rad.powi(2) / self.beam_div_rad.powi(2)).exp();
        self.tx_power_w * gt * gr * fs * self.eta_trans * self.eta_atm * jitter
    }

    /// Photon energy at 1550 nm.
    pub fn photon_energy_j() -> f64 {
        H_PLANCK * C_LIGHT / LAMBDA_COMM_M
    }

    /// Mean detected photons per 16-PPM pulse slot.
    pub fn photons_per_pulse(&self, pr_w: f64) -> f64 {
        // 12 Gbps → 3 Gslots/s for 16-PPM (log2 16 = 4 bits/symbol)
        let slots_per_s = 12.0e9 / 4.0;
        pr_w / Self::photon_energy_j() / slots_per_s
    }

    /// Achievable data rate — capped by the 12 Gbps design point (GATE-20).
    pub fn data_rate_gbps(&self, pr_w: f64) -> f64 {
        // Simple photon-starvation roll-off vs. 1e-9 W reference sensitivity.
        let margin = (pr_w / 1e-9).clamp(0.0, 1.0);
        12.0 * margin
    }

    /// Rytov scintillation variance σ_R² = 1.23 Cn² k^{7/6} L^{11/6}.
    pub fn rytov_variance(&self, cn2: f64, path_m: f64) -> f64 {
        let k = 2.0 * PI / LAMBDA_COMM_M;
        1.23 * cn2 * k.powf(7.0 / 6.0) * path_m.powf(11.0 / 6.0)
    }
}
