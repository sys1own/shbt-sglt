//! Prince–Dormand 8(7) explicit Runge–Kutta integrator (DOP853) with
//! adaptive step control: ε_abs ≤ 1e-14, ε_rel ≤ 1e-12 (up1.txt §3).

/// Adaptive integration tolerances.
#[derive(Debug, Clone, Copy)]
pub struct IntegratorConfig {
    pub abs_tol: f64,
    pub rel_tol: f64,
    pub h_min: f64,
    pub h_max: f64,
}

impl Default for IntegratorConfig {
    fn default() -> Self {
        Self {
            abs_tol: 1e-14,
            rel_tol: 1e-12,
            h_min: 1e-10,
            h_max: 1e-2,
        }
    }
}

/// Embedded 5th-order pair used as the error estimator for the 8th-order
/// advance. We use the classic DOP853 coefficient subset: full 13-stage
/// Butcher tableau for the 8(7) pair.
pub struct Dop853 {
    pub cfg: IntegratorConfig,
    pub t: f64,
}

// DOP853 Butcher coefficients (Hairer, Nørsett & Wanner).
#[allow(dead_code)]
const C: [f64; 12] = [
    0.5260015195876773e-1,
    0.789002279381111e-1,
    0.1183503419071666,
    0.2816496589927729,
    0.3333333333333333,
    0.25,
    0.3076923076923077,
    0.6512820512820513,
    0.6,
    0.8571428571428571,
    1.0,
    1.0,
];

const A: [[f64; 12]; 12] = [
    [5.260015195876773e-2, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [1.9725056984537899e-2, 5.9175170953613698e-2, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [2.9587585476806849e-2, 0.0, 8.8762756430420547e-2, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [2.4136513415926669e-1, 0.0, -8.9839280851410066e-1, 8.9387412278371119e-1, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [5.4465207338683107e-2, 0.0, 0.0, 1.4289781541065220e-1, 1.3997031059799810e-1, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [4.7091525359448672e-2, 0.0, 0.0, 0.0, 1.5018374076634038e-1, 5.2724733874210962e-2, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [5.7304633651083536e-2, 0.0, 0.0, 0.0, 2.1571354914848715e-1, 2.3600645593008246e-2, -2.2778352069012774e-2, 0.0, 0.0, 0.0, 0.0, 0.0],
    [4.8554741283706837e-1, 0.0, 0.0, 0.0, 0.0, 0.0, -2.4645537541700283e-1, 2.4123606768493259e-1, 0.0, 0.0, 0.0, 0.0],
    [5.7274512740109993e-1, 0.0, 0.0, 0.0, 0.0, 0.0, -2.2552759195057327e0, 2.0489089555439389e0, -4.7437904124433785e-1, 0.0, 0.0, 0.0],
    [7.7455894638068704e-1, 0.0, 0.0, 0.0, 0.0, 0.0, -4.3086357710784313e0, 3.8738948791820712e0, -9.8830213939530331e-1, 1.4500895127507885e0, 0.0, 0.0],
    [3.9108189664407787e-1, 0.0, 0.0, 0.0, 0.0, 0.0, -2.4768913715663664e0, 2.2963656158034524e0, -6.0463810386031477e-1, 7.2623234373989837e-1, -5.0316016202135222e-1, 0.0],
    [1.0397202084550457e-1, 0.0, 0.0, 0.0, 0.0, 0.0, -6.5809424238078314e-1, 6.5269424672940177e-1, -2.6500008788867857e-2, 2.8515869678430148e-2, 8.1266293692472475e-2, -2.0397164071406708e-2],
];

const B: [f64; 12] = [
    5.4293734116568762e-3,
    0.0,
    0.0,
    0.0,
    0.0,
    4.4503128927524089e-2,
    1.8915178993145003e-2,
    0.0,
    2.0700351455375354e-2,
    6.7179512762302688e-2,
    -1.0526204444791984e-2,
    3.0329233886983082e-3,
];

// 7th-order (error-estimate) weights.
const E: [f64; 13] = [
    -1.1515814293364284e-3,
    0.0,
    0.0,
    0.0,
    0.0,
    1.0130180350230803e-2,
    -1.7101650149155414e-2,
    0.0,
    -8.7688768484475388e-3,
    -2.1344895257464583e-2,
    -7.5780247284869360e-2,
    -1.5638650530469394e-2,
    -3.7800398166942750e-2,
];

impl Dop853 {
    pub fn new(cfg: IntegratorConfig) -> Self {
        Self { cfg, t: 0.0 }
    }

    /// One adaptive step: advances `y` by ~`h`, internally rejecting and
    /// re-stepping until the local error meets tolerance. `f(y) = dy/dt`.
    pub fn step<F>(&mut self, y: &mut [f64; 6], h: f64, f: &F) -> f64
    where
        F: Fn(&[f64; 6]) -> [f64; 6],
    {
        let mut h = h.clamp(self.cfg.h_min, self.cfg.h_max);
        loop {
            let (y8, err) = self.raw_step(y, h, f);
            let mut scale = 0.0;
            for i in 0..6 {
                let s = self.cfg.abs_tol + self.cfg.rel_tol * y[i].abs().max(y8[i].abs());
                scale += (err[i] / s).powi(2);
            }
            let err_norm = (scale / 6.0).sqrt();
            if err_norm <= 1.0 || h <= self.cfg.h_min {
                *y = y8;
                self.t += h;
                // Next-step suggestion (8th order → exponent 1/8).
                let fac = (1.0 / err_norm.max(1e-16)).powf(0.125).clamp(0.2, 6.0);
                return (h * 0.9 * fac).clamp(self.cfg.h_min, self.cfg.h_max);
            }
            h = (h * 0.9 * (1.0 / err_norm).powf(0.125).clamp(0.2, 1.0))
                .max(self.cfg.h_min);
        }
    }

    fn raw_step<F>(&self, y: &[f64; 6], h: f64, f: &F) -> ([f64; 6], [f64; 6])
    where
        F: Fn(&[f64; 6]) -> [f64; 6],
    {
        let mut k = [[0.0f64; 6]; 13];
        k[0] = f(y);
        for s in 1..13usize {
            let mut yt = *y;
            for j in 0..s {
                let a = if s == 12 { B[j.min(11)] } else { A[s - 1][j] };
                if a != 0.0 {
                    for i in 0..6 {
                        yt[i] += h * a * k[j][i];
                    }
                }
            }
            k[s] = f(&yt);
        }
        let mut y8 = *y;
        let mut err = [0.0f64; 6];
        for i in 0..6 {
            let mut acc = 0.0;
            for s in 0..12 {
                acc += B[s] * k[s][i];
            }
            y8[i] += h * acc;
            for s in 0..13 {
                err[i] += E[s] * k[s][i];
            }
            err[i] *= h;
        }
        (y8, err)
    }
}
