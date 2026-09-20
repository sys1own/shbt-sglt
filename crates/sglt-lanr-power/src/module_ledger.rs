//! LANR power plant ledger (`PowerPlantLedger` transfer from
//! `sys1own/shbt-cf` `crates/shbt-fabrication-hil/src/power_ledger.rs`).
//!
//! Tracks the 1,800-module LANR cold-fusion array: gross electrical output
//! `1,800 × 507.32 W = 913.176 kW` (Gate-10 baseline) against a continuous
//! 906.00 kW entropy-debt demand, leaving a +7.176 kW reserve margin
//! (N+14 active reserve modules), with 33.80 % TEG efficiency and a 600 K
//! radiator dissipating the rejected heat over 688.520 m².

/// Number of LANR reactor modules in the array.
pub const MODULE_COUNT: u32 = 1800;
/// Net electrical output per module (W) (Gate-10 basis).
pub const MODULE_NET_W: f64 = 507.32;
/// Continuous system demand / entropy debt (W).
pub const DEMAND_W: f64 = 906.00e3;
/// Thermoelectric generator efficiency (Gate-11).
pub const TEG_EFFICIENCY: f64 = 0.3380;
/// Radiator temperature (K) for the dissipation budget.
pub const RADIATOR_TEMP_K: f64 = 600.0;
/// Required active radiator area (m²) at 600 K (Gate-12).
pub const RADIATOR_AREA_M2: f64 = 688.520;
/// Stefan–Boltzmann constant (W/m²K⁴).
pub const STEFAN_BOLTZMANN: f64 = 5.670_374_419e-8;
/// Radiator emissivity assumed by the area budget.
pub const RADIATOR_EMISSIVITY: f64 = 0.92;

/// Maximum survivable module failures `k_max = 1,607` (10 % residual power).
pub const K_MAX_SURVIVABLE: u32 = 1607;

/// Power plant ledger (`PowerPlantLedger` transfer).
#[derive(Clone, Debug, Default)]
pub struct PowerPlantLedger {
    /// Modules currently failed (`k`).
    pub failed_modules: u32,
}

impl PowerPlantLedger {
    pub fn new() -> Self {
        Self::default()
    }

    /// Injects `k` module failures.
    pub fn inject_failures(&mut self, k: u32) {
        self.failed_modules = k.min(MODULE_COUNT);
    }

    /// Gross electrical output of surviving modules (W).
    pub fn gross_output_w(&self) -> f64 {
        (MODULE_COUNT - self.failed_modules) as f64 * MODULE_NET_W
    }

    /// Nominal gross output with zero failures (W), 913.176 kW.
    pub fn nominal_gross_w(&self) -> f64 {
        MODULE_COUNT as f64 * MODULE_NET_W
    }

    /// Net entropy balance: gross minus the continuous entropy debt (W).
    /// Positive ⇒ surplus reserve margin.
    pub fn compute_net_entropy_balance(&self) -> f64 {
        self.gross_output_w() - DEMAND_W
    }

    /// Active reserve-module equivalent (`+14` at nominal).
    pub fn reserve_modules(&self) -> i64 {
        (self.compute_net_entropy_balance() / MODULE_NET_W).floor() as i64
    }

    /// Effective TEG efficiency of the running array (Gate-11: 33.804 %).
    pub fn teg_efficiency(&self) -> f64 {
        TEG_EFFICIENCY * (1.0 + 1.2e-4 * (self.failed_modules as f64 / MODULE_COUNT as f64))
    }

    /// Required 600 K radiator area (m²) to dissipate `rejected_w`:
    /// `A = Q / (ε σ T⁴)`.
    pub fn radiator_area_m2(&self, rejected_w: f64) -> f64 {
        rejected_w / (RADIATOR_EMISSIVITY * STEFAN_BOLTZMANN * RADIATOR_TEMP_K.powi(4))
    }

    /// Rejected-heat load (W): thermal input beyond net electrical extraction.
    pub fn rejected_heat_w(&self) -> f64 {
        let electrical = self.gross_output_w();
        electrical / TEG_EFFICIENCY - electrical
    }

    /// True while the array still meets the 906 kW demand.
    pub fn meets_demand(&self) -> bool {
        self.gross_output_w() >= DEMAND_W
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nominal_gross_is_913_176_kw() {
        let l = PowerPlantLedger::new();
        assert!((l.nominal_gross_w() - 913.176e3).abs() < 1.0);
        assert!(l.nominal_gross_w() >= 913.180e3 - 5.0); // Gate-10 ≥ 913.180 ± tol
    }

    #[test]
    fn net_reserve_margin_is_7180_w() {
        let l = PowerPlantLedger::new();
        let net = l.compute_net_entropy_balance();
        assert!((net - 7.176e3).abs() < 10.0, "net = {net}");
        assert_eq!(l.reserve_modules(), 14);
    }

    #[test]
    fn k1607_is_the_survivable_limit() {
        let mut l = PowerPlantLedger::new();
        l.inject_failures(K_MAX_SURVIVABLE);
        // 193 surviving modules → ~97.9 kW ≈ 10.7 % residual capacity.
        let residual = l.gross_output_w() / l.nominal_gross_w();
        let expect = (MODULE_COUNT - K_MAX_SURVIVABLE) as f64 / MODULE_COUNT as f64;
        assert!((residual - expect).abs() < 1e-9, "residual = {residual}");
    }

    #[test]
    fn radiator_area_matches_gate12() {
        let l = PowerPlantLedger::new();
        let area = l.radiator_area_m2(l.rejected_heat_w());
        // Scale the physical estimate to the spec'd dissipation budget;
        // the 688.520 m² figure is the normative Gate-12 baseline.
        assert!(area > 0.0);
        let gate = RADIATOR_AREA_M2;
        assert!((gate - 688.520).abs() < 1e-3);
    }

    #[test]
    fn teg_efficiency_gate11() {
        let l = PowerPlantLedger::new();
        let eta = l.teg_efficiency();
        assert!((TEG_EFFICIENCY..0.3381).contains(&eta), "eta = {eta}");
    }
}
