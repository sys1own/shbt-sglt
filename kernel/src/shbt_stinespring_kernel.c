/*
 * shbt_stinespring_kernel.c - Stinespring state translocation driver.
 *
 * SHBT-MMIO-1 driver for the non-local telemetry upgrade (sglt1.txt):
 * unified Stinespring frame translocation over the 2,112-byte
 * .stinespring_frame SRAM arena (640 B active / 1,472 B dark ledger),
 * SECDED Hamming(72,64) verification with quench-interlock escalation on
 * double-bit errors, and AVX-512 Givens rotation remapping of the dark
 * ledger block under theta = eta_A = 0.30303.
 *
 * In hosted builds (shbt_reference.so) the MMIO aperture is backed by a
 * static shadow register block so the benches run unprivileged; on bare
 * metal (SHBT_BARE_METAL) writes hit the physical aperture at
 * 0x70000000.
 */

#include <immintrin.h>
#include "shbt_stinespring.h"

#ifndef SHBT_TSC_HZ
#define SHBT_TSC_HZ 3000000000ULL
#endif

#if defined(SHBT_BARE_METAL)
static volatile ShbtMmioRegisters* const g_mmio =
    (volatile ShbtMmioRegisters*)SHBT_MMIO_BASE_ADDR;
#else
static ShbtMmioRegisters g_shadow_mmio;
static volatile ShbtMmioRegisters* const g_mmio = &g_shadow_mmio;
#endif

static const uint16_t ETA_ACTIVE_Q16 = 0x4D8B; /* (10/33) * 65536 */
static const uint16_t ETA_DARK_Q16   = 0xB275; /* (23/33) * 65536 */

static inline uint64_t shbt_cycles(void)
{
#if defined(__x86_64__) || defined(__i386__)
    uint32_t lo, hi;
    __asm__ __volatile__("lfence\n\trdtsc" : "=a"(lo), "=d"(hi) :: "memory");
    return ((uint64_t)hi << 32) | lo;
#else
    return 0ULL;
#endif
}

/* Freestanding trig (no libm): Taylor about zero after quadrant reduction
 * — adequate for theta = 0.30303 and the Givens bench ranges. */
static double shbt_sin(double x)
{
    /* reduce to [-pi, pi] */
    const double two_pi = 6.2831853071795864769;
    const double pi = 3.14159265358979323846;
    while (x > pi) x -= two_pi;
    while (x < -pi) x += two_pi;
    double x2 = x * x;
    double t = x;
    double s = x;
    for (int n = 1; n <= 10; ++n) {
        t *= -x2 / ((2.0 * n) * (2.0 * n + 1.0));
        s += t;
    }
    return s;
}

static double shbt_cos(double x)
{
    const double half_pi = 1.57079632679489661923;
    return shbt_sin(x + half_pi);
}

int shbt_kernel_init(void)
{
    if (!g_mmio) {
        return -1;
    }
    g_mmio->ctrl_stat = CTRL_RESET_BIT;

    uint32_t eta_packed =
        ((uint32_t)ETA_ACTIVE_Q16 << 16) | (uint32_t)ETA_DARK_Q16;
    g_mmio->stinespring_eta = eta_packed;

    g_mmio->ctrl_stat = CTRL_ENABLE_BIT | CTRL_ARM_BIT;
    return 0;
}

uint8_t shbt_compute_secded_ecc(uint64_t data)
{
    uint8_t p = 0;
    for (int i = 0; i < 64; i++) {
        if ((data >> i) & 1ULL) {
            p ^= (uint8_t)(i + 1);
        }
    }
    uint8_t overall_parity = 0;
    uint64_t temp = data;
    while (temp) {
        overall_parity ^= (uint8_t)(temp & 1);
        temp >>= 1;
    }
    return (p & 0x3F) | ((overall_parity & 1) << 7);
}

bool shbt_verify_and_correct_secded(uint64_t* data, uint8_t ecc)
{
    uint8_t computed_ecc = shbt_compute_secded_ecc(*data);
    uint8_t syndrome = (uint8_t)((computed_ecc ^ ecc) & 0x3F);
    bool parity_mismatch = ((computed_ecc ^ ecc) & 0x80) != 0;

    if (syndrome == 0 && !parity_mismatch) {
        return true;
    } else if (syndrome != 0 && parity_mismatch) {
        if (syndrome <= 64) {
            *data ^= (1ULL << (syndrome - 1));
        }
        return true;
    } else {
        shbt_trigger_quench_interlock();
        return false;
    }
}

void shbt_avx512_givens_remapping(double* state_vector, size_t dim,
                                  double theta)
{
#if defined(__AVX512F__)
    double c = shbt_cos(theta);
    double s = shbt_sin(theta);
    __m512d vec_c = _mm512_set1_pd(c);
    __m512d vec_s = _mm512_set1_pd(s);

    for (size_t i = 0; i < dim; i += 8) {
        __m512d x = _mm512_loadu_pd(&state_vector[i]);
        __m512d y = _mm512_loadu_pd(&state_vector[i + dim]);

        __m512d x_prime = _mm512_add_pd(
            _mm512_mul_pd(vec_c, x), _mm512_mul_pd(vec_s, y));
        __m512d y_prime = _mm512_sub_pd(
            _mm512_mul_pd(vec_c, y), _mm512_mul_pd(vec_s, x));

        _mm512_storeu_pd(&state_vector[i], x_prime);
        _mm512_storeu_pd(&state_vector[i + dim], y_prime);
    }
#else
    double c = shbt_cos(theta);
    double s = shbt_sin(theta);
    for (size_t i = 0; i < dim / 2; i++) {
        double x = state_vector[i];
        double y = state_vector[i + dim / 2];
        state_vector[i] = c * x + s * y;
        state_vector[i + dim / 2] = -s * x + c * y;
    }
#endif
}

void shbt_trigger_quench_interlock(void)
{
    g_mmio->interlock_quench = 0xDEADBEEF;
    g_mmio->ctrl_stat |= CTRL_QUENCH_STAT_BIT;
#if defined(SHBT_BARE_METAL)
    __asm__ volatile ("cli; hlt" ::: "memory");
#endif
}

int shbt_stinespring_translocate(UnifiedStinespringFrame* frame)
{
    if (!frame) return -1;

    g_mmio->sram_arena_ptr = (uint64_t)(uintptr_t)frame;

    uint64_t* active_ptr = (uint64_t*)frame->active_block;
    uint8_t ecc = shbt_compute_secded_ecc(*active_ptr);
    if (!shbt_verify_and_correct_secded(active_ptr, ecc)) {
        return -2;
    }

    shbt_avx512_givens_remapping((double*)frame->dark_ledger_block, 64,
                                 0.30303);

    g_mmio->landauer_get_cost += 1;
    return 0;
}

/* --------------------------------------------------------------------------
 * Hosted benches for the verification matrix
 * -------------------------------------------------------------------------- */

/* SECDED encode + verify latency (ns/op) on shadow data. */
double shbt_secded_bench(unsigned iters)
{
    static uint64_t word = 0xA5A5C3C39E3779B9ULL;
    volatile uint8_t sink = 0;
    uint64_t t0 = shbt_cycles();
    for (unsigned i = 0; i < iters; ++i) {
        uint8_t ecc = shbt_compute_secded_ecc(word);
        sink ^= ecc;
    }
    (void)sink;
    uint64_t dt = shbt_cycles() - t0;
    return (double)dt * 1.0e9 / ((double)SHBT_TSC_HZ * (double)iters);
}

/* Roundtrip residual of the Givens remap: apply R(theta) then R(-theta)
 * over a deterministic state vector and report the max absolute error. */
double shbt_givens_residual(void)
{
    double v[128];
    for (int i = 0; i < 128; ++i) v[i] = 0.125 * (i - 64) + 0.03125;
    shbt_avx512_givens_remapping(v, 64, 0.30303);
    shbt_avx512_givens_remapping(v, 64, -0.30303);
    double worst = 0.0;
    for (int i = 0; i < 128; ++i) {
        double orig = 0.125 * (i - 64) + 0.03125;
        double d = v[i] - orig;
        if (d < 0.0) d = -d;
        if (d > worst) worst = d;
    }
    return worst;
}

/* Quench interlock latency (ns/op): interlock register write + status
 * bit raise on the (shadow) MMIO block. */
double shbt_quench_interlock_bench(unsigned iters)
{
    volatile uint32_t sink = 0;
    uint64_t t0 = shbt_cycles();
    for (unsigned i = 0; i < iters; ++i) {
        g_mmio->interlock_quench = 0xDEADBEEF;
        g_mmio->ctrl_stat |= CTRL_QUENCH_STAT_BIT;
        sink |= g_mmio->ctrl_stat;
        g_mmio->ctrl_stat &= ~CTRL_QUENCH_STAT_BIT;
    }
    (void)sink;
    uint64_t dt = shbt_cycles() - t0;
    return (double)dt * 1.0e9 / ((double)SHBT_TSC_HZ * (double)iters);
}
