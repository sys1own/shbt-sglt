//! 4-body CR3BP equations of motion in the Sun–Earth rotating frame
//! (up1.txt §3), with SRP and Jupiter third-body perturbations.

/// CR3BP + perturbation parameters (normalized units).
#[derive(Debug, Clone, Copy)]
pub struct Cr3bpParams {
    /// Earth–Moon / (Sun + Earth–Moon) mass ratio ≈ 3.003e-6.
    pub mu: f64,
    /// Jupiter gravitational parameter in normalized units.
    pub mu_jupiter: f64,
    /// SRP coefficient S0·CR·A/(c·m) in normalized units.
    pub srp_coeff: f64,
}

impl Default for Cr3bpParams {
    fn default() -> Self {
        Self {
            mu: 3.003489614915e-6,
            mu_jupiter: 9.547919e-4,
            srp_coeff: 4.56e-8, // ~2 m²/kg-class spacecraft
        }
    }
}

/// Effective scalar potential
/// Ω = ½(x²+y²) + (1−μ)/r1 + μ/r2 + ½μ(1−μ).
pub fn omega_potential(x: f64, y: f64, z: f64, mu: f64) -> f64 {
    let r1 = ((x + mu).powi(2) + y * y + z * z).sqrt();
    let r2 = ((x - 1.0 + mu).powi(2) + y * y + z * z).sqrt();
    0.5 * (x * x + y * y) + (1.0 - mu) / r1 + mu / r2 + 0.5 * mu * (1.0 - mu)
}

/// Jacobi constant C_J = 2Ω − (ẋ² + ẏ² + ż²). GATE-09: |ΔC_J| ≤ 1e-12.
pub fn jacobi_constant(state: &[f64; 6], mu: f64) -> f64 {
    2.0 * omega_potential(state[0], state[1], state[2], mu)
        - (state[3].powi(2) + state[4].powi(2) + state[5].powi(2))
}

/// Jupiter position in the rotating frame at normalized time `t`
/// (Jupiter orbits at ~5.2 AU → dimensionless radius r_j; its angular rate
/// in the rotating frame is n_j − 1 where n_j is Jupiter's mean motion).
pub fn jupiter_rotating_position(t: f64) -> (f64, f64, f64) {
    const R_J: f64 = 5.2026;
    const N_J: f64 = 0.0834; // normalized mean motion of Jupiter
    let th = (N_J - 1.0) * t;
    (R_J * th.cos(), R_J * th.sin(), 0.0)
}

/// Rotating-frame equations of motion, y = [x y z ẋ ẏ ż]:
///   ẍ − 2ẏ = ∂Ω/∂x + a_SRP,x + a_Jup,x
///   ÿ + 2ẋ = ∂Ω/∂y + a_SRP,y + a_Jup,y
///   z̈     = ∂Ω/∂z + a_SRP,z + a_Jup,z
/// `t` is the normalized time (used for Jupiter ephemeris).
pub fn cr3bp_acceleration_at(state: &[f64; 6], t: f64, p: &Cr3bpParams) -> [f64; 6] {
    let (x, y, z, vx, vy, vz) = (state[0], state[1], state[2], state[3], state[4], state[5]);
    let mu = p.mu;

    let r1v = [x + mu, y, z];
    let r2v = [x - 1.0 + mu, y, z];
    let r1 = (r1v[0].powi(2) + r1v[1].powi(2) + r1v[2].powi(2)).sqrt();
    let r2 = (r2v[0].powi(2) + r2v[1].powi(2) + r2v[2].powi(2)).sqrt();
    let r1c = r1.powi(3);
    let r2c = r2.powi(3);

    // ∂Ω/∂· components
    let gx = x - (1.0 - mu) * r1v[0] / r1c - mu * r2v[0] / r2c;
    let gy = y - (1.0 - mu) * r1v[1] / r1c - mu * r2v[1] / r2c;
    let gz = -(1.0 - mu) * r1v[2] / r1c - mu * r2v[2] / r2c;

    // SRP: a = srp_coeff / r1² · (r1_0/r1)² · r̂1   (r1_0 = 1 AU normalized)
    let a_srp = p.srp_coeff / (r1 * r1 * r1 * r1);
    let srp = [a_srp * r1v[0], a_srp * r1v[1], a_srp * r1v[2]];

    // Jupiter third-body indirect + direct terms
    let (jx, jy, jz) = jupiter_rotating_position(t);
    let dv = [jx - x, jy - y, jz - z];
    let d3 = (dv[0].powi(2) + dv[1].powi(2) + dv[2].powi(2)).sqrt().powi(3);
    let rj3 = (jx * jx + jy * jy + jz * jz).sqrt().powi(3);
    let jup = [
        p.mu_jupiter * (dv[0] / d3 - jx / rj3),
        p.mu_jupiter * (dv[1] / d3 - jy / rj3),
        p.mu_jupiter * (dv[2] / d3 - jz / rj3),
    ];

    [
        vx,
        vy,
        vz,
        gx + 2.0 * vy + srp[0] + jup[0],
        gy - 2.0 * vx + srp[1] + jup[1],
        gz + srp[2] + jup[2],
    ]
}

/// Time-independent wrapper (Jupiter at its epoch position).
pub fn cr3bp_acceleration(state: &[f64; 6], p: &Cr3bpParams) -> [f64; 6] {
    cr3bp_acceleration_at(state, 0.0, p)
}
