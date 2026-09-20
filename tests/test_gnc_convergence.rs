//! GNC convergence integration test: closed-loop station-keeping, metrology
//! noise bounds, and minimum-jerk slew limits (gates 04–07 + §3).

use sglt_flight_gnc::halo_orbit_dynamics::SEL2Environment;
use sglt_flight_gnc::heterodyne_metrology::{
    HeterodyneInterferometer, DISPLACEMENT_3SIGMA_LIMIT_NM, DWS_SIGMA_LIMIT_NRAD,
};
use sglt_flight_gnc::hybrid_controller::HybridGNCController;
use sglt_flight_gnc::minimum_jerk::{self, MinJerkTrajectory};
use sglt_flight_gnc::propellant_budget::PropellantBudget;

#[test]
fn metrology_noise_within_gates() {
    let i = HeterodyneInterferometer::new();
    assert!(i.displacement_noise_pm() <= 0.170);
    assert!(i.displacement_3sigma_nm(0.25e6 / 9.0) <= DISPLACEMENT_3SIGMA_LIMIT_NM);
}

#[test]
fn dws_precision_within_gate07() {
    let i = HeterodyneInterferometer::new();
    // Phase noise floor ≈ 0.4 mrad over a 0.18 m beam → ~11.4 nrad.
    let sigma = i.dws_sigma_nrad(0.18, 4.0e-4);
    assert!(sigma <= DWS_SIGMA_LIMIT_NRAD, "sigma = {sigma}");
}

#[test]
fn hybrid_controller_bounds_actuators() {
    let c = HybridGNCController::new();
    let out = c.execute_dual_tier_control([1e-6, -2e-6, 0.0], [1e-9; 3], &[0.05; 64]);
    for t in out.thrust_un {
        assert!((0.1..=150.0).contains(&t));
    }
    for row in &out.phase_shifter_v {
        for v in row {
            assert!((3.8..=7.4).contains(v));
        }
    }
}

#[test]
fn environment_disturbances_match_spec() {
    let env = SEL2Environment::new(30.0 * 86_400.0);
    assert!((env.differential_srp_m_s2() - 3.42e-8).abs() < 0.05e-8);
    assert!(env.outgassing_m_s2() < 5.0e-9);
    assert!((env.thermal_baseline_drift_nm(10.0) - 4.5).abs() < 1e-9);
}

#[test]
fn minimum_jerk_peaks_exact() {
    assert_eq!(minimum_jerk::s_dot(0.5), 1.8750);
    assert!((minimum_jerk::s_ddot(0.2113248654051871).abs() - 5.7735).abs() < 1e-3);
    let traj = MinJerkTrajectory::new(3600.0, 1523.69);
    assert!(traj.peak_velocity_m_s() < 1.0);
}

#[test]
fn propellant_budget_matches_spec() {
    let b = PropellantBudget::ten_year();
    assert!((b.lens_kg - 7.09).abs() < 0.01);
    assert!((b.sensor_kg - 3.38).abs() < 0.02);
}
