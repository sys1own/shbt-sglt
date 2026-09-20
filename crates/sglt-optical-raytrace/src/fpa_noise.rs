//! FPA CMOS/EMCCD noise pipeline (up1.txt §2).
//!
//! N_total(x,y) = Poisson(I·η_QE·t + I_dark(T)·t) + N(0, σ_read²) + N_cosmic,
//! with cosmic-ray charge clusters from a Landau distribution.

/// Detector waveband.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectorBand {
    /// 200–400 nm
    Uv,
    /// 400–1100 nm
    VisNir,
    /// 1.1–5.0 µm
    Swir,
}

impl DetectorBand {
    pub fn for_wavelength_nm(lambda_nm: f64) -> Self {
        if lambda_nm < 400.0 {
            Self::Uv
        } else if lambda_nm <= 1100.0 {
            Self::VisNir
        } else {
            Self::Swir
        }
    }
}

/// Per-band FPA parameters from the spec table.
#[derive(Debug, Clone, Copy)]
pub struct FpaParams {
    pub quantum_efficiency: f64,
    pub dark_current_e_s: f64, // e⁻/pixel/s at 4.2 K
    pub read_noise_e: f64,     // e⁻ RMS
    pub full_well_e: f64,
    pub pixel_pitch_um: f64,
    pub cosmic_rate_s: f64,    // events/pixel/s
}

impl FpaParams {
    pub fn band(b: DetectorBand) -> Self {
        match b {
            DetectorBand::Uv => Self {
                quantum_efficiency: 0.785,
                dark_current_e_s: 1.2e-4,
                read_noise_e: 1.1,
                full_well_e: 90_000.0,
                pixel_pitch_um: 6.5,
                cosmic_rate_s: 1.5e-3,
            },
            DetectorBand::VisNir => Self {
                quantum_efficiency: 0.92,
                dark_current_e_s: 2.5e-5,
                read_noise_e: 0.8,
                full_well_e: 120_000.0,
                pixel_pitch_um: 6.5,
                cosmic_rate_s: 1.5e-3,
            },
            DetectorBand::Swir => Self {
                quantum_efficiency: 0.835,
                dark_current_e_s: 8.0e-4,
                read_noise_e: 2.4,
                full_well_e: 150_000.0,
                pixel_pitch_um: 10.0,
                cosmic_rate_s: 1.5e-3,
            },
        }
    }
}

/// Deterministic xorshift64* PRNG — keeps the noise model reproducible in
/// HIL loops without pulling in `rand`.
pub struct XorShift64(pub u64);

impl XorShift64 {
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn uniform(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    pub fn gaussian(&mut self) -> f64 {
        let u1 = self.uniform().max(1e-300);
        let u2 = self.uniform();
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }
    /// Poisson(λ) — Knuth for small λ, normal approximation for large.
    pub fn poisson(&mut self, lambda: f64) -> f64 {
        if lambda <= 0.0 {
            return 0.0;
        }
        if lambda > 50.0 {
            return (lambda + lambda.sqrt() * self.gaussian()).max(0.0).round();
        }
        let l = (-lambda).exp();
        let mut k = 0u32;
        let mut p = 1.0;
        loop {
            p *= self.uniform();
            if p <= l {
                return k as f64;
            }
            k += 1;
        }
    }
    /// Landau-like cosmic-ray deposition: µ = 1200 e⁻, ξ = 180 e⁻ (Moyal
    /// approximation to the Landau density).
    pub fn landau(&mut self) -> f64 {
        let u = self.uniform().clamp(1e-12, 1.0 - 1e-12);
        let z = -(-(u.ln())).ln(); // Moyal quantile proxy
        1200.0 + 180.0 * z
    }
}

/// FPA noise engine: renders one frame of electron counts.
pub struct FpaNoiseModel {
    pub params: FpaParams,
    pub rng: XorShift64,
}

impl FpaNoiseModel {
    pub fn new(band: DetectorBand, seed: u64) -> Self {
        Self {
            params: FpaParams::band(band),
            rng: XorShift64(seed.max(1)),
        }
    }

    /// Electrons for one pixel given incident irradiance `i_photons_s`
    /// (photons/pixel/s) over `t_exp` seconds. Clips at full well.
    pub fn pixel_electrons(&mut self, i_photons_s: f64, t_exp_s: f64) -> f64 {
        let p = self.params;
        let signal = i_photons_s * p.quantum_efficiency * t_exp_s;
        let dark = p.dark_current_e_s * t_exp_s;
        let mut n = self.rng.poisson(signal + dark) + p.read_noise_e * self.rng.gaussian();
        if self.rng.uniform() < p.cosmic_rate_s * t_exp_s {
            n += self.rng.landau();
        }
        n.clamp(0.0, p.full_well_e)
    }
}
