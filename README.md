# shbt-sglt — SGLT Formation-Flight Simulation Stack

Implementation of the SGLT architectural blueprint (`sglt.txt`): a 6-crate
Rust workspace, a freestanding C11 `shbt-os` microkernel, EDA exporters, and a
Python orchestration CLI.

## Layout

- `crates/sglt-core-metric` — 512-bit `rug`/MPFR interference tensor
  (`M_seed = α_seed·ΔN`), ADM 3+1 foliation auditor (`|det(g)+1| ≤ 1e-12`),
  non-paraxial eikonal pre-distortion (`Θ_tilt = 5°→15°`).
- `crates/sglt-flight-gnc` — heterodyne metrology (1064.5 nm,
  σ_r ≤ 0.17 pm/√Hz), SE-L2 halo-orbit dynamics (Δa_SRP ≈ 3.42e-8 m/s²),
  2-tier hybrid controller, minimum-jerk slew, Tsiolkovsky propellant budget.
- `crates/sglt-transducer-fea` — sapphire/aerogel acoustic tamping FEA.
- `crates/sglt-lanr-power` — 1,800-module LANR ledger (913.18 kW,
  +507.32 W_net/module), N-k fault derating (ΔN = ⌊ΔP_net/P_bit⌋) with GUM-S1
  Monte Carlo uncertainty.
- `crates/sglt-hil-microkernel` — FFI surface over the C kernel: SECDED
  Hamming(72,64), AVX-512 shunt interlock, quench recovery, POSIX shm ring.
- `crates/sglt-astro-reconstruction` — Richardson–Lucy deconvolution
  (D_eff = 2.00 m, θ_res = 0.0629″, S ≥ 0.999999980).
- `kernel/` — freestanding C11 runtime (`shbt_core_runtime.c`,
  `shbt_ecc_avx512.c`, `shbt_hardware.h` @ `0x70000000`, `linker.ld` with the
  2,112-byte `.stinespring_frame` SRAM allocation).
- `eda/exporters/` — `pdk_hexamer_exporter.py` (8×8 HBT GDSII) and
  `export_interposer_rf.py` (12-layer RO4350B `.s2p`).
- `python/shbt_sglt/` — PyO3 FFI bindings, Streamlit HUD, `cli/main.py`.
- `tests/` — Rust integration suites + `test_hil_latency.py`.

## Commands

```bash
python python/shbt_sglt/cli/main.py build-kernel        # → crates/sglt-hil-microkernel/bin/shbt_reference.so
cargo test --workspace                                   # Rust unit + integration suites
python tests/run_all_tests.py                            # cargo test + HIL latency benches
python python/shbt_sglt/cli/main.py sim --duration 3600  # multi-domain HIL co-simulation
python python/shbt_sglt/cli/main.py export-eda           # eda/masks/hbt_array.gds + eda/rf/interposer.s2p
python python/shbt_sglt/cli/main.py verify --suite full-audit --json-report verification_matrix.json
```

## Prerequisites

- Rust ≥ 1.83, gcc with AVX-512F (`-mavx512f`), Python ≥ 3.10.
- `libgmp-dev libmpfr-dev libmpc-dev m4` for the `rug` crate.
- Optional: `streamlit` for `dashboard_hud.py`; `maturin`/`pyo3` toolchain to
  build `python/shbt_sglt/ffi_bindings.rs` into `shbt_sglt_native`.
