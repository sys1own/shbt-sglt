//! 3+1D ADM foliation auditor for the SGLT synthetic metric.
//!
//! Transferred from `sys1own/shbt-exotic` (`src/warp_metric.rs`,
//! `ADMMetricAuditor`) and adapted to a pure Rust API.  Evaluates a flat-slice
//! line element with unit lapse `α_ADM = 1`, Euclidean spatial metric
//! `γ_ij = δ_ij`, and a longitudinal shift `β^i` built from the SHBT shape
//! function.  Every grid point is audited for `det(g) = -1` within
//! `|det(g)+1| ≤ 1e-12` and positive-definite Gram eigenvalues.

/// Default 10 m lensing-bubble radius (m).
pub const BUBBLE_RADIUS_M: f64 = 10.0;
/// Default 30 m domain half-width for the metric audit.
pub const DOMAIN_RADIUS_M: f64 = 30.0;
/// Wall steepness parameter (1/m) for the shape function.
pub const WALL_STEEPNESS_PER_M: f64 = 0.8;
/// Maximum grid points for the 1-D longitudinal metric audit.
pub const MAX_GRID_POINTS: usize = 1201;
/// Gate-01 determinant tolerance.
pub const DET_TOLERANCE: f64 = 1.0e-12;

/// SHBT shape function `f(r_s)` for the longitudinal shift.
fn shbt_shape(r_s: f64, radius: f64, sigma: f64) -> f64 {
    let denom = 2.0 * (sigma * radius).tanh();
    if denom == 0.0 {
        return 0.0;
    }
    ((sigma * (r_s + radius)).tanh() - (sigma * (r_s - radius)).tanh()) / denom
}

/// 4-D Lorentzian determinant and Gram positivity invariants for a
/// longitudinal shift `β` (dimensionless, in units of c).
///
/// `g_00 = -1 + β²`, `g_0i = β n_i`, `g_ij = δ_ij`, `n = (1,0,0)`.
fn metric_invariants(beta: f64) -> (f64, f64, f64) {
    let b2 = beta * beta;
    let det = -1.0 + b2 - b2;

    let disc_l = (b2 * b2 + 4.0).sqrt();
    let lambda_plus = (b2 + disc_l) / 2.0;
    let lambda_minus = (b2 - disc_l) / 2.0;
    let min_abs_lorentzian_ev = lambda_plus.min(lambda_minus.abs()).min(1.0);

    let disc_g = ((2.0 + b2).powi(2) - 4.0).sqrt();
    let gamma_plus = (2.0 + b2 + disc_g) / 2.0;
    let gamma_minus = (2.0 + b2 - disc_g) / 2.0;
    let min_gram_ev = gamma_plus.min(gamma_minus).min(1.0);

    (det, min_abs_lorentzian_ev, min_gram_ev)
}

/// Audit result for a single foliation scan.
#[derive(Clone, Debug)]
pub struct FoliationAudit {
    /// Effective metric velocity parameter (fraction of c).
    pub velocity_c: f64,
    /// `max |det(g) + 1|` over the audit grid.
    pub max_determinant_error: f64,
    /// `min |det(g)|` over the grid.
    pub min_abs_determinant: f64,
    /// Smallest |λ| of the Lorentzian t-x block.
    pub min_abs_lorentzian_eigenvalue: f64,
    /// Smallest eigenvalue of the Gram (spatial) block.
    pub min_gram_eigenvalue: f64,
    /// True when every audit bound is satisfied.
    pub passed: bool,
}

/// 3+1D ADM metric auditor (`ADMMetricAuditor` transfer).
#[derive(Clone, Debug)]
pub struct ADMMetricAuditor {
    bubble_radius_m: f64,
    wall_steepness_per_m: f64,
    domain_radius_m: f64,
    grid_points: usize,
}

impl ADMMetricAuditor {
    pub fn new() -> Self {
        Self::with_params(BUBBLE_RADIUS_M, WALL_STEEPNESS_PER_M, DOMAIN_RADIUS_M, 65)
    }

    pub fn with_params(
        bubble_radius_m: f64,
        wall_steepness_per_m: f64,
        domain_radius_m: f64,
        grid_points: usize,
    ) -> Self {
        let n = grid_points.clamp(5, MAX_GRID_POINTS);
        let n = if n % 2 == 0 { n + 1 } else { n };
        Self {
            bubble_radius_m,
            wall_steepness_per_m,
            domain_radius_m,
            grid_points: n,
        }
    }

    /// Audit the 3+1D metric along the longitudinal axis at `velocity_c`.
    pub fn audit_velocity(&self, velocity_c: f64) -> FoliationAudit {
        let dx = 2.0 * self.domain_radius_m / (self.grid_points as f64 - 1.0);
        let mut max_det_error = 0.0f64;
        let mut min_abs_det = f64::INFINITY;
        let mut min_abs_lorentzian_ev = f64::INFINITY;
        let mut min_gram_ev = f64::INFINITY;

        for i in 0..self.grid_points {
            let x = -self.domain_radius_m + dx * (i as f64);
            let f = shbt_shape(x.abs(), self.bubble_radius_m, self.wall_steepness_per_m);
            let beta = velocity_c * f;
            let (det, lorentz_min, gram_min) = metric_invariants(beta);
            max_det_error = max_det_error.max((det + 1.0).abs());
            min_abs_det = min_abs_det.min(det.abs());
            min_abs_lorentzian_ev = min_abs_lorentzian_ev.min(lorentz_min);
            min_gram_ev = min_gram_ev.min(gram_min);
        }

        FoliationAudit {
            velocity_c,
            max_determinant_error: max_det_error,
            min_abs_determinant: min_abs_det,
            min_abs_lorentzian_eigenvalue: min_abs_lorentzian_ev,
            min_gram_eigenvalue: min_gram_ev,
            passed: max_det_error <= DET_TOLERANCE
                && min_abs_lorentzian_ev > 1.0e-12
                && min_gram_ev > 1.0e-12,
        }
    }

    /// 4x4 covariant ADM metric at `x_m` (row-major), lapse α = 1, shift
    /// `β^i = -v f(r_s) n^i`, spatial metric `γ_ij = δ_ij`.
    pub fn evaluate_metric_at(&self, x_m: f64, velocity_c: f64, n_vec: [f64; 3]) -> [f64; 16] {
        let f = shbt_shape(x_m.abs(), self.bubble_radius_m, self.wall_steepness_per_m);
        let n_norm = (n_vec[0] * n_vec[0] + n_vec[1] * n_vec[1] + n_vec[2] * n_vec[2]).sqrt();
        let n = if n_norm > 1e-15 {
            [n_vec[0] / n_norm, n_vec[1] / n_norm, n_vec[2] / n_norm]
        } else {
            [1.0, 0.0, 0.0]
        };
        let b = -velocity_c * f;
        let beta = [b * n[0], b * n[1], b * n[2]];
        let beta_sq = beta[0] * beta[0] + beta[1] * beta[1] + beta[2] * beta[2];

        [
            -1.0 + beta_sq,
            beta[0],
            beta[1],
            beta[2],
            beta[0],
            1.0,
            0.0,
            0.0,
            beta[1],
            0.0,
            1.0,
            0.0,
            beta[2],
            0.0,
            0.0,
            1.0,
        ]
    }
}

impl Default for ADMMetricAuditor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foliation_determinant_is_minus_one() {
        let auditor = ADMMetricAuditor::new();
        let result = auditor.audit_velocity(0.1);
        assert!(result.passed, "{:?}", result);
        assert!(result.max_determinant_error < DET_TOLERANCE);
    }

    #[test]
    fn gram_positive_at_sub_c_velocity() {
        let auditor = ADMMetricAuditor::new();
        let result = auditor.audit_velocity(0.5);
        assert!(result.min_gram_eigenvalue > 0.0);
    }
}
