
use core::ffi::c_int;
use libm::{pow, sin};

pub const GRAVITATIONAL_RADIUS_SUN: f64 = 1476.625; // meters (GM/c^2)
pub const SOLAR_RADIUS: f64 = 6.9634e8; // meters
pub const ELECTRON_CHARGE: f64 = 1.602_176_634e-19; // C
pub const ELECTRON_MASS: f64 = 9.109_383_7015e-31; // kg
pub const VACUUM_PERMITTIVITY: f64 = 8.854_187_8128e-12; // F/m
pub const SPEED_OF_LIGHT: f64 = 299_792_458.0; // m/s

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct CoronalOpticsParams {
    pub heliocentric_r_solar_radii: f64,
    pub tilt_angle_deg: f64,
    pub optical_frequency_hz: f64,
    pub cme_factor: f64,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct EikonalPhaseResult {
    pub grav_2pn_phase_rad: f64,
    pub plasma_phase_rad: f64,
    pub total_predistortion_phase_rad: f64,
}

#[no_mangle]
pub extern "C" fn sglt_2pn_coronal_optics_compute_eikonal(
    params: *const CoronalOpticsParams,
    out_result: *mut EikonalPhaseResult,
) -> c_int {
    if params.is_null() || out_result.is_null() {
        return -1;
    }

    let p = unsafe { &*params };

    if p.heliocentric_r_solar_radii <= 1.0 {
        return -2; // Inside solar interior
    }

    let r_m = p.heliocentric_r_solar_radii * SOLAR_RADIUS;
    let r_norm = p.heliocentric_r_solar_radii;

    // Baumbach-Allen model: Ne = A/r^6 + B/r^2
    let a_term = 1.55e14 / pow(r_norm, 6.0);
    let b_term = 3.6e12 / pow(r_norm, 2.0);
    let n_e = (a_term + b_term) * (1.0 + p.cme_factor);

    let omega = 2.0 * core::f64::consts::PI * p.optical_frequency_hz;
    let k0 = omega / SPEED_OF_LIGHT;

    // 2PN Gravitational Refractive Index term: 4 r_g / r + 7 r_g^2 / r^2
    let term_1pn = (4.0 * GRAVITATIONAL_RADIUS_SUN) / r_m;
    let term_2pn = (7.0 * GRAVITATIONAL_RADIUS_SUN * GRAVITATIONAL_RADIUS_SUN) / (r_m * r_m);
    let n_grav_minus_1 = 0.5 * (term_1pn + term_2pn);

    // Plasma Refractive Index term: - e^2 N_e / (2 epsilon_0 m_e omega^2)
    let omega_p_sq = (ELECTRON_CHARGE * ELECTRON_CHARGE * n_e) / (VACUUM_PERMITTIVITY * ELECTRON_MASS);
    let n_plasma_minus_1 = -0.5 * (omega_p_sq / (omega * omega));

    let path_length_m = r_m * sin(p.tilt_angle_deg * (core::f64::consts::PI / 180.0));

    let grav_phase = k0 * n_grav_minus_1 * path_length_m;
    let plasma_phase = k0 * n_plasma_minus_1 * path_length_m;
    let total_phase = grav_phase + plasma_phase;

    unsafe {
        (*out_result).grav_2pn_phase_rad = grav_phase;
        (*out_result).plasma_phase_rad = plasma_phase;
        (*out_result).total_predistortion_phase_rad = total_phase;
    }

    0
}
/// C-ABI alias matching the unified `sglt_abi.h` surface name.
#[no_mangle]
pub extern "C" fn sglt_2pn_coronal_optics_evaluate(
    params: *const CoronalOpticsParams,
    out_result: *mut EikonalPhaseResult,
) -> c_int {
    sglt_2pn_coronal_optics_compute_eikonal(params, out_result)
}
