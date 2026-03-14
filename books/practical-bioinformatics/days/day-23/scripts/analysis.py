"""Day 23: Batch Processing and Automation — Python equivalent.

Parses a sample sheet, processes all samples with quality filtering,
tracks errors per sample, aggregates results into per-sample and
per-group summaries, flags outliers, and writes a batch report.
"""

import csv
import json
import os
import time
from collections import defaultdict
from datetime import datetime
from glob import glob
from multiprocessing import Pool, cpu_count
from pathlib import Path
from statistics import mean, stdev
from typing import Any


def load_config(path: str) -> dict:
    with open(path) as f:
        return json.load(f)


def parse_sample_sheet(path: str) -> list[dict]:
    samples = []
    with open(path) as f:
        reader = csv.DictReader(f)
        for row in reader:
            samples.append({
                "id": row["sample_id"],
                "fastq": row["fastq_file"],
                "tissue": row["tissue"],
                "group": row["group"],
            })
    return samples


def validate_sample_sheet(samples: list[dict]) -> list[str]:
    missing = []
    for s in samples:
        if not os.path.exists(s["fastq"]):
            missing.append(s["fastq"])
    return missing


def parse_fastq(path: str) -> list[dict]:
    reads = []
    with open(path) as f:
        while True:
            header = f.readline().strip()
            if not header:
                break
            seq = f.readline().strip()
            f.readline()
            qual_str = f.readline().strip()
            quals = [ord(c) - 33 for c in qual_str]
            reads.append({"id": header[1:], "seq": seq, "qual": quals})
    return reads


def gc_content(seq: str) -> float:
    if not seq:
        return 0.0
    gc = sum(1 for c in seq.upper() if c in "GC")
    return gc / len(seq)


def process_sample(args: tuple) -> dict:
    sample, config = args
    t0 = time.time()
    try:
        reads = parse_fastq(sample["fastq"])
        total = len(reads)

        min_qual = config["min_quality"]
        min_len = config["min_length"]

        filtered = [r for r in reads if mean(r["qual"]) >= min_qual]
        passed = [r for r in filtered if len(r["seq"]) >= min_len]
        pass_count = len(passed)

        gc_values = [gc_content(r["seq"]) for r in passed]
        lengths = [len(r["seq"]) for r in passed]

        return {
            "sample_id": sample["id"],
            "tissue": sample["tissue"],
            "group": sample["group"],
            "total_reads": total,
            "passed_reads": pass_count,
            "pass_rate": pass_count / total if total else 0,
            "gc_mean": mean(gc_values) if gc_values else 0,
            "gc_stdev": stdev(gc_values) if len(gc_values) > 1 else 0,
            "length_mean": mean(lengths) if lengths else 0,
            "length_min": min(lengths) if lengths else 0,
            "length_max": max(lengths) if lengths else 0,
            "elapsed": time.time() - t0,
            "status": "ok",
            "error": None,
        }
    except Exception as e:
        return {
            "sample_id": sample["id"],
            "tissue": sample["tissue"],
            "group": sample["group"],
            "status": "error",
            "error": str(e),
            "total_reads": 0,
            "passed_reads": 0,
            "pass_rate": 0,
            "gc_mean": 0,
            "gc_stdev": 0,
            "length_mean": 0,
            "length_min": 0,
            "length_max": 0,
            "elapsed": time.time() - t0,
        }


def flag_outliers(results: list[dict], field: str) -> list[str]:
    values = [r[field] for r in results if r["status"] == "ok"]
    if len(values) < 3:
        return []
    m = mean(values)
    s = stdev(values)
    lower = m - 2 * s
    upper = m + 2 * s
    return [
        r["sample_id"]
        for r in results
        if r["status"] == "ok" and (r[field] < lower or r[field] > upper)
    ]


def summarize_by_group(results: list[dict]) -> list[dict]:
    groups: dict[str, list] = defaultdict(list)
    for r in results:
        if r["status"] == "ok":
            groups[r["group"]].append(r)

    summaries = []
    for group_name, group_results in sorted(groups.items()):
        summaries.append({
            "group": group_name,
            "n_samples": len(group_results),
            "mean_pass_rate": mean(r["pass_rate"] for r in group_results),
            "mean_gc": mean(r["gc_mean"] for r in group_results),
            "mean_reads": mean(r["total_reads"] for r in group_results),
        })
    return summaries


def main():
    config = load_config("config.json")
    samples = parse_sample_sheet(config["sample_sheet"])

    missing = validate_sample_sheet(samples)
    if missing:
        raise SystemExit(f"Missing input files: {missing}")

    os.makedirs(config["output_dir"], exist_ok=True)
    os.makedirs(config["log_dir"], exist_ok=True)

    t0 = time.time()
    args_list = [(s, config) for s in samples]

    with Pool(processes=min(cpu_count(), len(samples))) as pool:
        all_results = pool.map(process_sample, args_list)

    total_time = time.time() - t0

    results = [r for r in all_results if r["status"] == "ok"]
    errors = [
        {"sample_id": r["sample_id"], "error": r["error"]}
        for r in all_results
        if r["status"] == "error"
    ]

    # Per-sample summary
    summary_path = os.path.join(config["output_dir"], "batch_summary.csv")
    fieldnames = [
        "sample_id", "tissue", "group", "total_reads",
        "passed_reads", "pass_rate", "gc_mean", "length_mean",
    ]
    with open(summary_path, "w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        writer.writeheader()
        for r in results:
            writer.writerow({k: r[k] for k in fieldnames})

    # Group summary
    group_stats = summarize_by_group(all_results)
    group_path = os.path.join(config["output_dir"], "group_summary.csv")
    group_fields = ["group", "n_samples", "mean_pass_rate", "mean_gc", "mean_reads"]
    with open(group_path, "w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=group_fields)
        writer.writeheader()
        for g in group_stats:
            writer.writerow(g)

    # Outlier detection
    gc_outliers = flag_outliers(results, "gc_mean")
    rate_outliers = flag_outliers(results, "pass_rate")

    # Batch report
    report = {
        "timestamp": datetime.now().strftime("%Y-%m-%d %H:%M:%S"),
        "total_samples": len(samples),
        "succeeded": len(results),
        "failed": len(errors),
        "total_time": total_time,
        "gc_outliers": gc_outliers,
        "rate_outliers": rate_outliers,
        "errors": errors,
    }
    report_path = os.path.join(config["log_dir"], "batch_report.json")
    with open(report_path, "w") as f:
        json.dump(report, f, indent=2)

    print(f"Batch complete: {len(results)}/{len(samples)} succeeded in {total_time:.1f}s")
    print(f"Summary: {summary_path}")
    print(f"Report:  {report_path}")


if __name__ == "__main__":
    main()
