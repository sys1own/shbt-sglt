
use core::ffi::c_int;
use libm::{exp, log10, sqrt};

pub const SPEED_OF_LIGHT: f64 = 299_792_458.0; // m/s
pub const PLANCK_CONSTANT: f64 = 6.626_070_15e-34; // J*s
pub const REDUCED_PLANCK: f64 = 1.054_571_817e-34; // J*s

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct SqueezedMetrologyConfig {
    pub wavelength_m: f64,
    pub carrier_power_w: f64,
    pub squeezing_param_r: f64,
    pub integration_bandwidth_hz: f64,
    pub heterodyne_efficiency: f64,
}

impl Default for SqueezedMetrologyConfig {
    fn default() -> Self {
        Self {
            wavelength_m: 1064.0e-9,
            carrier_power_w: 10.0e-3,
            squeezing_param_r: 2.50,
            integration_bandwidth_hz: 10000.0,
            heterodyne_efficiency: 0.985,
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct MetrologyMetrics {
    pub quadrature_variance: f64,
    pub squeezing_db: f64,
    pub range_noise_density_m_sqrt_hz: f64,
    pub range_uncertainty_3sigma_m: f64,
}

/// # Safety
/// `config` and `out_metrics` must be valid, non-null pointers.
#[no_mangle]
pub unsafe extern "C" fn sglt_squeezed_metrology_evaluate_phase(
    config: *const SqueezedMetrologyConfig,
    out_metrics: *mut MetrologyMetrics,
) -> c_int {
    if config.is_null() || out_metrics.is_null() {
        return -1;
    }

    let cfg = &*config;

    if cfg.squeezing_param_r < 0.0 || cfg.carrier_power_w <= 0.0 {
        return -2;
    }

    let r = cfg.squeezing_param_r;
    let exp_minus_2r = exp(-2.0 * r);
    let quad_var = 0.25 * exp_minus_2r;
    let squeezing_db = -10.0 * log10(exp_minus_2r);

    let omega = (2.0 * core::f64::consts::PI * SPEED_OF_LIGHT) / cfg.wavelength_m;
    let hbar_omega = REDUCED_PLANCK * omega;

    let sql_phase_density = sqrt(hbar_omega / (2.0 * cfg.carrier_power_w * cfg.heterodyne_efficiency));
    let squeezed_phase_density = sql_phase_density * exp(-r);

    let conversion_factor = cfg.wavelength_m / (2.0 * core::f64::consts::PI);
    let range_density = conversion_factor * squeezed_phase_density;
    let sigma_range = range_density * sqrt(cfg.integration_bandwidth_hz);
    let uncertainty_3sigma = 3.0 * sigma_range;

    (*out_metrics).quadrature_variance = quad_var;
    (*out_metrics).squeezing_db = squeezing_db;
    (*out_metrics).range_noise_density_m_sqrt_hz = range_density;
    (*out_metrics).range_uncertainty_3sigma_m = uncertainty_3sigma;

    0
}