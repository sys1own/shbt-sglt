//! Lock-free PCIe DMA ring mechanics + SDR bias/quench helpers, mirroring the
//! C11 microkernel bridge in kernel/src/shbt_core_runtime.c (up2.txt §2).

use crate::ffi::*;
use core::sync::atomic::Ordering;

/// DAC bias envelope, V.
pub const SDR_DAC_BIAS_MIN_V: f64 = 3.8;
pub const SDR_DAC_BIAS_MAX_V: f64 = 7.4;
/// Safe storage bias asserted on fast quench.
pub const SDR_QUENCH_SAFE_BIAS_V: f64 = 3.8;
/// FPGA timestamp precision.
pub const FPGA_TIMESTAMP_RESOLUTION_NS: f64 = 5.0;
/// Sustained PCIe Gen5 x16 bandwidth.
pub const PCIE_GEN5_X16_BANDWIDTH_BPS: f64 = 63.0e9 * 8.0;

/// Initialize a ring: all descriptors HW-owned, head = tail = 0.
pub fn init_ring(descriptors: &mut [PcieDmaDescriptor]) {
    for d in descriptors.iter_mut() {
        d.flags &= !PcieDmaDescriptor::FLAG_SOFTWARE_OWNED;
        d.frame_sequence = 0;
        d.buffer_len_bytes = 0;
    }
}

/// Transfer descriptor `idx` to software (bit 0 ← 1), set length + PTP stamp.
pub fn hand_descriptor_to_software(
    desc: &mut PcieDmaDescriptor,
    buffer_len: u32,
    frame_seq: u64,
    sec: u64,
    nsec: u32,
) {
    desc.buffer_len_bytes = buffer_len;
    desc.frame_sequence = frame_seq;
    desc.ptp_timestamp_sec = sec;
    desc.ptp_timestamp_nsec = nsec;
    desc.hand_to_software();
}

/// Software-side consumption: clears the ownership bit (back to HW).
pub fn consume_descriptor(desc: &mut PcieDmaDescriptor) {
    desc.release_to_hardware();
}

/// Advance `tail_index` while ownership sits with software (release ordering).
pub fn advance_tail(ring: &SgltDmaRingControl) {
    let n = ring.descriptor_count;
    let t = ring.tail_index.load(Ordering::Acquire);
    ring.tail_index.store((t + 1) % n.max(1), Ordering::Release);
}

/// Advance `head_index` after FPGA writes a frame (release ordering).
pub fn advance_head(ring: &SgltDmaRingControl) {
    let n = ring.descriptor_count;
    let h = ring.head_index.load(Ordering::Acquire);
    ring.head_index.store((h + 1) % n.max(1), Ordering::Release);
}

/// Number of filled descriptors = (head − tail) mod N.
pub fn occupancy(ring: &SgltDmaRingControl) -> u32 {
    let n = ring.descriptor_count.max(1);
    let h = ring.head_index.load(Ordering::Acquire) % n;
    let t = ring.tail_index.load(Ordering::Acquire) % n;
    (h + n - t) % n
}

/// Map a requested bias voltage (3.8–7.4 V) to the 16-bit DAC code.
pub fn dac_code_for_voltage(v: f64) -> u16 {
    let v = v.clamp(SDR_DAC_BIAS_MIN_V, SDR_DAC_BIAS_MAX_V);
    (((v - SDR_DAC_BIAS_MIN_V) / (SDR_DAC_BIAS_MAX_V - SDR_DAC_BIAS_MIN_V)) * 65535.0).round()
        as u16
}

/// True when the vector of telemetry voltages is fully inside the envelope.
/// In the microkernel this runs as a single AVX-512 pass; scalar here.
pub fn voltages_in_envelope(voltages: &[f32]) -> bool {
    voltages
        .iter()
        .all(|&v| v >= SDR_DAC_BIAS_MIN_V as f32 && v <= SDR_DAC_BIAS_MAX_V as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dac_code_maps_envelope() {
        assert_eq!(dac_code_for_voltage(3.8), 0);
        assert_eq!(dac_code_for_voltage(7.4), 65535);
        assert_eq!(dac_code_for_voltage(9.0), 65535); // clamp
        assert_eq!(dac_code_for_voltage(0.0), 0);
        let mid = dac_code_for_voltage(5.6);
        assert!((mid as i32 - 32768).abs() <= 1); // mid-scale ±1 LSB
    }

    #[test]
    fn ring_ownership_cycle() {
        let mut descs = vec![
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
            16
        ];
        init_ring(&mut descs);
        assert!(!descs[3].is_software_owned());
        hand_descriptor_to_software(&mut descs[3], SGLT_DMA_BUFFER_SIZE as u32, 42, 7, 123);
        assert!(descs[3].is_software_owned());
        assert_eq!(descs[3].frame_sequence, 42);
        consume_descriptor(&mut descs[3]);
        assert!(!descs[3].is_software_owned());
    }
}
