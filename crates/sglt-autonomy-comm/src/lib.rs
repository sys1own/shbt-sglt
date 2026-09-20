//! sglt-autonomy-comm — 8-phase mission autonomy FSM plus the 12 Gbps
//! deep-space optical downlink and 32 GHz Ka-band backup link (up1.txt §5).

pub mod fsm_engine;
pub mod mppm_fec;
pub mod optical_link;
pub mod rf_link;
pub mod rl_autonomy;

pub use rl_autonomy::{
    MinimumJerkProfile, RLTrajectoryPlanner, ReactorHealthState, KINEMATIC_MAX_ACCELERATION_BOUND,
    KINEMATIC_MAX_VELOCITY_BOUND, MIN_EOL_PROPELLANT_RESERVE_MARGIN,
};

pub use fsm_engine::{MissionPhase, MissionStateMachine};
pub use mppm_fec::{ppm_ber, rs_correctable, LDPC_RATE};
pub use optical_link::OpticalLink;
pub use rf_link::KaBandLink;

/// `repr(C)` mirror of `sglt_comm_link_t` in `include/sglt_v2_abi.h`.
#[repr(C)]
pub struct CommLinkC {
    pub downlink_rate_gbps: f64,
    pub rytov_variance: f64,
    pub bit_error_rate: f64,
    pub optical_pointing_lock: bool,
    pub ka_band_fallback_active: bool,
}

/// Evaluate the combined comm link state at `distance_au`.
///
/// # Safety
/// `link` must be a valid, non-null pointer.
#[no_mangle]
pub unsafe extern "C" fn sglt_comm_link_evaluate(
    link: *mut CommLinkC,
    distance_au: f64,
    jitter_rad: f64,
) -> i32 {
    if link.is_null() {
        return -1;
    }
    let l = &mut *link;
    let opt = OpticalLink::default();
    let pr_w = opt.received_power_w(distance_au, jitter_rad);
    l.downlink_rate_gbps = opt.data_rate_gbps(pr_w);
    l.rytov_variance = opt.rytov_variance(opt.cn2_nominal, opt.path_length_m);
    l.bit_error_rate = ppm_ber(16, opt.photons_per_pulse(pr_w));
    l.optical_pointing_lock = jitter_rad <= 50e-9;
    l.ka_band_fallback_active = !l.optical_pointing_lock;
    if l.ka_band_fallback_active {
        l.downlink_rate_gbps = KaBandLink::default().data_rate_bps(distance_au) / 1e9;
    }
    0
}
