//! Eikonal image-plane amplitude and SGL point-spread function (up1.txt §2).

use crate::consts;
use std::f64::consts::PI;

/// Zeroth-order Bessel function J0 via power series (|x| ≤ ~30 accurate to
/// ~1e-15; adequate for the 10 m field grid at λ ≥ 200 nm).
pub fn bessel_j0(x: f64) -> f64 {
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

/// Complex eikonal amplitude E(ρ, φ, z; λ) as (re, im):
///   E = E0 e^{ikz} √(2π² r_g / λ) · J0((2π/λ)√(2r_g/z) ρ)
///       · exp(i [kρ²/2z + (r_g/λ) ln(2kz)])
pub fn eikonal_amplitude(rho: f64, lambda_m: f64, z_m: f64) -> (f64, f64) {
    let k = 2.0 * PI / lambda_m;
    let prefactor = (2.0 * PI * PI * consts::R_G / lambda_m).sqrt();
    let j0 = bessel_j0(k * (2.0 * consts::R_G / z_m).sqrt() * rho);
    let phase = k * rho * rho / (2.0 * z_m)
        + (consts::R_G / lambda_m) * (2.0 * k * z_m).ln()
        + k * z_m;
    let amp = prefactor * j0;
    (amp * phase.cos(), amp * phase.sin())
}

/// SGL PSF P(ρ; λ) = |E(ρ, λ)|² (unnormalized diffraction intensity).
pub fn psf(rho: f64, lambda_m: f64, z_m: f64) -> f64 {
    let (re, im) = eikonal_amplitude(rho, lambda_m, z_m);
    re * re + im * im
}

/// Convolve an extended uniform-disk source of angular radius `src_rad` with
/// the SGL PSF on a square grid — I(x,y) = ∫ B(x',y') P(|r−r'|) dx' dy'.
/// `image` is row-major nx×ny covering [-extent, extent]² in metres.
pub fn convolve_extended_disk(
    image: &mut [f64],
    nx: usize,
    ny: usize,
    extent_m: f64,
    src_radius_m: f64,
    lambda_m: f64,
    z_m: f64,
) {
    // Midpoint-rule convolution over the source disk sampled on a coarse
    // sub-grid; B is uniform inside src_radius_m, zero outside.
    let src_samples = 16usize;
    let disk_area = PI * src_radius_m * src_radius_m;
    let b_uniform = 1.0 / disk_area;
    for j in 0..ny {
        for i in 0..nx {
            let x = (i as f64 / (nx - 1) as f64 - 0.5) * 2.0 * extent_m;
            let y = (j as f64 / (ny - 1) as f64 - 0.5) * 2.0 * extent_m;
            let mut acc = 0.0;
            for sj in 0..src_samples {
                for si in 0..src_samples {
                    let xp = ((si + 1) as f64 / (src_samples + 1) as f64 - 0.5)
                        * 2.0
                        * src_radius_m;
                    let yp = ((sj + 1) as f64 / (src_samples + 1) as f64 - 0.5)
                        * 2.0
                        * src_radius_m;
                    if xp * xp + yp * yp > src_radius_m * src_radius_m {
                        continue;
                    }
                    let dx = x - xp;
                    let dy = y - yp;
                    acc += b_uniform * psf((dx * dx + dy * dy).sqrt(), lambda_m, z_m);
                }
            }
            let cell = (2.0 * src_radius_m / src_samples as f64).powi(2);
            image[j * nx + i] = acc * cell;
        }
    }
}
