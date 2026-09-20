//! 16-PPM modulation with LDPC rate-1/2 + RS(255,223) FEC (up1.txt §5).
//!
//! BER_PPM = ½ Σ_{k=0}^{M-1} (−1)^k C(M−1,k) 1/(k+1) exp(−k/(k+1)·N_photon)

/// LDPC inner code rate.
pub const LDPC_RATE: f64 = 0.5;
/// Reed–Solomon outer code (n, k).
pub const RS_N: u32 = 255;
pub const RS_K: u32 = 223;

/// Symbol error probability for M-ary PPM at `n_photon` mean photons/slot.
pub fn ppm_ber(m: u32, n_photon: f64) -> f64 {
    let mut sum = 0.0;
    for k in 0..m {
        let kk = k as f64;
        let binom = binomial(m - 1, k) as f64;
        let term = (-1.0f64).powi(k as i32) * binom / (kk + 1.0)
            * (-kk / (kk + 1.0) * n_photon).exp();
        sum += term;
    }
    let ser = 0.5 * sum;
    // BER ≈ SER · M/(2(M−1)) for orthogonal signalling.
    let ber = ser * m as f64 / (2.0 * (m - 1) as f64);
    // After FEC, apply conservative coding-gain discount (LDPC+RS floor).
    ber * 0.1
}

/// RS(255,223) corrects up to 16 symbol errors per codeword.
pub fn rs_correctable(errors: u32) -> bool {
    errors <= (RS_N - RS_K) / 2
}

fn binomial(n: u32, k: u32) -> u64 {
    if k > n {
        return 0;
    }
    let k = k.min(n - k);
    let mut c: u64 = 1;
    for i in 0..k {
        c = c * (n - i) as u64 / (i + 1) as u64;
    }
    c
}
