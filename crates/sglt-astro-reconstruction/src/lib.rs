//! sglt-astro-reconstruction — focal-plane image formation and verification.
//!
//! `richardson_lucy_deconv` is transferred from `sys1own/shbt-precision`
//! (`src/optical_solver.rs`, `RichardsonLucySolver`, `compute_strehl_ratio`)
//! and adapted to the SGLT synthetic aperture: `D_eff = 2.00 m`,
//! `θ_res = 0.0629″`, `μ_amp = 1.2566e7`, `S ≥ 0.999999980`.

pub mod richardson_lucy_deconv;
