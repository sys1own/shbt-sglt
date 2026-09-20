/*
 * shbt_hardware.h - SHBT-MMIO-1 normative memory-mapped hardware interface.
 *
 * Freestanding: depends only on compiler-provided <stdint.h> and <stddef.h>.
 *
 * The MMIO aperture for the SHBT-R quantum computer control plane is a
 * compact, 56-byte volatile register block at 0x70000000.  The ABI version
 * register reports interface revision 1.
 */
#ifndef SHBT_HARDWARE_H
#define SHBT_HARDWARE_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* --------------------------------------------------------------------------
 * Aperture and ABI version
 * -------------------------------------------------------------------------- */
#define SHBT_MMIO_BASE              0x70000000U
#define SHBT_MMIO_ABI_VERSION       1U

/* --------------------------------------------------------------------------
 * status register bitmasks (SHBT-MMIO-1 §3.1)
 * -------------------------------------------------------------------------- */
#define SHBT_STATUS_OVERTEMP        (1U << 0)
#define SHBT_STATUS_PLL_LOCK        (1U << 1)
#define SHBT_STATUS_ECC_ERR         (1U << 2)
#define SHBT_STATUS_FAULT_ST        (1U << 3)

/* --------------------------------------------------------------------------
 * Normative MMIO register block
 *
 * Offsets 0x00 .. 0x34 inclusive, 14 x uint32_t fields, total span 0x38 bytes.
 * -------------------------------------------------------------------------- */
typedef struct {
    volatile uint32_t status;           /* 0x00 */
    volatile uint32_t blank;            /* 0x04 */
    volatile uint32_t fifo_data;        /* 0x08 */
    volatile uint32_t pll_ctrl;         /* 0x0C */
    volatile uint32_t ecc_low;          /* 0x10 */
    volatile uint32_t ecc_high;         /* 0x14 */
    volatile uint32_t ecc_check;        /* 0x18 */
    volatile uint32_t ecc_commit;       /* 0x1C */
    volatile uint32_t fault_latch;      /* 0x20 */
    volatile uint32_t channel_select;   /* 0x24 */
    volatile uint32_t abi_version;      /* 0x28 */
    volatile uint32_t phase_offset;     /* 0x2C */
    volatile uint32_t ecc_counts;       /* 0x30 */
    volatile uint32_t control;          /* 0x34 */
} ShbtRegisters;                          /* 0x38 */

#define SHBT_MMIO   ((ShbtRegisters *)SHBT_MMIO_BASE)

/* --------------------------------------------------------------------------
 * Compile-time layout verification
 * -------------------------------------------------------------------------- */
_Static_assert(offsetof(ShbtRegisters, status)         == 0x00U, "ShbtRegisters.status offset");
_Static_assert(offsetof(ShbtRegisters, blank)          == 0x04U, "ShbtRegisters.blank offset");
_Static_assert(offsetof(ShbtRegisters, fifo_data)      == 0x08U, "ShbtRegisters.fifo_data offset");
_Static_assert(offsetof(ShbtRegisters, pll_ctrl)       == 0x0CU, "ShbtRegisters.pll_ctrl offset");
_Static_assert(offsetof(ShbtRegisters, ecc_low)        == 0x10U, "ShbtRegisters.ecc_low offset");
_Static_assert(offsetof(ShbtRegisters, ecc_high)       == 0x14U, "ShbtRegisters.ecc_high offset");
_Static_assert(offsetof(ShbtRegisters, ecc_check)      == 0x18U, "ShbtRegisters.ecc_check offset");
_Static_assert(offsetof(ShbtRegisters, ecc_commit)     == 0x1CU, "ShbtRegisters.ecc_commit offset");
_Static_assert(offsetof(ShbtRegisters, fault_latch)    == 0x20U, "ShbtRegisters.fault_latch offset");
_Static_assert(offsetof(ShbtRegisters, channel_select) == 0x24U, "ShbtRegisters.channel_select offset");
_Static_assert(offsetof(ShbtRegisters, abi_version)    == 0x28U, "ShbtRegisters.abi_version offset");
_Static_assert(offsetof(ShbtRegisters, phase_offset)   == 0x2CU, "ShbtRegisters.phase_offset offset");
_Static_assert(offsetof(ShbtRegisters, ecc_counts)     == 0x30U, "ShbtRegisters.ecc_counts offset");
_Static_assert(offsetof(ShbtRegisters, control)         == 0x34U, "ShbtRegisters.control offset");
_Static_assert(sizeof(ShbtRegisters)                   == 0x38U, "ShbtRegisters total size");

/* --------------------------------------------------------------------------
 * v3.0: PCIe Gen5 x16 FPGA SDR DMA ring (up2.txt §2)
 * -------------------------------------------------------------------------- */
#include <stdatomic.h>

#define SGLT_DMA_RING_SIZE          4096U
#define SGLT_CACHE_LINE_SIZE        64U
#define SGLT_DMA_BUFFER_SIZE        (16U * 1024U * 1024U)
#define SGLT_DMA_FLAG_SW_OWNED      (1U << 0)

/* 64-byte descriptor, hardware-fixed offsets 0x00..0x3F. */
typedef struct {
    uint64_t host_phys_addr;      /* 0x00 */
    uint64_t fpga_local_addr;     /* 0x08 */
    uint32_t buffer_len_bytes;    /* 0x10 */
    uint32_t flags;               /* 0x14  bit0 Owner: 0=HW 1=SW */
    uint64_t frame_sequence;      /* 0x18 */
    uint64_t ptp_timestamp_sec;   /* 0x20 */
    uint32_t ptp_timestamp_nsec;  /* 0x28 */
    uint32_t descriptor_crc32;    /* 0x2C */
    uint8_t  reserved[16];        /* 0x30 */
} sglt_dma_descriptor_t;

_Static_assert(sizeof(sglt_dma_descriptor_t)          == 64U,   "desc size");
_Static_assert(offsetof(sglt_dma_descriptor_t, host_phys_addr)     == 0x00U, "desc 0x00");
_Static_assert(offsetof(sglt_dma_descriptor_t, fpga_local_addr)    == 0x08U, "desc 0x08");
_Static_assert(offsetof(sglt_dma_descriptor_t, buffer_len_bytes)   == 0x10U, "desc 0x10");
_Static_assert(offsetof(sglt_dma_descriptor_t, flags)              == 0x14U, "desc 0x14");
_Static_assert(offsetof(sglt_dma_descriptor_t, frame_sequence)     == 0x18U, "desc 0x18");
_Static_assert(offsetof(sglt_dma_descriptor_t, ptp_timestamp_sec)  == 0x20U, "desc 0x20");
_Static_assert(offsetof(sglt_dma_descriptor_t, ptp_timestamp_nsec) == 0x28U, "desc 0x28");
_Static_assert(offsetof(sglt_dma_descriptor_t, descriptor_crc32)   == 0x2CU, "desc 0x2C");
_Static_assert(offsetof(sglt_dma_descriptor_t, reserved)           == 0x30U, "desc 0x30");

/* Lock-free SPSC ring control shared with the FPGA DMA engine. */
typedef struct {
    _Atomic uint32_t head_index;
    _Atomic uint32_t tail_index;
    sglt_dma_descriptor_t *descriptors;
    uint32_t descriptor_count;
    uint8_t  _reserved[36];
} sglt_dma_ring_control_t;

_Static_assert(sizeof(sglt_dma_ring_control_t) <= 64U, "ring ctrl cache line");

/* --------------------------------------------------------------------------
 * v3.0 FPGA SDR MMIO aperture (up2.txt §2.2)
 * -------------------------------------------------------------------------- */
#define FPGA_SDR_REG_BASE           0x8000000000000000ULL
#define REG_SDR_QUENCH_OFFSET       0x0004ULL
#define REG_SDR_DAC_BIAS_BASE       0x0100ULL
#define SDR_DAC_BIAS_MIN_V          3.8
#define SDR_DAC_BIAS_MAX_V          7.4
#define SDR_DAC_CODE_MAX            65535U

int32_t  shbt_dma_init_ring(sglt_dma_ring_control_t *ring);
int32_t  shbt_dma_process_rx_interrupt(sglt_dma_ring_control_t *ring,
                                       uint32_t descriptor_idx);
int32_t  shbt_sdr_set_dac_bias_voltage(double volts);
void     shbt_sdr_assert_fast_quench(void);
uint32_t evaluate_rf_interlock_avx512(const float *telemetry16);

/* --------------------------------------------------------------------------
 * Dark-ledger TQEC + metamaterial self-healing bridge
 * -------------------------------------------------------------------------- */
#define SGLT_DARK_LEDGER_BYTES      1472U
#define SGLT_BRAID_DESCRIPTORS      124U
#define SGLT_SYNDROME_INTERVAL_US   100U
#define GST_HEAL_FLUENCE_MJ_CM2     27.9
#define SGLT_UF_MWPM_CROSSOVER      0.025

/* Extract one stabilizer syndrome round over the 1,472-byte dark ledger.
 * Returns popcount of non-trivial syndromes; writes packed bits to
 * *syndrome_bits_out. */
uint32_t shbt_dark_ledger_extract_syndrome(const uint64_t *ledger,
                                           uint64_t *syndrome_bits_out);

/* Issue a nanosecond GST self-healing pulse if `fluence_mj_cm2` meets the
 * 27.9 mJ/cm^2 threshold. Returns 0 when the pulse fires. */
int32_t  shbt_metamaterial_heal_pulse(double fluence_mj_cm2,
                                      uint32_t duration_ns);

#ifdef __cplusplus
}
#endif

#endif /* SHBT_HARDWARE_H */
