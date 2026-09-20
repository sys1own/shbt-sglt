//! Heliocentric and relative equations of motion (up2.txt §4):
//!   a = −μ r/r³ + a_J2 + a_GR + a_SRP + a_thrust,
//!   a_J2 = (3/2) J₂ μ R_☉² / r⁵ (…),  a_GR = μ/c²r³[(4μ/r − v²)r + 4(r·v)v],
//!   a_SRP = C_r A/m P_☉ (AU/r)²,  δr̈ = a_sensor − a_lens + a_ctrl.

use crate::{Vector3D, AU_TO_METERS};

/// Solar gravitational parameter, m³/s².
pub const MU_SUN: f64 = 1.32712440018e20;
/// Solar oblateness.
pub const J2_SUN: f64 = 2.11e-7;
/// Solar radius, m.
pub const R_SUN: f64 = 6.957e8;
/// Speed of light, m/s.
pub const C_LIGHT: f64 = 299_792_458.0;
/// Solar radiation pressure at 1 AU, N/m².
pub const P_1AU: f64 = 4.56e-6;

fn add(a: Vector3D, b: Vector3D) -> Vector3D {
    Vector3D::new(a.x + b.x, a.y + b.y, a.z + b.z)
}
fn scale(a: Vector3D, s: f64) -> Vector3D {
    Vector3D::new(a.x * s, a.y * s, a.z * s)
}

/// Point-mass + J₂ quadrupole acceleration.
pub fn accel_j2(r: Vector3D) -> Vector3D {
    let rn = r.norm();
    let base = -MU_SUN / (rn * rn * rn);
    let mut a = scale(r, base);

    let f = 1.5 * J2_SUN * MU_SUN * R_SUN * R_SUN / rn.powi(5);
    let zratio = r.z / rn;
    a.x += f * r.x * (5.0 * zratio * zratio - 1.0);
    a.y += f * r.y * (5.0 * zratio * zratio - 1.0);
    a.z += f * r.z * (5.0 * zratio * zratio - 3.0);
    a
}

/// First post-Newtonian correction.
pub fn accel_1pn(r: Vector3D, v: Vector3D) -> Vector3D {
    let rn = r.norm();
    let rdotv = r.x * v.x + r.y * v.y + r.z * v.z;
    let v2 = v.x * v.x + v.y * v.y + v.z * v.z;
    let pre = MU_SUN / (C_LIGHT * C_LIGHT * rn * rn * rn);
    let term_r = scale(r, 4.0 * MU_SUN / rn - v2);
    let term_v = scale(v, 4.0 * rdotv);
    scale(add(term_r, term_v), pre)
}

/// Solar radiation pressure acceleration (cannonball).
pub fn accel_srp(r: Vector3D, cr: f64, area_m2: f64, mass_kg: f64) -> Vector3D {
    let rn = r.norm();
    let p = P_1AU * (AU_TO_METERS / rn).powi(2);
    let amag = cr * area_m2 / mass_kg * p;
    scale(r, -amag / rn)
}

/// Total heliocentric acceleration.
pub fn heliocentric_accel(
    r: Vector3D,
    v: Vector3D,
    cr: f64,
    area_m2: f64,
    mass_kg: f64,
    thrust: Vector3D,
) -> Vector3D {
    let a = accel_j2(r);
    let a = add(a, accel_1pn(r, v));
    let a = add(a, accel_srp(r, cr, area_m2, mass_kg));
    add(a, thrust)
}

/// Relative acceleration between sensor craft and lens craft (bounded-error
/// relative flight, GATE-41 target 3σ ≤ 1 nm with metrology feedback).
pub fn relative_accel(
    r_sensor: Vector3D,
    v_sensor: Vector3D,
    r_lens: Vector3D,
    v_lens: Vector3D,
    ctrl: Vector3D,
) -> Vector3D {
    let a_s = heliocentric_accel(
        r_sensor,
        v_sensor,
        1.3,
        2.0,
        150.0,
        Vector3D::new(0.0, 0.0, 0.0),
    );
    let a_l = heliocentric_accel(
        r_lens,
        v_lens,
        1.3,
        2.0,
        400.0,
        Vector3D::new(0.0, 0.0, 0.0),
    );
    add(a_s.sub(&a_l), ctrl)
}

/// Jacobi-like energy drift monitor: relative error in the specific energy
/// C_J = ½v² − μ/r − ½Ω²-type integral (heliocentric: C = v²/2 − μ/r).
/// GATE-42 requires |ΔC_J| ≤ 1e-12 over 1000 orbits for the integrator.
pub fn jacobi_energy(r: Vector3D, v: Vector3D) -> f64 {
    0.5 * (v.x * v.x + v.y * v.y + v.z * v.z) - MU_SUN / r.norm()
}
