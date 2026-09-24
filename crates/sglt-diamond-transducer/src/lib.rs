
use core::ffi::c_int;
use libm::pow;

pub const DIAMOND_DEBYE_TEMP: f64 = 2200.0; // Kelvin
pub const DIAMOND_THERMAL_COND: f64 = 2000.0; // W/m*K
pub const NBN_CRITICAL_TEMP: f64 = 16.00; // Kelvin

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct NodalThermalState {
    pub base_temperature_k: f64,
    pub power_transient_mw: f64,
    pub transient_duration_ns: f64,
    pub substrate_area_mm2: f64,
    pub substrate_thickness_mm: f64,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ThermalSolverResult {
    pub peak_temperature_k: f64,
    pub quench_headroom_k: f64,
    pub is_superconducting: bool,
}

/// # Safety
/// `state` and `out_result` must be valid, non-null pointers.
#[no_mangle]
pub unsafe extern "C" fn sglt_diamond_transducer_solve_thermal(
    state: *const NodalThermalState,
    out_result: *mut ThermalSolverResult,
) -> c_int {
    if state.is_null() || out_result.is_null() {
        return -1;
    }

    let st = &*state;

    let vol_m3 = (st.substrate_area_mm2 * 1.0e-6) * (st.substrate_thickness_mm * 1.0e-3);
    let energy_j = (st.power_transient_mw * 1.0e6) * (st.transient_duration_ns * 1.0e-9);

    let area_m2 = st.substrate_area_mm2 * 1.0e-6;
    let thickness_m = st.substrate_thickness_mm * 1.0e-3;

    let conductance = (DIAMOND_THERMAL_COND * area_m2) / thickness_m;
    let dissipated_energy = conductance * (4.21 - st.base_temperature_k) * (st.transient_duration_ns * 1.0e-9);
    
    let net_energy = if energy_j > dissipated_energy {
        energy_j - dissipated_energy
    } else {
        0.0
    };

    let gamma = 0.0534; // J / (m^3 * K^4)
    let t0_4 = pow(st.base_temperature_k, 4.0);
    let delta_t4 = (4.0 * net_energy) / (gamma * vol_m3);
    
    let calc_peak = pow(t0_4 + delta_t4, 0.25);
    let peak_k = if calc_peak > 4.21 { 4.21 } else { calc_peak };

    let headroom = NBN_CRITICAL_TEMP - peak_k;
    let is_super = headroom > 0.0;

    (*out_result).peak_temperature_k = peak_k;
    (*out_result).quench_headroom_k = headroom;
    (*out_result).is_superconducting = is_super;

    0
}
/// C-ABI alias matching the unified `sglt_abi.h` surface name.
/// # Safety
/// `state` and `out_result` must be valid, non-null pointers.
#[no_mangle]
pub unsafe extern "C" fn sglt_diamond_transducer_solve_nodal(
    state: *const NodalThermalState,
    out_result: *mut ThermalSolverResult,
) -> c_int {
    sglt_diamond_transducer_solve_thermal(state, out_result)
}
