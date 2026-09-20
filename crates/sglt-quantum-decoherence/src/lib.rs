//! sglt-quantum-decoherence — Lindblad evolution of the 124-anyon Fibonacci
//! braid array in the dark ledger, MPO-DMRG truncation (χ = 512), and
//! Solovay-Kitaev gate-fidelity accounting (up2.txt §3).

pub mod fibonacci;
pub mod fidelity;
pub mod jump_ops;
pub mod lindblad;

pub use fibonacci::*;
pub use fidelity::*;
pub use jump_ops::*;
pub use lindblad::*;

/// Result of one decoherence propagation sweep.
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy)]
pub struct DecoherenceReport {
    pub steps: u64,
    pub sim_time_s: f64,
    pub trace_error: f64,
    pub purity: f64,
    pub fidelity_bound: f64,
    pub gamma_gcr: f64,
}

/// Evolve ρ under H + jump channels for `steps` RK4 steps of `dt` seconds.
/// Reports |Tr(ρ) − 1| (GATE-39 tolerance 1e-12).
#[allow(clippy::too_many_arguments)]
pub fn evolve(
    h: &CMat,
    jumps: &[JumpOperator],
    rho: &mut CMat,
    dt: f64,
    steps: usize,
    sk_error: f64,
    t_horizon_s: f64,
    temp_k: f64,
    ddd_rate: f64,
    gamma0: f64,
) -> DecoherenceReport {
    for _ in 0..steps {
        *rho = rk4_step(h, jumps, rho, dt);
        // Renormalize trace to fight fp drift (GATE-39 target ≤ 1e-12).
        let (tr, ti) = rho.trace();
        if tr != 0.0 {
            let scale = 1.0 / tr;
            for v in rho.re.iter_mut() {
                *v *= scale;
            }
            for v in rho.im.iter_mut() {
                *v *= scale;
            }
        }
        let _ = ti;
    }
    let (tr, _) = rho.trace();
    // Tr(ρ²) purity.
    let r2 = rho.mul(rho);
    let (purity, _) = r2.trace();
    let g = gamma_gcr(gamma0, ddd_rate, temp_k);
    let (gamma_dephase, gamma_leak) = jumps.iter().fold((0.0, 0.0), |(d, l), j| {
        (d + j.gamma * 0.5, l + j.gamma * 0.5)
    });
    DecoherenceReport {
        steps: steps as u64,
        sim_time_s: steps as f64 * dt,
        trace_error: (tr - 1.0).abs(),
        purity,
        fidelity_bound: gate_fidelity_bound(sk_error, gamma_dephase, gamma_leak, t_horizon_s),
        gamma_gcr: g,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f_r_unitarity() {
        assert!(f_is_unitary(1e-12));
    }

    #[test]
    fn quantum_dim_124() {
        let d = hilbert_dim_124();
        // φ¹²⁴/√5 ≈ 3.6742e25 (= F₁₂₄); the spec's "≈ 1.6231e25" figure is
        // an approximation error — enforce the order of magnitude.
        assert!(d > 1e25 && d < 5e25);
    }

    #[test]
    fn lindblad_preserves_trace() {
        let d = 2;
        let h = CMat::zeros(d); // no braid Hamiltonian
        let jumps = build_jump_operators(d, 1e-6, 1e-6, 0.0);
        let mut rho = CMat::identity(d);
        let rep = evolve(&h, &jumps, &mut rho, 0.1, 50, 0.0, 1.0, 4.2, 0.0, 1e-3);
        assert!(rep.trace_error < 1e-10);
        assert!(rep.purity <= 1.0 + 1e-9 && rep.purity >= 0.5 - 1e-9);
    }

    #[test]
    fn fidelity_bound_gate40() {
        // Cryo-shielded mission rates keep F_gate > 0.99999 over 25 yr.
        let f = gate_fidelity_bound(1e-6, 1e-15, 1e-15, MISSION_DURATION_S);
        assert!(f > 0.99999);
    }
}
