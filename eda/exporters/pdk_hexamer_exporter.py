"""GDSII mask exporter for the SGLT 8x8 InP/InGaAs HBT phase-shifter array.

Transferred from sys1own/shbt-qc (`eda/exporters/pdk_hexamer.py`,
`HexamerExporter`, `generate_gdsii_mask`) and adapted to emit a self-contained
binary GDSII stream — no external layout library required — for the 8x8 array
on a 50 µm grid pitch (sglt.txt §4).
"""

from __future__ import annotations

import math
import pathlib
import struct
from typing import List, Sequence, Tuple

ARRAY_DIM = 8               # 8x8 HBT array
GRID_PITCH_UM = 50.0        # 50 µm cell pitch
DBU_UM = 1e-3               # GDSII database unit: 1 nm in µm user units


def _rec(tag: int, data: bytes) -> bytes:
    return struct.pack(">HH", 4 + len(data), tag) + data


def _i16(tag: int, vals: Sequence[int]) -> bytes:
    return _rec(tag, struct.pack(">" + "h" * len(vals), *vals))


def _str(tag: int, s: str) -> bytes:
    b = s.encode("ascii")
    if len(b) % 2:
        b += b"\x00"
    return _rec(tag, b)


def _real8(tag: int, val: float) -> bytes:
    """GDSII REAL8: sign + 7-bit excess-64 exponent, 56-bit mantissa."""
    if val == 0:
        return _rec(tag, b"\x00" * 8)
    sign = 0x80 if val < 0 else 0x00
    v = abs(val)
    exp = 64
    while v >= 1.0:
        v /= 16.0
        exp += 1
    while v < 1.0 / 16.0:
        v *= 16.0
        exp -= 1
    mant = int(round(v * (1 << 56)))
    return _rec(tag, bytes([sign | exp]) + mant.to_bytes(7, "big"))


def _xy(points: Sequence[Tuple[float, float]]) -> bytes:
    vals: List[int] = []
    for x, y in points:
        vals += [int(round(x / DBU_UM)), int(round(y / DBU_UM))]
    return _rec(0x1003, struct.pack(">" + "i" * len(vals), *vals))


def _boundary(layer: int, points: Sequence[Tuple[float, float]]) -> bytes:
    pts = list(points) + [points[0]]
    return b"".join([
        _i16(0x0800, []),        # BOUNDARY
        _i16(0x0D02, [layer]),   # LAYER
        _i16(0x0E02, [0]),       # DATATYPE
        _xy(pts),                # XY
        _i16(0x1100, []),        # ENDEL
    ])


def _hbt_cell_polygon(cx: float, cy: float) -> List[Tuple[float, float]]:
    """Emitter mesa footprint for one HBT cell (µm)."""
    return [
        (cx - 10, cy - 10), (cx + 10, cy - 10),
        (cx + 10, cy + 10), (cx - 10, cy + 10),
    ]


def _pad_ring(cx: float, cy: float, r: float, n: int = 8) -> List[Tuple[float, float]]:
    return [(cx + r * math.cos(2 * math.pi * i / n), cy + r * math.sin(2 * math.pi * i / n)) for i in range(n)]


class HexamerExporter:
    """Generates the 8x8 HBT array GDSII mask (50 µm pitch)."""

    def __init__(self, pitch_um: float = GRID_PITCH_UM, dim: int = ARRAY_DIM):
        self.pitch_um = pitch_um
        self.dim = dim

    def build_stream(self) -> bytes:
        out = [
            _i16(0x0002, [600]),                          # HEADER v600
            _i16(0x0102, [2026, 9, 20, 0, 0, 0] * 2),     # BGNLIB
            _str(0x0206, "SHBT.SGLT.HBT8X8.DB"),          # LIBNAME
            _real8(0x0305, 1e-3),                         # UNITS user (µm→dbu)
            _real8(0x0305, 1e-9),                         # UNITS dbu (m)
            _i16(0x0502, [2026, 9, 20, 0, 0, 0] * 2),     # BGNSTR
            _str(0x0606, "HBT_ARRAY_8X8"),                # STRNAME
        ]
        for ix in range(self.dim):
            for iy in range(self.dim):
                cx, cy = ix * self.pitch_um, iy * self.pitch_um
                out.append(_boundary(1, _hbt_cell_polygon(cx, cy)))       # mesa (layer 1)
                out.append(_boundary(2, _pad_ring(cx, cy, 18.0)))         # pad ring (layer 2)
        out.append(_i16(0x0700, []))                   # ENDSTR
        out.append(_rec(0x0400, b""))                  # ENDLIB
        return b"".join(out)

    def generate_gdsii_mask(self, out_path: str) -> pathlib.Path:
        path = pathlib.Path(out_path)
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(self.build_stream())
        return path


def main() -> None:
    import argparse

    p = argparse.ArgumentParser(description="Export 8x8 HBT array GDSII mask")
    p.add_argument("--out", default="eda/masks/hbt_array.gds")
    args = p.parse_args()
    path = HexamerExporter().generate_gdsii_mask(args.out)
    print(f"wrote {path} ({path.stat().st_size} bytes)")


if __name__ == "__main__":
    main()
