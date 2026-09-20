//! sglt-transducer-fea — optoelectronic transducer array and acoustic
//! waveguide finite-element analysis (sglt.txt §1–2).
//!
//! `acoustic_waveguide_fea` is adapted from `sys1own/shbt-exotic`
//! (`src/acoustic_fea.rs` → `src/acoustic_impedance.rs`,
//! `AcousticTampingFEA`): transient acoustic coupling through a single-crystal
//! sapphire waveguide (`Z1 = 44.178 MRayl`) and a nanoporous silica aerogel
//! quarter-wave tamping layer (`d_m = 6.395 nm`, `Z_m = 1.1512 MRayl`).

pub mod acoustic_waveguide_fea;
