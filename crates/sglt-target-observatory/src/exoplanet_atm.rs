//! Exoplanet atmospheric transmission spectroscopy (up1.txt §7):
//!   δ(λ) = (R_p/R_*)² + (2R_p/R_*²) ∫₀^z_max
//!          (1 − exp[−Σ_m ∫ σ_m(λ,T,P) n_m ds]) (R_p + z) dz

use crate::spectra_gen::{molecular_cross_section, Species};

/// Isothermal/hydrostatic atmosphere model.
#[derive(Debug, Clone, Copy)]
pub struct Atmosphere {
    pub planet_radius_m: f64,
    pub star_radius_m: f64,
    pub scale_height_m: f64,
    pub surface_pressure_pa: f64,
    pub temperature_k: f64,
    /// Number-density fraction per species, order = SPECIES.
    pub mixing_ratios: [f64; 5],
    pub z_max_m: f64,
}

impl Default for Atmosphere {
    /// Earth-twin default.
    fn default() -> Self {
        Self {
            planet_radius_m: 6.371e6,
            star_radius_m: 6.957e8,
            scale_height_m: 8.4e3,
            surface_pressure_pa: 1.01325e5,
            temperature_k: 288.0,
            mixing_ratios: [1.0e-3, 0.21, 4.0e-4, 1.8e-6, 4.0e-8],
            z_max_m: 120e3,
        }
    }
}

/// Total slant optical depth through the limb at tangent altitude `z`,
/// summing all five species. n_m(z) = n_m,0 · exp(−z/H).
fn optical_depth(atm: &Atmosphere, z: f64, lambda_um: f64) -> f64 {
    let k_b = 1.380649e-23;
    let n_total0 = atm.surface_pressure_pa / (k_b * atm.temperature_k);
    // Slant path enhancement for hydrostatic atmosphere ≈ √(2π R_p/H).
    let slant = (2.0 * std::f64::consts::PI * atm.planet_radius_m / atm.scale_height_m).sqrt();
    let mut tau = 0.0;
    for (i, sp) in Species::ALL.iter().enumerate() {
        let n_m = n_total0 * atm.mixing_ratios[i] * (-z / atm.scale_height_m).exp();
        tau += molecular_cross_section(*sp, lambda_um) * n_m * atm.scale_height_m * slant;
    }
    tau
}

/// Transit depth δ(λ) via midpoint integration over z ∈ [0, z_max].
pub fn transit_depth(atm: &Atmosphere, lambda_um: f64, nz: usize) -> f64 {
    let rp2 = atm.planet_radius_m.powi(2);
    let rs2 = atm.star_radius_m.powi(2);
    let dz = atm.z_max_m / nz as f64;
    let mut integral = 0.0;
    for i in 0..nz {
        let z = (i as f64 + 0.5) * dz;
        let absorb = 1.0 - (-optical_depth(atm, z, lambda_um)).exp();
        integral += absorb * (atm.planet_radius_m + z) * dz;
    }
    rp2 / rs2 + 2.0 * atm.planet_radius_m / rs2 * integral
}
