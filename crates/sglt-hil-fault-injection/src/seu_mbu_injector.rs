//! Poisson SEU/MBU mutation engine for `.stinespring_frame` (up1.txt §6):
//!   P(k SEUs in Δt) = (λ Δt)^k e^(−λΔt) / k!
//!   MBU mask corrupts M contiguous bits: P(M=m) = (1−p)^{m−1} p.

/// `.stinespring_frame` SRAM extent, bytes (1472 B dark ledger + 640 B
/// POSIX overlay).
pub const STINESPRING_FRAME_BYTES: usize = 2112;
/// Dark ledger region offset within the frame.
pub const DARK_LEDGER_OFFSET: usize = 16;
/// Dark ledger extent, bytes.
pub const DARK_LEDGER_BYTES: usize = 1472;

/// Tiny xorshift PRNG for reproducible fault sequences.
pub struct XorShift(pub u64);
impl XorShift {
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    pub fn uniform(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    /// Poisson(k | λΔt) count of SEUs in Δt.
    pub fn poisson_events(&mut self, lambda_dt: f64) -> u32 {
        let l = (-lambda_dt).exp();
        let mut k = 0u32;
        let mut p = 1.0;
        loop {
            p *= self.uniform().max(1e-300);
            if p <= l {
                return k;
            }
            k += 1;
        }
    }
}

/// Spatial-temporal Poisson SEU/MBU injector.
pub struct FaultInjector {
    /// SEU rate λ (events/s).
    pub seu_rate_s: f64,
    /// MBU run-length termination probability p_mbu.
    pub p_mbu: f64,
    pub rng: XorShift,
}

impl FaultInjector {
    pub fn new(seu_rate_s: f64, seed: u64) -> Self {
        Self {
            seu_rate_s,
            p_mbu: 0.4,
            rng: XorShift(seed.max(1)),
        }
    }

    /// Inject SEU/MBU events into `frame` over interval `dt_s`; returns the
    /// number of bits flipped.
    pub fn inject(&mut self, frame: &mut [u8; STINESPRING_FRAME_BYTES], dt_s: f64) -> u32 {
        let events = self.rng.poisson_events(self.seu_rate_s * dt_s);
        let total_bits = STINESPRING_FRAME_BYTES * 8;
        let mut flipped = 0u32;
        for _ in 0..events {
            let start = (self.rng.uniform() * total_bits as f64) as usize;
            // Geometric MBU run length M ≥ 1.
            let mut m = 1u32;
            while self.rng.uniform() >= self.p_mbu {
                m += 1;
            }
            for b in 0..m as usize {
                let bit = (start + b) % total_bits;
                frame[bit / 8] ^= 1 << (bit % 8);
                flipped += 1;
            }
        }
        flipped
    }
}
