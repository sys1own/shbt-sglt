#!/usr/bin/env python3
"""shbt-sglt master test runner (sglt.txt §1, §4 verify path).

Executes the integration test pipeline:
  1. cargo test --workspace (Rust unit + integration suites)
  2. tests/test_hil_latency.py (HIL latency benches over shbt_reference.so)
"""

from __future__ import annotations

import pathlib
import subprocess
import sys

REPO_ROOT = pathlib.Path(__file__).resolve().parents[1]


def run(cmd: list[str], **kw) -> int:
    print(f"\n=== {' '.join(cmd)} ===", flush=True)
    return subprocess.run(cmd, cwd=REPO_ROOT, **kw).returncode


def main() -> int:
    rc = run(["cargo", "test", "--workspace"])
    if rc != 0:
        return rc
    rc = run([sys.executable, str(REPO_ROOT / "tests" / "test_hil_latency.py")])
    return rc


if __name__ == "__main__":
    sys.exit(main())
