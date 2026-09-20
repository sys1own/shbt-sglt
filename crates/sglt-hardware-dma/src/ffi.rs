//! FFI mirror of `sglt_dma_descriptor_t` / `sglt_dma_ring_control_t`
//! (up2.txt §2). Field offsets are fixed by the hardware contract:
//! offsets 0x00, 0x08, 0x10, 0x14, 0x18, 0x20, 0x28, 0x2C, 0x30.

use core::sync::atomic::AtomicU32;

pub const SGLT_DMA_RING_SIZE: usize = 4096;
pub const SGLT_CACHE_LINE_SIZE: usize = 64;
/// Per-buffer payload, 16 MiB.
pub const SGLT_DMA_BUFFER_SIZE: usize = 16 * 1024 * 1024;

/// 64-byte PCIe DMA descriptor; flags bit 0: 0 = HW owns, 1 = SW owns.
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy)]
pub struct PcieDmaDescriptor {
    pub host_phys_addr: u64,
    pub fpga_local_addr: u64,
    pub buffer_len_bytes: u32,
    pub flags: u32,
    pub frame_sequence: u64,
    pub ptp_timestamp_sec: u64,
    pub ptp_timestamp_nsec: u32,
    pub descriptor_crc32: u32,
    pub _reserved: [u8; 16],
}

impl PcieDmaDescriptor {
    pub const FLAG_SOFTWARE_OWNED: u32 = 1 << 0;

    pub fn is_software_owned(&self) -> bool {
        self.flags & Self::FLAG_SOFTWARE_OWNED != 0
    }

    pub fn hand_to_software(&mut self) {
        self.flags |= Self::FLAG_SOFTWARE_OWNED;
    }

    pub fn release_to_hardware(&mut self) {
        self.flags &= !Self::FLAG_SOFTWARE_OWNED;
    }
}

/// Lock-free SPSC ring control block shared with the FPGA DMA engine.
#[repr(C, align(64))]
pub struct SgltDmaRingControl {
    pub head_index: AtomicU32,
    pub tail_index: AtomicU32,
    pub descriptors: *mut PcieDmaDescriptor,
    pub descriptor_count: u32,
    pub _reserved: [u8; 36],
}

// Shared-memory device structure; raw pointer is not dereferenced by safe code.
unsafe impl Send for SgltDmaRingControl {}
unsafe impl Sync for SgltDmaRingControl {}
