//! Reactionless traction drive (transferred from `sys1own/shbt-ghost`
//! `ghost-propulsion-drive`): geodesic acceleration vectoring along the
//! forward offset vector `r_offset`,
//! `a_thrust = -∇Φ_seed(r_offset)`, and discrete delta-V bit-stepping
//! `ΔN(k) = floor(ΔP_net / P_bit)` bounded by the LANR surplus margin.

use crate::Vector3D;

/// Gravitational constant (SI).
pub const G_SI: f64 = 6.67430e-11;
/// Solar mass (kg).
pub const M_SUN_KG: f64 = 1.98847e30;
/// Speed of light (m/s).
pub const C_LIGHT: f64 = 299_792_458.0;

/// LANR array net power output (W): 999.054 kW.
pub const LANR_NET_W: f64 = 999_054.0;
/// Minimum LANR surplus margin per node (W): +93.054 kW floor.
pub const LANR_SURPLUS_MIN_W: f64 = 93_054.0;
/// Per-bit power quantum for swarm bit-stepping (W/bit).
pub const P_BIT_SWARM_W: f64 = 1.482;
/// Required active overflow for the nominal 1e-6 M_sun seed (bits).
pub const DELTA_N0_BITS: f64 = 7.542426e44;

/// Reactionless traction acceleration along the forward offset vector:
/// `a_thrust = -∇Φ_seed(r_offset) = -G M_seed / r² · r̂`.
/// Returns zero for a degenerate offset.
pub fn traction_accel(r_offset: Vector3D, seed_mass_msun: f64) -> Vector3D {
    let r = r_offset.norm();
    if r == 0.0 {
        return Vector3D::new(0.0, 0.0, 0.0);
    }
    let a = G_SI * seed_mass_msun * M_SUN_KG / (r * r);
    Vector3D::new(-r_offset.x / r * a, -r_offset.y / r * a, -r_offset.z / r * a)
}

/// Discrete delta-V bit-stepping:
/// `ΔN(k) = floor(ΔP_net / P_bit)`.
pub fn delta_n_bit_step(delta_p_net_w: f64, p_bit_w: f64) -> u64 {
    if delta_p_net_w <= 0.0 || p_bit_w <= 0.0 {
        return 0;
    }
    (delta_p_net_w / p_bit_w).floor() as u64
}

/// Power-aware bit-stepping allocator: throttles telemetry/thrust bit
/// steps when the net surplus drains toward the `+93.054 kW` floor.
#[derive(Debug, Clone)]
pub struct PowerAwareBitAllocator {
    /// Minimum surplus floor (W); below it allocation halts.
    pub surplus_min_w: f64,
    /// Power quantum per bit (W/bit).
    pub p_bit_w: f64,
}

impl PowerAwareBitAllocator {
    pub fn new() -> Self {
        Self { surplus_min_w: LANR_SURPLUS_MIN_W, p_bit_w: P_BIT_SWARM_W }
    }

    /// `ΔN_i(k) = floor(ΔP_net / P_bit)` with
    /// `ΔP_net = P_LANR - P_baseline - P_thrust - P_thermal`; returns 0 when
    /// the surplus floor is breached.
    pub fn allocate(
        &self,
        p_lanr: f64,
        p_baseline: f64,
        p_thrust: f64,
        p_thermal: f64,
    ) -> u64 {
        let p_net = p_lanr - p_baseline - p_thrust - p_thermal;
        if p_net < self.surplus_min_w {
            return 0;
        }
        delta_n_bit_step(p_net, self.p_bit_w)
    }
}

impl Default for PowerAwareBitAllocator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn traction_points_toward_seed() {
        let a = traction_accel(Vector3D::new(1.0e15, 0.0, 0.0), 1e-6);
        assert!(a.x < 0.0 && a.y == 0.0 && a.z == 0.0);
        let expected = G_SI * 1e-6 * M_SUN_KG / 1e30;
        assert!((a.x.abs() - expected).abs() / expected < 1e-12);
        assert_eq!(traction_accel(Vector3D::new(0.0, 0.0, 0.0), 1e-6).norm(), 0.0);
    }

    #[test]
    fn bit_step_floors() {
        assert_eq!(delta_n_bit_step(7.5, 5.0), 1);
        assert_eq!(delta_n_bit_step(LANR_NET_W, 1.0), LANR_NET_W as u64);
        assert_eq!(delta_n_bit_step(0.0, 1.482), 0);
        assert_eq!(delta_n_bit_step(95054.0, 1.482), 64139);
    }

    #[test]
    fn allocator_respects_surplus_floor() {
        let alloc = PowerAwareBitAllocator::new();
        assert_eq!(alloc.allocate(999_054.0, 900_000.0, 3_000.0, 1_000.0), 64139);
        assert_eq!(alloc.allocate(999_054.0, 900_000.0, 6_001.0, 1_000.0), 0);
    }
}
