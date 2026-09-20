//! Jump-operator channel construction (up2.txt §3):
//!   L_leak    = √γ |1⟩⟨τ|          (non-topological leakage)
//!   L_dephase = √γ (|1⟩⟨1| − |τ⟩⟨τ|) (differential phase)
//!   L_hop     = √γ_GCR(D) F⁻¹ P_occ F (radiation-induced hopping)
//!   γ_GCR = γ0 (1 + α_rad Ḋ_DDD) exp(−Δ_top / k_B T), Δ_top ≈ 1.2 meV.

use crate::fibonacci::f_matrix;
use crate::lindblad::{CMat, JumpOperator};

/// Topological gap, ~1.2 meV → K.
pub const DELTA_TOP_K: f64 = 13.927; // 1.2 meV / k_B in kelvin
/// Radiation coupling coefficient.
pub const ALPHA_RAD: f64 = 0.02;

/// GCR-driven error rate (s⁻¹) given local DDD dose rate (rad/s) and T (K).
pub fn gamma_gcr(gamma0: f64, ddd_rate: f64, temp_k: f64) -> f64 {
    gamma0 * (1.0 + ALPHA_RAD * ddd_rate) * (-DELTA_TOP_K / temp_k).exp()
}

/// Build the leakage + dephasing + hopping jump operators on the τ-sector
/// {|1⟩, |τ⟩} subspace embedded in dimension `d`.
pub fn build_jump_operators(
    d: usize,
    gamma_leak: f64,
    gamma_dephase: f64,
    gamma_hop: f64,
) -> Vec<JumpOperator> {
    let mut out = Vec::with_capacity(3);

    // L_leak = |1⟩⟨τ| (indices 0 = "1", 1 = "τ" in the reduced basis).
    if d >= 2 {
        let mut l = CMat::zeros(d);
        l.re[1] = 1.0;
        out.push(JumpOperator {
            l,
            gamma: gamma_leak,
        });
    }

    // L_dephase = |1⟩⟨1| − |τ⟩⟨τ|.
    if d >= 2 {
        let mut l = CMat::zeros(d);
        l.re[0] = 1.0;
        l.re[d + 1] = -1.0;
        out.push(JumpOperator {
            l,
            gamma: gamma_dephase,
        });
    }

    // L_hop = F⁻¹ P_occ F on the 2×2 sector. F is symmetric & unitary,
    // so F⁻¹ = F.
    if d >= 2 {
        let f = f_matrix();
        // P_occ = |1⟩⟨1| (projector onto vacuum fusion channel).
        // L = F P F (F symmetric).
        let mut l = CMat::zeros(d);
        for i in 0..2 {
            for j in 0..2 {
                l.re[i * d + j] = f[i][0] * f[0][j];
            }
        }
        out.push(JumpOperator {
            l,
            gamma: gamma_hop,
        });
    }

    out
}
