"""Module-based sequence analysis pipeline."""

import csv
import os
import sys
from pathlib import Path


# --- seq_utils module equivalent ---

def validate_dna(seq):
    upper_seq = seq.upper()
    valid = set("ACGTN")
    invalid = [c for c in upper_seq if c not in valid]
    if invalid:
        raise ValueError(f"Invalid DNA characters: {', '.join(invalid)}")
    return upper_seq


def gc_content(seq):
    if not seq:
        return 0.0
    gc = sum(1 for c in seq.upper() if c in "GC")
    return gc / len(seq)


def classify_gc(seq):
    clean = validate_dna(seq)
    gc = gc_content(clean)
    if gc > 0.6:
        return {"class": "high", "gc": gc, "label": "GC-rich"}
    elif gc < 0.4:
        return {"class": "low", "gc": gc, "label": "AT-rich"}
    else:
        return {"class": "moderate", "gc": gc, "label": "balanced"}


def find_all_motifs(seq, motif):
    clean = validate_dna(seq)
    motif_upper = motif.upper()
    positions = []
    start = 0
    while True:
        pos = clean.find(motif_upper, start)
        if pos == -1:
            break
        positions.append(pos)
        start = pos + 1
    return {
        "motif": motif_upper,
        "count": len(positions),
        "positions": positions,
    }


def batch_gc(sequences):
    results = []
    for seq in sequences:
        result = classify_gc(seq["sequence"])
        results.append({
            "id": seq["id"],
            "length": len(seq["sequence"]),
            "gc": result["gc"],
            "class": result["class"],
            "label": result["label"],
        })
    return results


def sequence_summary(sequences):
    classified = batch_gc(sequences)
    high = sum(1 for s in classified if s["class"] == "high")
    low = sum(1 for s in classified if s["class"] == "low")
    moderate = sum(1 for s in classified if s["class"] == "moderate")
    gc_values = [s["gc"] for s in classified]
    mean_gc = sum(gc_values) / len(gc_values) if gc_values else 0.0
    variance = sum((g - mean_gc) ** 2 for g in gc_values) / len(gc_values) if gc_values else 0.0
    stdev_gc = variance ** 0.5
    return {
        "total": len(sequences),
        "high_gc": high,
        "low_gc": low,
        "moderate_gc": moderate,
        "mean_gc": mean_gc,
        "stdev_gc": stdev_gc,
    }


# --- qc module equivalent ---

def length_stats(sequences):
    lengths = [len(s["sequence"]) for s in sequences]
    sorted_lengths = sorted(lengths)
    n = len(sorted_lengths)
    median_len = sorted_lengths[n // 2] if n % 2 else (sorted_lengths[n // 2 - 1] + sorted_lengths[n // 2]) / 2
    return {
        "count": len(lengths),
        "min_len": min(lengths),
        "max_len": max(lengths),
        "mean_len": sum(lengths) / len(lengths),
        "median_len": median_len,
    }


def gc_distribution(sequences):
    gc_values = [gc_content(s["sequence"]) for s in sequences]
    mean_gc = sum(gc_values) / len(gc_values)
    variance = sum((g - mean_gc) ** 2 for g in gc_values) / len(gc_values)
    return {
        "mean_gc": mean_gc,
        "min_gc": min(gc_values),
        "max_gc": max(gc_values),
        "stdev_gc": variance ** 0.5,
    }


def flag_outliers(sequences, min_len, max_len, min_gc, max_gc):
    results = []
    for s in sequences:
        gc = gc_content(s["sequence"])
        slen = len(s["sequence"])
        flags = []
        if slen < min_len:
            flags.append("too_short")
        if slen > max_len:
            flags.append("too_long")
        if gc < min_gc:
            flags.append("low_gc")
        if gc > max_gc:
            flags.append("high_gc")
        results.append({
            "id": s["id"],
            "length": slen,
            "gc": gc,
            "flags": flags,
            "pass": len(flags) == 0,
        })
    return results


def qc_summary(sequences):
    lstats = length_stats(sequences)
    gc_dist = gc_distribution(sequences)
    flagged = flag_outliers(sequences, 50, 10000, 0.2, 0.8)
    passing = sum(1 for f in flagged if f["pass"])
    failing = sum(1 for f in flagged if not f["pass"])
    return {
        "total": lstats["count"],
        "passing": passing,
        "failing": failing,
        "pass_rate": passing / lstats["count"] if lstats["count"] else 0,
        "length": lstats,
        "gc": gc_dist,
    }


def format_qc_report(summary):
    return [
        f"Sequences: {summary['total']}",
        f"Passing QC: {summary['passing']}",
        f"Failing QC: {summary['failing']}",
        f"Length range: {summary['length']['min_len']}-{summary['length']['max_len']}",
        f"Mean length: {summary['length']['mean_len']}",
        f"Mean GC: {summary['gc']['mean_gc']}",
        f"GC stdev: {summary['gc']['stdev_gc']}",
    ]


# --- I/O ---

def read_fasta(path):
    sequences = []
    current_id = None
    current_seq = ""
    with open(path) as f:
        for line in f:
            line = line.strip()
            if line.startswith(">"):
                if current_id is not None:
                    sequences.append({"id": current_id, "sequence": current_seq})
                current_id = line[1:]
                current_seq = ""
            else:
                current_seq += line
    if current_id is not None:
        sequences.append({"id": current_id, "sequence": current_seq})
    return sequences


def main():
    sequences = read_fasta("data/sequences.fasta")
    qc_reads = read_fasta("data/qc_reads.fasta")

    summary = sequence_summary(sequences)
    classified = batch_gc(sequences)

    tata_hits = [find_all_motifs(s["sequence"], "TATAAA") for s in sequences]
    ecori_hits = [find_all_motifs(s["sequence"], "GAATTC") for s in sequences]

    qc_result = qc_summary(qc_reads)
    qc_report = format_qc_report(qc_result)

    flagged = flag_outliers(qc_reads, 50, 10000, 0.2, 0.8)
    passing_ids = {f["id"] for f in flagged if f["pass"]}
    failing = [f for f in flagged if not f["pass"]]

    passing_seqs = [s for s in qc_reads if s["id"] in passing_ids]
    passing_classified = batch_gc(passing_seqs)

    Path("data/output").mkdir(parents=True, exist_ok=True)

    with open("data/output/report.txt", "w") as f:
        f.write("# Module-Based Analysis Report\n\n")
        f.write("## Sequence Summary (seq_utils module)\n")
        f.write(f"Total sequences: {summary['total']}\n")
        f.write(f"High GC: {summary['high_gc']}\n")
        f.write(f"Low GC: {summary['low_gc']}\n")
        f.write(f"Moderate GC: {summary['moderate_gc']}\n")
        f.write(f"Mean GC: {summary['mean_gc']}\n")
        f.write(f"GC stdev: {summary['stdev_gc']}\n\n")
        f.write("## GC Classification\n")
        for c in classified:
            f.write(f"  {c['id']}: {c['class']} (GC={c['gc']})\n")
        f.write("\n## Motif Search\n")
        f.write(f"TATAAA hits: {sum(h['count'] for h in tata_hits)}\n")
        f.write(f"GAATTC (EcoRI) hits: {sum(h['count'] for h in ecori_hits)}\n")
        f.write("\n## QC Report (qc module)\n")
        for line in qc_report:
            f.write(f"{line}\n")
        f.write("\n## Failing Sequences\n")
        for fail in failing:
            f.write(f"  {fail['id']}: flags={', '.join(fail['flags'])}\n")
        f.write("\n## Passing Sequences - GC Classification\n")
        for c in passing_classified:
            f.write(f"  {c['id']}: {c['class']} (GC={c['gc']})\n")

    with open("data/output/gc_classification.csv", "w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=["id", "length", "gc", "class", "label"])
        writer.writeheader()
        for row in classified:
            writer.writerow(row)

    print("Report written to data/output/report.txt")


if __name__ == "__main__":
    main()
