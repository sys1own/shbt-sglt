//! Power-derating integration test: LANR module fault injection, bit
//! decrements, seed-mass scaling, and focal-baseline extension (gates 10–12
//! plus the §3 N-k chain).

use sglt_lanr_power::fault_derating_protocol::{
    delta_n_bits, derate, focal_baseline_m, run_gum_monte_carlo, seed_mass_msun, F_MAX_M,
    M_SEED_MIN_MSUN, M_SEED_NOMINAL_MSUN,
};
use sglt_lanr_power::module_ledger::{PowerPlantLedger, K_MAX_SURVIVABLE};

#[test]
fn ledger_nominal_margins() {
    let l = PowerPlantLedger::new();
    assert!((l.nominal_gross_w() - 999.054e3).abs() < 1.0);
    assert!(l.compute_net_entropy_balance() > 93.0e3);
    assert_eq!(l.reserve_modules(), 167);
}

#[test]
fn derating_chain_is_monotonic() {
    let mut f_prev = focal_baseline_m(0);
    let mut m_prev = seed_mass_msun(0);
    for k in [1u32, 12, 100, 800, K_MAX_SURVIVABLE] {
        let m = seed_mass_msun(k);
        let f = focal_baseline_m(k);
        assert!(m < m_prev && f > f_prev, "k = {k}");
        m_prev = m;
        f_prev = f;
    }
    assert!((m_prev - M_SEED_MIN_MSUN).abs() < 1e-16);
    assert!((f_prev - F_MAX_M).abs() < 0.05);
    assert_eq!(seed_mass_msun(0), M_SEED_NOMINAL_MSUN);
}

#[test]
fn bit_decrement_scaling() {
    assert_eq!(delta_n_bits(0), 0);
    assert!(delta_n_bits(1469) > delta_n_bits(12));
    assert!(derate(K_MAX_SURVIVABLE).survivable);
    assert!(!derate(K_MAX_SURVIVABLE + 1).survivable);
}

#[test]
fn monte_carlo_uncertainty_on_derating() {
    // GUM-S1: k ~ N(12, 0.5²) → ΔN statistics.
    let r = run_gum_monte_carlo(12.0, 0.5, |k| (k * 555.03 / 8.9506e-4).floor(), 50_000, 7);
    let expect = (12.0f64 * 555.03 / 8.9506e-4).floor();
    assert!((r.mean - expect).abs() < expect * 0.02);
    assert!(r.coverage_95.0 < r.coverage_95.1);
}
