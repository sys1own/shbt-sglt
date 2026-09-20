//! Wideband wave optics: 1PN deflection, chromatic focal length, peak gain.

use crate::consts;
use crate::OpticsError;

/// Valid wideband coverage, metres (200 nm – 5.0 µm).
pub const LAMBDA_MIN_M: f64 = 200e-9;
pub const LAMBDA_MAX_M: f64 = 5.0e-6;

/// Operational raytrace configuration (native mirror of the C-ABI struct).
#[derive(Debug, Clone, Copy)]
pub struct RaytraceConfig {
    pub wavelength_min_nm: f64,
    pub wavelength_max_nm: f64,
    pub focal_distance_au: f64,
    pub primary_aperture_m: f64,
    pub grid_resolution_x: u32,
    pub grid_resolution_y: u32,
}

impl RaytraceConfig {
    pub fn validate(&self) -> Result<(), OpticsError> {
        if self.wavelength_min_nm < 200.0 || self.wavelength_max_nm > 5000.0 {
            return Err(OpticsError::WavelengthOutOfRange(self.wavelength_min_nm));
        }
        if self.focal_distance_au < consts::Z0_AU {
            return Err(OpticsError::BelowFocalOnset(self.focal_distance_au));
        }
        if self.grid_resolution_x == 0 || self.grid_resolution_y == 0 {
            return Err(OpticsError::BadGrid(
                self.grid_resolution_x,
                self.grid_resolution_y,
            ));
        }
        Ok(())
    }
}

/// Einstein deflection angle θ(b) = 4GM/(c²b) = 2r_g/b (radians).
pub fn deflection_angle(impact_parameter_m: f64) -> f64 {
    2.0 * consts::R_G / impact_parameter_m
}

/// Chromatic effective focal length at receiver distance `z_m`:
/// f(λ) = r₀² c² / (4 G M_seed(λ)).  `m_seed` carries the plasma-refractive
/// correction δn(λ,r) folded into an effective mass-energy (kg).
pub fn effective_focal_length(aperture_radius_m: f64, m_seed_kg: f64, lambda_m: f64) -> f64 {
    // Plasma dispersion δn ∝ 1/ω² modulates the effective seed mass.
    let omega = 2.0 * std::f64::consts::PI * consts::C / lambda_m;
    let _plasma_weight = 1.0 / (omega * omega);
    let m_eff = m_seed_kg; // M_seed(λ) caller-supplied effective mass
    aperture_radius_m * aperture_radius_m * consts::C * consts::C / (4.0 * consts::G * m_eff)
}

/// On-axis light amplification µ₀(λ) = 4π² r_g / λ.
pub fn peak_amplification(lambda_m: f64) -> f64 {
    4.0 * std::f64::consts::PI.powi(2) * consts::R_G / lambda_m
}

/// Corona plasma refractive shift δn(λ,r) = −e² N_e(r) / (2 ε₀ m_e ω²).
pub fn plasma_refractive_shift(electron_density_m3: f64, lambda_m: f64) -> f64 {
    const E: f64 = 1.602176634e-19;
    const EPS0: f64 = 8.8541878128e-12;
    const ME: f64 = 9.1093837015e-31;
    let omega = 2.0 * std::f64::consts::PI * consts::C / lambda_m;
    -(E * E * electron_density_m3) / (2.0 * EPS0 * ME * omega * omega)
}
