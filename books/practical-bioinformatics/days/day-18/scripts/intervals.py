#!/usr/bin/env python3
"""Day 18: Genomic Coordinates and Intervals (Python version)

Uses pyranges for interval trees and pybedtools for BED/VCF operations.
"""

import pyranges as pr
import pandas as pd
import numpy as np
from io import StringIO


def main():
    # ── 1. Creating intervals ─────────────────────────────────────────

    print("=== Creating Intervals ===\n")

    brca1 = pr.PyRanges(pd.DataFrame({
        "Chromosome": ["chr17"],
        "Start": [43044295],
        "End": [43125483],
    }))
    print(f"BRCA1: {brca1}")
    print(f"  Length: {43125483 - 43044295} bp")

    tp53 = pr.PyRanges(pd.DataFrame({
        "Chromosome": ["chr17"],
        "Start": [7668402],
        "End": [7687550],
    }))
    print(f"\nTP53: {tp53}")
    print(f"  Length: {7687550 - 7668402} bp")

    # ── 2. Reading BED files ──────────────────────────────────────────

    print("\n=== Reading BED Files ===\n")

    exons = pr.read_bed("data/exons.bed")
    print(f"Exon regions: {len(exons)}")

    total_bp = (exons.End - exons.Start).sum()
    print(f"Total exonic bases: {total_bp}")

    print("\nFirst 5 exons:")
    df = exons.df.head(5)
    for _, row in df.iterrows():
        size = row["End"] - row["Start"]
        print(f"  {row['Name']}: {row['Chromosome']}:{row['Start']}-{row['End']} ({size} bp)")

    # ── 3. Interval trees and overlap queries ─────────────────────────

    print("\n=== Interval Trees ===\n")

    regions = pr.PyRanges(pd.DataFrame({
        "Chromosome": ["chr17"] * 4,
        "Start": [43044295, 43060000, 43080000, 43100000],
        "End": [43050000, 43070000, 43090000, 43125483],
    }))

    query = pr.PyRanges(pd.DataFrame({
        "Chromosome": ["chr17"],
        "Start": [43065000],
        "End": [43085000],
    }))

    hits = regions.overlap(query)
    print(f"Query: chr17:43065000-43085000")
    print(f"Overlapping regions: {len(hits)}")

    # Bulk overlaps
    print("\nBulk overlap queries:")
    bulk_queries = pr.PyRanges(pd.DataFrame({
        "Chromosome": ["chr17"] * 3,
        "Start": [43045000, 43065000, 43095000],
        "End": [43046000, 43066000, 43096000],
        "QueryID": [0, 1, 2],
    }))

    for i in range(3):
        q = pr.PyRanges(pd.DataFrame({
            "Chromosome": ["chr17"],
            "Start": [bulk_queries.df.iloc[i]["Start"]],
            "End": [bulk_queries.df.iloc[i]["End"]],
        }))
        n = len(regions.overlap(q))
        print(f"  Query {i}: {n} overlaps")

    # ── 4. Variant-in-region filtering ────────────────────────────────

    print("\n=== Variant-in-Region Filtering ===\n")

    # Read VCF manually (pyranges doesn't read VCF natively)
    vcf_variants = []
    with open("data/variants.vcf") as f:
        for line in f:
            if line.startswith("#"):
                continue
            parts = line.strip().split("\t")
            vcf_variants.append({
                "Chromosome": parts[0],
                "Start": int(parts[1]) - 1,  # VCF 1-based -> 0-based
                "End": int(parts[1]),
                "Pos_VCF": int(parts[1]),
                "Ref": parts[3],
                "Alt": parts[4],
            })

    variants_gr = pr.PyRanges(pd.DataFrame(vcf_variants))

    exonic = variants_gr.overlap(exons)
    total_v = len(variants_gr)
    exonic_n = len(exonic)

    print(f"Total variants: {total_v}")
    print(f"Exonic variants: {exonic_n}")
    print(f"Intronic/intergenic: {total_v - exonic_n}")

    # ── 5. Coordinate conversion ──────────────────────────────────────

    print("\n=== Coordinate Conversion ===\n")

    bed_start, bed_end = 43044294, 43044295
    vcf_pos = bed_start + 1
    print(f"BED {bed_start}-{bed_end} -> VCF pos {vcf_pos}")

    vcf_p = 43044295
    bed_s, bed_e = vcf_p - 1, vcf_p
    print(f"VCF pos {vcf_p} -> BED {bed_s}-{bed_e}")

    roundtrip = bed_s + 1
    print(f"Round-trip VCF pos: {roundtrip} (should be {vcf_p})")

    # ── 6. Writing BED files ──────────────────────────────────────────

    print("\n=== Writing BED Files ===\n")

    high_cov = pd.DataFrame({
        "Chromosome": ["chr17", "chr17"],
        "Start": [43044295, 43100000],
        "End": [43050000, 43125483],
    })
    high_cov.to_csv("results/high_coverage.bed", sep="\t", header=False, index=False)
    print("Wrote high-coverage regions to BED file")

    # ── 7. Exome coverage report ──────────────────────────────────────

    print("\n=== Exome Coverage Report ===\n")

    targets = pr.read_bed("data/exons.bed")
    total_target_bp = (targets.End - targets.Start).sum()
    print(f"Target regions: {len(targets)}")
    print(f"Total target bases: {total_target_bp}")

    on_target = variants_gr.overlap(targets)
    off_target = len(variants_gr) - len(on_target)
    on_rate = round(len(on_target) / len(variants_gr) * 100, 1)
    print(f"\nVariant classification:")
    print(f"  On-target:  {len(on_target)}")
    print(f"  Off-target: {off_target}")
    print(f"  On-target rate: {on_rate}%")

    on_target.df.to_csv("results/on_target_variants.csv", index=False)
    print("\nResults saved")
    print("\n=== Report complete ===")


if __name__ == "__main__":
    main()
