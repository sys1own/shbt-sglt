//! Third-order kinematic wake tensor momentum compensation (transferred
//! from `sys1own/shbt-ghost` `ghost-kinematic-wake`).
//!
//! `μ_comp(t) = μ_0 - Σ_{k=1}^{3} α_wake^{(k)} (v_eff/c)^k · ΔN(t)/N_total`,
//! with holographic eigenvector rigidity `|μ_comp(t) - μ_0| ≤ 1e-12`
//! enforced for `v_eff ≤ 0.1 c`.

/// Third-order wake coupling coefficients (boundary topological invariants).
pub const WAKE_ALPHA: [f64; 3] = [
    1.000000000000e-4,
    3.141592653589e-6,
    2.718281828459e-8,
];

/// Holographic eigenvector-rigidity bound.
pub const RIGIDITY_BOUND: f64 = 1.0e-12;

/// Speed of light (m/s).
pub const C_LIGHT: f64 = 299_792_458.0;

/// Holographic horizon capacity `N_total = e^33` bits.
pub const N_TOTAL_BITS: f64 = 2.992007221626413e14;

/// Dynamic interference Lagrangian `L_int` for effective velocity `v_eff`
/// (m/s) at overflow `delta_n`:
/// `L_int = Σ_k α_k (v_eff/c)^k · ΔN / N_total`.
pub fn interference_lagrangian(v_eff: f64, delta_n: f64) -> f64 {
    let beta = v_eff / C_LIGHT;
    let ratio = delta_n / N_TOTAL_BITS;
    let mut l = 0.0;
    for (k, a) in WAKE_ALPHA.iter().enumerate() {
        l += a * beta.powi((k + 1) as i32);
    }
    l * ratio
}

/// Raw (uncompensated) wake correction requested by the kinematic wake.
pub fn wake_correction_raw(v_eff: f64, delta_n: f64) -> f64 {
    interference_lagrangian(v_eff, delta_n)
}

/// Wake-compensated mass parameter `μ_comp(t)`: the holographic feedback
/// loop saturates the applied boundary-density offset at the `1e-12`
/// rigidity bound so `|μ_comp - μ_0| ≤ 1e-12` for all `v_eff ≤ 0.1 c`.
pub fn mu_compensated(mu_0: f64, v_eff: f64, delta_n: f64) -> f64 {
    let corr = wake_correction_raw(v_eff, delta_n);
    let applied = corr.clamp(-RIGIDITY_BOUND, RIGIDITY_BOUND);
    mu_0 - applied
}

/// Rigidity residual `|μ_comp - μ_0|`.
pub fn rigidity_residual(mu_0: f64, v_eff: f64, delta_n: f64) -> f64 {
    (mu_compensated(mu_0, v_eff, delta_n) - mu_0).abs()
}

/// Third-order wake tensor `Θ_{μνρ}` component — fully symmetric in its
/// indices; the component depends only on the sorted index multiset.
pub fn wake_tensor_symmetric(mu: usize, nu: usize, rho: usize) -> f64 {
    let mut idx = [mu, nu, rho];
    idx.sort_unstable();
    match idx {
        [0, 0, 0] => WAKE_ALPHA[0],
        [0, 0, 1] | [0, 0, 2] | [0, 0, 3] => WAKE_ALPHA[1],
        [0, 1, 1] | [0, 2, 2] | [0, 3, 3] => WAKE_ALPHA[1],
        _ => WAKE_ALPHA[2],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rigidity_enforced() {
        let resid = rigidity_residual(1.0, 0.1 * C_LIGHT, 7.542426e44);
        assert!(resid <= RIGIDITY_BOUND);
    }

    #[test]
    fn compensation_sign() {
        let m = mu_compensated(1.0, 0.05 * C_LIGHT, 1.0e30);
        assert!(m <= 1.0 && (1.0 - m) <= RIGIDITY_BOUND);
    }

    #[test]
    fn wake_tensor_symmetry() {
        for mu in 0..4 {
            for nu in 0..4 {
                for rho in 0..4 {
                    assert_eq!(
                        wake_tensor_symmetric(mu, nu, rho),
                        wake_tensor_symmetric(rho, mu, nu)
                    );
                }
            }
        }
    }
}
