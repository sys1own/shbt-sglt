//! Multi-spectral atmosphere/surface unmixing (up2.txt §1 table):
//! H₂O (1.4/1.9 µm), O₂ (0.76 µm A-band), CO₂ (2.0/4.3 µm),
//! CH₄ (1.66/2.3 µm), O₃ (0.25/9.6 µm).

/// Target biosignature species and their diagnostic bands (µm).
#[derive(Debug, Clone, Copy)]
pub struct SpeciesChannel {
    pub name: &'static str,
    pub bands_um: &'static [f64],
}

pub const SPECIES: [SpeciesChannel; 5] = [
    SpeciesChannel {
        name: "H2O",
        bands_um: &[1.4, 1.9],
    },
    SpeciesChannel {
        name: "O2",
        bands_um: &[0.762],
    },
    SpeciesChannel {
        name: "CO2",
        bands_um: &[2.0, 4.3],
    },
    SpeciesChannel {
        name: "CH4",
        bands_um: &[1.66, 2.3],
    },
    SpeciesChannel {
        name: "O3",
        bands_um: &[0.25, 9.6],
    },
];

/// Differential continuum ratioing: fractional line depth of channel `band`
/// relative to continuum estimate `continuum`. Positive ⇒ absorption.
pub fn line_depth(band_flux: f64, continuum_flux: f64) -> f64 {
    if continuum_flux <= 0.0 {
        return 0.0;
    }
    (continuum_flux - band_flux) / continuum_flux
}

/// Spectral selectivity between two species masks over a wavelength grid:
/// fraction of the union support attributable to the target species.
/// GATE-34 requires > 99.8 %.
pub fn selectivity(target_mask: &[f64], contam_mask: &[f64]) -> f64 {
    let t: f64 = target_mask.iter().sum();
    let c: f64 = contam_mask.iter().sum();
    if t + c <= 0.0 {
        return 1.0;
    }
    t / (t + c)
}

/// Non-negative least-squares unmixing (2-pass NNLS approximation):
/// solve min ‖S x − y‖ s.t. x ≥ 0 via projected Jacobi iterations.
pub fn unmix_nnls(design: &[Vec<f64>], observation: &[f64], iters: usize) -> Vec<f64> {
    let m = observation.len();
    let k = design.len();
    let mut x = vec![0.0; k];
    for _ in 0..iters {
        for j in 0..k {
            let mut num = 0.0;
            let mut den = 1e-30;
            for i in 0..m {
                let mut r = observation[i];
                for (l, col) in design.iter().enumerate() {
                    if l != j {
                        r -= col[i] * x[l];
                    }
                }
                num += design[j][i] * r;
                den += design[j][i] * design[j][i];
            }
            x[j] = (x[j] + num / den).max(0.0);
        }
    }
    x
}
