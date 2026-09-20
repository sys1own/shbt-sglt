/*
 * sglt_v3_abi.h - SGLT v3.0 C-ABI surface (up2.txt).
 *
 * Exports the repr(C, align(64)) structs shared between the Rust crates and
 * the C11 freestanding microkernel, plus the FFI entry points:
 *   sglt_neural_optics_create / process / destroy
 *   sglt_swarm_engine_create / compute_distances_simd / destroy
 *   sglt_lindblad_evolve (opaque solver handle)
 *   shbt_dma_init_ring, shbt_dma_process_rx_interrupt,
 *   shbt_sdr_set_dac_bias_voltage, shbt_sdr_assert_fast_quench,
 *   evaluate_rf_interlock_avx512 (kernel side)
 */
#ifndef SGLT_V3_ABI_H
#define SGLT_V3_ABI_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

#define SGLT_ABI_VERSION_3 3U
#define SGLT_DMA_RING_SIZE 4096U
#define SGLT_CACHE_LINE_SIZE 64U
#define SGLT_DMA_BUFFER_SIZE (16U * 1024U * 1024U)

/* --------------------------------------------------------------------------
 * sglt-neural-optics
 * -------------------------------------------------------------------------- */
typedef struct {
    uint32_t grid_res_x;
    uint32_t grid_res_y;
    uint32_t grid_res_z;
    uint32_t num_spectral_bands;
    float    lambda_min_nm;
    float    lambda_max_nm;
    float    lambda_phys;
    float    lambda_reg;
    float    focal_distance_au;
    uint8_t  enable_cuda;
    uint8_t  _pad[31];
} __attribute__((aligned(64))) sglt_neural_optics_config_t;

typedef struct {
    uint64_t frame_id;
    uint64_t timestamp_ns;
    float    mean_pinn_residual;
    float    convergence_eps;
    uint64_t data_payload_bytes;
    int32_t  status_code;
    uint8_t  _pad[28];
} __attribute__((aligned(64))) sglt_reconstruction_result_header_t;

void *sglt_neural_optics_create(const sglt_neural_optics_config_t *config);
int32_t sglt_neural_optics_process(void *engine,
                                 const float *input, size_t input_len,
                                 float *albedo_out, float *atm_out,
                                 sglt_reconstruction_result_header_t *header_out);
void sglt_neural_optics_destroy(void *engine);

/* --------------------------------------------------------------------------
 * sglt-hardware-dma — mirrors sglt_dma_descriptor_t in shbt_hardware.h
 * -------------------------------------------------------------------------- */
typedef struct {
    uint64_t host_phys_addr;      /* 0x00 */
    uint64_t fpga_local_addr;     /* 0x08 */
    uint32_t buffer_len_bytes;    /* 0x10 */
    uint32_t flags;               /* 0x14  bit0: 0=HW 1=SW */
    uint64_t frame_sequence;      /* 0x18 */
    uint64_t ptp_timestamp_sec;   /* 0x20 */
    uint32_t ptp_timestamp_nsec;  /* 0x28 */
    uint32_t descriptor_crc32;    /* 0x2C */
    uint8_t  reserved[16];        /* 0x30 */
} __attribute__((aligned(64))) sglt_dma_descriptor_abi_t;

/* --------------------------------------------------------------------------
 * sglt-swarm-dynamics
 * -------------------------------------------------------------------------- */
typedef struct {
    double x, y, z, _pad;
} __attribute__((aligned(32))) sglt_vector3d_t;

typedef struct {
    sglt_vector3d_t position_m;
    sglt_vector3d_t velocity_mps;
    uint32_t node_id;
    uint8_t  role;                 /* 0 lens, 1 UV, 2 optical, 3 MWIR */
    uint8_t  _padding[7];
    double   mass_kg;
    uint64_t last_ptp_sync_ns;
    uint64_t metrology_link_mask;
} __attribute__((aligned(64))) sglt_swarm_node_state_t;

typedef struct {
    double link_ranges_pm[4][4];
    double link_dws_theta_nrad[4][4];
    uint64_t timestamp_ptp_ns;
    uint8_t  _reserved[56];
} __attribute__((aligned(64))) sglt_metrology_mesh_t;

void *sglt_swarm_engine_create(void);
int32_t sglt_swarm_compute_distances_simd(void *engine, double *out_4x4);
void sglt_swarm_engine_destroy(void *engine);

/* --------------------------------------------------------------------------
 * sglt-quantum-decoherence report (Lindblad evolution over the 124-anyon array)
 * -------------------------------------------------------------------------- */
typedef struct {
    uint64_t steps;
    double   sim_time_s;
    double   trace_error;
    double   purity;
    double   fidelity_bound;
    double   gamma_gcr;
    uint8_t  _pad[16];
} __attribute__((aligned(64))) sglt_decoherence_report_t;

/* --------------------------------------------------------------------------
 * Microkernel bridge (kernel/src/shbt_core_runtime.c)
 * -------------------------------------------------------------------------- */
int32_t shbt_dma_process_rx_interrupt(uint32_t descriptor_idx);
void    shbt_sdr_assert_fast_quench(void);
int32_t shbt_sdr_set_dac_bias_voltage(double volts);
uint32_t evaluate_rf_interlock_avx512(const float *telemetry16);

#ifdef __cplusplus
}
#endif

#endif /* SGLT_V3_ABI_H */
