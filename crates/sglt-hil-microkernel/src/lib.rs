//! sglt-hil-microkernel — bridge between the Rust simulation pipelines and the
//! freestanding C11 shbt-os microkernel (sglt.txt §1–2).
//!
//! * `simd_shunt_interlock` — AVX-512 current-shunt check transferred from
//!   `sys1own/shbt-qc` (`kernel/src/avx512_interlock.c` → compiled by
//!   `build.rs` from `kernel/src/shbt_ecc_avx512.c`), < 2.50 ns (Gate-14).
//! * `quench_recovery` — 4-step post-quench recovery driver transferred from
//!   `sys1own/shbt-qc` (`kernel/src/quench_driver.c`), ≤ 120.00 ns (Gate-15).
//! * `posix_shm_ring` — `shm_open`/`mmap` zero-copy SPSC telemetry ring
//!   transferred from `sys1own/shbt-cf`
//!   (`crates/shbt-fabrication-hil/src/{shm,ring,telemetry}.rs`), < 1.0 µs
//!   (Gate-16).

pub mod posix_shm_ring;
pub mod quench_recovery;
pub mod simd_shunt_interlock;
