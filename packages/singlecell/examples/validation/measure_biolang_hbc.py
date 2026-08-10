#!/usr/bin/env python3
"""Run the BioLang HBC notebook and record wall time and peak host memory."""

from __future__ import annotations

import argparse
import ctypes
import json
import os
from pathlib import Path
import subprocess
import sys
import time


class _ProcessMemoryCounters(ctypes.Structure):
    _fields_ = [
        ("cb", ctypes.c_ulong),
        ("PageFaultCount", ctypes.c_ulong),
        ("PeakWorkingSetSize", ctypes.c_size_t),
        ("WorkingSetSize", ctypes.c_size_t),
        ("QuotaPeakPagedPoolUsage", ctypes.c_size_t),
        ("QuotaPagedPoolUsage", ctypes.c_size_t),
        ("QuotaPeakNonPagedPoolUsage", ctypes.c_size_t),
        ("QuotaNonPagedPoolUsage", ctypes.c_size_t),
        ("PagefileUsage", ctypes.c_size_t),
        ("PeakPagefileUsage", ctypes.c_size_t),
    ]


def peak_working_set(process: subprocess.Popen[bytes]) -> int:
    """Return the Windows peak working set, or zero on unsupported systems."""
    if os.name != "nt":
        return 0
    counters = _ProcessMemoryCounters()
    counters.cb = ctypes.sizeof(counters)
    query = ctypes.windll.psapi.GetProcessMemoryInfo
    if not query(int(process._handle), ctypes.byref(counters), counters.cb):
        return 0
    return int(counters.PeakWorkingSetSize)


def normalized_environment() -> dict[str, str]:
    """Collapse case-duplicate Windows keys such as Path/PATH."""
    if os.name != "nt":
        return dict(os.environ)
    result: dict[str, tuple[str, str]] = {}
    for key, value in os.environ.items():
        result[key.casefold()] = (key, value)
    return {key: value for key, value in result.values()}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--executable", default="target/release/bl.exe")
    parser.add_argument(
        "--notebook",
        default="packages/singlecell/examples/hbc_course_validation.bln",
    )
    parser.add_argument(
        "--output", default="validation-results/hbc-biolang-current"
    )
    parser.add_argument("--gpu", choices=("auto", "off", "on"), default="off")
    args = parser.parse_args()

    repo = Path(__file__).resolve().parents[4]
    executable = (repo / args.executable).resolve()
    notebook = (repo / args.notebook).resolve()
    output = (repo / args.output).resolve()
    if output.exists():
        raise SystemExit(f"output directory already exists: {output}")
    output.mkdir(parents=True)

    environment = normalized_environment()
    environment["BIOLANG_GPU"] = args.gpu
    environment["BIOLANG_HBC_OUTPUT"] = str(output)
    command = [str(executable), "notebook", str(notebook)]
    creation_flags = subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0

    started = time.perf_counter()
    with (output / "stdout.log").open("wb") as stdout, (
        output / "stderr.log"
    ).open("wb") as stderr:
        process = subprocess.Popen(
            command,
            cwd=repo,
            env=environment,
            stdout=stdout,
            stderr=stderr,
            creationflags=creation_flags,
        )
        peak_bytes = 0
        while process.poll() is None:
            peak_bytes = max(peak_bytes, peak_working_set(process))
            time.sleep(0.25)
        peak_bytes = max(peak_bytes, peak_working_set(process))

    measurement = {
        "executable": str(executable),
        "notebook": str(notebook),
        "gpu": args.gpu,
        "exit_code": process.returncode,
        "wall_seconds": round(time.perf_counter() - started, 3),
        "peak_working_set_bytes": peak_bytes,
        "peak_working_set_gib": round(peak_bytes / 1024**3, 3),
    }
    (output / "resources.json").write_text(
        json.dumps(measurement, indent=2) + "\n", encoding="utf-8"
    )
    print(json.dumps(measurement, indent=2))
    if process.returncode:
        print(f"validation failed; see {output / 'stderr.log'}", file=sys.stderr)
    return int(process.returncode or 0)


if __name__ == "__main__":
    raise SystemExit(main())
