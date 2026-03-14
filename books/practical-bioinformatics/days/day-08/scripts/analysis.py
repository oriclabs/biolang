#!/usr/bin/env python3
"""Day 8: Processing Large Files — Python streaming equivalent.

Uses generators and itertools for constant-memory processing.
Run: python scripts/analysis.py
Requires: pip install -r python/requirements.txt
"""

import os
import sys
from collections import Counter
from itertools import islice
from functools import reduce

# ---- Streaming FASTQ parser (generator) ----

def fastq_stream(path):
    """Yield FASTQ records one at a time. Constant memory."""
    with open(path) as f:
        while True:
            header = f.readline().strip()
            if not header:
                break
            seq = f.readline().strip()
            f.readline()  # + line
            qual = f.readline().strip()
            read_id = header[1:].split()[0]
            yield {"id": read_id, "seq": seq, "qual": qual}


def mean_phred(qual_str):
    """Mean Phred quality score from ASCII quality string."""
    if not qual_str:
        return 0.0
    scores = [ord(c) - 33 for c in qual_str]
    return sum(scores) / len(scores)


def gc_content(seq):
    """GC fraction of a sequence."""
    if not seq:
        return 0.0
    gc = sum(1 for b in seq.upper() if b in "GC")
    return gc / len(seq)


# ---- Streaming FASTA parser ----

def fasta_stream(path):
    """Yield FASTA records one at a time."""
    with open(path) as f:
        name = None
        seq_parts = []
        for line in f:
            line = line.strip()
            if line.startswith(">"):
                if name is not None:
                    yield {"id": name, "seq": "".join(seq_parts)}
                name = line[1:].split()[0]
                seq_parts = []
            else:
                seq_parts.append(line)
        if name is not None:
            yield {"id": name, "seq": "".join(seq_parts)}


# ---- Streaming VCF parser ----

def vcf_stream(path):
    """Yield VCF records one at a time, skipping headers."""
    with open(path) as f:
        for line in f:
            if line.startswith("#"):
                continue
            fields = line.strip().split("\t")
            if len(fields) >= 7:
                yield {
                    "chrom": fields[0],
                    "pos": int(fields[1]),
                    "ref": fields[3],
                    "alt": fields[4],
                    "qual": fields[5],
                    "filter": fields[6],
                }


# ---- Streaming BED parser ----

def bed_stream(path):
    """Yield BED records one at a time."""
    with open(path) as f:
        for line in f:
            fields = line.strip().split("\t")
            if len(fields) >= 3:
                yield {
                    "chrom": fields[0],
                    "start": int(fields[1]),
                    "end": int(fields[2]),
                    "name": fields[3] if len(fields) > 3 else "",
                }


# ---- Chunked iterator ----

def stream_chunks(iterable, size):
    """Yield lists of `size` items from an iterable."""
    it = iter(iterable)
    while True:
        chunk = list(islice(it, size))
        if not chunk:
            break
        yield chunk


# ============================================================
# Main analysis
# ============================================================

def main():
    print("=" * 60)
    print("Day 8: Processing Large Files — Python Streaming")
    print("=" * 60)

    # ----------------------------------------------------------
    # 1. Eager vs Streaming
    # ----------------------------------------------------------
    print("\n--- 1. Eager vs Streaming ---")

    # Eager: load all
    all_reads = list(fastq_stream("data/reads.fastq"))
    print(f"Eager: loaded {len(all_reads)} reads (type: {type(all_reads).__name__})")

    # Streaming: generator
    stream = fastq_stream("data/reads.fastq")
    print(f"Stream type: generator")
    stream_count = sum(1 for _ in stream)
    print(f"Stream count: {stream_count}")

    # ----------------------------------------------------------
    # 2. Constant-Memory Patterns
    # ----------------------------------------------------------
    print("\n--- 2. Constant-Memory Patterns ---")

    # Count
    total = sum(1 for _ in fastq_stream("data/reads.fastq"))
    print(f"Total reads: {total}")

    # Filter and count
    passed = sum(1 for r in fastq_stream("data/reads.fastq")
                 if mean_phred(r["qual"]) >= 20)
    print(f"Passed Q20: {passed}")

    # Reduce for mean GC
    gc_acc = reduce(
        lambda a, b: {"gc": a["gc"] + b["gc"], "n": a["n"] + b["n"]},
        ({"gc": gc_content(r["seq"]), "n": 1}
         for r in fastq_stream("data/reads.fastq"))
    )
    mean_gc_val = gc_acc["gc"] / gc_acc["n"]
    print(f"Mean GC: {round(mean_gc_val * 100, 1)}%")

    # Sample first 5
    print("\nFirst 5 reads:")
    for r in islice(fastq_stream("data/reads.fastq"), 5):
        q = mean_phred(r["qual"])
        print(f"  {r['id']}: {len(r['seq'])} bp, Q={round(q, 1)}")

    # ----------------------------------------------------------
    # 3. Lazy Pipeline
    # ----------------------------------------------------------
    print("\n--- 3. Lazy Pipeline ---")

    hq = list(islice(
        ({"id": r["id"], "gc": gc_content(r["seq"]), "length": len(r["seq"])}
         for r in fastq_stream("data/reads.fastq")
         if mean_phred(r["qual"]) >= 30),
        1000
    ))
    print(f"High-quality reads (Q>=30): {len(hq)}")

    # ----------------------------------------------------------
    # 4. Chunked Processing
    # ----------------------------------------------------------
    print("\n--- 4. Chunked Processing ---")

    for i, chunk in enumerate(stream_chunks(fastq_stream("data/reads.fastq"), 100), 1):
        gc_vals = [gc_content(r["seq"]) for r in chunk]
        avg_gc = sum(gc_vals) / len(gc_vals)
        print(f"  Batch {i}: {len(chunk)} reads, mean GC: {round(avg_gc * 100, 1)}%")

    # ----------------------------------------------------------
    # 5. Streaming All Formats
    # ----------------------------------------------------------
    print("\n--- 5. Streaming All Formats ---")

    # FASTA: highest GC
    fasta_gc = [{"id": s["id"], "gc": gc_content(s["seq"])}
                for s in fasta_stream("data/sequences.fasta")]
    fasta_gc.sort(key=lambda x: x["gc"])
    best = fasta_gc[-1]
    print(f"FASTA highest GC: {best['id']} at {round(best['gc'] * 100, 1)}%")

    # VCF: PASS counts by chrom
    chr_counts = Counter(
        v["chrom"] for v in vcf_stream("data/variants.vcf")
        if v["filter"] == "PASS"
    )
    print(f"VCF PASS by chrom: {dict(chr_counts)}")

    # BED: total covered
    total_bp = sum(r["end"] - r["start"] for r in bed_stream("data/regions.bed"))
    print(f"BED total covered: {total_bp} bp")

    # ----------------------------------------------------------
    # 6. Tee Pattern (manual in Python)
    # ----------------------------------------------------------
    print("\n--- 6. Tee Pattern ---")

    kept = 0
    for r in fastq_stream("data/reads.fastq"):
        print(f"  Checking: {r['id']}")
        if mean_phred(r["qual"]) >= 35:
            kept += 1
            if kept >= 3:
                break
    print(f"Kept {kept} reads with Q>=35")

    # ----------------------------------------------------------
    # 7. QC Report
    # ----------------------------------------------------------
    print("\n--- 7. Streaming QC Report ---")

    total_reads = 0
    total_bases = 0
    for r in fastq_stream("data/reads.fastq"):
        total_reads += 1
        total_bases += len(r["seq"])
    print(f"Total reads: {total_reads}")
    print(f"Total bases: {total_bases}")

    quality_bins = Counter()
    for r in fastq_stream("data/reads.fastq"):
        q = mean_phred(r["qual"])
        if q >= 30:
            quality_bins["excellent"] += 1
        elif q >= 20:
            quality_bins["good"] += 1
        else:
            quality_bins["poor"] += 1

    print("Quality distribution:")
    for cat in ["excellent", "good", "poor"]:
        if cat in quality_bins:
            print(f"  {cat}: {quality_bins[cat]}")

    lengths_list = [len(r["seq"]) for r in fastq_stream("data/reads.fastq")]
    print(f"Length mean: {round(sum(lengths_list)/len(lengths_list), 1)}")
    print(f"Length min: {min(lengths_list)}, max: {max(lengths_list)}")

    # ----------------------------------------------------------
    # 8. Write Filtered Output
    # ----------------------------------------------------------
    print("\n--- 8. Write Filtered Output ---")

    os.makedirs("results", exist_ok=True)
    filtered_reads = [r for r in fastq_stream("data/reads.fastq")
                      if len(r["seq"]) >= 100 and mean_phred(r["qual"]) >= 25]

    with open("results/filtered.fastq", "w") as out:
        for r in filtered_reads:
            out.write(f"@{r['id']}\n{r['seq']}\n+\n{r['qual']}\n")
    print(f"Wrote {len(filtered_reads)} filtered reads to results/filtered.fastq")

    print()
    print("=" * 60)
    print("Day 8 complete! You can now process files of any size.")
    print("=" * 60)


if __name__ == "__main__":
    main()
