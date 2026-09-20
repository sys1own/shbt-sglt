//! Multi-resolution Fourier feature encoding (up2.txt §1):
//!   γ(x) = ⊕_{l=0}^{L−1} ( sin(2^l π B x) ; cos(2^l π B x) )
//! x ∈ ℝ⁴ = (r/r̄, log(λ/λ̄)), B ~ N(0, σ_b²), σ_b = 16, L = 12.

pub const L_LEVELS: usize = 12;
pub const SIGMA_B: f64 = 16.0;
pub const INPUT_DIM: usize = 4;
/// γ(x) ∈ ℝ^{2L×4}.
pub const ENCODED_DIM: usize = 2 * L_LEVELS * INPUT_DIM;

/// Deterministic Gaussian projection matrix B (4×4), seeded reproducibly.
pub struct FourierFeatures {
    b: [[f64; INPUT_DIM]; INPUT_DIM],
}

impl Default for FourierFeatures {
    fn default() -> Self {
        // Deterministic N(0, σ_b²) fill via a fixed xorshift64 stream.
        let mut rng = 0x9E3779B97F4A7C15u64;
        let mut next = || {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            (rng >> 11) as f64 / (1u64 << 53) as f64
        };
        let gauss = |n: &mut dyn FnMut() -> f64| {
            let u1 = n().max(1e-300);
            let u2 = n();
            (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos() * SIGMA_B
        };
        let mut b = [[0.0; INPUT_DIM]; INPUT_DIM];
        for row in &mut b {
            for v in row.iter_mut() {
                *v = gauss(&mut next);
            }
        }
        Self { b }
    }
}

impl FourierFeatures {
    /// Project (r/ r̄, log λ/λ̄) into the 96-dim Fourier feature vector.
    pub fn encode(&self, x: [f64; INPUT_DIM]) -> [f64; ENCODED_DIM] {
        // y = B x
        let mut y = [0.0; INPUT_DIM];
        for (yi, row) in y.iter_mut().zip(self.b.iter()) {
            for (bij, xj) in row.iter().zip(x.iter()) {
                *yi += bij * xj;
            }
        }
        let mut out = [0.0; ENCODED_DIM];
        for l in 0..L_LEVELS {
            let w = (1u64 << l) as f64 * std::f64::consts::PI;
            for c in 0..INPUT_DIM {
                out[l * 8 + c] = (w * y[c]).sin();
                out[l * 8 + 4 + c] = (w * y[c]).cos();
            }
        }
        out
    }
}
