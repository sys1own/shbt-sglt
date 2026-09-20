//! sglt-optical-raytrace — wideband multi-spectral wave-optics raytracer for the
//! Solar Gravitational Lens (up1.txt §2).
//!
//! Implements the 1PN relativistic deflection model over
//! λ ∈ [200 nm, 5.0 µm], eikonal image-plane amplitude, extended-disk source
//! convolution, coronagraphic suppression, and the CMOS/EMCCD FPA noise model.

pub mod coronagraph;
pub mod eikonal;
pub mod fpa_noise;
pub mod wave_optics;

pub use coronagraph::Coronagraph;
pub use eikonal::{eikonal_amplitude, psf};
pub use fpa_noise::{DetectorBand, FpaNoiseModel, FpaParams};
pub use wave_optics::{deflection_angle, effective_focal_length, peak_amplification, RaytraceConfig};

/// Physical constants (SI).
pub mod consts {
    /// Schwarzschild radius of the Sun, m (≈ 2.95 km).
    pub const R_G: f64 = 2.95325008e3;
    /// Solar radius, m.
    pub const R_SUN: f64 = 6.957e8;
    /// Gravitational constant.
    pub const G: f64 = 6.67430e-11;
    /// Solar mass, kg.
    pub const M_SUN: f64 = 1.98847e30;
    /// Speed of light, m/s.
    pub const C: f64 = 2.99792458e8;
    /// One astronomical unit, m.
    pub const AU: f64 = 1.495978707e11;
    /// SGL focal-line onset, AU (R_☉² / 2r_g ≈ 547.8).
    pub const Z0_AU: f64 = R_SUN * R_SUN / (2.0 * R_G) / AU;
}

/// Errors from the optical pipeline.
#[derive(Debug, thiserror::Error)]
pub enum OpticsError {
    #[error("wavelength {0} nm outside [200, 5000] nm band")]
    WavelengthOutOfRange(f64),
    #[error("focal distance {0} AU below SGL focal-line onset")]
    BelowFocalOnset(f64),
    #[error("invalid grid resolution {0}x{1}")]
    BadGrid(u32, u32),
    #[error("null pointer passed to C-ABI entry point")]
    NullPointer,
}

/// Execute a full image-plane raytrace and write the irradiance grid into
/// `output_image_plane` (row-major, `grid_resolution_x * grid_resolution_y`).
///
/// Returns 0 on success, negative error code otherwise.
///
/// # Safety
/// `config` and `output_image_plane` must be valid, non-null pointers;
/// `output_image_plane` must have room for nx*ny f32 elements.
#[no_mangle]
pub unsafe extern "C" fn sglt_optics_raytrace_execute(
    config: *const RaytraceConfigC,
    output_image_plane: *mut f32,
) -> i32 {
    if config.is_null() || output_image_plane.is_null() {
        return -1;
    }
    let cfg = &*config;
    let nx = cfg.grid_resolution_x as usize;
    let ny = cfg.grid_resolution_y as usize;
    if nx == 0 || ny == 0 {
        return -2;
    }
    let out = std::slice::from_raw_parts_mut(output_image_plane, nx * ny);
    let mid_nm = 0.5 * (cfg.wavelength_min_nm + cfg.wavelength_max_nm);
    let lambda_m = mid_nm * 1e-9;
    let z_m = cfg.focal_distance_au * consts::AU;
    // Image plane extent: 10 m radius field per up1.txt GATE-03.
    let extent = 10.0_f64;
    for j in 0..ny {
        for i in 0..nx {
            let x = (i as f64 / (nx - 1) as f64 - 0.5) * 2.0 * extent;
            let y = (j as f64 / (ny - 1) as f64 - 0.5) * 2.0 * extent;
            let rho = (x * x + y * y).sqrt();
            out[j * nx + i] = psf(rho, lambda_m, z_m) as f32;
        }
    }
    0
}

/// `repr(C)` mirror of `sglt_raytrace_config_t` in `include/sglt_v2_abi.h`.
#[repr(C)]
pub struct RaytraceConfigC {
    pub wavelength_min_nm: f64,
    pub wavelength_max_nm: f64,
    pub focal_distance_au: f64,
    pub primary_aperture_m: f64,
    pub grid_resolution_x: u32,
    pub grid_resolution_y: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn onset_is_547_au() {
        assert!((consts::Z0_AU - 547.8).abs() < 1.0);
    }

    #[test]
    fn amplification_matches_gate04() {
        // µ0(1 µm) = 4π² r_g / λ ≈ 1.17e11
        let mu = peak_amplification(1.0e-6);
        assert!((mu / 1.17e11 - 1.0).abs() < 0.01, "{mu}");
    }
}
