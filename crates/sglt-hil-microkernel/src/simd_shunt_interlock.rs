//! AVX-512 SIMD current-shunt interlock check.
//!
//! Transferred from `sys1own/shbt-qc` (`kernel/src/avx512_interlock.c`,
//! `int shbt_simd_shunt_check(const float*)`): the instruction chain
//! `vmovaps → vcmpps → vmovmskps → mov` inspects 16 shunt current channels in
//! a single pass, bounded below the 2.500 ns latency limit (Gate-14 verified
//! baseline 1.412 ns).

/// Number of shunt current channels inspected per pass.
pub const SHUNT_CHANNELS: usize = 16;
/// Interlock trip threshold (A).
pub const SHUNT_TRIP_A: f32 = 7.5;

#[cfg(target_arch = "x86_64")]
extern "C" {
    /// C ABI: `int shbt_simd_shunt_check(const float *currents_a)` — returns a
    /// 16-bit trip mask (bit i set ⇒ channel i over threshold).
    fn shbt_simd_shunt_check(currents: *const f32) -> i32;
    /// C ABI: `double shbt_simd_shunt_bench(unsigned iters)` — rdtsc-timed
    /// mean single-call latency in ns on the host TSC.
    fn shbt_simd_shunt_bench(iters: u32) -> f64;
}

/// Scalar fallback on non-x86_64 targets.
#[cfg(not(target_arch = "x86_64"))]
fn scalar_check(currents: &[f32; SHUNT_CHANNELS]) -> u32 {
    let mut mask = 0u32;
    for (i, &c) in currents.iter().enumerate() {
        if c > SHUNT_TRIP_A {
            mask |= 1 << i;
        }
    }
    mask
}

/// Runs the AVX-512 shunt check over 16 current channels (A); returns the
/// trip mask.
pub fn simd_shunt_check(currents: &[f32; SHUNT_CHANNELS]) -> u32 {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        shbt_simd_shunt_check(currents.as_ptr()) as u32
    }
    #[cfg(not(target_arch = "x86_64"))]
    scalar_check(currents)
}

/// True when no channel exceeds the trip threshold.
pub fn shunt_interlock_ok(currents: &[f32; SHUNT_CHANNELS]) -> bool {
    simd_shunt_check(currents) == 0
}

/// Measured single-call latency (ns) from the in-C rdtsc bench (x86_64 only;
/// returns `None` elsewhere).
pub fn bench_latency_ns(iters: u32) -> Option<f64> {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        Some(shbt_simd_shunt_bench(iters))
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
    fn clean_currents_do_not_trip() {
        assert_eq!(simd_shunt_check(&[1.0; SHUNT_CHANNELS]), 0);
        assert!(shunt_interlock_ok(&[1.0; SHUNT_CHANNELS]));
    }

    #[test]
    fn overcurrent_channels_trip() {
        let mut c = [0.0; SHUNT_CHANNELS];
        c[3] = 9.0;
        c[15] = 12.0;
        let mask = simd_shunt_check(&c);
        assert_eq!(mask, (1 << 3) | (1 << 15));
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn interlock_latency_under_2500_ps() {
        if let Some(ns) = bench_latency_ns(100_000) {
            assert!(ns < 2.5, "shunt latency {ns} ns exceeds 2.5 ns budget");
        }
    }
}
