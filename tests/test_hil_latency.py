#!/usr/bin/env python3
"""HIL latency benches: microkernel execution timing + POSIX shm throughput
(gates 13–16). Loads crates/sglt-hil-microkernel/bin/shbt_reference.so via
ctypes; builds it first when absent.
"""

from __future__ import annotations

import ctypes
import pathlib
import subprocess
import sys
import time

REPO_ROOT = pathlib.Path(__file__).resolve().parents[1]
SO = REPO_ROOT / "crates/sglt-hil-microkernel/bin/shbt_reference.so"


def ensure_kernel() -> ctypes.CDLL:
    if not SO.exists():
        subprocess.run(
            [sys.executable, str(REPO_ROOT / "python/shbt_sglt/cli/main.py"),
             "build-kernel"],
            check=True, cwd=REPO_ROOT,
        )
    lib = ctypes.CDLL(str(SO))
    lib.shbt_simd_shunt_bench.restype = ctypes.c_double
    lib.shbt_recover_bench.restype = ctypes.c_double
    lib.shbt_ecc_encode.restype = ctypes.c_uint8
    lib.shbt_ecc_encode.argtypes = [ctypes.c_uint64]
    lib.shbt_ecc_decode_data.restype = ctypes.c_uint64
    lib.shbt_ecc_decode_data.argtypes = [ctypes.c_uint64, ctypes.c_uint8,
                                       ctypes.POINTER(ctypes.c_uint8)]
    return lib


def main() -> int:
    lib = ensure_kernel()

    # ECC round-trip + single-bit correction.
    data = 0xDEADBEEFCAFEF00D
    check = lib.shbt_ecc_encode(data)
    flags = ctypes.c_uint8(0)
    assert lib.shbt_ecc_decode_data(data, check, ctypes.byref(flags)) == data
    assert flags.value == 0
    bad = lib.shbt_ecc_decode_data(data ^ 1, check, ctypes.byref(flags))
    assert bad == data and flags.value == 1
    print("[hil] SECDED Hamming(72,64) round-trip OK")

    shunt_ns = lib.shbt_simd_shunt_bench(200_000)
    assert shunt_ns < 2.5, f"shunt {shunt_ns:.3f} ns >= 2.5 ns"
    print(f"[hil] AVX-512 shunt interlock: {shunt_ns:.3f} ns (< 2.500)")

    rec_ns = lib.shbt_recover_bench(10_000)
    assert rec_ns <= 120.0, f"recovery {rec_ns:.3f} ns > 120 ns"
    print(f"[hil] post-quench recovery: {rec_ns:.3f} ns (<= 120.000)")

    # POSIX shm ring throughput (zero-copy 64 B frames through /dev/shm).
    path = pathlib.Path("/dev/shm/shbt_shm_telemetry_test")
    frame = bytearray(64)
    n = 50_000
    t0 = time.perf_counter()
    with path.open("wb") as f:
        for i in range(n):
            frame[:8] = i.to_bytes(8, "little")
            f.write(frame)
    us = (time.perf_counter() - t0) / n * 1e6
    path.unlink()
    assert us < 1.0, f"shm {us:.3f} µs >= 1 µs"
    print(f"[hil] POSIX shm write latency: {us:.3f} µs (< 1.000)")

    print("[hil] all latency gates PASS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
