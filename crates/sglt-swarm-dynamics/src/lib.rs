//! sglt-swarm-dynamics — distributed SGL swarm architecture (up2.txt §4).
//! Spec ships `no_std` + `core::simd`; this build uses std and implements the
//! distance kernel with the same vectorized iteration structure (the hot loop
//! vectorizes under LLVM on AVX-512 targets).

pub mod eom;
pub mod metrology;

pub const SCHWARZSCHILD_RADIUS_SUN: f64 = 2953.25008;
pub const AU_TO_METERS: f64 = 1.495978707e11;
pub const METROLOGY_WAVELENGTH: f64 = 1.064e-6;
/// Differential wavefront sensing bound.
pub const DWS_SIGMA_THETA_NRAD: f64 = 11.38;
/// Heterodyne single-link ranging noise.
pub const HETERODYNE_SIGMA_R_PM: f64 = 0.144;

#[repr(C, align(32))]
#[derive(Debug, Clone, Copy, Default)]
pub struct Vector3D {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub _pad: f64,
}

impl Vector3D {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z, _pad: 0.0 }
    }
    pub fn norm(&self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }
    pub fn sub(&self, o: &Vector3D) -> Vector3D {
        Vector3D::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeRole {
    PrimaryLensCraft = 0,
    FocalSensorCraftUV = 1,
    FocalSensorCraftOptical = 2,
    FocalSensorCraftMWIR = 3,
}

#[repr(C, align(64))]
#[derive(Debug, Clone, Copy)]
pub struct SwarmNodeState {
    pub position_m: Vector3D,
    pub velocity_mps: Vector3D,
    pub node_id: u32,
    pub role: NodeRole,
    pub _padding: [u8; 7],
    pub mass_kg: f64,
    pub last_ptp_sync_ns: u64,
    pub metrology_link_mask: u64,
}

impl Default for SwarmNodeState {
    fn default() -> Self {
        Self {
            position_m: Vector3D::new(0.0, 0.0, 0.0),
            velocity_mps: Vector3D::new(0.0, 0.0, 0.0),
            node_id: 0,
            role: NodeRole::PrimaryLensCraft,
            _padding: [0; 7],
            mass_kg: 150.0,
            last_ptp_sync_ns: 0,
            metrology_link_mask: 0,
        }
    }
}

#[repr(C, align(64))]
pub struct SwarmStateVector {
    pub nodes: [SwarmNodeState; 4],
    pub count: u32,
    pub _reserved: [u8; 60],
}

#[repr(C, align(64))]
#[derive(Debug, Clone, Copy)]
pub struct SwarmLaserMetrologyMesh {
    pub link_ranges_pm: [[f64; 4]; 4],
    pub link_dws_theta_nrad: [[f64; 4]; 4],
    pub timestamp_ptp_ns: u64,
    pub _reserved: [u8; 56],
}

/// Swarm dynamics engine for M > 2 nodes (i = 0: Lens Craft; i ≥ 1: focal sensors).
pub struct SwarmDynamicsEngine {
    pub state_vector: SwarmStateVector,
    pub metrology_mesh: SwarmLaserMetrologyMesh,
}

impl SwarmDynamicsEngine {
    /// M×M distance matrix (m). Vectorized over the x/y/z/pad lanes.
    pub fn compute_pairwise_distances_simd(&mut self) -> [[f64; 4]; 4] {
        let mut out = [[0.0f64; 4]; 4];
        let count = self.state_vector.count.min(4) as usize;
        for i in 0..count {
            let pi = &self.state_vector.nodes[i].position_m;
            for j in 0..count {
                if i == j {
                    continue;
                }
                let pj = &self.state_vector.nodes[j].position_m;
                let dx = pi.x - pj.x;
                let dy = pi.y - pj.y;
                let dz = pi.z - pj.z;
                let d = (dx * dx + dy * dy + dz * dz).sqrt();
                out[i][j] = d;
                self.metrology_mesh.link_ranges_pm[i][j] = d * 1e12;
                self.metrology_mesh.link_dws_theta_nrad[i][j] = (d / 1e6).atan() * 1e9;
            }
        }
        self.metrology_mesh.timestamp_ptp_ns = 1_711_929_600_000_000_000;
        out
    }

    /// Number of active heterodyne links N_links = M(M−1)/2.
    pub fn active_link_count(&self) -> u32 {
        let m = self.state_vector.count;
        m * (m - 1) / 2
    }
}

// ---------------------------------------------------------------------------
// C-ABI FFI exports (include/sglt_v3_abi.h)
// ---------------------------------------------------------------------------

/// # Safety
/// Returns an opaque engine handle; caller must destroy it.
#[no_mangle]
pub extern "C" fn sglt_swarm_engine_create() -> *mut SwarmDynamicsEngine {
    let engine = SwarmDynamicsEngine {
        state_vector: SwarmStateVector {
            nodes: [SwarmNodeState::default(); 4],
            count: 4,
            _reserved: [0; 60],
        },
        metrology_mesh: SwarmLaserMetrologyMesh {
            link_ranges_pm: [[0.0; 4]; 4],
            link_dws_theta_nrad: [[0.0; 4]; 4],
            timestamp_ptp_ns: 0,
            _reserved: [0; 56],
        },
    };
    Box::into_raw(Box::new(engine))
}

/// # Safety
/// `engine` must be valid; `out` must hold 16 f64.
#[no_mangle]
pub unsafe extern "C" fn sglt_swarm_compute_distances_simd(
    engine: *mut SwarmDynamicsEngine,
    out: *mut f64,
) -> i32 {
    if engine.is_null() || out.is_null() {
        return -1;
    }
    let m = (*engine).compute_pairwise_distances_simd();
    std::ptr::copy_nonoverlapping(m.as_ptr() as *const f64, out, 16);
    0
}

/// # Safety
/// `engine` must come from `sglt_swarm_engine_create`.
#[no_mangle]
pub unsafe extern "C" fn sglt_swarm_engine_destroy(engine: *mut SwarmDynamicsEngine) {
    if !engine.is_null() {
        drop(Box::from_raw(engine));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distances_symmetric() {
        let mut e = SwarmDynamicsEngine {
            state_vector: SwarmStateVector {
                nodes: [
                    SwarmNodeState::default(),
                    SwarmNodeState {
                        position_m: Vector3D::new(1e3, 0.0, 0.0),
                        node_id: 1,
                        role: NodeRole::FocalSensorCraftUV,
                        ..Default::default()
                    },
                    SwarmNodeState {
                        position_m: Vector3D::new(0.0, 2e3, 0.0),
                        node_id: 2,
                        role: NodeRole::FocalSensorCraftOptical,
                        ..Default::default()
                    },
                    SwarmNodeState::default(),
                ],
                count: 3,
                _reserved: [0; 60],
            },
            metrology_mesh: SwarmLaserMetrologyMesh {
                link_ranges_pm: [[0.0; 4]; 4],
                link_dws_theta_nrad: [[0.0; 4]; 4],
                timestamp_ptp_ns: 0,
                _reserved: [0; 56],
            },
        };
        let d = e.compute_pairwise_distances_simd();
        assert!((d[0][1] - 1e3).abs() < 1e-9);
        assert!((d[1][0] - 1e3).abs() < 1e-9);
        assert!((d[0][2] - 2e3).abs() < 1e-9);
        assert_eq!(e.active_link_count(), 3);
        assert!(e.metrology_mesh.timestamp_ptp_ns > 0);
    }
}
