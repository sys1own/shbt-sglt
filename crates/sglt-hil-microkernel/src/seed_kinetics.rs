//! Non-equilibrium seed transient kinetics (transferred from
//! `sys1own/shbt-ghost` `ghost-core-engine::SeedKineticsEngine`).
//!
//! Models `ΔN(t) = ΔN_0 · exp(-t/τ_quench)·Θ(t)` emergency-quench decay,
//! the back-EMF surge in the coupling coils, the 94.20%-efficient SiC
//! crowbar inductive energy capture handling 142.08 MW transient thermal
//! surges, and 1D semi-infinite thermal diffusion in the CVD
//! diamond-on-GaN cold plate.

/// GaN current-shunt quench interlock latency bound (ns): `τ_quench ≤ 2.18`.
pub const QUENCH_LATENCY_NS: f64 = 2.18;
/// SiC crowbar energy-recovery efficiency (94.20%).
pub const SIC_RECOVERY_EFFICIENCY: f64 = 0.9420;
/// Transient surge handled by the SiC crowbar (MW).
pub const TRANSIENT_SURGE_MW: f64 = 142.08;
/// Minimum thermal headroom at the cold plate (K).
pub const THERMAL_HEADROOM_MIN_K: f64 = 11.79;

extern "C" {
    /// C ABI: transient seed ignition on the SHBT-MMIO-1 block.
    fn shbt_seed_ignition_transient(delta_n0_bits: u64);
    /// C ABI: sub-2.50 ns emergency current-shunt quench.
    fn shbt_emergency_current_shunt();
    /// C ABI: `DeltaN(t) = DeltaN0 exp(-t/tau_quench) Theta(t)` (t in ns).
    fn shbt_quench_transient_delta_n(delta_n0_bits: u64, t_ns: f64) -> f64;
    /// C ABI: SiC crowbar recovered power (MW) at 94.20% efficiency.
    fn shbt_sic_crowbar_capture_mw(surge_mw: f64) -> f64;
    /// C ABI: write derate/decrement flags at MMIO 0x70000010.
    fn shbt_lanr_derate_interlock(flags: u32);
}

/// LANR power derating flag bit (`0x70000010` bit 0).
pub const DERATE_LANR_BIT: u32 = 1 << 0;
/// Seed mass decrement flag bit (`0x70000010` bit 1).
pub const DERATE_SEED_MASS_BIT: u32 = 1 << 1;

/// Hosted (non-bare-metal) callers hit a shadow aperture; bare-metal
/// targets hit the physical 0x70000010 word.
pub fn seed_ignition_transient(delta_n0_bits: u64) {
    unsafe { shbt_seed_ignition_transient(delta_n0_bits) }
}

/// Trigger the sub-2.50 ns emergency current-shunt quench.
pub fn emergency_current_shunt() {
    unsafe { shbt_emergency_current_shunt() }
}

/// Surviving overflow bits `ΔN(t)` after `t_ns` of quench decay.
pub fn quench_transient_delta_n(delta_n0_bits: u64, t_ns: f64) -> f64 {
    unsafe { shbt_quench_transient_delta_n(delta_n0_bits, t_ns) }
}

/// SiC crowbar recovered power (MW) from a transient surge.
pub fn sic_crowbar_capture_mw(surge_mw: f64) -> f64 {
    unsafe { shbt_sic_crowbar_capture_mw(surge_mw) }
}

/// Write the LANR derate/decrement flag word.
pub fn lanr_derate_interlock(flags: u32) {
    unsafe { shbt_lanr_derate_interlock(flags) }
}

/// Non-equilibrium seed kinetics engine.
pub struct SeedKineticsEngine {
    /// Initial active overflow `ΔN_0` (bits).
    pub delta_n_0: f64,
    /// Seed ignition time constant (s).
    pub tau_ign: f64,
    /// Emergency quench time constant (s), `≤ 2.18 ns`.
    pub tau_quench: f64,
    /// Effective coupling-coil inductance (H).
    pub l_eff: f64,
    /// SiC crowbar harvesting efficiency.
    pub sic_efficiency: f64,
}

impl SeedKineticsEngine {
    pub fn new() -> Self {
        Self {
            delta_n_0: 1.0e18,
            tau_ign: 0.45e-9,
            tau_quench: 2.18e-9,
            l_eff: 8.42e-3,
            sic_efficiency: SIC_RECOVERY_EFFICIENCY,
        }
    }

    /// Quench transient `ΔN(t) = ΔN_0 exp(-t/τ_quench) Θ(t)`:
    /// returns `(ΔN, i_eff, v_surge)` at time `t`.
    pub fn compute_quench_transient(&self, t: f64) -> (f64, f64, f64) {
        if t < 0.0 {
            return (self.delta_n_0, 0.0, 0.0);
        }
        let decay = (-t / self.tau_quench).exp();
        let delta_n = self.delta_n_0 * decay;
        let gamma_0 = 1.602176634e-19;
        let i_eff = gamma_0 * (self.delta_n_0 / self.tau_quench) * decay;
        let v_surge =
            self.l_eff * gamma_0 * (self.delta_n_0 / self.tau_quench.powi(2)) * decay;
        (delta_n, i_eff, v_surge)
    }

    /// SiC crowbar inductive energy capture: energy recovered (J) from a
    /// dump of `e_in` J at 94.20% efficiency.
    pub fn crowbar_recovered_j(&self, e_in: f64) -> f64 {
        self.sic_efficiency * e_in
    }

    /// 1D semi-infinite thermal diffusion under surge `p_surge` (W) over
    /// plate `area` (m²): the unrecovered 5.80% dissipates as `q_0`,
    /// `T(t) = T_0 + (2 q_0 / effusivity) √t`. Verifies
    /// `T_c − T_peak ≥ 11.79 K`.
    pub fn verify_thermal_headroom(
        &self,
        p_surge: f64,
        area: f64,
        t_0: f64,
        t_c: f64,
    ) -> Result<f64, &'static str> {
        let unrecovered_fraction = 1.0 - self.sic_efficiency;
        let q_0 = (unrecovered_fraction * p_surge) / area;
        let k_dia = 2200.0;
        let rho_dia = 3515.0;
        let cp_dia = 520.0;
        let thermal_eff = (std::f64::consts::PI * k_dia * rho_dia * cp_dia).sqrt();
        let delta_t_max = (2.0 * q_0 / thermal_eff) * self.tau_quench.sqrt();
        let t_peak = t_0 + delta_t_max;
        let headroom = t_c - t_peak;
        if headroom >= THERMAL_HEADROOM_MIN_K {
            Ok(headroom)
        } else {
            Err("CRITICAL: Thermal headroom violated threshold (delta_T < 11.79 K)")
        }
    }
}

impl Default for SeedKineticsEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quench_transient_decays() {
        let e = SeedKineticsEngine::new();
        let (dn0, ..) = e.compute_quench_transient(0.0);
        assert_eq!(dn0, e.delta_n_0);
        let (dn, i, v) = e.compute_quench_transient(2.18e-9);
        assert!((dn / e.delta_n_0 - (-1.0f64).exp()).abs() < 1e-12);
        assert!(i > 0.0 && v > 0.0);
        assert_eq!(e.compute_quench_transient(-1.0).0, e.delta_n_0);
        assert!(e.tau_quench * 1e9 <= QUENCH_LATENCY_NS + 1e-9);
    }

    #[test]
    fn crowbar_recovers_94_20() {
        let e = SeedKineticsEngine::new();
        assert!((e.crowbar_recovered_j(1.0) - 0.9420).abs() < 1e-15);
        assert_eq!(TRANSIENT_SURGE_MW, 142.08);
    }
}
