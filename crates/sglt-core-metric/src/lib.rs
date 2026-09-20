//! sglt-core-metric — 512-bit arbitrary-precision spacetime metric math for the
//! Synthetic Gravitational Lensing Telescope (sglt.txt §1, transfer matrix §2).
//!
//! Modules adapted from `sys1own/shbt-exotic`:
//! * `interference_tensor` — `MassCongestionEngine` (src/mass_congestion_engine.rs
//!   + src/shbt/mass_congestion.rs): `I_{μν}` tensor and `M_seed = α_seed ΔN`.
//! * `adm_foliation` — `ADMMetricAuditor` (src/warp_metric.rs): 3+1D ADM
//!   lapse/shift/spatial metric audit with `|det(g)+1| ≤ 1e-12`.
//! * `eikonal_predistortion` — non-paraxial `W(x, y; Θ)` phase maps for
//!   `Θ_tilt = 5° → 15°`.

pub mod adm_foliation;
pub mod eikonal_predistortion;
pub mod interference_tensor;

/// 512-bit MPFR precision for all arbitrary-precision registers.
pub const PREC: u32 = 512;

/// Speed of light (m/s).
pub const SPEED_OF_LIGHT_M_S: f64 = 299_792_458.0;

/// Eigenvector density-multiplier rigidity threshold `|μ - μ0| ≤ 1e-12`.
pub const EIGENVECTOR_RIGIDITY_THRESHOLD: f64 = 1.0e-12;
