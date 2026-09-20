//! Acoustic waveguide FEA: sapphire guide + silica aerogel tamping layer.
//!
//! Transferred from `sys1own/shbt-exotic` (`AcousticTampingFEA`,
//! `simulate_transient_wave`) and adapted for the SGLT transducer domain.
//! A single-crystal sapphire (Al2O3) waveguide carries the acoustic transient;
//! a quarter-wavelength nanoporous silica aerogel matching layer
//! (`Z_m = sqrt(Z_sapphire · Z_bath)`, `d_m = 6.395 nm`) couples the guide to
//! the cryogenic bath and protects the InP/InGaAs HBT substrate.

/// Acoustic impedance of single-crystal sapphire (Al2O3), MRayl (Gate-08).
pub const SAPPHIRE_IMPEDANCE_MRAYL: f64 = 44.178;

/// Aerogel matching-layer impedance, MRayl — geometric-mean quarter-wave
/// condition (Gate-09 companion constant).
pub const AEROGEL_IMPEDANCE_MRAYL: f64 = 1.1512;

/// Silica aerogel tamping matching-layer thickness, nm (Gate-09).
pub const AEROGEL_TAMPING_THICKNESS_NM: f64 = 6.395;

/// Acoustic impedance of the liquid He-4 bath implied by the matching-layer
/// condition `Z_m = sqrt(Z_s Z_L)`, MRayl.
pub const HELIUM4_IMPEDANCE_MRAYL: f64 =
    AEROGEL_IMPEDANCE_MRAYL * AEROGEL_IMPEDANCE_MRAYL / SAPPHIRE_IMPEDANCE_MRAYL;

/// Representative acoustic impedance of the InP substrate, MRayl.
pub const INP_IMPEDANCE_MRAYL: f64 = 23.1;

/// Conservative structural yield limit for InP, GPa.
pub const INP_YIELD_STRENGTH_GPA: f64 = 10.0;

/// Sapphire waveguide cross-sectional area (m²), 5 mm radius spot.
pub const WAVEGUIDE_AREA_M2: f64 = std::f64::consts::PI * 5.0e-3 * 5.0e-3;

/// Result of one transient-wave FEA propagation step.
#[derive(Clone, Copy, Debug)]
pub struct TransientWaveState {
    /// Simulation time (s).
    pub t_s: f64,
    /// Pressure in the sapphire waveguide (GPa).
    pub p_waveguide_gpa: f64,
    /// Pressure inside the aerogel matching layer (GPa).
    pub p_matching_gpa: f64,
    /// Pressure delivered to the He-4 bath (GPa).
    pub p_bath_gpa: f64,
    /// Pressure transmitted into the InP substrate (GPa).
    pub p_substrate_gpa: f64,
}

/// Sapphire/aerogel acoustic tamping FEA engine (`AcousticTampingFEA` transfer).
#[derive(Clone, Debug)]
pub struct AcousticTampingFEA {
    /// Transient peak power (W).
    pub power_w: f64,
    /// Waveguide cross-sectional area (m²).
    pub area_m2: f64,
    /// Acoustic carrier frequency (Hz).
    pub frequency_hz: f64,
    /// Tamping layer thickness (m).
    pub tamping_thickness_m: f64,
}

impl AcousticTampingFEA {
    pub fn new(power_w: f64, area_m2: f64, frequency_hz: f64, tamping_thickness_m: f64) -> Self {
        Self {
            power_w,
            area_m2,
            frequency_hz,
            tamping_thickness_m,
        }
    }

    /// Quarter-wave tamping thickness for the acoustic carrier (m).
    pub fn quarter_wave_thickness_m(&self, sound_speed_m_s: f64) -> f64 {
        sound_speed_m_s / (4.0 * self.frequency_hz)
    }

    /// Peak acoustic pressure in the sapphire waveguide (GPa):
    /// `P = sqrt(2 I Z)` with `I = P_transient / A`.
    pub fn peak_waveguide_pressure_gpa(&self) -> f64 {
        let intensity = self.power_w / self.area_m2;
        (2.0 * intensity * SAPPHIRE_IMPEDANCE_MRAYL * 1e6).sqrt() / 1e9
    }

    /// Pressure transmitted across a boundary between impedances (MRayl).
    pub fn transmitted_pressure_gpa(&self, p_source: f64, z_source: f64, z_target: f64) -> f64 {
        (2.0 * z_target / (z_source + z_target)) * p_source
    }

    /// Pressure inside the aerogel matching layer (GPa).
    pub fn matching_layer_pressure_gpa(&self) -> f64 {
        self.transmitted_pressure_gpa(
            self.peak_waveguide_pressure_gpa(),
            SAPPHIRE_IMPEDANCE_MRAYL,
            AEROGEL_IMPEDANCE_MRAYL,
        )
    }

    /// Pressure entering the bath after the matching layer (GPa).
    pub fn bath_pressure_gpa(&self) -> f64 {
        self.transmitted_pressure_gpa(
            self.matching_layer_pressure_gpa(),
            AEROGEL_IMPEDANCE_MRAYL,
            HELIUM4_IMPEDANCE_MRAYL,
        )
    }

    /// Pressure transmitted into the InP substrate (GPa).
    pub fn inp_substrate_pressure_gpa(&self) -> f64 {
        self.transmitted_pressure_gpa(
            self.peak_waveguide_pressure_gpa(),
            SAPPHIRE_IMPEDANCE_MRAYL,
            INP_IMPEDANCE_MRAYL,
        )
    }

    /// True when the InP substrate pressure is below its yield limit.
    pub fn is_inp_within_yield(&self) -> bool {
        self.inp_substrate_pressure_gpa() < INP_YIELD_STRENGTH_GPA
    }

    /// Optimal matching impedance `sqrt(Z_s Z_L)` (MRayl).
    pub fn optimal_matching_impedance_mrayl(&self) -> f64 {
        (SAPPHIRE_IMPEDANCE_MRAYL * HELIUM4_IMPEDANCE_MRAYL).sqrt()
    }

    /// Transient-wave propagation step: evolves the front through the layer
    /// stack at time `t_s` with a causal ramp `min(1, t/t_rise)`.
    pub fn simulate_transient_wave(&self, t_s: f64, rise_time_s: f64) -> TransientWaveState {
        let ramp = if rise_time_s > 0.0 {
            (t_s / rise_time_s).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let p0 = self.peak_waveguide_pressure_gpa() * ramp;
        let p_match =
            self.transmitted_pressure_gpa(p0, SAPPHIRE_IMPEDANCE_MRAYL, AEROGEL_IMPEDANCE_MRAYL);
        let p_bath = self.transmitted_pressure_gpa(
            p_match,
            AEROGEL_IMPEDANCE_MRAYL,
            HELIUM4_IMPEDANCE_MRAYL,
        );
        let p_sub =
            self.transmitted_pressure_gpa(p0, SAPPHIRE_IMPEDANCE_MRAYL, INP_IMPEDANCE_MRAYL);
        TransientWaveState {
            t_s,
            p_waveguide_gpa: p0,
            p_matching_gpa: p_match,
            p_bath_gpa: p_bath,
            p_substrate_gpa: p_sub,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_engine() -> AcousticTampingFEA {
        AcousticTampingFEA::new(
            142.08e6,
            WAVEGUIDE_AREA_M2,
            72.0e9,
            AEROGEL_TAMPING_THICKNESS_NM * 1e-9,
        )
    }

    #[test]
    fn sapphire_impedance_matches_gate08() {
        assert!((SAPPHIRE_IMPEDANCE_MRAYL - 44.178).abs() <= 0.01);
    }

    #[test]
    fn aerogel_tamping_thickness_matches_gate09() {
        assert!((AEROGEL_TAMPING_THICKNESS_NM - 6.395).abs() <= 0.005);
    }

    #[test]
    fn matching_impedance_is_geometric_mean() {
        let e = default_engine();
        assert!((e.optimal_matching_impedance_mrayl() - AEROGEL_IMPEDANCE_MRAYL).abs() < 1e-4);
    }

    #[test]
    fn inp_substrate_stays_within_yield() {
        let e = default_engine();
        assert!(e.is_inp_within_yield());
        assert!(e.inp_substrate_pressure_gpa() > 0.0);
    }

    #[test]
    fn transient_ramps_to_steady_state() {
        let e = default_engine();
        let early = e.simulate_transient_wave(0.5e-9, 2.5e-9);
        let late = e.simulate_transient_wave(10e-9, 2.5e-9);
        assert!(early.p_waveguide_gpa < late.p_waveguide_gpa);
        assert!((late.p_waveguide_gpa - e.peak_waveguide_pressure_gpa()).abs() < 1e-12);
    }
}
