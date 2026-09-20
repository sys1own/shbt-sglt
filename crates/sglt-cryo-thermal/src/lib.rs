//! sglt-cryo-thermal — 3D nodal transient thermal-fluid solver for the
//! cryogenic chain (up1.txt §4): He-4 bath, sapphire waveguide, aerogel
//! isolation, deep-space radiators, C/SiC bench FEM OPD, boil-off kinetics.

pub mod boiloff;
pub mod fem_structural;
pub mod he4_fluid;
pub mod nodal_solver;

pub use boiloff::BoiloffKinetics;
pub use fem_structural::{BenchFea, OPD_LIMIT_NM};
pub use he4_fluid::{He4Bath, HE4_LHV_J_KG};
pub use nodal_solver::{NodalNetwork, ThermalNode};

/// `repr(C)` mirror of `sglt_cryo_state_t` in `include/sglt_v2_abi.h`.
#[repr(C)]
pub struct CryoStateC {
    pub temp_he4_bath_k: f64,
    pub temp_sapphire_k: f64,
    pub temp_aerogel_k: f64,
    pub temp_radiator_k: f64,
    pub bench_opd_perturbation_nm: f64,
    pub cryogen_mass_kg: f64,
}

/// Advance the 4-node cryo chain by `dt_sec` seconds.
///
/// # Safety
/// `thermal_state` must be a valid, non-null pointer.
#[no_mangle]
pub unsafe extern "C" fn sglt_cryo_solve_nodal_step(
    thermal_state: *mut CryoStateC,
    dt_sec: f64,
) -> i32 {
    if thermal_state.is_null() {
        return -1;
    }
    let s = &mut *thermal_state;
    let mut net = NodalNetwork::cryo_chain(
        s.temp_he4_bath_k,
        s.temp_sapphire_k,
        s.temp_aerogel_k,
        s.temp_radiator_k,
    );
    net.step(dt_sec);
    s.temp_he4_bath_k = net.nodes[0].t_k;
    s.temp_sapphire_k = net.nodes[1].t_k;
    s.temp_aerogel_k = net.nodes[2].t_k;
    s.temp_radiator_k = net.nodes[3].t_k;
    let fea = BenchFea::default();
    s.bench_opd_perturbation_nm = fea.opd_nm(s.temp_sapphire_k - s.temp_he4_bath_k);
    let parasitic_w = net.parasitic_heat_w();
    s.cryogen_mass_kg -= BoiloffKinetics::mass_loss_kg(parasitic_w, dt_sec);
    0
}
