"""AgriCUDa kernel client for APSInsight workers.

AgriCUDa (SbruiceS/AgriCUDa) is an agriculture-oriented programming language
with kernel launch syntax. GPU/FPGA backends are not implemented; the CPU
device runs blocks in order.

This wrapper calls the `agricuda` CLI when present. It never invents results.
"""

import shutil
import subprocess
from typing import Any


def is_available() -> bool:
    return shutil.which("agricuda") is not None


def capabilities() -> dict[str, Any]:
    return {
        "available": is_available(),
        "language": "AgriCUDa",
        "version": "0.1",
        "features": [
            "check",
            "run",
            "kernel fn + launch <<<grid, block>>>",
            "CPU device",
            "parallel_for",
        ],
        "not_implemented": ["GPU", "FPGA", "native object code"],
    }


def run(source: str) -> dict[str, Any]:
    if not is_available():
        return {
            "status": "test_double",
            "value": None,
            "note": "agricuda binary not found. No kernel result fabricated.",
        }

    proc = subprocess.run(
        ["agricuda", "run", "-"],
        input=source,
        text=True,
        capture_output=True,
        check=False,
    )
    if proc.returncode != 0:
        raise RuntimeError(proc.stderr or "agricuda run failed")

    return {
        "status": "succeeded",
        "stdout": proc.stdout,
        "value": proc.stdout.strip(),
    }
