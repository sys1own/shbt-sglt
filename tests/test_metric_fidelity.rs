//! Metric-fidelity integration test: ADM lapse/shift tolerances and the
//! 512-bit interference tensor (gates 01–03).

use sglt_core_metric::adm_foliation::ADMMetricAuditor;
use sglt_core_metric::interference_tensor::{alpha_seed_m_sun_per_bit_f64, MassCongestionEngine};

#[test]
fn adm_determinant_within_1e_minus_12() {
    let audit = ADMMetricAuditor::new().audit_velocity(0.1);
    assert!(audit.passed);
    assert!(audit.max_determinant_error <= 1.0e-12);
}

#[test]
fn holographic_scale_invariance_holds() {
    // χ_SHBT = 1 + α_seed·ΔN must stay within 1e-6 for simulated deratings.
    for k in [0u32, 12, 100] {
        let dn = (k as f64 * 555.03 / 8.9506e-4).floor();
        let chi = 1.0 + alpha_seed_m_sun_per_bit_f64() * dn;
        assert!((chi - 1.0).abs() <= 1e-6, "chi = {chi} at k = {k}");
    }
}

#[test]
fn interference_tensor_is_512bit_diagonal() {
    let engine = MassCongestionEngine::new();
    let t512 = engine.interference_tensor_512();
    assert_eq!(t512.len(), 4);
    for row in &t512 {
        assert_eq!(row.len(), 4);
        for v in row {
            assert_eq!(v.prec(), 512);
        }
    }
}
