//! Solovay–Kitaev gate-synthesis error tracking (up1.txt §6):
//!   ε_{k+1} = c · ε_k^{3/2},  c ≈ 3√3
//! F_gate ≥ 1 − Σ ε_i − ∫Γ_dephase dt ≥ 0.99991  (GATE-27: ε ≤ 1e-4).

/// Solovay–Kitaev constant c ≈ 3√3.
pub const SK_C: f64 = 5.196152422706632;

pub struct SolovayKitaev {
    /// Base approximation error ε₀.
    pub epsilon0: f64,
    /// Per-gate synthesis errors.
    pub gate_errors: Vec<f64>,
    /// Dephasing rate Γ_dephase (1/s).
    pub dephase_rate: f64,
}

impl Default for SolovayKitaev {
    fn default() -> Self {
        Self {
            epsilon0: 0.15,
            gate_errors: Vec::new(),
            dephase_rate: 1e-6,
        }
    }
}

impl SolovayKitaev {
    /// Error after `depth` SK recursion levels: ε_{k+1} = c ε_k^{3/2}.
    pub fn error_at_depth(&self, depth: u32) -> f64 {
        let mut e = self.epsilon0;
        for _ in 0..depth {
            e = SK_C * e.powf(1.5);
        }
        e
    }

    /// Depth needed for ε ≤ target.
    pub fn depth_for(&self, target: f64) -> u32 {
        let mut d = 0;
        while self.error_at_depth(d) > target && d < 64 {
            d += 1;
        }
        d
    }

    /// Composite gate fidelity lower bound:
    ///   F ≥ 1 − Σ_i ε_i − Γ_dephase·T_hold.
    pub fn gate_fidelity(&self, t_hold_s: f64) -> f64 {
        let sum_e: f64 = self.gate_errors.iter().sum();
        1.0 - sum_e - self.dephase_rate * t_hold_s
    }
}
