//! sglt-flight-gnc — SE-L2 formation flight, heterodyne laser metrology, and
//! 2-tier hybrid GNC (sglt.txt §2–3).
//!
//! Modules adapted from `sys1own/shbt-precision`:
//! * `heterodyne_metrology` — `HeterodyneInterferometer`: 1064.500 nm
//!   dual-wavelength 80 MHz beat interferometry + DWS.
//! * `halo_orbit_dynamics` — `SEL2Environment`: SRP, tidal tensor, outgassing,
//!   C/SiC thermal expansion.
//! * `hybrid_controller` — `HybridGNCController`: electrospray thrusters
//!   (f < 0.1 Hz) + 8×8 InP phase shifters (0.1 Hz ≤ f ≤ 100 kHz).
//!
//! SGLT-specific additions (§3): `minimum_jerk` (5th-order Sensor Craft slew)
//! and `propellant_budget` (10-year Tsiolkovsky bookkeeping).

pub mod halo_orbit_dynamics;
pub mod heterodyne_metrology;
pub mod hybrid_controller;
pub mod minimum_jerk;
pub mod propellant_budget;
/// 3rd-order kinematic wake tensor momentum compensation (shbt-ghost transfer).
pub mod wake_compensation;
