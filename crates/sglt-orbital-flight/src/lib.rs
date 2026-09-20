//! sglt-orbital-flight — ephemeris-driven 4-body CR3BP astrodynamics
//! (up1.txt §3): Sun–Earth/Moon–Jupiter rotating-frame dynamics with
//! Prince–Dormand 8(7) DOP853 adaptive integration, Jacobi energy tracking,
//! SPICE ephemeris sync, and NIEL/Messenger–Spratt radiation dose.

pub mod cr3bp_4body;
pub mod dop853;
pub mod radiation_ddd;
pub mod spice_ephem;

pub use cr3bp_4body::{cr3bp_acceleration, jacobi_constant, Cr3bpParams};
pub use dop853::{Dop853, IntegratorConfig};
pub use radiation_ddd::DddTracker;
pub use spice_ephem::Ephemeris;

/// `repr(C)` mirror of `sglt_cr3bp_state_t` in `include/sglt_v2_abi.h`.
#[repr(C)]
pub struct Cr3bpStateC {
    pub epoch_jd: f64,
    pub state_vector: [f64; 6],
    pub mass_ratio_mu: f64,
    pub jacobi_constant: f64,
    pub radiation_dose_ddd_gy: f64,
}

/// Advance one DOP853 step in normalized rotating-frame time.
/// `step_size_sec` is interpreted against a 1-year characteristic time
/// (seconds/yr → normalized units via the Sun–Earth system).
///
/// # Safety
/// `state` must be a valid, non-null pointer.
#[no_mangle]
pub unsafe extern "C" fn sglt_orbital_dop853_step(
    state: *mut Cr3bpStateC,
    step_size_sec: f64,
) -> i32 {
    if state.is_null() {
        return -1;
    }
    let s = &mut *state;
    let params = Cr3bpParams {
        mu: s.mass_ratio_mu,
        ..Default::default()
    };
    let mut integ = Dop853::new(IntegratorConfig::default());
    let f = |y: &[f64; 6]| -> [f64; 6] { cr3bp_acceleration(y, &params) };
    // Normalized time: Earth year ≈ 365.25 d.
    let dt = step_size_sec / (365.25 * 86400.0) * 2.0 * std::f64::consts::PI;
    integ.step(&mut s.state_vector, dt, &f);
    s.jacobi_constant = jacobi_constant(&s.state_vector, params.mu);
    s.epoch_jd += step_size_sec / 86400.0;
    0
}
