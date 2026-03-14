#!/usr/bin/env python3
"""Day 25: Error Handling in Production - Python equivalent."""

import csv
import json
import logging
import os
import sys
import time
from datetime import datetime
from pathlib import Path
from typing import Any, Callable, TypeVar

from tenacity import retry, stop_after_attempt, wait_exponential, retry_if_exception_type

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s %(levelname)s %(message)s",
    handlers=[
        logging.FileHandler("data/output/pipeline.log"),
        logging.StreamHandler(),
    ],
)
logger = logging.getLogger(__name__)

T = TypeVar("T")


class TransientError(Exception):
    """Retryable error."""
    pass


class DataError(Exception):
    """Non-retryable data corruption."""
    pass


def classify_error(err_msg: str) -> str:
    """Classify an error message into a category."""
    msg = str(err_msg).lower()
    if "not found" in msg or "no such file" in msg:
        return "missing"
    elif "permission" in msg:
        return "access"
    elif "timeout" in msg:
        return "transient"
    elif "parse" in msg or "invalid" in msg:
        return "data_corrupt"
    elif "disk" in msg or "space" in msg or "quota" in msg:
        return "resource"
    else:
        return "unknown"


def validate_fastq_file(path: str) -> bool:
    """Validate that a FASTQ file exists and is non-empty."""
    p = Path(path)
    if not p.exists():
        raise FileNotFoundError(f"File not found: {path}")
    if not path.endswith(".fastq"):
        raise ValueError(f"Expected .fastq file, got: {path}")
    if p.stat().st_size == 0:
        raise ValueError(f"File is empty: {path}")
    return True


def parse_fastq(path: str) -> list[dict]:
    """Parse a FASTQ file into records."""
    records = []
    with open(path, "r") as f:
        lines = [line.rstrip("\n") for line in f if line.strip()]

    i = 0
    while i + 3 < len(lines):
        if not lines[i].startswith("@"):
            raise DataError(f"Expected @ at line {i + 1}, got: {lines[i][:20]}")
        record = {
            "id": lines[i][1:],
            "sequence": lines[i + 1],
            "quality": lines[i + 3],
        }
        records.append(record)
        i += 4

    return records


def gc_content(seq: str) -> float:
    """Calculate GC content of a sequence."""
    if len(seq) == 0:
        return 0.0
    gc = sum(1 for c in seq.upper() if c in ("G", "C"))
    return gc / len(seq)


def quality_filter(records: list[dict], min_qual: int = 20) -> list[dict]:
    """Filter records by minimum average quality score."""
    passed = []
    for rec in records:
        quals = [ord(c) - 33 for c in rec["quality"]]
        if quals and (sum(quals) / len(quals)) >= min_qual:
            passed.append(rec)
    return passed


class ErrorLog:
    """Structured error log that writes to CSV."""

    def __init__(self):
        self.entries: list[dict] = []

    def log(self, source: str, severity: str, message: str):
        entry = {
            "timestamp": datetime.now().strftime("%Y-%m-%d %H:%M:%S"),
            "source": source,
            "severity": severity,
            "message": message,
        }
        self.entries.append(entry)
        if severity == "ERROR":
            logger.error(f"{source}: {message}")
        elif severity == "WARN":
            logger.warning(f"{source}: {message}")
        else:
            logger.info(f"{source}: {message}")

    def save(self, path: str):
        with open(path, "w", newline="") as f:
            writer = csv.DictWriter(
                f, fieldnames=["timestamp", "source", "severity", "message"]
            )
            writer.writeheader()
            for entry in self.entries:
                writer.writerow(entry)


@retry(
    stop=stop_after_attempt(3),
    wait=wait_exponential(multiplier=1, min=1, max=30),
    retry=retry_if_exception_type(TransientError),
)
def read_with_retry(path: str) -> list[dict]:
    """Read FASTQ with retry for transient errors."""
    return parse_fastq(path)


def process_file(path: str, error_log: ErrorLog) -> dict | None:
    """Process a single FASTQ file with full error handling."""
    filename = os.path.basename(path)

    try:
        validate_fastq_file(path)
    except (FileNotFoundError, ValueError) as e:
        error_log.log(filename, "ERROR", str(e))
        return None

    try:
        records = parse_fastq(path)
    except DataError as e:
        error_log.log(filename, "ERROR", f"FASTQ parse failed: {e}")
        return None
    except Exception as e:
        error_log.log(filename, "ERROR", f"Unexpected read error: {e}")
        return None

    if len(records) == 0:
        error_log.log(filename, "WARN", "No records found, skipping")
        return None

    valid = [r for r in records if len(r["sequence"]) == len(r["quality"])]
    if len(valid) < len(records):
        dropped = len(records) - len(valid)
        error_log.log(
            filename, "WARN", f"{dropped} records had seq/qual length mismatch"
        )

    filtered = quality_filter(valid, min_qual=20)
    if len(filtered) == 0:
        error_log.log(filename, "WARN", "All records filtered out by quality threshold")

    gc_values = [gc_content(r["sequence"]) for r in filtered] if filtered else []
    mean_gc = sum(gc_values) / len(gc_values) if gc_values else 0.0

    return {
        "file": filename,
        "total_records": len(records),
        "valid_records": len(valid),
        "passed_qc": len(filtered),
        "pct_passed": (len(filtered) * 100 // len(valid)) if valid else 0,
        "mean_gc": round(mean_gc, 4),
    }


def summarize_run(
    total: int, successes: int, failures: int, error_log: ErrorLog
) -> dict:
    """Generate pipeline run summary."""
    success_rate = (successes * 100 // total) if total > 0 else 0
    if failures == 0:
        status = "COMPLETE"
    elif success_rate > 90:
        status = "PARTIAL_SUCCESS"
    else:
        status = "FAILED"

    return {
        "total_samples": total,
        "succeeded": successes,
        "failed": failures,
        "success_rate_pct": success_rate,
        "error_count": len(error_log.entries),
        "status": status,
    }


def main():
    input_dir = "data/fastq"
    output_dir = "data/output"
    os.makedirs(output_dir, exist_ok=True)

    error_log = ErrorLog()
    results = []

    try:
        files = sorted(
            f for f in os.listdir(input_dir) if f.endswith(".fastq")
        )
    except OSError as e:
        error_log.log(input_dir, "FATAL", f"Cannot list directory: {e}")
        error_log.save(os.path.join(output_dir, "error_log.csv"))
        sys.exit(1)

    if not files:
        logger.error(f"No FASTQ files found in {input_dir}")
        sys.exit(1)

    for filename in files:
        path = os.path.join(input_dir, filename)
        result = process_file(path, error_log)
        if result is not None:
            results.append(result)

    summary = summarize_run(len(files), len(results), len(files) - len(results), error_log)

    if results:
        with open(os.path.join(output_dir, "qc_results.csv"), "w", newline="") as f:
            writer = csv.DictWriter(
                f,
                fieldnames=[
                    "file", "total_records", "valid_records",
                    "passed_qc", "pct_passed", "mean_gc",
                ],
            )
            writer.writeheader()
            for row in results:
                writer.writerow(row)

    error_log.save(os.path.join(output_dir, "error_log.csv"))

    with open(os.path.join(output_dir, "summary.json"), "w") as f:
        json.dump(summary, f, indent=2)

    logger.info(
        f"Pipeline {summary['status']}: "
        f"{summary['succeeded']}/{summary['total_samples']} samples processed"
    )


if __name__ == "__main__":
    main()
