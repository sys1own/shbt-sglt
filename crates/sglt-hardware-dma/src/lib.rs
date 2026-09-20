//! sglt-hardware-dma — PCIe Gen5 x16 zero-copy DMA ring and SDR bias/quench
//! control bridge (up2.txt §2).

pub mod ffi;
pub mod ring;

pub use ffi::*;
pub use ring::*;

use std::sync::atomic::AtomicU32;

/// Host-side ring control block wrapping `SgltDmaRingControl` over a boxed
/// descriptor array (simulated shared-memory transport for the HIL bench).
pub struct DmaRing {
    pub control: SgltDmaRingControl,
    descriptors: Vec<PcieDmaDescriptor>,
}

impl DmaRing {
    pub fn new() -> Self {
        let mut descriptors = vec![
            PcieDmaDescriptor {
                host_phys_addr: 0,
                fpga_local_addr: 0,
                buffer_len_bytes: 0,
                flags: 0,
                frame_sequence: 0,
                ptp_timestamp_sec: 0,
                ptp_timestamp_nsec: 0,
                descriptor_crc32: 0,
                _reserved: [0; 16],
            };
            SGLT_DMA_RING_SIZE
        ];
        ring::init_ring(&mut descriptors);
        let control = SgltDmaRingControl {
            head_index: AtomicU32::new(0),
            tail_index: AtomicU32::new(0),
            descriptors: descriptors.as_mut_ptr(),
            descriptor_count: SGLT_DMA_RING_SIZE as u32,
            _reserved: [0; 36],
        };
        Self {
            control,
            descriptors,
        }
    }

    /// Simulate one FPGA RX interrupt: write a frame into descriptor[head]
    /// and advance head. Returns filled-descriptor count after the write.
    pub fn simulate_rx_interrupt(&mut self, buffer_len: u32, frame_seq: u64) -> u32 {
        let head = self
            .control
            .head_index
            .load(core::sync::atomic::Ordering::Acquire);
        let idx = head as usize % self.descriptors.len();
        let sec = 1_711_929_600u64;
        let nsec = ((frame_seq % 200_000_000) * FPGA_TIMESTAMP_RESOLUTION_NS as u64) as u32;
        ring::hand_descriptor_to_software(
            &mut self.descriptors[idx],
            buffer_len,
            frame_seq,
            sec,
            nsec,
        );
        ring::advance_head(&self.control);
        ring::occupancy(&self.control)
    }

    /// Consume up to `max` software-owned descriptors; returns count consumed.
    pub fn drain(&mut self, max: usize) -> usize {
        let mut done = 0;
        while done < max && ring::occupancy(&self.control) > 0 {
            let tail = self
                .control
                .tail_index
                .load(core::sync::atomic::Ordering::Acquire);
            let idx = tail as usize % self.descriptors.len();
            ring::consume_descriptor(&mut self.descriptors[idx]);
            ring::advance_tail(&self.control);
            done += 1;
        }
        done
    }
}

impl Default for DmaRing {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interrupt_and_drain_roundtrip() {
        let mut r = DmaRing::new();
        assert_eq!(r.simulate_rx_interrupt(4096, 1), 1);
        assert_eq!(r.simulate_rx_interrupt(4096, 2), 2);
        assert_eq!(r.drain(8), 2);
        assert_eq!(ring::occupancy(&r.control), 0);
    }

    #[test]
    fn descriptor_is_64b_aligned() {
        assert_eq!(core::mem::size_of::<PcieDmaDescriptor>(), 64);
        assert_eq!(core::mem::align_of::<PcieDmaDescriptor>(), 64);
    }
}
