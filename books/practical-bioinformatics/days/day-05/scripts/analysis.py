#!/usr/bin/env python3
"""Day 5: Data Structures for Biology — Python equivalent.

Demonstrates lists, dicts, DataFrames, sets, and interval overlaps
using standard Python libraries (pandas, statistics).
"""

import statistics
import pandas as pd

print("=" * 60)
print("Day 5: Data Structures for Biology (Python)")
print("=" * 60)

# ============================================================
# 1. LISTS: Ordered Collections
# ============================================================
print("\n--- 1. Lists: Ordered Collections ---")

expression = [2.1, 5.4, 3.2, 8.7, 1.1, 6.3]
print(f"Expression values: {expression}")
print(f"Mean: {round(statistics.mean(expression), 2)}")
print(f"Median: {round(statistics.median(expression), 2)}")
print(f"Stdev: {round(statistics.stdev(expression), 2)}")
print(f"Min: {min(expression)}, Max: {max(expression)}")

sorted_expr = sorted(expression, reverse=True)
top3 = sorted_expr[:3]
print(f"Top 3 values: {top3}")

# Sample filtering
samples = ["control_1", "control_2", "treated_1", "treated_2", "treated_3"]
treated = [s for s in samples if "treated" in s]
print(f"Treated samples: {treated}")
print(f"Total: {len(samples)}, Treated: {len(treated)}")

# Nested lists (matrix-like)
data = [
    [2.1, 3.4, 5.6],
    [1.8, 4.2, 6.1],
    [3.0, 2.9, 4.8],
]
print(f"Sample 2, Gene 3: {data[1][2]}")

# ============================================================
# 2. RECORDS (DICTS): Structured Metadata
# ============================================================
print("\n--- 2. Dicts: Structured Metadata ---")

gene = {
    "symbol": "BRCA1",
    "name": "BRCA1 DNA repair associated",
    "chromosome": "17",
    "start": 43044295,
    "end": 43125483,
    "strand": "+",
    "biotype": "protein_coding",
}

print(f"{gene['symbol']} on chr{gene['chromosome']}")
print(f"Length: {gene['end'] - gene['start']} bp")
print(f"Keys: {list(gene.keys())}")
print(f"Has strand: {'strand' in gene}")
print(f"Has expression: {'expression' in gene}")

# List of dicts
variants = [
    {"chrom": "chr17", "pos": 43091434, "ref": "A", "alt": "G", "gene": "BRCA1"},
    {"chrom": "chr17", "pos": 7674220, "ref": "C", "alt": "T", "gene": "TP53"},
    {"chrom": "chr7", "pos": 55249071, "ref": "C", "alt": "T", "gene": "EGFR"},
]

chr17_vars = [v for v in variants if v["chrom"] == "chr17"]
print(f"Chr17 variants: {len(chr17_vars)}")

genes = [v["gene"] for v in variants]
print(f"Affected genes: {genes}")

# ============================================================
# 3. TABLES (DataFrames): The Workhorse
# ============================================================
print("\n--- 3. DataFrames: Analysis Results ---")

results = pd.DataFrame(
    [
        {"gene": "BRCA1", "log2fc": 2.4, "pval": 0.001},
        {"gene": "TP53", "log2fc": -1.1, "pval": 0.23},
        {"gene": "EGFR", "log2fc": 3.8, "pval": 0.000001},
        {"gene": "MYC", "log2fc": 1.9, "pval": 0.04},
        {"gene": "KRAS", "log2fc": -0.3, "pval": 0.67},
    ]
)

print(f"Rows: {len(results)}, Columns: {len(results.columns)}")
print(f"Columns: {list(results.columns)}")

# Filter and sort
significant = results[results["pval"] < 0.05].sort_values("log2fc")
print("Significant genes (sorted by log2fc):")
print(significant.head().to_string(index=False))

# Select columns
gene_pvals = results[["gene", "pval"]]
print("Gene-pval pairs:")
print(gene_pvals.head(3).to_string(index=False))

# Mutate — add column
annotated = results.copy()
annotated["significant"] = annotated["pval"] < 0.05
print("With significance flag:")
print(annotated.head(3).to_string(index=False))

# Group by direction
results_copy = results.copy()
results_copy["direction"] = results_copy["log2fc"].apply(
    lambda x: "up" if x > 0 else "down"
)
direction_counts = results_copy.groupby("direction").size().reset_index(name="count")
print("Counts by direction:")
print(direction_counts.to_string(index=False))

# ============================================================
# 4. SETS: Unique Membership and Comparisons
# ============================================================
print("\n--- 4. Sets: Venn Diagram Logic ---")

experiment_a = {"BRCA1", "TP53", "EGFR", "MYC", "KRAS"}
experiment_b = {"TP53", "EGFR", "PTEN", "RB1", "MYC"}

shared = experiment_a & experiment_b
only_a = experiment_a - experiment_b
only_b = experiment_b - experiment_a
all_genes = experiment_a | experiment_b

print(f"Experiment A: {experiment_a}")
print(f"Experiment B: {experiment_b}")
print(f"Shared genes: {shared}")
print(f"Only in A: {only_a}")
print(f"Only in B: {only_b}")
print(f"Total unique: {len(all_genes)}")

# ============================================================
# 5. GENOMIC INTERVALS: Coordinates and Overlaps
# ============================================================
print("\n--- 5. Genomic Intervals ---")

# Manual interval overlap (no pybedtools dependency)
regions = [
    {"chrom": "chr17", "start": 43125283, "end": 43125483, "name": "promoter"},
    {"chrom": "chr17", "start": 43124017, "end": 43124115, "name": "exon1"},
    {"chrom": "chr17", "start": 43125000, "end": 43125600, "name": "enhancer"},
]

print(f"Promoter: chr17:{regions[0]['start']}-{regions[0]['end']}")
print(f"Exon 1: chr17:{regions[1]['start']}-{regions[1]['end']}")
print(f"Enhancer: chr17:{regions[2]['start']}-{regions[2]['end']}")

# Query overlaps manually
q_chrom, q_start, q_end = "chr17", 43125300, 43125400
hits = [
    r
    for r in regions
    if r["chrom"] == q_chrom and r["start"] < q_end and r["end"] > q_start
]
print(f"Regions overlapping {q_chrom}:{q_start}-{q_end}: {len(hits)}")
for h in hits:
    print(f"  {h['name']}: {h['chrom']}:{h['start']}-{h['end']}")

# ============================================================
# 6. COMBINING STRUCTURES: Real Analysis Pattern
# ============================================================
print("\n--- 6. Combining Structures ---")

samples = [
    {"id": "S1", "condition": "control", "genes": {"BRCA1", "TP53", "EGFR"}},
    {"id": "S2", "condition": "treated", "genes": {"TP53", "MYC", "KRAS", "EGFR"}},
    {"id": "S3", "condition": "treated", "genes": {"BRCA1", "TP53", "PTEN"}},
]

# Core genes across all samples
from functools import reduce

all_gene_sets = [s["genes"] for s in samples]
common = reduce(lambda a, b: a & b, all_gene_sets)
print(f"Core genes (in all samples): {common}")

# Treatment-specific genes
treated_genes = reduce(
    lambda a, b: a | b, [s["genes"] for s in samples if s["condition"] == "treated"]
)
control_genes = reduce(
    lambda a, b: a | b, [s["genes"] for s in samples if s["condition"] == "control"]
)
treatment_specific = treated_genes - control_genes
print(f"Treatment-specific genes: {treatment_specific}")

# Summary
summary = pd.DataFrame(
    [
        {"category": "Core (all samples)", "count": len(common)},
        {"category": "Treatment-specific", "count": len(treatment_specific)},
        {"category": "Control genes", "count": len(control_genes)},
        {"category": "Treated genes", "count": len(treated_genes)},
    ]
)
print("Summary:")
print(summary.to_string(index=False))

print("\n" + "=" * 60)
print("Day 5 complete! Week 1 foundations finished.")
print("=" * 60)
