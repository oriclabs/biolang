#!/usr/bin/env python3
"""Run the GPL R oracle and MIT BioLang implementation as separate processes.

The driver deliberately communicates through fixture/result files. It does not
import, link, or translate either implementation. Each process is measured on
the same host, and the numeric comparator adds resource gates when both
measurements are available.
"""

from __future__ import annotations

import argparse
import csv
import ctypes
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import platform
import shutil
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


class _ProcessEntry32W(ctypes.Structure):
    _fields_ = [
        ("dwSize", ctypes.c_ulong),
        ("cntUsage", ctypes.c_ulong),
        ("th32ProcessID", ctypes.c_ulong),
        ("th32DefaultHeapID", ctypes.c_size_t),
        ("th32ModuleID", ctypes.c_ulong),
        ("cntThreads", ctypes.c_ulong),
        ("th32ParentProcessID", ctypes.c_ulong),
        ("pcPriClassBase", ctypes.c_long),
        ("dwFlags", ctypes.c_ulong),
        ("szExeFile", ctypes.c_wchar * 260),
    ]


def normalized_environment() -> dict[str, str]:
    """Collapse case-duplicate Windows keys such as Path/PATH."""
    if os.name != "nt":
        return dict(os.environ)
    result: dict[str, tuple[str, str]] = {}
    for key, value in os.environ.items():
        result[key.casefold()] = (key, value)
    return {key: value for key, value in result.values()}


def windows_process_tree(root_pid: int) -> set[int]:
    kernel32 = ctypes.windll.kernel32
    kernel32.CreateToolhelp32Snapshot.restype = ctypes.c_void_p
    kernel32.Process32FirstW.argtypes = [ctypes.c_void_p, ctypes.POINTER(_ProcessEntry32W)]
    kernel32.Process32NextW.argtypes = [ctypes.c_void_p, ctypes.POINTER(_ProcessEntry32W)]
    kernel32.CloseHandle.argtypes = [ctypes.c_void_p]
    snapshot = kernel32.CreateToolhelp32Snapshot(0x00000002, 0)
    if snapshot == ctypes.c_void_p(-1).value:
        return {root_pid}
    parent_by_pid: dict[int, int] = {}
    entry = _ProcessEntry32W()
    entry.dwSize = ctypes.sizeof(entry)
    try:
        found = kernel32.Process32FirstW(snapshot, ctypes.byref(entry))
        while found:
            parent_by_pid[int(entry.th32ProcessID)] = int(entry.th32ParentProcessID)
            found = kernel32.Process32NextW(snapshot, ctypes.byref(entry))
    finally:
        kernel32.CloseHandle(snapshot)
    tree = {root_pid}
    changed = True
    while changed:
        changed = False
        for pid, parent in parent_by_pid.items():
            if parent in tree and pid not in tree:
                tree.add(pid)
                changed = True
    return tree


def windows_pid_memory(pid: int) -> tuple[int, int]:
    kernel32 = ctypes.windll.kernel32
    kernel32.OpenProcess.restype = ctypes.c_void_p
    kernel32.CloseHandle.argtypes = [ctypes.c_void_p]
    handle = kernel32.OpenProcess(0x0410, False, pid)
    if not handle:
        return 0, 0
    counters = _ProcessMemoryCounters()
    counters.cb = ctypes.sizeof(counters)
    try:
        query = ctypes.windll.psapi.GetProcessMemoryInfo
        if query(handle, ctypes.byref(counters), counters.cb):
            return int(counters.WorkingSetSize), int(counters.PeakWorkingSetSize)
        return 0, 0
    finally:
        kernel32.CloseHandle(handle)


def process_memory_bytes(process: subprocess.Popen[bytes]) -> tuple[int, int]:
    """Return summed current and recorded-peak RSS for the process tree."""
    if os.name == "nt":
        observations = [windows_pid_memory(pid) for pid in windows_process_tree(process.pid)]
        return sum(value[0] for value in observations), sum(value[1] for value in observations)

    status = Path(f"/proc/{process.pid}/status")
    if status.exists():
        pending = [process.pid]
        seen: set[int] = set()
        current_total = 0
        peak_total = 0
        while pending:
            pid = pending.pop()
            if pid in seen:
                continue
            seen.add(pid)
            try:
                children = Path(f"/proc/{pid}/task/{pid}/children").read_text(
                    encoding="utf-8"
                )
                pending.extend(int(value) for value in children.split())
                for line in Path(f"/proc/{pid}/status").read_text(
                    encoding="utf-8"
                ).splitlines():
                    if line.startswith(("VmRSS:", "VmHWM:")):
                        name, amount, _unit = line.split()
                        if name == "VmRSS:":
                            current_total += int(amount) * 1024
                        else:
                            peak_total += int(amount) * 1024
            except (OSError, ValueError):
                continue
        return current_total, peak_total

    # macOS and other Unix platforms do not expose /proc. Polling `ps` keeps
    # this dependency-free; the observed maximum is labelled as sampled below.
    try:
        output = subprocess.check_output(
            ["ps", "-o", "rss=", "-p", str(process.pid)],
            text=True,
            stderr=subprocess.DEVNULL,
        )
        rss = int(output.strip()) * 1024
        return rss, 0
    except (OSError, subprocess.SubprocessError, ValueError):
        return 0, 0


def run_measured(
    command: list[str],
    *,
    cwd: Path,
    environment: dict[str, str],
    stdout_path: Path,
    stderr_path: Path,
    resource_path: Path,
    label: str,
    poll_interval: float = 0.05,
) -> dict[str, object]:
    started = time.perf_counter()
    sampled_peak = 0
    os_peak = 0
    creation_flags = subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0
    with stdout_path.open("wb") as stdout, stderr_path.open("wb") as stderr:
        process = subprocess.Popen(
            command,
            cwd=cwd,
            env=environment,
            stdout=stdout,
            stderr=stderr,
            creationflags=creation_flags,
        )
        while process.poll() is None:
            current, recorded_peak = process_memory_bytes(process)
            sampled_peak = max(sampled_peak, current)
            os_peak = max(os_peak, recorded_peak)
            time.sleep(poll_interval)
        current, recorded_peak = process_memory_bytes(process)
        sampled_peak = max(sampled_peak, current)
        os_peak = max(os_peak, recorded_peak)

    peak = max(sampled_peak, os_peak)
    measurement: dict[str, object] = {
        "label": label,
        "command": command,
        "exit_code": int(process.returncode or 0),
        "wall_seconds": round(time.perf_counter() - started, 6),
        "peak_working_set_bytes": peak,
        "peak_working_set_gib": round(peak / 1024**3, 6),
        "sampled_peak_working_set_bytes": sampled_peak,
        "os_recorded_peak_working_set_bytes": os_peak,
        "poll_interval_seconds": poll_interval,
        "platform": platform.platform(),
        "measurement_scope": "process_tree",
        "stdout": str(stdout_path),
        "stderr": str(stderr_path),
    }
    resource_path.parent.mkdir(parents=True, exist_ok=True)
    resource_path.write_text(json.dumps(measurement, indent=2) + "\n", encoding="utf-8")
    return measurement


def rscript_candidates() -> list[Path]:
    candidates: list[Path] = []
    on_path = shutil.which("Rscript")
    if on_path:
        candidates.append(Path(on_path))
    if os.name == "nt":
        program_files = Path(os.environ.get("ProgramFiles", "C:/Program Files"))
        candidates.extend(
            sorted(
                (program_files / "R").glob("R-*/bin/Rscript.exe"),
                reverse=True,
            )
        )
    result: list[Path] = []
    seen: set[str] = set()
    for candidate in candidates:
        key = str(candidate.resolve()).casefold()
        if key not in seen and candidate.is_file():
            seen.add(key)
            result.append(candidate.resolve())
    return result


def resolve_rscript(explicit: str | None, environment: dict[str, str]) -> Path:
    if explicit:
        path = Path(explicit).resolve()
        if not path.is_file():
            raise SystemExit(f"Rscript does not exist: {path}")
        return path
    probe = (
        "p<-Sys.getenv('BIOLANG_VALIDATION_R_LIB');"
        "if(nzchar(p)) .libPaths(c(p,.libPaths()));"
        "quit(status=if (requireNamespace('sctransform',quietly=TRUE) && "
        "requireNamespace('Matrix',quietly=TRUE)) 0 else 1)"
    )
    for candidate in rscript_candidates():
        completed = subprocess.run(
            [str(candidate), "-e", probe],
            env=environment,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        )
        if completed.returncode == 0:
            return candidate
    raise SystemExit(
        "no Rscript with the separately installed sctransform and Matrix packages was found; "
        "pass --rscript and, if needed, set BIOLANG_VALIDATION_R_LIB"
    )


def resolve_executable(repo: Path, value: str) -> Path:
    path = Path(value)
    candidate = path if path.is_absolute() else repo / path
    if candidate.is_file():
        return candidate.resolve()
    on_path = shutil.which(value)
    if on_path:
        return Path(on_path).resolve()
    raise SystemExit(
        f"BioLang executable does not exist and is not on PATH: {value}; "
        "pass --executable or set BIOLANG_EXE"
    )


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Run and compare independent SCTransform implementations"
    )
    parser.add_argument(
        "--mode", choices=("synthetic", "sampling", "tenx"), default="synthetic"
    )
    parser.add_argument("--input", help="10x MEX directory; required for --mode tenx")
    parser.add_argument("--output", required=True, help="fresh validation result directory")
    parser.add_argument(
        "--executable",
        default=os.environ.get(
            "BIOLANG_EXE",
            "target/release/bl.exe" if os.name == "nt" else "target/release/bl",
        ),
    )
    parser.add_argument("--rscript")
    parser.add_argument(
        "--r-library",
        help="validation-only R library prepended to .libPaths()",
    )
    parser.add_argument(
        "--oracle-method",
        choices=("glmGamPoi_offset", "nb_offset", "any"),
        default="glmGamPoi_offset",
        help="required method reported by sctransform; default is the calibrated backend",
    )
    parser.add_argument("--gpu", choices=("auto", "off", "on"), default="off")
    args = parser.parse_args()

    script_dir = Path(__file__).resolve().parent
    repo = script_dir.parents[3]
    output = Path(args.output)
    if not output.is_absolute():
        output = repo / output
    output = output.resolve()
    if output.exists():
        raise SystemExit(f"output must not already exist: {output}")
    if args.mode == "tenx" and not args.input:
        raise SystemExit("--input is required for --mode tenx")
    output.mkdir(parents=True)

    environment = normalized_environment()
    if args.r_library:
        r_library = Path(args.r_library).resolve()
        if not r_library.is_dir():
            raise SystemExit(f"R validation library does not exist: {r_library}")
        environment["BIOLANG_VALIDATION_R_LIB"] = str(r_library)
    validation_library = environment.get("BIOLANG_VALIDATION_R_LIB", "")
    bundled_validation_library = repo / ".validation-r-library"
    if not validation_library and bundled_validation_library.is_dir():
        environment["BIOLANG_VALIDATION_R_LIB"] = str(bundled_validation_library)

    rscript = resolve_rscript(args.rscript, environment)
    executable = resolve_executable(repo, args.executable)
    oracle_dir = output / "oracle"
    biolang_dir = output / "biolang"
    fixture_dir = (
        Path(args.input).resolve() if args.mode == "tenx" else output / "fixture"
    )
    if args.mode == "tenx" and not fixture_dir.is_dir():
        raise SystemExit(f"10x input directory does not exist: {fixture_dir}")

    oracle_command = [
        str(rscript),
        str(script_dir / "sctransform_oracle.R"),
        args.mode,
        str(fixture_dir),
        str(oracle_dir),
        "3000",
        "64",
    ]
    print("running standalone R oracle...", flush=True)
    oracle_resources = run_measured(
        oracle_command,
        cwd=repo,
        environment=environment,
        stdout_path=output / "oracle.stdout.log",
        stderr_path=output / "oracle.stderr.log",
        resource_path=oracle_dir / "resources.json",
        label="sctransform R oracle",
    )
    if oracle_resources["exit_code"] != 0:
        print(f"R oracle failed; see {output / 'oracle.stderr.log'}", file=sys.stderr)
        return int(oracle_resources["exit_code"])
    with (oracle_dir / "manifest.csv").open(
        newline="", encoding="utf-8-sig"
    ) as handle:
        oracle_manifest = next(csv.DictReader(handle))
    actual_method = oracle_manifest.get("actual_method", "")
    if args.oracle_method != "any" and actual_method != args.oracle_method:
        print(
            f"R oracle selected {actual_method!r}, expected {args.oracle_method!r}; "
            "install/select the pinned validation backend or pass --oracle-method explicitly",
            file=sys.stderr,
        )
        return 3

    biolang_dir.mkdir()
    biolang_environment = dict(environment)
    biolang_environment["BIOLANG_GPU"] = args.gpu
    biolang_environment["BIOLANG_SCT_INPUT"] = str(fixture_dir)
    biolang_environment["BIOLANG_SCT_OUTPUT"] = str(biolang_dir)
    biolang_command = [
        str(executable),
        "run",
        str(script_dir / "sctransform_biolang.bl"),
    ]
    print("running standalone BioLang implementation...", flush=True)
    biolang_resources = run_measured(
        biolang_command,
        cwd=repo,
        environment=biolang_environment,
        stdout_path=output / "biolang.stdout.log",
        stderr_path=output / "biolang.stderr.log",
        resource_path=biolang_dir / "resources.json",
        label="BioLang SCTransform",
    )
    if biolang_resources["exit_code"] != 0:
        print(f"BioLang run failed; see {output / 'biolang.stderr.log'}", file=sys.stderr)
        return int(biolang_resources["exit_code"])

    comparison_path = output / "comparison.json"
    comparator_command = [
        sys.executable,
        str(script_dir / "compare_sctransform_results.py"),
        str(oracle_dir),
        str(biolang_dir),
        str(comparison_path),
        "3000",
    ]
    comparison = subprocess.run(comparator_command, cwd=repo, check=False)
    run_manifest = {
        "created_utc": datetime.now(timezone.utc).isoformat(),
        "mode": args.mode,
        "input": str(fixture_dir),
        "output": str(output),
        "gpu": args.gpu,
        "oracle_method": actual_method,
        "rscript": str(rscript),
        "biolang_executable": str(executable),
        "oracle_exit_code": oracle_resources["exit_code"],
        "biolang_exit_code": biolang_resources["exit_code"],
        "comparison_exit_code": comparison.returncode,
        "comparison": str(comparison_path),
    }
    (output / "run.json").write_text(
        json.dumps(run_manifest, indent=2) + "\n", encoding="utf-8"
    )
    if comparison.returncode == 0:
        print(f"validation passed: {comparison_path}")
    else:
        print(f"validation completed with failed gates: {comparison_path}")
    return comparison.returncode


if __name__ == "__main__":
    raise SystemExit(main())
