//! Molecular line absorption model for {H₂O, O₂, CO₂, CH₄, O₃} (up1.txt §7).
//!
//! Cross sections σ_m(λ, T, P) are represented as a sum of Gaussian band
//! absorbers centred on the spec's key features.

/// Target molecular species.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Species {
    H2o,
    O2,
    Co2,
    Ch4,
    O3,
}

impl Species {
    pub const ALL: [Species; 5] = [
        Species::H2o,
        Species::O2,
        Species::Co2,
        Species::Ch4,
        Species::O3,
    ];

    /// (centre µm, half-width µm, peak cross section m²/molecule) per band.
    pub fn bands(self) -> &'static [(f64, f64, f64)] {
        match self {
            Species::H2o => &[
                (0.94, 0.05, 4.0e-29),
                (1.14, 0.06, 6.0e-29),
                (1.40, 0.08, 9.0e-29),
                (1.87, 0.10, 7.0e-29),
            ],
            Species::O2 => &[
                (0.688, 0.004, 3.0e-30), // Fraunhofer B
                (0.762, 0.006, 8.0e-30), // Fraunhofer A
            ],
            Species::Co2 => &[(2.00, 0.05, 5.0e-29), (4.30, 0.15, 2.0e-27)],
            Species::Ch4 => &[
                (1.66, 0.05, 4.0e-29),
                (2.30, 0.07, 6.0e-29),
                (3.30, 0.10, 8.0e-29),
            ],
            Species::O3 => &[(0.25, 0.03, 1.0e-28), (9.60, 0.6, 3.0e-27)],
        }
    }
}

/// σ_m(λ) at wavelength λ (µm); T/P corrections folded into the tabulated
/// peaks (line-by-line refinement is a spectra_gen follow-up).
pub fn molecular_cross_section(sp: Species, lambda_um: f64) -> f64 {
    sp.bands()
        .iter()
        .map(|&(c, w, peak)| peak * (-0.5 * ((lambda_um - c) / w).powi(2)).exp())
        .sum()
}

/// Detect which species' bands are resolved in a spectrum `depth[λ]` over
/// `wavelengths_um`. Returns the 5-bit presence mask (H₂O…O₃ order).
pub fn detected_species_mask(wavelengths_um: &[f64], depth: &[f64]) -> u8 {
    let mut mask = 0u8;
    for (i, sp) in Species::ALL.iter().enumerate() {
        for &(c, _, _) in sp.bands() {
            // Find nearest channel
            if let Some((j, _)) = wavelengths_um
                .iter()
                .enumerate()
                .min_by(|a, b| {
                    (a.1 - c).abs().partial_cmp(&(b.1 - c).abs()).unwrap()
                })
            {
                if depth[j] > 1.05 * depth[0].min(1.0) {
                    mask |= 1 << i;
                    break;
                }
            }
        }
    }
    mask
}
