"""shbt_sglt — Python orchestration layer for the SGLT HIL co-simulation suite.

Bridges the Rust sub-crates to high-level analysis pipelines via PyO3 dynamic
bindings (`ffi_bindings.rs`).  When the native extension is not built, the
pure-Python reference models in `cli/main.py` provide the same numerics.
"""

__version__ = "0.1.0"

try:  # Native PyO3 bindings (optional acceleration path).
    import shbt_sglt_native as _native  # noqa: F401

    HAVE_NATIVE = True
except ImportError:  # pragma: no cover - native module not built
    HAVE_NATIVE = False
