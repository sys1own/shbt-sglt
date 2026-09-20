//! PyO3 dynamic FFI bindings for the shbt-sglt Rust sub-crates.
//!
//! Compiled into the `shbt_sglt_native` Python extension module by the
//! `build-kernel --with-pyo3` pipeline (maturin).  Exposes the transferred
//! solver structures: `MassCongestionEngine`, `ADMMetricAuditor`,
//! `HeterodyneInterferometer`, `HybridGNCController`, `PowerPlantLedger`,
//! `AcousticTampingFEA`, and `RichardsonLucySolver`.

use pyo3::prelude::*;
use pyo3::types::PyDict;

use sglt_astro_reconstruction::richardson_lucy_deconv::RichardsonLucySolver;
use sglt_core_metric::adm_foliation::ADMMetricAuditor;
use sglt_core_metric::interference_tensor::MassCongestionEngine;
use sglt_flight_gnc::heterodyne_metrology::HeterodyneInterferometer;
use sglt_flight_gnc::hybrid_controller::HybridGNCController;
use sglt_lanr_power::module_ledger::PowerPlantLedger;
use sglt_transducer_fea::acoustic_waveguide_fea::AcousticTampingFEA;

/// Seed mass `M_seed = α_seed ΔN` in solar masses.
#[pyfunction]
fn compute_seed_mass(delta_n: f64) -> f64 {
    MassCongestionEngine::new().compute_seed_mass(delta_n)
}

/// ADM foliation audit at `velocity_c`; returns a dict of audit metrics.
#[pyfunction]
fn adm_audit(py: Python<'_>, velocity_c: f64) -> PyResult<Bound<'_, PyDict>> {
    let r = ADMMetricAuditor::new().audit_velocity(velocity_c);
    let d = PyDict::new(py);
    d.set_item("velocity_c", r.velocity_c)?;
    d.set_item("max_determinant_error", r.max_determinant_error)?;
    d.set_item("min_gram_eigenvalue", r.min_gram_eigenvalue)?;
    d.set_item("passed", r.passed)?;
    Ok(d)
}

/// Single-axis displacement noise density (pm/√Hz).
#[pyfunction]
fn displacement_noise_pm() -> f64 {
    HeterodyneInterferometer::new().displacement_noise_pm()
}

/// One dual-tier control step; returns the coarse thrust vector (µN).
#[pyfunction]
fn dual_tier_control(pos_err_m: [f64; 3], vel_err_m_s: [f64; 3]) -> [f64; 3] {
    HybridGNCController::new()
        .execute_dual_tier_control(pos_err_m, vel_err_m_s, &[0.0; 64])
        .thrust_un
}

/// LANR net entropy balance after `k` module failures (W).
#[pyfunction]
fn lanr_net_balance_w(k: u32) -> f64 {
    let mut l = PowerPlantLedger::new();
    l.inject_failures(k);
    l.compute_net_entropy_balance()
}

/// Peak sapphire waveguide pressure for a transient (GPa).
#[pyfunction]
fn waveguide_pressure_gpa(power_w: f64, area_m2: f64, freq_hz: f64) -> f64 {
    AcousticTampingFEA::new(power_w, area_m2, freq_hz, 6.395e-9).peak_waveguide_pressure_gpa()
}

/// Richardson–Lucy point-source reconstruction fidelity after `iters`.
#[pyfunction]
fn rl_point_fidelity(iters: usize) -> f64 {
    let s = RichardsonLucySolver::new();
    let truth = s.point_source();
    let obs = s.convolve(&truth);
    s.reconstruction_fidelity(&truth, &s.deconvolve(&obs, iters))
}

/// `shbt_sglt_native` extension module.
#[pymodule]
fn shbt_sglt_native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(compute_seed_mass, m)?)?;
    m.add_function(wrap_pyfunction!(adm_audit, m)?)?;
    m.add_function(wrap_pyfunction!(displacement_noise_pm, m)?)?;
    m.add_function(wrap_pyfunction!(dual_tier_control, m)?)?;
    m.add_function(wrap_pyfunction!(lanr_net_balance_w, m)?)?;
    m.add_function(wrap_pyfunction!(waveguide_pressure_gpa, m)?)?;
    m.add_function(wrap_pyfunction!(rl_point_fidelity, m)?)?;
    Ok(())
}
