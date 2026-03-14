"""Day 22: Reproducible QC Pipeline — Python equivalent.

Reads config from config.json, processes FASTQ files with quality/length
filtering, computes summary statistics, writes CSV output and a provenance
JSON log with SHA-256 checksums for all inputs and outputs.
"""

import json
import hashlib
import csv
import os
import time
import logging
from datetime import datetime
from pathlib import Path
from statistics import mean, stdev, median
from typing import Any

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(message)s",
    datefmt="%Y-%m-%d %H:%M:%S",
)
log = logging.getLogger("fastq_qc")


def load_config(path: str) -> dict:
    with open(path) as f:
        return json.load(f)


def validate_config(config: dict) -> list[str]:
    errors = []
    required = [
        "pipeline_name", "version", "input_files",
        "output_dir", "min_quality", "min_length",
    ]
    for key in required:
        if key not in config:
            errors.append(f"Missing required field: {key}")
    for f in config.get("input_files", []):
        if not os.path.exists(f):
            errors.append(f"Missing input file: {f}")
    mq = config.get("min_quality", 0)
    if mq < 0 or mq > 40:
        errors.append(f"min_quality must be 0-40, got {mq}")
    ml = config.get("min_length", 1)
    if ml < 1:
        errors.append(f"min_length must be >= 1, got {ml}")
    return errors


def sha256_file(path: str) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(8192), b""):
            h.update(chunk)
    return h.hexdigest()


def checksum_files(paths: list[str]) -> list[dict]:
    return [{"file": p, "sha256": sha256_file(p)} for p in paths]


def parse_fastq(path: str) -> list[dict]:
    reads = []
    with open(path) as f:
        while True:
            header = f.readline().strip()
            if not header:
                break
            seq = f.readline().strip()
            f.readline()  # +
            qual_str = f.readline().strip()
            quals = [ord(c) - 33 for c in qual_str]
            reads.append({"id": header[1:], "seq": seq, "qual": quals})
    return reads


def gc_content(seq: str) -> float:
    if not seq:
        return 0.0
    gc = sum(1 for c in seq.upper() if c in "GC")
    return gc / len(seq)


def process_sample(file_path: str, config: dict) -> dict:
    t0 = time.time()
    reads = parse_fastq(file_path)
    total_count = len(reads)

    min_qual = config["min_quality"]
    min_len = config["min_length"]

    filtered = [r for r in reads if mean(r["qual"]) >= min_qual]
    length_filtered = [r for r in filtered if len(r["seq"]) >= min_len]
    pass_count = len(length_filtered)

    gc_values = [gc_content(r["seq"]) for r in length_filtered]
    lengths = [len(r["seq"]) for r in length_filtered]
    qualities = [mean(r["qual"]) for r in length_filtered]

    elapsed = time.time() - t0

    return {
        "file": file_path,
        "total_reads": total_count,
        "passed_reads": pass_count,
        "pass_rate": pass_count / total_count if total_count else 0,
        "gc_mean": mean(gc_values) if gc_values else 0,
        "gc_stdev": stdev(gc_values) if len(gc_values) > 1 else 0,
        "length_mean": mean(lengths) if lengths else 0,
        "length_min": min(lengths) if lengths else 0,
        "length_max": max(lengths) if lengths else 0,
        "quality_mean": mean(qualities) if qualities else 0,
        "elapsed_seconds": elapsed,
    }


def create_provenance(config: dict) -> dict:
    return {
        "pipeline": config["pipeline_name"],
        "version": config["version"],
        "started_at": datetime.now().strftime("%Y-%m-%d %H:%M:%S"),
        "parameters": config,
        "input_checksums": [],
        "steps": [],
        "output_checksums": [],
        "finished_at": None,
        "status": "running",
    }


def log_step(prov: dict, step_name: str, details: Any) -> dict:
    prov["steps"].append({
        "name": step_name,
        "timestamp": datetime.now().strftime("%Y-%m-%d %H:%M:%S"),
        "details": details,
    })
    return prov


def finish_provenance(prov: dict, status: str) -> dict:
    prov["finished_at"] = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    prov["status"] = status
    return prov


def save_provenance(prov: dict, log_dir: str) -> str:
    ts = datetime.now().strftime("%Y%m%d_%H%M%S")
    filename = os.path.join(log_dir, f"provenance_{ts}.json")
    with open(filename, "w") as f:
        json.dump(prov, f, indent=2)
    return filename


def main():
    config = load_config("config.json")

    errors = validate_config(config)
    if errors:
        for e in errors:
            log.error(e)
        raise SystemExit("Configuration invalid")

    os.makedirs(config["output_dir"], exist_ok=True)
    os.makedirs(config.get("log_dir", "logs"), exist_ok=True)

    prov = create_provenance(config)
    log.info("Pipeline %s v%s started", config["pipeline_name"], config["version"])

    input_checksums = checksum_files(config["input_files"])
    prov["input_checksums"] = input_checksums
    log_step(prov, "checksum_inputs", {"file_count": len(input_checksums)})
    for c in input_checksums:
        log.info("  Input: %s -> %s", c["file"], c["sha256"])

    results = []
    for f in config["input_files"]:
        log.info("Processing: %s", f)
        result = process_sample(f, config)
        log.info(
            "  %d/%d reads passed (%d%%)",
            result["passed_reads"],
            result["total_reads"],
            int(result["pass_rate"] * 100),
        )
        results.append(result)

    log_step(prov, "process_samples", {
        "sample_count": len(results),
        "total_reads": sum(r["total_reads"] for r in results),
        "total_passed": sum(r["passed_reads"] for r in results),
    })

    output_path = os.path.join(config["output_dir"], "qc_summary.csv")
    fieldnames = [
        "file", "total_reads", "passed_reads", "pass_rate",
        "gc_mean", "length_mean", "quality_mean",
    ]
    with open(output_path, "w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        writer.writeheader()
        for r in results:
            writer.writerow({k: r[k] for k in fieldnames})

    log.info("Summary written to: %s", output_path)
    log_step(prov, "write_results", {"output_file": output_path})

    output_checksums = checksum_files([output_path])
    prov["output_checksums"] = output_checksums
    finish_provenance(prov, "success")

    log_dir = config.get("log_dir", "logs")
    prov_file = save_provenance(prov, log_dir)
    log.info("Provenance saved to: %s", prov_file)
    log.info("Pipeline completed successfully.")


if __name__ == "__main__":
    main()
