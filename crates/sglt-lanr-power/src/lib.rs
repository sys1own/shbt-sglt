//! sglt-lanr-power — LANR cold-fusion power plant ledger and N-k
//! fault-tolerant derating (sglt.txt §2–3).
//!
//! * `module_ledger` — `PowerPlantLedger` transferred from
//!   `sys1own/shbt-cf` (`crates/shbt-fabrication-hil/src/power_ledger.rs`):
//!   1,800 LANR modules, 999.05 kW gross vs 906.00 kW demand, 33.80 % TEG.
//! * `fault_derating_protocol` — GUM Supplement 1 Monte Carlo dual-number
//!   engine transferred from `sys1own/shbt-cf`
//!   (`crates/shbt-metrology-gum/src/{dual,engine}.rs`) plus the SGLT `N-k`
//!   seed-mass/focal-baseline derating chain.

pub mod fault_derating_protocol;
pub mod module_ledger;
