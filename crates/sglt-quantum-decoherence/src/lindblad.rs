//! Dense Lindblad master-equation solver over the MPO-truncated manifold
//! (up2.txt §3):
//!   dρ/dt = −i[H_braid, ρ] + Σ_k γ_k (L_k ρ L_k† − ½{L_k†L_k, ρ}).
//!
//! The full d_124 ≈ 1.6e25 Hilbert space is never materialized; evolution is
//! evaluated on the χ-bounded reduced subspace (d_eff ≤ χ = 512) per the
//! spec's MPO-DMRG constraint. Classical RK4 with fixed dt.

use crate::fibonacci::CHI_MPO;

/// Row-major complex matrix, dimension d×d.
#[derive(Debug, Clone)]
pub struct CMat {
    pub d: usize,
    pub re: Vec<f64>,
    pub im: Vec<f64>,
}

impl CMat {
    pub fn zeros(d: usize) -> Self {
        Self {
            d,
            re: vec![0.0; d * d],
            im: vec![0.0; d * d],
        }
    }

    pub fn identity(d: usize) -> Self {
        let mut m = Self::zeros(d);
        for i in 0..d {
            m.re[i * d + i] = 1.0;
        }
        m
    }

    /// ρ = |0⟩⟨0| initial state.
    pub fn ground(d: usize) -> Self {
        let mut m = Self::zeros(d);
        m.re[0] = 1.0;
        m
    }

    pub fn trace(&self) -> (f64, f64) {
        let (mut re, mut im) = (0.0, 0.0);
        for i in 0..self.d {
            re += self.re[i * self.d + i];
            im += self.im[i * self.d + i];
        }
        (re, im)
    }

    /// C = A·B.
    pub fn mul(&self, o: &Self) -> Self {
        let d = self.d;
        let mut c = Self::zeros(d);
        for i in 0..d {
            for k in 0..d {
                let a_re = self.re[i * d + k];
                let a_im = self.im[i * d + k];
                if a_re == 0.0 && a_im == 0.0 {
                    continue;
                }
                for j in 0..d {
                    let b_re = o.re[k * d + j];
                    let b_im = o.im[k * d + j];
                    c.re[i * d + j] += a_re * b_re - a_im * b_im;
                    c.im[i * d + j] += a_re * b_im + a_im * b_re;
                }
            }
        }
        c
    }

    /// C = A ± B.
    pub fn add_scaled(&self, o: &Self, s: f64) -> Self {
        let mut c = self.clone();
        for i in 0..c.re.len() {
            c.re[i] += s * o.re[i];
            c.im[i] += s * o.im[i];
        }
        c
    }

    /// A†.
    pub fn dagger(&self) -> Self {
        let d = self.d;
        let mut c = Self::zeros(d);
        for i in 0..d {
            for j in 0..d {
                c.re[i * d + j] = self.re[j * d + i];
                c.im[i * d + j] = -self.im[j * d + i];
            }
        }
        c
    }

    /// In-place ρ += s·K.
    pub fn accumulate(&mut self, k: &Self, s: f64) {
        for i in 0..self.re.len() {
            self.re[i] += s * k.re[i];
            self.im[i] += s * k.im[i];
        }
    }
}

/// A Lindblad channel: jump operator L and rate γ.
pub struct JumpOperator {
    pub l: CMat,
    pub gamma: f64,
}

/// dρ/dt = −i[H,ρ] + Σ γ (LρL† − ½{L†L,ρ}).
pub fn rhs(h: &CMat, jumps: &[JumpOperator], rho: &CMat) -> CMat {
    let d = rho.d;
    let mut out = CMat::zeros(d);

    // −i[H, ρ] = −i(Hρ − ρH).
    let hr = h.mul(rho);
    let rh = rho.mul(h);
    for i in 0..out.re.len() {
        let c_re = hr.re[i] - rh.re[i];
        let c_im = hr.im[i] - rh.im[i];
        out.re[i] += c_im; // −i(c_re + i c_im) = c_im − i c_re
        out.im[i] += -c_re;
    }

    for jp in jumps {
        let ldag = jp.l.dagger();
        let lpl = jp.l.mul(rho).mul(&ldag); // L ρ L†
        let ll = ldag.mul(&jp.l); // L†L
        let anti = ll.mul(rho).add_scaled(&rho.mul(&ll), 1.0); // {L†L, ρ}
        for i in 0..out.re.len() {
            out.re[i] += jp.gamma * (lpl.re[i] - 0.5 * anti.re[i]);
            out.im[i] += jp.gamma * (lpl.im[i] - 0.5 * anti.im[i]);
        }
    }
    out
}

/// Fixed-step classical RK4 propagator (up2.txt §3).
pub fn rk4_step(h: &CMat, jumps: &[JumpOperator], rho: &CMat, dt: f64) -> CMat {
    let k1 = rhs(h, jumps, rho);
    let r2 = rho.add_scaled(&k1, dt * 0.5);
    let k2 = rhs(h, jumps, &r2);
    let r3 = rho.add_scaled(&k2, dt * 0.5);
    let k3 = rhs(h, jumps, &r3);
    let r4 = rho.add_scaled(&k3, dt);
    let k4 = rhs(h, jumps, &r4);

    let mut out = rho.clone();
    for i in 0..out.re.len() {
        out.re[i] += dt / 6.0 * (k1.re[i] + 2.0 * k2.re[i] + 2.0 * k3.re[i] + k4.re[i]);
        out.im[i] += dt / 6.0 * (k1.im[i] + 2.0 * k2.im[i] + 2.0 * k3.im[i] + k4.im[i]);
    }
    out
}

/// Effective reduced dimension for the solver (≤ χ).
pub fn effective_dim(requested: usize) -> usize {
    requested.min(CHI_MPO)
}
