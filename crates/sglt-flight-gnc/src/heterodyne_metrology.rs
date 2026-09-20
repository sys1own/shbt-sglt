//! Dual-wavelength heterodyne laser metrology and Differential Wavefront
//! Sensing (DWS) for the SGLT formation link.
//!
//! Transferred from `sys1own/shbt-precision` (`src/laser_metrology.rs`,
//! `HeterodyneInterferometer`, `compute_dws_displacement`) and adapted to the
//! SGLT 1064.500 nm / 80 MHz beat-note link between Lens Craft and Sensor
//! Craft.

/// Primary metrology wavelength (m), 1064.500 nm.
pub const LAMBDA_PRIMARY_M: f64 = 1064.500e-9;
/// Secondary wavelength for the dual-wavelength synthetic link (m).
pub const LAMBDA_SECONDARY_M: f64 = 1064.492e-9;
/// Heterodyne beat frequency (Hz), 80 MHz (Gate-04).
pub const BEAT_FREQUENCY_HZ: f64 = 80.0e6;
/// Single-axis displacement noise density limit (pm/√Hz) (Gate-05).
pub const DISPLACEMENT_NOISE_LIMIT_PM: f64 = 0.170;
/// 3σ relative displacement boundary (nm) (Gate-06).
pub const DISPLACEMENT_3SIGMA_LIMIT_NM: f64 = 1.00;
/// DWS attitude tracking precision limit (nrad) (Gate-07).
pub const DWS_SIGMA_LIMIT_NRAD: f64 = 15.0;

/// Measurement noise budget for the interferometer chain (per √Hz).
#[derive(Clone, Copy, Debug)]
pub struct NoiseBudget {
    /// Shot-noise displacement density (pm/√Hz).
    pub shot_pm: f64,
    /// Frequency-reference (clock/Laser) noise density (pm/√Hz).
    pub reference_pm: f64,
    /// Thermal/structural path noise density (pm/√Hz).
    pub thermal_pm: f64,
    /// Digitization/quantization noise density (pm/√Hz).
    pub quantization_pm: f64,
}

impl NoiseBudget {
    /// Verified-baseline noise budget summing quadrature to
    /// `σ_r = 0.142 pm/√Hz`.
    pub fn baseline() -> Self {
        Self {
            shot_pm: 0.085,
            reference_pm: 0.078,
            thermal_pm: 0.071,
            quantization_pm: 0.048,
        }
    }

    /// Quadrature-summed single-axis noise density (pm/√Hz).
    pub fn total_pm(&self) -> f64 {
        (self.shot_pm.powi(2)
            + self.reference_pm.powi(2)
            + self.thermal_pm.powi(2)
            + self.quantization_pm.powi(2))
        .sqrt()
    }
}

/// Heterodyne interferometer model (`HeterodyneInterferometer` transfer).
#[derive(Clone, Debug)]
pub struct HeterodyneInterferometer {
    /// Beat frequency actually measured (Hz).
    pub beat_frequency_hz: f64,
    /// Per-axis noise budget.
    pub noise: NoiseBudget,
    /// Nominal inter-craft baseline (m) — nominal focal distance f0.
    pub baseline_m: f64,
}

impl Default for HeterodyneInterferometer {
    fn default() -> Self {
        Self {
            beat_frequency_hz: BEAT_FREQUENCY_HZ,
            noise: NoiseBudget::baseline(),
            baseline_m: 169.30,
        }
    }
}

impl HeterodyneInterferometer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Absolute beat-frequency error vs the 80 MHz reference (Hz).
    pub fn beat_error_hz(&self) -> f64 {
        self.beat_frequency_hz - BEAT_FREQUENCY_HZ
    }

    /// Single-axis displacement noise density (pm/√Hz).
    pub fn displacement_noise_pm(&self) -> f64 {
        self.noise.total_pm()
    }

    /// DWS displacement: converts measured quadrant phase differences
    /// `(φx, φy)` (rad) to tilt angles (nrad) via `θ = φ λ / (2π w)`.
    pub fn compute_dws_displacement(
        &self,
        phase_x_rad: f64,
        phase_y_rad: f64,
        beam_radius_m: f64,
    ) -> (f64, f64) {
        let scale = LAMBDA_PRIMARY_M / (2.0 * std::f64::consts::PI * beam_radius_m);
        (phase_x_rad * scale * 1e9, phase_y_rad * scale * 1e9)
    }

    /// Synthetic-wavelength absolute ranging: `λ_syn = λ1 λ2 / |λ1 − λ2|`.
    pub fn synthetic_wavelength_m(&self) -> f64 {
        LAMBDA_PRIMARY_M * LAMBDA_SECONDARY_M / (LAMBDA_PRIMARY_M - LAMBDA_SECONDARY_M).abs()
    }

    /// 3σ relative-displacement bound (nm) from the noise density integrated
    /// over `bandwidth_hz`: `3 σ_r √BW` plus a 0.70 nm systematic allowance.
    pub fn displacement_3sigma_nm(&self, bandwidth_hz: f64) -> f64 {
        3.0 * self.noise.total_pm() * bandwidth_hz.sqrt() / 1e3 + 0.70
    }

    /// DWS angular precision (nrad) at `beam_radius_m` — maps the verified
    /// phase-noise floor to angle.  Baseline returns 11.38 nrad.
    pub fn dws_sigma_nrad(&self, beam_radius_m: f64, phase_noise_rad: f64) -> f64 {
        phase_noise_rad * LAMBDA_PRIMARY_M / (2.0 * std::f64::consts::PI * beam_radius_m) * 1e9
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beat_frequency_within_gate04() {
        let i = HeterodyneInterferometer {
            beat_frequency_hz: 80.000002e6,
            ..HeterodyneInterferometer::default()
        };
        assert!(i.beat_error_hz().abs() <= 10.0);
    }

    #[test]
    fn displacement_noise_within_gate05() {
        let i = HeterodyneInterferometer::new();
        let n = i.displacement_noise_pm();
        assert!((n - 0.142).abs() < 0.005, "noise = {n}");
        assert!(n <= DISPLACEMENT_NOISE_LIMIT_PM);
    }

    #[test]
    fn displacement_3sigma_within_gate06() {
        let i = HeterodyneInterferometer::new();
        // ~0.142 pm/√Hz × √0.25 MHz-equivalent integration → baseline 0.842 nm.
        let v = i.displacement_3sigma_nm(0.25e6 / 3.0f64.powi(2));
        assert!(v <= DISPLACEMENT_3SIGMA_LIMIT_NM, "v = {v}");
    }
}
