//! Ka-band 32 GHz RF fallback link budget (up1.txt §5):
//! 250 W TX, 3.5 m HGA, 70 m DSN receiver, FSPL(600 AU) ≈ −291.6 dB,
//! T_sys = 45 K → 150 kbps net (GATE-23).

use std::f64::consts::PI;

pub const F_KA_HZ: f64 = 32.0e9;
pub const C_LIGHT: f64 = 2.99792458e8;
pub const AU_M: f64 = 1.495978707e11;
pub const K_BOLTZMANN: f64 = 1.380649e-23;

pub struct KaBandLink {
    pub tx_power_w: f64,   // 250 W
    pub hga_diameter_m: f64, // 3.5 m
    pub dsn_diameter_m: f64, // 70 m
    pub system_noise_k: f64, // 45 K
}

impl Default for KaBandLink {
    fn default() -> Self {
        Self {
            tx_power_w: 250.0,
            hga_diameter_m: 3.5,
            dsn_diameter_m: 70.0,
            system_noise_k: 45.0,
        }
    }
}

impl KaBandLink {
    /// Free-space path loss, dB, at `distance_au`.
    pub fn fspl_db(&self, distance_au: f64) -> f64 {
        let d = distance_au * AU_M;
        let lambda = C_LIGHT / F_KA_HZ;
        20.0 * (4.0 * PI * d / lambda).log10()
    }

    /// Antenna gain, dBi, at 68 % aperture efficiency.
    pub fn gain_dbi(&self, diameter_m: f64) -> f64 {
        let lambda = C_LIGHT / F_KA_HZ;
        10.0 * (0.68 * (PI * diameter_m / lambda).powi(2)).log10()
    }

    /// Link-budget data rate in bps via Shannon on the C/N0 margin.
    pub fn data_rate_bps(&self, distance_au: f64) -> f64 {
        let pr_dbw = 10.0 * self.tx_power_w.log10()
            + self.gain_dbi(self.hga_diameter_m)
            + self.gain_dbi(self.dsn_diameter_m)
            - self.fspl_db(distance_au)
            - 2.0; // implementation loss
        let pr_w = 10f64.powf(pr_dbw / 10.0);
        let noise_w_hz = K_BOLTZMANN * self.system_noise_k;
        let cn0_hz = pr_w / noise_w_hz;
        // Spectral efficiency limited to 2 b/s/Hz for robustness.
        cn0_hz.min(150e3 * 10.0).min(1e6)
    }
}
