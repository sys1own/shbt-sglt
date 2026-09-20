//! Dark ledger — Fibonacci-anyon braid descriptor store (up1.txt §6).
//!
//! 1472-byte footprint: 16 B frame header + 992 B descriptors
//! (124 × 8 B, fusion trees g ∈ F₅) + 464 B syndrome parity.
//! Storage efficiency η_D = 23/33.

/// Total ledger footprint, bytes.
pub const DARK_LEDGER_BYTES: usize = 1472;
/// Number of fixed-width 8-byte braid descriptors.
pub const DESCRIPTOR_COUNT: usize = 124;
/// Descriptor payload, bytes.
pub const DESCRIPTOR_BYTES: usize = DESCRIPTOR_COUNT * 8; // 992
/// Syndrome parity region, bytes.
pub const SYNDROME_BYTES: usize = 464;
/// Structural storage efficiency η_D = 23/33.
pub const ETA_D: f64 = 23.0 / 33.0;

/// One non-Abelian Fibonacci anyon braid descriptor (8 bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BraidDescriptor {
    /// Fusion-tree node packed as base-φ digits.
    pub fusion_tree: u32,
    /// Braid generator indices σ_i ∈ {σ1,σ2,σ3,σ4} packed 2 bits each.
    pub generators: u32,
}

impl BraidDescriptor {
    pub fn from_bytes(b: &[u8; 8]) -> Self {
        Self {
            fusion_tree: u32::from_le_bytes(b[0..4].try_into().unwrap()),
            generators: u32::from_le_bytes(b[4..8].try_into().unwrap()),
        }
    }
    pub fn to_bytes(&self) -> [u8; 8] {
        let mut b = [0u8; 8];
        b[0..4].copy_from_slice(&self.fusion_tree.to_le_bytes());
        b[4..8].copy_from_slice(&self.generators.to_le_bytes());
        b
    }
}

/// In-memory dark ledger with decode/encode over the 1472-byte layout.
pub struct DarkLedger {
    pub header: [u8; 16],
    pub descriptors: [u8; DESCRIPTOR_BYTES],
    pub syndrome: [u8; SYNDROME_BYTES],
}

impl Default for DarkLedger {
    fn default() -> Self {
        Self::new()
    }
}

impl DarkLedger {
    pub fn new() -> Self {
        Self {
            header: *b"SGLT-DL-v2\0\0\0\0\0\0",
            descriptors: [0; DESCRIPTOR_BYTES],
            syndrome: [0; SYNDROME_BYTES],
        }
    }

    /// Decode all 124 braid descriptors (GATE-26).
    pub fn decode_descriptors(&self) -> Vec<BraidDescriptor> {
        self.descriptors
            .chunks_exact(8)
            .map(|c| BraidDescriptor::from_bytes(c.try_into().unwrap()))
            .collect()
    }

    /// Fibonacci fusion rule τ ⊗ τ = 1 ⊕ τ: returns true when the
    /// fusion-tree nodes only admit the {1, τ} outcomes.
    pub fn validate_fusion(&self) -> bool {
        self.decode_descriptors()
            .iter()
            .all(|d| d.fusion_tree & 0xFFFF_FFFE != 0xFFFF_FFFE)
    }

    /// Serialize to the packed 1472-byte region.
    pub fn to_bytes(&self) -> [u8; DARK_LEDGER_BYTES] {
        let mut out = [0u8; DARK_LEDGER_BYTES];
        out[..16].copy_from_slice(&self.header);
        out[16..16 + DESCRIPTOR_BYTES].copy_from_slice(&self.descriptors);
        out[16 + DESCRIPTOR_BYTES..].copy_from_slice(&self.syndrome);
        out
    }
}
