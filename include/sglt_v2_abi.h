#ifndef SGLT_V2_ABI_H
#define SGLT_V2_ABI_H

#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Operational Optical Configuration Struct */
typedef struct {
    double wavelength_min_nm;
    double wavelength_max_nm;
    double focal_distance_au;
    double primary_aperture_m;
    uint32_t grid_resolution_x;
    uint32_t grid_resolution_y;
} sglt_raytrace_config_t;

/* 4-Body CR3BP State Vector Struct */
typedef struct {
    double epoch_jd;
    double state_vector[6]; /* x, y, z, vx, vy, vz in normalized rotating frame */
    double mass_ratio_mu;
    double jacobi_constant;
    double radiation_dose_ddd_gy;
} sglt_cr3bp_state_t;

/* Cryogenic Transient State Struct */
typedef struct {
    double temp_he4_bath_k;
    double temp_sapphire_k;
    double temp_aerogel_k;
    double temp_radiator_k;
    double bench_opd_perturbation_nm;
    double cryogen_mass_kg;
} sglt_cryo_state_t;

/* Communication Link Telemetry Struct */
typedef struct {
    double downlink_rate_gbps;
    double rytov_variance;
    double bit_error_rate;
    bool optical_pointing_lock;
    bool ka_band_fallback_active;
} sglt_comm_link_t;

/* Dark Ledger Anyon Descriptor Region (1472 Bytes Total Footprint) */
typedef struct {
    uint8_t  frame_header[16];
    uint8_t  braid_descriptors[992]; /* 124 Fibonacci descriptors x 8 bytes */
    uint8_t  syndrome_parity[464];
} sglt_dark_ledger_t;

/* Target Observatory Datacube Definition */
typedef struct {
    uint32_t num_wavelength_channels;
    uint32_t spatial_dim_x;
    uint32_t spatial_dim_y;
    float*   datacube_ptr;
    double   contrast_rejection_ratio;
} sglt_fits_export_t;

/* Core Library C-ABI Function Declarations */
int32_t sglt_optics_raytrace_execute(const sglt_raytrace_config_t* config, float* output_image_plane);
int32_t sglt_orbital_dop853_step(sglt_cr3bp_state_t* state, double step_size_sec);
int32_t sglt_cryo_solve_nodal_step(sglt_cryo_state_t* thermal_state, double dt_sec);
int32_t sglt_hil_inject_fault_seu(sglt_dark_ledger_t* ledger, uint32_t bit_index);
int32_t sglt_fits_export_file(const char* filepath, const sglt_fits_export_t* export_data);
int32_t sglt_verify_all_gates(uint32_t* passed_gates_mask);

#ifdef __cplusplus
}
#endif

#endif /* SGLT_V2_ABI_H */
