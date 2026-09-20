//! Gate fidelity accounting (up2.txt §3):
//!   F_gate(t) ≥ 1 − Σ ε_SK − ∫₀ᵗ (Γ_dephase + Γ_leak) dt > 0.99999
//! Solovay-Kitaev Kuperberg exponent c ≈ 1.44042; a base gate-set with
//! net length l ≈ 50 primitives suffices for ε = 1e-4 per gate.

/// Kuperberg SK exponent c ≈ 1.44042.
pub const SK_KUPERBERG_EXPONENT: f64 = 1.44042;
/// 25-year cruise duration.
pub const MISSION_DURATION_S: f64 = 25.0 * 365.25 * 86400.0;

/// SK overhead: gates needed ≈ A · log(1/ε)^c. Returns synthesis error ε
/// remaining after `n_gates` primitives with prefactor A.
pub fn sk_residual_error(n_gates: usize, prefactor: f64) -> f64 {
    if n_gates < 1 {
        return 1.0;
    }
    (prefactor / (n_gates as f64).powf(1.0 / SK_KUPERBERG_EXPONENT)).min(1.0)
}

/// Lower bound on braid-evolution fidelity at time t:
/// F ≥ 1 − ε_SK_total − ∫(Γ_dephase + Γ_leak) dt.
/// Assumes constant rates.
pub fn gate_fidelity_bound(
    sk_total_error: f64,
    gamma_dephase: f64,
    gamma_leak: f64,
    t_s: f64,
) -> f64 {
    1.0 - sk_total_error - (gamma_dephase + gamma_leak) * t_s
}

/// T_exp[Z] per Eq. of §3: Z_GCR = α_GCR / T_coh^2 (growth bound).
pub fn decoherence_growth(alpha_gcr: f64, t_coh: f64) -> f64 {
    alpha_gcr / (t_coh * t_coh)
}
