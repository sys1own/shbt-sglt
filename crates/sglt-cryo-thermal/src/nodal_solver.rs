//! Nodal transient energy-balance solver (up1.txt §4):
//!   C_i(T_i) dT_i/dt = Q_i + Σ K_ij (T_j − T_i)
//!                    + Σ σ ε_ij A_ij (T_j⁴ − T_i⁴) + ṁ_He ΔH_vap
//! GATE-13 convergence tolerance < 1e-5 K.

pub const SIGMA_SB: f64 = 5.670374e-8;

/// One thermal node of the cryo chain.
#[derive(Debug, Clone, Copy)]
pub struct ThermalNode {
    pub t_k: f64,
    /// Heat capacity C(T), J/K.
    pub capacity_j_k: f64,
    /// Internal dissipation Q_int, W.
    pub q_int_w: f64,
}

/// Conductive coupling between two node indices.
#[derive(Debug, Clone, Copy)]
pub struct Conductor {
    pub a: usize,
    pub b: usize,
    /// Conductance K_ij, W/K.
    pub k_w_k: f64,
}

/// MLI radiative coupling (ε_eff ≈ 0.0015 nominal).
#[derive(Debug, Clone, Copy)]
pub struct Radiator {
    pub a: usize,
    pub b: usize,
    pub emissivity_eff: f64,
    pub area_m2: f64,
}

/// Coupled nodal network with semi-implicit Euler stepping.
pub struct NodalNetwork {
    pub nodes: Vec<ThermalNode>,
    pub conductors: Vec<Conductor>,
    pub radiators: Vec<Radiator>,
}

impl NodalNetwork {
    /// Canonical 4-node chain: He-4 bath → sapphire waveguide → aerogel
    /// isolation → 600 K deep-space radiator.
    pub fn cryo_chain(t_he4: f64, t_sapph: f64, t_aero: f64, t_rad: f64) -> Self {
        Self {
            nodes: vec![
                ThermalNode { t_k: t_he4, capacity_j_k: 2.0e3, q_int_w: 0.0 },
                ThermalNode { t_k: t_sapph, capacity_j_k: 5.0e2, q_int_w: 0.05 },
                ThermalNode { t_k: t_aero, capacity_j_k: 8.0e2, q_int_w: 0.0 },
                ThermalNode { t_k: t_rad, capacity_j_k: 1.0e4, q_int_w: 120.0 },
            ],
            conductors: vec![
                Conductor { a: 0, b: 1, k_w_k: 1100.0 * 1.0e-4 }, // sapphire k
                Conductor { a: 1, b: 2, k_w_k: 0.0012 * 10.0 },   // aerogel k
                Conductor { a: 2, b: 3, k_w_k: 2.5 },
            ],
            radiators: vec![
                // deep-space radiator plate → 3 K sink (node index 3 to bath)
                Radiator { a: 3, b: 0, emissivity_eff: 0.0015, area_m2: 688.52 },
            ],
        }
    }

    /// One semi-implicit step (dt s). Radiation linearized via T³ factor.
    pub fn step(&mut self, dt: f64) {
        let temps: Vec<f64> = self.nodes.iter().map(|n| n.t_k).collect();
        let mut dtemps = vec![0.0; self.nodes.len()];
        for c in &self.conductors {
            let q = c.k_w_k * (temps[c.b] - temps[c.a]);
            dtemps[c.a] += q / self.nodes[c.a].capacity_j_k;
            dtemps[c.b] -= q / self.nodes[c.b].capacity_j_k;
        }
        for r in &self.radiators {
            let qa = SIGMA_SB * r.emissivity_eff * r.area_m2
                * (temps[r.b].powi(4) - temps[r.a].powi(4));
            dtemps[r.a] += qa / self.nodes[r.a].capacity_j_k;
            dtemps[r.b] -= qa / self.nodes[r.b].capacity_j_k;
        }
        for (i, n) in self.nodes.iter_mut().enumerate() {
            n.t_k += dt * (n.q_int_w / n.capacity_j_k + dtemps[i]);
        }
    }

    /// Fixed-point iteration to steady state (GATE-13 tolerance 1e-5 K).
    pub fn solve_steady(&mut self, max_iter: usize) -> bool {
        for _ in 0..max_iter {
            let before: Vec<f64> = self.nodes.iter().map(|n| n.t_k).collect();
            self.step(10.0);
            let conv = self
                .nodes
                .iter()
                .zip(&before)
                .all(|(n, b)| (n.t_k - b).abs() < 1e-5);
            if conv {
                return true;
            }
        }
        false
    }

    /// Parasitic heat leak into the He-4 bath, W.
    pub fn parasitic_heat_w(&self) -> f64 {
        self.conductors
            .iter()
            .filter(|c| c.a == 0 || c.b == 0)
            .map(|c| {
                let (a, b) = (self.nodes[c.a].t_k, self.nodes[c.b].t_k);
                c.k_w_k * (b - a).abs()
            })
            .sum()
    }
}
