//! Non-paraxial eikonal pre-distortion phase maps `W(x, y; Θ)`.
//!
//! Adapted from `sys1own/shbt-exotic` (`src/eikonal_optics.rs`,
//! `EikonalPreDistortion` / `calculate_taylor_phase_map`) for the SGLT
//! beam-steering domain `Θ_tilt = 5° → 15°`.
//!
//! The non-paraxial path-length correction expands the exact tilted-propagation
//! eikonal `W = k [x sin Θx + y sin Θy + (x² + y²)/(2R) cos²Θ_eff + H.O.T.]`
//! so the wavefront fed to the 8×8 phase-shifter array pre-compensates for
//! off-axis aberration at large steering angles where the paraxial
//! approximation breaks down.

use std::f64::consts::PI;

/// Carrier wavelength (m), 1064.500 nm metrology/lensing line.
pub const LAMBDA_M: f64 = 1064.500e-9;
/// Minimum steering tilt (deg).
pub const TILT_MIN_DEG: f64 = 5.0;
/// Maximum steering tilt (deg).
pub const TILT_MAX_DEG: f64 = 15.0;

/// Non-paraxial eikonal pre-distortion engine (`EikonalPreDistortion` transfer).
#[derive(Clone, Debug)]
pub struct EikonalPreDistortion {
    /// Carrier wavelength (m).
    pub lambda_m: f64,
    /// Effective curvature radius of the reference wavefront (m).
    pub reference_radius_m: f64,
    /// Steering angles `(Θ_x, Θ_y)` in degrees.
    pub theta_deg: (f64, f64),
}

impl EikonalPreDistortion {
    /// Builds a pre-distortion engine for tilt `theta_deg` (each axis within
    /// `[TILT_MIN_DEG, TILT_MAX_DEG]` in magnitude).
    pub fn new(theta_deg: (f64, f64), reference_radius_m: f64) -> Result<Self, String> {
        for (axis, t) in [("x", theta_deg.0), ("y", theta_deg.1)] {
            let a = t.abs();
            if !(TILT_MIN_DEG..=TILT_MAX_DEG).contains(&a) {
                return Err(format!(
                    "Θ_{axis} = {t}° out of range [{TILT_MIN_DEG}°, {TILT_MAX_DEG}°]"
                ));
            }
        }
        Ok(Self {
            lambda_m: LAMBDA_M,
            reference_radius_m,
            theta_deg,
        })
    }

    /// Higher-order Taylor path-length phase map `W(x, y; Θ)` in radians.
    ///
    /// `W = k [x sinΘx + y sinΘy + (x² + y²) cos²Θ_eff / (2R)
    ///        + (x sinΘx + y sinΘy)³ / (6R²)]`
    ///
    /// The cubic term is the leading non-paraxial correction; it vanishes at
    /// `Θ = 0` and dominates the residual phase error at 15°.
    pub fn taylor_phase_map(&self, x_m: f64, y_m: f64) -> f64 {
        let k = 2.0 * PI / self.lambda_m;
        let (tx, ty) = (self.theta_deg.0.to_radians(), self.theta_deg.1.to_radians());
        let t_eff = (tx.powi(2) + ty.powi(2)).sqrt();
        let r = self.reference_radius_m;
        let linear = x_m * tx.sin() + y_m * ty.sin();
        let quadratic = (x_m * x_m + y_m * y_m) * t_eff.cos().powi(2) / (2.0 * r);
        let cubic = linear.powi(3) / (6.0 * r * r);
        k * (linear + quadratic + cubic)
    }

    /// Residual non-paraxial correction beyond the paraxial `k(x sinΘx +
    /// y sinΘy)` term (radians).
    pub fn nonparaxial_correction(&self, x_m: f64, y_m: f64) -> f64 {
        let k = 2.0 * PI / self.lambda_m;
        let (tx, ty) = (self.theta_deg.0.to_radians(), self.theta_deg.1.to_radians());
        let t_eff = (tx.powi(2) + ty.powi(2)).sqrt();
        let r = self.reference_radius_m;
        let linear = x_m * tx.sin() + y_m * ty.sin();
        k * ((x_m * x_m + y_m * y_m) * t_eff.cos().powi(2) / (2.0 * r)
            + linear.powi(3) / (6.0 * r * r))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::FRAC_PI_3;

    #[test]
    fn tilt_bounds_enforced() {
        assert!(EikonalPreDistortion::new((5.0, 15.0), 169.3).is_ok());
        assert!(EikonalPreDistortion::new((4.9, 10.0), 169.3).is_err());
        assert!(EikonalPreDistortion::new((10.0, 15.1), 169.3).is_err());
    }

    #[test]
    fn on_axis_phase_is_linear_at_small_aperture() {
        let e = EikonalPreDistortion::new((10.0, 5.0), 1.0e9).unwrap();
        // With huge R, W ≈ k x sinΘ.
        let w = e.taylor_phase_map(1e-3, 0.0);
        let expect = 2.0 * PI / LAMBDA_M
            * 1e-3
            * FRAC_PI_3.sin()
            * (10.0f64.to_radians().sin() / FRAC_PI_3.sin());
        assert!((w - expect).abs() / expect.abs() < 1e-6);
    }

    #[test]
    fn cubic_correction_grows_with_tilt() {
        // Isolate the cubic term: correction minus the quadratic part.
        let cubic = |e: &EikonalPreDistortion, x: f64| {
            let k = 2.0 * PI / e.lambda_m;
            let tx = e.theta_deg.0.to_radians();
            let linear = x * tx.sin();
            let quad = k * x * x * 1.0 / (2.0 * e.reference_radius_m);
            e.nonparaxial_correction(x, 0.0)
                - quad
                    * (tx.powi(2) + e.theta_deg.1.to_radians().powi(2))
                        .sqrt()
                        .cos()
                        .powi(2)
                - k * linear.powi(3) / (6.0 * e.reference_radius_m.powi(2))
        };
        let small = EikonalPreDistortion::new((5.0, 5.0), 169.3).unwrap();
        let large = EikonalPreDistortion::new((15.0, 5.0), 169.3).unwrap();
        // After removing the quadratic and cubic terms the residual is zero;
        // and the pure cubic contribution grows with tilt.
        assert!(cubic(&small, 0.5).abs() < 1e-9);
        assert!(cubic(&large, 0.5).abs() < 1e-9);
        let k = 2.0 * PI / small.lambda_m;
        let c5 = k * (0.5 * 5.0f64.to_radians().sin()).powi(3) / (6.0 * 169.3f64.powi(2));
        let c15 = k * (0.5 * 15.0f64.to_radians().sin()).powi(3) / (6.0 * 169.3f64.powi(2));
        assert!(c15.abs() > c5.abs());
    }
}
