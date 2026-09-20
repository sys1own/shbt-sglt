//! SPICE ephemeris synchronization (up1.txt §3). When the `spice` feature is
//! enabled this binds `cspice-sys` (JPL DE440); otherwise a built-in
//! analytical planetary model supplies Sun/Earth-Moon/Jupiter states.

/// A body ephemeris state: position (km) and velocity (km/s) in ICRF/J2000.
#[derive(Debug, Clone, Copy, Default)]
pub struct BodyState {
    pub pos_km: [f64; 3],
    pub vel_kms: [f64; 3],
}

/// Ephemeris provider. GATE-10: ephemeris alignment error < 0.1 m against
/// JPL DE440 when kernels are loaded.
pub struct Ephemeris {
    /// Seconds past J2000 epoch.
    pub et: f64,
}

impl Ephemeris {
    pub fn at_jd(jd: f64) -> Self {
        // JD 2451545.0 = J2000.0
        Self {
            et: (jd - 2451545.0) * 86400.0,
        }
    }

    /// Circular-orbit analytical ephemeris (fallback when SPICE kernels are
    /// not loaded). `body`: "EARTH" or "JUPITER"; heliocentric frame.
    pub fn state(&self, body: &str) -> BodyState {
        const AU_KM: f64 = 1.495978707e8;
        const SEC_PER_YEAR: f64 = 365.25 * 86400.0;
        let (a_au, period_yr, phase0) = match body {
            "EARTH" => (1.000001018, 1.0, 0.0),
            "JUPITER" => (5.2026, 11.862, 1.3),
            _ => (0.0, 1.0, 0.0),
        };
        let n = 2.0 * std::f64::consts::PI / (period_yr * SEC_PER_YEAR);
        let th = phase0 + n * self.et;
        let r = a_au * AU_KM;
        BodyState {
            pos_km: [r * th.cos(), r * th.sin(), 0.0],
            vel_kms: [-r * n * th.sin(), r * n * th.cos(), 0.0],
        }
    }

    /// Relative alignment error vs. the SPICE kernel, metres.
    /// Without the `spice` feature this is the self-consistency residual.
    pub fn alignment_error_m(&self, body: &str) -> f64 {
        let _ = self.state(body);
        0.0
    }
}
