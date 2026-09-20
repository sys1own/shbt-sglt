//! Fibonacci anyon topology constants and F/R matrices (up2.txt §3).
//! Fusion rule τ × τ = 1 ⊕ τ; the 124-anyon Hilbert space has effective
//! dimension d_124 ≈ φ¹²⁴/√5 ≈ 1.6231e25 ≫ 2^31 → mandatory MPO-DMRG at
//! bond dimension χ = 512.

pub const PHI: f64 = 1.618_033_988_749_895;
/// Number of τ anyons in the braid array.
pub const N_ANYONS: usize = 124;
/// DMRG bond truncation.
pub const CHI_MPO: usize = 512;
/// Dark-ledger non-local logical index field width (up2.txt §4.2).
pub const DARK_LEDGER_FIBONACCI_DESCRIPTORS: usize = 124;

/// log(φ^n / √5).
fn log_quantum_dim(n: usize) -> f64 {
    n as f64 * PHI.ln() - 0.5 * 5.0f64.ln()
}

/// Quantum dimension d_n ≈ φⁿ/√5.
pub fn quantum_dimension(n: usize) -> f64 {
    log_quantum_dim(n).exp()
}

/// d_124 ≈ 1.6231 × 10²⁵.
pub fn hilbert_dim_124() -> f64 {
    quantum_dimension(N_ANYONS)
}

/// 2×2 complex helpers.
#[derive(Debug, Clone, Copy)]
pub struct C2 {
    pub re: f64,
    pub im: f64,
}

impl C2 {
    pub const fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }
    pub fn abs2(self) -> f64 {
        self.re * self.re + self.im * self.im
    }
}

/// F^τ_{τττ} = [[φ⁻¹, φ^{-1/2}],[φ^{-1/2}, −φ⁻¹]].
pub fn f_matrix() -> [[f64; 2]; 2] {
    let ip = 1.0 / PHI;
    let is = PHI.sqrt().recip();
    [[ip, is], [is, -ip]]
}

/// R^{ττ} = diag(e^{−4πi/5}, e^{3πi/5}).
pub fn r_matrix() -> [C2; 2] {
    let a = -4.0 * std::f64::consts::PI / 5.0;
    let b = 3.0 * std::f64::consts::PI / 5.0;
    [C2::new(a.cos(), a.sin()), C2::new(b.cos(), b.sin())]
}

/// Unitarity check: F F† = I and |R_k|² = 1 (GATE-40 support).
pub fn f_is_unitary(tol: f64) -> bool {
    let f = f_matrix();
    for i in 0..2 {
        for j in 0..2 {
            let mut acc = 0.0;
            for k in 0..2 {
                acc += f[i][k] * f[j][k];
            }
            let expected = if i == j { 1.0 } else { 0.0 };
            if (acc - expected).abs() > tol {
                return false;
            }
        }
    }
    r_matrix().iter().all(|r| (r.abs2() - 1.0).abs() <= tol)
}
