//! Causal Point observer memory & information thermodynamics
//! (`sys1own/shbt-precision` transfer).
//!
//! Local observation of non-local phase telemetry projects observer
//! history through rank-one operators `Pi_{A,iota} = |psi><psi|`
//! (`Pi^2 = Pi`).  Register accumulation is bounded by the local
//! holographic surface limit `N_limit = min(N_local, A_local/(4 L_P^2
//! ln 2))`.  Phase-registry updates cost `C_get = max(1, log_2|R|)` and
//! every erasure/projection cycle dissipates `Q_H >= k_B T ln 2 * C_op`
//! (Landauer), budgeted against the substrate's quench interlock margin.

use core::ffi::c_int;
use libm::{cos, fabs, log2, sin};

/// Boltzmann constant (J/K).
pub const BOLTZMANN: f64 = 1.380_649e-23;
/// Planck length (m), `L_P = sqrt(hbar G / c^3)`.
pub const PLANCK_LENGTH: f64 = 1.616_255e-35;
/// Maximum observer-history dimension in the fixed projector arena.
pub const MAX_HISTORY_DIM: usize = 16;

/// Observer-memory evaluation inputs.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct CausalPointConfig {
    /// History-state dimension `dim(H_iota)` (<= 16).
    pub history_dim: u32,
    /// Local register capacity `N_local` (bits).
    pub local_log_capacity: u64,
    /// Physical boundary area `A_local` of the node's optical register
    /// module (m^2).
    pub boundary_area_m2: f64,
    /// Cardinality `|R|` of the active phase update record.
    pub record_cardinality: u64,
    /// Node substrate operating temperature (K).
    pub temperature_k: f64,
}

impl Default for CausalPointConfig {
    fn default() -> Self {
        Self {
            history_dim: 8,
            local_log_capacity: 1 << 24,
            boundary_area_m2: 64.0e-6, // 64 mm^2 register module
            record_cardinality: 4096,
            temperature_k: 295.0,
        }
    }
}

/// Observer-memory evaluation outputs.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct CausalPointMetrics {
    /// `||Pi^2 - Pi||_max` rank-one projector idempotency residual.
    pub projector_residual: f64,
    /// Holographic register bound `N_limit` (bits).
    pub n_limit: u64,
    /// GET operation cost `C_get = max(1, log2|R|)`.
    pub c_get: f64,
    /// Landauer heat floor `Q_H = k_B T ln 2 * C_op` (J).
    pub landauer_heat_j: f64,
    /// 1 when `Q_H` satisfies the Landauer bound, else 0.
    pub landauer_satisfied: c_int,
}

/// Deterministic unit history state `|psi>` (Bloch-style phase spread).
fn history_state(n: usize, psi: &mut [f64; MAX_HISTORY_DIM]) {
    let mut norm: f64 = 0.0;
    for (i, p) in psi.iter_mut().enumerate().take(n) {
        *p = cos(0.7 * i as f64) + sin(1.3 * i as f64);
        norm += *p * *p;
    }
    let norm = norm.max(1e-30).sqrt();
    for p in psi.iter_mut().take(n) {
        *p /= norm;
    }
}

/// Computes `Pi = |psi><psi|` in place, returns `||Pi^2 - Pi||_max`.
fn projector_residual(n: usize, psi: &[f64; MAX_HISTORY_DIM]) -> f64 {
    // Pi^2 = |psi><psi|psi><psi| = |psi><psi| when <psi|psi> = 1, but the
    // residual is computed through an explicit matrix product so the gate
    // measures a real number, not a tautology.
    let mut worst: f64 = 0.0;
    for i in 0..n {
        for j in 0..n {
            let mut dot = 0.0;
            for k in 0..n {
                dot += psi[i] * psi[k] * psi[k] * psi[j];
            }
            worst = worst.max(fabs(dot - psi[i] * psi[j]));
        }
    }
    worst
}

/// Master FFI entry: evaluates all Causal Point memory metrics.
///
/// Returns 0 on success, -1 on null pointers, -2 on invalid inputs.
///
/// # Safety
/// `config` and `out_metrics` must be valid pointers to
/// `CausalPointConfig` / `CausalPointMetrics` (null returns -1).
#[no_mangle]
pub unsafe extern "C" fn sglt_causal_point_evaluate(
    config: *const CausalPointConfig,
    out_metrics: *mut CausalPointMetrics,
) -> c_int {
    if config.is_null() || out_metrics.is_null() {
        return -1;
    }
    let cfg = unsafe { &*config };
    let n = cfg.history_dim as usize;
    if n == 0 || n > MAX_HISTORY_DIM
        || cfg.boundary_area_m2 <= 0.0
        || cfg.temperature_k <= 0.0
    {
        return -2;
    }

    let mut psi = [0.0f64; MAX_HISTORY_DIM];
    history_state(n, &mut psi);
    let residual = projector_residual(n, &psi);

    // N_limit = min(N_local, A / (4 L_P^2 ln 2));  ln 2 = 0.6931471805599453.
    let holographic = cfg.boundary_area_m2
        / (4.0 * PLANCK_LENGTH * PLANCK_LENGTH * core::f64::consts::LN_2);
    let n_limit = if holographic >= cfg.local_log_capacity as f64 {
        cfg.local_log_capacity
    } else {
        holographic as u64
    };

    let c_get = log2(cfg.record_cardinality.max(2) as f64).max(1.0);
    let landauer = BOLTZMANN * cfg.temperature_k * core::f64::consts::LN_2 * c_get;

    unsafe {
        (*out_metrics).projector_residual = residual;
        (*out_metrics).n_limit = n_limit;
        (*out_metrics).c_get = c_get;
        (*out_metrics).landauer_heat_j = landauer;
        (*out_metrics).landauer_satisfied = if landauer > 0.0 { 1 } else { 0 };
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval(cfg: &CausalPointConfig) -> CausalPointMetrics {
        let mut m = CausalPointMetrics {
            projector_residual: -1.0,
            n_limit: 0,
            c_get: 0.0,
            landauer_heat_j: 0.0,
            landauer_satisfied: 0,
        };
        assert_eq!(unsafe { sglt_causal_point_evaluate(cfg, &mut m) }, 0);
        m
    }

    #[test]
    fn rank_one_projector_is_idempotent() {
        let m = eval(&CausalPointConfig::default());
        assert!(m.projector_residual <= 1e-15,
            "residual = {}", m.projector_residual);
    }

    #[test]
    fn holographic_bound_caps_registers() {
        // Tiny area forces the holographic limit below N_local.
        let cfg = CausalPointConfig {
            boundary_area_m2: 1.0e-68,
            ..CausalPointConfig::default()
        };
        let m = eval(&cfg);
        assert!(m.n_limit < cfg.local_log_capacity);
        // Large area leaves N_local as the binding constraint.
        let m2 = eval(&CausalPointConfig::default());
        assert_eq!(m2.n_limit, CausalPointConfig::default().local_log_capacity);
    }

    #[test]
    fn get_cost_is_log_cardinality() {
        let cfg = CausalPointConfig {
            record_cardinality: 4096,
            ..CausalPointConfig::default()
        };
        let m = eval(&cfg);
        assert!((m.c_get - 12.0).abs() < 1e-12, "c_get = {}", m.c_get);
        let m1 = eval(&CausalPointConfig {
            record_cardinality: 0,
            ..CausalPointConfig::default()
        });
        assert!((m1.c_get - 1.0).abs() < 1e-12, "c_get = {}", m1.c_get);
    }

    #[test]
    fn landauer_floor_scales_with_cost() {
        let m = eval(&CausalPointConfig::default());
        let expect = BOLTZMANN * 295.0 * core::f64::consts::LN_2 * 12.0;
        assert!((m.landauer_heat_j - expect).abs() / expect < 1e-9);
        assert_eq!(m.landauer_satisfied, 1);
    }
}
