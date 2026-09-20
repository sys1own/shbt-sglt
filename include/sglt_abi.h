/*
 * sglt_abi.h - Unified SGLT C-ABI surface.
 *
 * Exports the repr(C, align(64)) structures and C FFI entry points shared
 * between the Rust sub-crates and the C11 freestanding microkernel:
 *
 *   sglt_squeezed_metrology_evaluate_phase   (sglt-squeezed-metrology)
 *   sglt_diamond_transducer_solve_nodal      (sglt-diamond-transducer)
 *   sglt_2pn_coronal_optics_evaluate         (sglt-2pn-coronal-optics)
 *   sglt_metamaterial_radiation_heal         (sglt-metamaterial-radiation)
 *   sglt_gpu_physics_init / process_frame_4k / destroy (sglt-gpu-physics)
 *   sglt_uncertainty_uq_evaluate             (sglt-uncertainty-uq)
 *   sglt_tqec_dark_ledger_decode             (sglt-tqec-dark-ledger)
 *   sglt_webgpu_vis_bind_frame               (sglt-webgpu-vis)
 */
#ifndef SGLT_ABI_H
#define SGLT_ABI_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

#define SGLT_ABI_VERSION_UNIFIED 4U
#define SGLT_CACHE_LINE_SIZE     64U

/* --------------------------------------------------------------------------
 * sglt-squeezed-metrology: TMSV + N00N-state sub-SQL metrology mesh
 * -------------------------------------------------------------------------- */
typedef struct {
    double   wavelength_m;
    double   carrier_power_w;
    double   squeezing_param_r;
    double   integration_bandwidth_hz;
    double   heterodyne_efficiency;
    uint8_t  _pad[24];
} __attribute__((aligned(64))) sglt_squeezed_metrology_config_t;

typedef struct {
    double   quadrature_variance;
    double   squeezing_db;
    double   range_noise_density_m_sqrt_hz;
    double   range_uncertainty_3sigma_m;
    uint8_t  _pad[32];
} __attribute__((aligned(64))) sglt_metrology_metrics_t;

int32_t sglt_squeezed_metrology_evaluate_phase(
    const sglt_squeezed_metrology_config_t *config,
    sglt_metrology_metrics_t *out_metrics);

/* --------------------------------------------------------------------------
 * sglt-diamond-transducer: Diamond-on-GaN + NbN/MgB2 routing, Debye T^3
 * -------------------------------------------------------------------------- */
typedef struct {
    double   base_temperature_k;
    double   power_transient_mw;
    double   transient_duration_ns;
    double   substrate_area_mm2;
    double   substrate_thickness_mm;
    uint8_t  _pad[24];
} __attribute__((aligned(64))) sglt_nodal_thermal_state_t;

typedef struct {
    double   peak_temperature_k;
    double   quench_headroom_k;
    uint8_t  is_superconducting;
    uint8_t  _pad[47];
} __attribute__((aligned(64))) sglt_thermal_solver_result_t;

int32_t sglt_diamond_transducer_solve_nodal(
    const sglt_nodal_thermal_state_t *state,
    sglt_thermal_solver_result_t *out_result);

/* --------------------------------------------------------------------------
 * sglt-2pn-coronal-optics: Baumbach-Allen corona + 2PN eikonal pre-distortion
 * -------------------------------------------------------------------------- */
typedef struct {
    double   heliocentric_r_solar_radii;
    double   tilt_angle_deg;
    double   optical_frequency_hz;
    double   cme_factor;
    uint8_t  _pad[32];
} __attribute__((aligned(64))) sglt_coronal_optics_params_t;

typedef struct {
    double   grav_2pn_phase_rad;
    double   plasma_phase_rad;
    double   total_predistortion_phase_rad;
    uint8_t  _pad[40];
} __attribute__((aligned(64))) sglt_eikonal_phase_result_t;

int32_t sglt_2pn_coronal_optics_evaluate(
    const sglt_coronal_optics_params_t *params,
    sglt_eikonal_phase_result_t *out_result);

/* --------------------------------------------------------------------------
 * sglt-metamaterial-radiation: GST chalcogenide self-healing routing
 * -------------------------------------------------------------------------- */
typedef struct {
    double   accumulated_ddd_krad;
    double   current_conductivity_s_m;
    double   initial_conductivity_s_m;
    uint8_t  _pad[40];
} __attribute__((aligned(64))) sglt_radiation_state_t;

typedef struct {
    double   pulse_energy_mj_cm2;
    double   pulse_duration_ns;
    uint8_t  _pad[48];
} __attribute__((aligned(64))) sglt_self_healing_pulse_config_t;

typedef struct {
    double   post_healing_conductivity_s_m;
    double   recovery_ratio;
    uint8_t  lattice_reorganized;
    uint8_t  _pad[47];
} __attribute__((aligned(64))) sglt_healing_result_t;

int32_t sglt_metamaterial_radiation_heal(
    const sglt_radiation_state_t *rad_state,
    const sglt_self_healing_pulse_config_t *pulse_cfg,
    sglt_healing_result_t *out_result);

/* --------------------------------------------------------------------------
 * sglt-gpu-physics: distributed 2PN wave-optics + PINN deconvolution
 * -------------------------------------------------------------------------- */
typedef struct {
    uint32_t grid_width;
    uint32_t grid_height;
    double   solar_mass_kg;
    double   quadrupole_j2;
    double   focal_distance_au;
    float    wavelength_nm;
    uint8_t  enable_2pn_corrections;
    int32_t  num_gpus;
    uint8_t  _pad[19];
} __attribute__((aligned(64))) sglt_gpu_config_t;

typedef struct {
    float   *data_ptr;
    uint32_t width;
    uint32_t height;
    uint32_t channels;
    uint8_t  _pad[4];
    uint64_t timestamp_ns;
    uint64_t frame_id;
    uint8_t  _pad2[16];
} __attribute__((aligned(64))) sglt_frame_buffer_t;

typedef struct {
    float    max_caustic_intensity;
    float    strehl_ratio;
    float    pinn_loss_val;
    uint32_t execution_time_us;
    int32_t  status_code;
    uint8_t  _pad[44];
} __attribute__((aligned(64))) sglt_raytrace_result_t;

int32_t sglt_gpu_physics_init(const sglt_gpu_config_t *config,
                              void **out_engine);
int32_t sglt_gpu_physics_process_frame_4k(void *engine,
                              const sglt_frame_buffer_t *input_frame,
                              sglt_frame_buffer_t *output_frame,
                              sglt_raytrace_result_t *result_metrics);
int32_t sglt_gpu_physics_destroy(void *engine);

/* --------------------------------------------------------------------------
 * sglt-uncertainty-uq: hyper-dual MC engine, GUM Supp 1/2 compliance
 * -------------------------------------------------------------------------- */
typedef struct {
    uint64_t num_samples;
    double   sigma_position_m;
    double   sigma_pointing_arcsec;
    double   sigma_thermal_k;
    uint64_t rng_seed;
    uint8_t  _pad[24];
} __attribute__((aligned(64))) sglt_uq_config_t;

typedef struct {
    uint64_t samples_evaluated;
    double   coverage_fraction;
    double   lower_3sigma[5];      /* H2O ppm, O2 ppm, CH4 ppb, CO2 ppm, O3 ppb */
    double   upper_3sigma[5];
    double   max_mahalanobis_sq;
    int32_t  gum_compliant;
    uint8_t  _pad[4];
} sglt_uq_result_t;

int32_t sglt_uncertainty_uq_evaluate(const sglt_uq_config_t *config,
                                     sglt_uq_result_t *out);

/* --------------------------------------------------------------------------
 * sglt-tqec-dark-ledger: Fibonacci code syndrome extraction + UF/MWPM decode
 * -------------------------------------------------------------------------- */
typedef struct {
    uint64_t syndrome_bits;
    double   defect_density;
    double   transit_time_s;
    uint8_t  _pad[40];
} __attribute__((aligned(64))) sglt_syndrome_frame_t;

typedef struct {
    int32_t  decoder_used;         /* 0 = distributed Union-Find, 1 = MWPM */
    double   decode_latency_ns;
    double   logical_error_rate;
    double   fidelity_logical;
    uint32_t corrected_defects;
    uint8_t  _pad[28];
} __attribute__((aligned(64))) sglt_decode_result_t;

int32_t sglt_tqec_dark_ledger_decode(const sglt_syndrome_frame_t *frame,
                                     sglt_decode_result_t *out);

/* --------------------------------------------------------------------------
 * sglt-webgpu-vis: Wasm/WebGPU telemetry binding
 * -------------------------------------------------------------------------- */
typedef struct {
    uint32_t data_ptr;
    uint32_t data_len;
    uint64_t frame_id;
    uint64_t timestamp_ns;
    uint8_t  _pad[40];
} __attribute__((aligned(64))) sglt_vis_telemetry_frame_t;

typedef struct {
    double   fps;
    uint64_t payload_bytes;
    uint64_t bound_buffer_bytes;
    uint8_t  _pad[40];
} __attribute__((aligned(64))) sglt_vis_render_stats_t;

int32_t sglt_webgpu_vis_bind_frame(
    const sglt_vis_telemetry_frame_t *frame,
    sglt_vis_render_stats_t *out_stats);

#ifdef __cplusplus
}
#endif

#endif /* SGLT_ABI_H */
