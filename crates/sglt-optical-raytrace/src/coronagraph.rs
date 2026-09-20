//! Coronagraphic suppression: Vector Vortex Coronagraph × SGL off-axis
//! selectivity, with KLIP speckle subtraction hooks (up1.txt §7).

/// Combined stellar suppression model. The achieved contrast
/// C(θ_sep) = C_SGL(θ_sep) · C_internal(θ_sep) must satisfy ≤ 1e-10.
pub struct Coronagraph {
    /// VVC internal rejection floor.
    pub internal_floor: f64,
    /// SGL off-axis angular selectivity scale, radians.
    pub sgl_selectivity_rad: f64,
}

impl Default for Coronagraph {
    fn default() -> Self {
        Self {
            internal_floor: 1.0e-5,
            sgl_selectivity_rad: 0.5e-3, // sub-mas SGL selectivity
        }
    }
}

impl Coronagraph {
    /// SGL contribution: Gaussian roll-off with angular separation.
    pub fn sgl_contrast(&self, theta_sep_rad: f64) -> f64 {
        (-0.5 * (theta_sep_rad / self.sgl_selectivity_rad).powi(2)).exp()
    }

    /// Combined contrast product; GATE-29 requires ≥ 1e10 rejection
    /// (contrast ≤ 1e-10) at the planet separation.
    pub fn achieved_contrast(&self, theta_sep_rad: f64) -> f64 {
        self.sgl_contrast(theta_sep_rad) * self.internal_floor
    }

    /// Karhunen-Loève speckle subtraction: project the target frame onto the
    /// orthogonal complement of the first `n_modes` KL basis vectors.
    /// `frame` and `basis` are flattened rows; `basis` is n_modes × npix.
    pub fn klip_subtract(frame: &mut [f64], basis: &[Vec<f64>], n_modes: usize) {
        for mode in basis.iter().take(n_modes) {
            let coeff: f64 = frame
                .iter()
                .zip(mode.iter())
                .map(|(f, b)| f * b)
                .sum::<f64>()
                / mode.iter().map(|b| b * b).sum::<f64>().max(1e-30);
            for (f, b) in frame.iter_mut().zip(mode.iter()) {
                *f -= coeff * b;
            }
        }
    }
}
