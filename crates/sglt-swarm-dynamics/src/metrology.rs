//! Distributed heterodyne metrology mesh (up2.txt §4):
//! λ0 = 1064 nm, N_links = M(M−1)/2, σ_r ≤ 0.144 pm/√Hz,
//! DWS: Δφ = (16/3π)(w0/λ0) kθ → σ_θ ≤ 11.38 nrad.

use crate::{DWS_SIGMA_THETA_NRAD, HETERODYNE_SIGMA_R_PM, METROLOGY_WAVELENGTH};

/// Beam waist at Rx aperture, m.
pub const W0_M: f64 = 1.0;

/// DWS longitudinal phase response: Δφ = (16/(3π))(w0/λ0)·k·θ.
pub fn dws_phase_response(theta_rad: f64, k_wavefront: f64) -> f64 {
    (16.0 / (3.0 * std::f64::consts::PI)) * (W0_M / METROLOGY_WAVELENGTH) * k_wavefront * theta_rad
}

/// Convert a phase-noise floor (rad/√Hz) to tilt noise (nrad/√Hz).
pub fn theta_noise_nrad(phi_noise_rad: f64, k_wavefront: f64) -> f64 {
    let resp = (16.0 / (3.0 * std::f64::consts::PI)) * (W0_M / METROLOGY_WAVELENGTH) * k_wavefront;
    phi_noise_rad / resp * 1e9
}

/// GATE-44 gate check.
pub fn range_noise_ok(sigma_r_pm: f64) -> bool {
    sigma_r_pm <= HETERODYNE_SIGMA_R_PM
}

/// GATE-45 gate check.
pub fn dws_noise_ok(sigma_theta_nrad: f64) -> bool {
    sigma_theta_nrad <= DWS_SIGMA_THETA_NRAD
}
