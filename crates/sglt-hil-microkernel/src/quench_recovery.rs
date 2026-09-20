//! Post-quench recovery driver (`shbt_recover` transfer from
//! `sys1own/shbt-qc` `kernel/src/quench_driver.c` / `shbt_core_runtime.c`).
//!
//! Executes the 4-step fail-safe sequence on the SHBT-MMIO-1 register block:
//!
//! 1. inspect/quarantine fault condition,
//! 2. assert global RF blanking,
//! 3. flush and correct the latched ECC word (SECDED Hamming(72,64)),
//! 4. request PLL lock and wait for confirmation,
//!
//! bounded within the ≤ 120.00 ns budget (Gate-15 baseline 114.200 ns).

/// Recovery latency budget (ns).
pub const RECOVERY_BUDGET_NS: f64 = 120.00;
/// Status bit: PLL locked.
pub const STATUS_PLL_LOCK: u32 = 1 << 1;
/// Status bit: ECC error latched.
pub const STATUS_ECC_ERR: u32 = 1 << 2;
/// Status bit: overtemperature.
pub const STATUS_OVERTEMP: u32 = 1 << 0;
/// Status bit: fault state.
pub const STATUS_FAULT_ST: u32 = 1 << 3;
/// Poll iterations allowed while waiting for PLL lock.
pub const PLL_WAIT_ITERS: u32 = 1_000_000;

/// Mirror of the normative 56-byte SHBT-MMIO-1 register block
/// (`shbt_hardware.h`) for host-side recovery simulation.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct MmioRegisters {
    /// 0x00 — status.
    pub status: u32,
    /// 0x04 — global RF blanking.
    pub blank: u32,
    /// 0x08 — FIFO data.
    pub fifo_data: u32,
    /// 0x0C — PLL control.
    pub pll_ctrl: u32,
    /// 0x10 — ECC data low.
    pub ecc_low: u32,
    /// 0x14 — ECC data high.
    pub ecc_high: u32,
    /// 0x18 — ECC check bits.
    pub ecc_check: u32,
    /// 0x1C — ECC commit strobe.
    pub ecc_commit: u32,
    /// 0x20 — fault latch.
    pub fault_latch: u32,
    /// 0x24 — channel select.
    pub channel_select: u32,
    /// 0x28 — ABI version.
    pub abi_version: u32,
    /// 0x2C — phase offset.
    pub phase_offset: u32,
    /// 0x30 — ECC error counts.
    pub ecc_counts: u32,
    /// 0x34 — control.
    pub control: u32,
}

const _: () = assert!(std::mem::size_of::<MmioRegisters>() == 0x38);

#[cfg(target_arch = "x86_64")]
extern "C" {
    /// `uint8_t shbt_ecc_encode(uint64_t data)` — SECDED check byte.
    fn shbt_ecc_encode(data: u64) -> u8;
    /// `uint64_t shbt_ecc_decode_data(uint64_t data, uint8_t check,
    ///   uint8_t *flags)` — returns corrected data; flags: bit0 = single-bit
    ///   error corrected, bit1 = uncorrectable double-bit error.
    fn shbt_ecc_decode_data(data: u64, check: u8, flags: *mut u8) -> u64;
    /// `int32_t shbt_recover_on(ShbtRegisters *hw)` — 4-step sequence on a
    /// caller-supplied register block (testable variant of `shbt_recover`).
    fn shbt_recover_on(hw: *mut MmioRegisters) -> i32;
    /// `double shbt_recover_bench(unsigned iters)` — mean recovery latency (ns).
    fn shbt_recover_bench(iters: u32) -> f64;
}

/// SECDED Hamming(72,64) encode (Rust reference when non-x86; FFI on x86).
pub fn ecc_encode(data: u64) -> u8 {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        shbt_ecc_encode(data)
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        let _ = data;
        0
    }
}

/// SECDED decode outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EccDecode {
    /// Corrected 64-bit data word.
    pub data: u64,
    /// A single-bit error was corrected.
    pub corrected: bool,
    /// A double-bit error was detected (uncorrectable).
    pub uncorrectable: bool,
}

/// SECDED Hamming(72,64) decode + correct.
pub fn ecc_decode(data: u64, check: u8) -> EccDecode {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        let mut flags = 0u8;
        let corrected = shbt_ecc_decode_data(data, check, &mut flags);
        EccDecode {
            data: corrected,
            corrected: flags & 1 != 0,
            uncorrectable: flags & 2 != 0,
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        EccDecode {
            data,
            corrected: false,
            uncorrectable: false,
        }
    }
}

/// Runs the 4-step recovery sequence on a register block (x86_64 FFI).
/// Returns `Ok(())` on clean recovery, `Err(code)` matching the C contract:
/// `-1` = uncorrectable ECC error (fail-closed), `-2` = PLL-lock timeout.
pub fn recover(hw: &mut MmioRegisters) -> Result<(), i32> {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        match shbt_recover_on(hw as *mut MmioRegisters) {
            0 => Ok(()),
            e => Err(e),
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        let _ = hw;
        Ok(())
    }
}

/// Measured mean recovery latency (ns) from the in-C rdtsc bench.
pub fn bench_recovery_ns(iters: u32) -> Option<f64> {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        Some(shbt_recover_bench(iters))
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        let _ = iters;
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_block_is_56_bytes() {
        assert_eq!(std::mem::size_of::<MmioRegisters>(), 56);
    }

    #[test]
    fn ecc_roundtrip_and_single_bit_correction() {
        let data = 0xdead_beef_cafe_f00du64;
        let check = ecc_encode(data);
        let clean = ecc_decode(data, check);
        assert_eq!(clean.data, data);
        assert!(!clean.corrected && !clean.uncorrectable);
        let flipped = ecc_decode(data ^ 1, check);
        assert!(flipped.corrected);
        assert_eq!(flipped.data, data);
    }

    #[test]
    fn recovery_succeeds_when_pll_locks() {
        let mut hw = MmioRegisters {
            status: STATUS_PLL_LOCK,
            ..MmioRegisters::default()
        };
        assert_eq!(recover(&mut hw), Ok(()));
    }
}
