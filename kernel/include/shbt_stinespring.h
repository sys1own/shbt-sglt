/*
 * shbt_stinespring.h - Unified Stinespring translocation driver ABI.
 *
 * SHBT-MMIO-1 extended register map for the non-local telemetry upgrade
 * (sglt1.txt, Microkernel Hardware & Register Map).  56-byte volatile
 * register block at 0x70000000 backed by the 2,112-byte
 * .stinespring_frame SRAM arena (640 B active + 1,472 B dark ledger
 * partitions).
 */
#ifndef SHBT_STINESPRING_H
#define SHBT_STINESPRING_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#define SHBT_MMIO_BASE_ADDR        0x70000000ULL
#define SHBT_SRAM_FRAME_SIZE       2112
#define SHBT_ACTIVE_PARTITION_SIZE 640
#define SHBT_DARK_PARTITION_SIZE   1472

#define REG_CTRL_STAT              0x0000
#define REG_STINESPRING_ETA        0x0004
#define REG_HEEGAARD_ADDR          0x0008
#define REG_ADM_SHIFT_COMP         0x0010
#define REG_CAUSAL_PROJ_PI         0x0018
#define REG_SRAM_ARENA_PTR         0x0020
#define REG_LANDAUER_GET_COST      0x0028
#define REG_INTERLOCK_QUENCH       0x002C
#define REG_ECC_STATUS_VECTOR      0x0030

#define CTRL_ENABLE_BIT            (1U << 0)
#define CTRL_ARM_BIT               (1U << 1)
#define CTRL_RESET_BIT             (1U << 2)
#define CTRL_QUENCH_STAT_BIT       (1U << 3)

typedef struct __attribute__((packed)) {
    uint8_t active_block[SHBT_ACTIVE_PARTITION_SIZE];
    uint8_t dark_ledger_block[SHBT_DARK_PARTITION_SIZE];
} UnifiedStinespringFrame;

typedef struct __attribute__((packed)) {
    volatile uint32_t ctrl_stat;
    volatile uint32_t stinespring_eta;
    volatile uint64_t heegaard_addr;
    volatile double   adm_shift_comp;
    volatile uint64_t causal_proj_pi;
    volatile uint64_t sram_arena_ptr;
    volatile uint32_t landauer_get_cost;
    volatile uint32_t interlock_quench;
    volatile uint64_t ecc_status_vector;
} ShbtMmioRegisters;

int shbt_kernel_init(void);
int shbt_stinespring_translocate(UnifiedStinespringFrame* frame);
void shbt_avx512_givens_remapping(double* state_vector, size_t dim, double theta);
uint8_t shbt_compute_secded_ecc(uint64_t data);
bool shbt_verify_and_correct_secded(uint64_t* data, uint8_t ecc);
void shbt_trigger_quench_interlock(void);

/* Hosted benches for the verification matrix. */
double shbt_secded_bench(unsigned iters);
double shbt_givens_residual(void);
double shbt_quench_interlock_bench(unsigned iters);

#endif /* SHBT_STINESPRING_H */
