//! PINN loss terms (up2.txt §1):
//!   L_PINN = L_data + λ_phys ‖∇²E + k²(1 + 4r_g/r) E‖² + λ_reg R(f_θ)
//! with Huber data loss (δ = 1e-3) and the exact SGL PSF
//!   P(ρ;λ) = |E0|² 2π²r_g/(λ(1−e^{−2πσ})) J0²(…), σ = 2πr_g/λ.

/// Schwarzschild radius of the Sun, m.
pub const R_G: f64 = 2.95325008e3;
/// Huber threshold δ.
pub const HUBER_DELTA: f64 = 1e-3;

/// Huber loss H_δ(e).
pub fn huber(e: f64) -> f64 {
    let a = e.abs();
    if a <= HUBER_DELTA {
        0.5 * e * e
    } else {
        HUBER_DELTA * (a - 0.5 * HUBER_DELTA)
    }
}

/// Mean data fidelity over observed vs. predicted image stack.
pub fn data_loss(obs: &[f32], pred: &[f32]) -> f64 {
    let n = obs.len().min(pred.len());
    if n == 0 {
        return 0.0;
    }
    let mut acc = 0.0;
    for i in 0..n {
        acc += huber((obs[i] - pred[i]) as f64);
    }
    acc / n as f64
}

/// J0 Bessel (same series as sglt-optical-raytrace; kept crate-local so the
/// neural-optics crate stays dependency-light).
fn bessel_j0(x: f64) -> f64 {
    let mut sum = 1.0;
    let mut term = 1.0;
    let xx = x * x / 4.0;
    for k in 1..=40 {
        term *= -xx / (k as f64 * k as f64);
        sum += term;
        if term.abs() < 1e-18 * sum.abs() {
            break;
        }
    }
    sum
}

/// Exact SGL PSF P(ρ; λ) including the (1 − e^{−2πσ}) denominator.
pub fn sgl_psf_exact(rho: f64, lambda_m: f64, z_m: f64, e0: f64) -> f64 {
    let sigma = 2.0 * std::f64::consts::PI * R_G / lambda_m;
    let denom = 1.0 - (-2.0 * std::f64::consts::PI * sigma).exp();
    let arg = (2.0 * std::f64::consts::PI / lambda_m) * (2.0 * R_G / z_m).sqrt() * rho;
    let j = bessel_j0(arg);
    e0 * e0 * (2.0 * std::f64::consts::PI.powi(2) * R_G) / (lambda_m * denom) * j * j
}

/// 1PN Helmholtz residual ‖∇²E + k²(1 + 4r_g/r) E‖², evaluated on a radial
/// sample grid via 3-point finite differences. GATE-33: L_phys < 1e-4 RMS.
pub fn helmholtz_residual_rms(e_field: &[f64], r_samples: &[f64], lambda_m: f64) -> f64 {
    let n = e_field.len().min(r_samples.len());
    if n < 3 {
        return 0.0;
    }
    let k = 2.0 * std::f64::consts::PI / lambda_m;
    let mut acc = 0.0;
    let mut count = 0;
    for i in 1..n - 1 {
        let dr = r_samples[i + 1] - r_samples[i];
        let d2e = (e_field[i + 1] - 2.0 * e_field[i] + e_field[i - 1]) / (dr * dr);
        let resid = d2e + k * k * (1.0 + 4.0 * R_G / r_samples[i]) * e_field[i];
        acc += resid * resid;
        count += 1;
    }
    (acc / count as f64).sqrt()
}

/// Total PINN loss assembled from parts.
pub fn total_loss(
    obs: &[f32],
    pred: &[f32],
    phys_residual_rms: f64,
    reg_tv: f64,
    lambda_phys: f64,
    lambda_reg: f64,
) -> f64 {
    data_loss(obs, pred) + lambda_phys * phys_residual_rms * phys_residual_rms + lambda_reg * reg_tv
}
