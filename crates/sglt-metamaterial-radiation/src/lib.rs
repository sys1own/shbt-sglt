
use core::ffi::c_int;

pub const HEALING_THRESHOLD_FLUENCE_MJ_CM2: f64 = 27.9; // mJ/cm^2
pub const GST_CRYSTALLIZATION_TEMP_K: f64 = 623.0; // Kelvin
pub const GST_MELT_TEMP_K: f64 = 888.0; // Kelvin

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct RadiationState {
    pub accumulated_ddd_krad: f64,
    pub current_conductivity_s_m: f64,
    pub initial_conductivity_s_m: f64,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct SelfHealingPulseConfig {
    pub pulse_energy_mj_cm2: f64,
    pub pulse_duration_ns: f64,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct HealingResult {
    pub post_healing_conductivity_s_m: f64,
    pub recovery_ratio: f64,
    pub lattice_reorganized: bool,
}

/// # Safety
/// `rad_state`, `pulse_cfg`, and `out_result` must be valid, non-null pointers.
#[no_mangle]
pub unsafe extern "C" fn sglt_metamaterial_trigger_self_healing(
    rad_state: *const RadiationState,
    pulse_cfg: *const SelfHealingPulseConfig,
    out_result: *mut HealingResult,
) -> c_int {
    if rad_state.is_null() || pulse_cfg.is_null() || out_result.is_null() {
        return -1;
    }

    let state = &*rad_state;

    let pulse = &*pulse_cfg;

    if pulse.pulse_energy_mj_cm2 < HEALING_THRESHOLD_FLUENCE_MJ_CM2 {
        (*out_result).post_healing_conductivity_s_m = state.current_conductivity_s_m;
        (*out_result).recovery_ratio = state.current_conductivity_s_m / state.initial_conductivity_s_m;
        (*out_result).lattice_reorganized = false;
        return 0;
    }

    let recovery = 0.9995;
    let healed_sigma = state.initial_conductivity_s_m * recovery;

    (*out_result).post_healing_conductivity_s_m = healed_sigma;
    (*out_result).recovery_ratio = recovery;
    (*out_result).lattice_reorganized = true;

    0
}
/// C-ABI alias matching the unified `sglt_abi.h` surface name.
/// # Safety
/// `rad_state`, `pulse_cfg`, and `out_result` must be valid, non-null pointers.
#[no_mangle]
pub unsafe extern "C" fn sglt_metamaterial_radiation_heal(
    rad_state: *const RadiationState,
    pulse_cfg: *const SelfHealingPulseConfig,
    out_result: *mut HealingResult,
) -> c_int {
    sglt_metamaterial_trigger_self_healing(rad_state, pulse_cfg, out_result)
}
