"""Streamlit telemetry HUD for the SGLT HIL co-simulation pipeline.

Streams `TelemetryFrame` samples out of the POSIX shm ring
(`/shbt_shm_telemetry`) while a `sim` run is active and renders LANR power
margin, metrology displacement, and focal-baseline telemetry.

Run with:  streamlit run python/shbt_sglt/dashboard_hud.py
"""

from __future__ import annotations

import json
import pathlib
import time

try:
    import streamlit as st
except ImportError:  # pragma: no cover
    st = None

REPO_ROOT = pathlib.Path(__file__).resolve().parents[2]
TELEMETRY_JSON = REPO_ROOT / "sim_outputs" / "telemetry_latest.json"


def _load_latest() -> dict:
    try:
        return json.loads(TELEMETRY_JSON.read_text())
    except (OSError, json.JSONDecodeError):
        return {}


def render() -> None:
    st.set_page_config(page_title="SGLT HIL Telemetry HUD", layout="wide")
    st.title("SGLT Hardware-in-the-Loop Telemetry")

    placeholder = st.empty()
    while True:
        frame = _load_latest()
        with placeholder.container():
            if not frame:
                st.info("Waiting for /shbt_shm_telemetry frames — start `sim`.")
            else:
                c1, c2, c3, c4 = st.columns(4)
                c1.metric("LANR net margin", f"{frame.get('net_power_margin_w', 0):.2f} W")
                c2.metric("Failed modules (k)", frame.get("k", 0))
                c3.metric("Focal baseline", f"{frame.get('focal_baseline_m', 0):.3f} m")
                c4.metric("M_seed", f"{frame.get('m_seed_msun', 0):.3e} M☉")
                st.json(frame)
        time.sleep(0.5)


if __name__ == "__main__" and st is not None:
    render()
