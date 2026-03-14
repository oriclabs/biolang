#!/usr/bin/env python3
"""Day 13: Gene Expression and RNA-seq — Python equivalent.

Uses pandas for data manipulation, scipy for statistics,
and statsmodels for multiple testing correction.

Usage:
    cd days/day-13
    python scripts/analysis.py
"""

import pandas as pd
import numpy as np
from scipy import stats
from statsmodels.stats.multitest import multipletests

# ── Step 1: Load Count Matrix ─────────────────────────────────────────────

print("=== Step 1: Loading Count Matrix ===")

counts = pd.read_csv("data/counts.csv")
print(f"Genes: {len(counts)}")
print(f"Columns: {list(counts.columns)}")
print(counts.head(5).to_string(index=False))
print()

# ── Step 2: Library Sizes ─────────────────────────────────────────────────

print("=== Step 2: Library Sizes ===")

samples = ["normal_1", "normal_2", "normal_3", "tumor_1", "tumor_2", "tumor_3"]
lib_sizes = pd.DataFrame({
    "sample": samples,
    "total": [counts[s].sum() for s in samples],
})
print(lib_sizes.to_string(index=False))
print()

# ── Step 3: CPM Normalization ─────────────────────────────────────────────

print("=== Step 3: CPM Normalization ===")

cpm = counts.copy()
for s in samples:
    total = counts[s].sum()
    cpm[s] = counts[s] / total * 1_000_000

print("CPM normalized (first 5 genes):")
cpm_display = cpm.head(5).copy()
for s in samples:
    cpm_display[s] = cpm_display[s].round(1)
print(cpm_display.to_string(index=False))
print()

# ── Step 4: TPM Normalization ─────────────────────────────────────────────

print("=== Step 4: TPM Normalization ===")

gene_lengths = pd.read_csv("data/gene_lengths.csv")
merged = counts.merge(gene_lengths, on="gene")

tpm = merged[["gene"]].copy()
for s in samples:
    rpk = merged[s] / (merged["length"] / 1000)
    rpk_sum = rpk.sum()
    tpm[s] = rpk / rpk_sum * 1_000_000

print("TPM normalized (first 5 genes):")
tpm_display = tpm.head(5).copy()
for s in samples:
    tpm_display[s] = tpm_display[s].round(1)
print(tpm_display.to_string(index=False))
print()

# ── Step 5: Mean Expression per Condition ─────────────────────────────────

print("=== Step 5: Mean Expression per Condition ===")

normals = ["normal_1", "normal_2", "normal_3"]
tumors = ["tumor_1", "tumor_2", "tumor_3"]

gene_means = pd.DataFrame({
    "gene": counts["gene"],
    "normal_mean": counts[normals].mean(axis=1).round(1),
    "tumor_mean": counts[tumors].mean(axis=1).round(1),
})
print(gene_means.to_string(index=False))
print()

# ── Step 6: Fold Change ──────────────────────────────────────────────────

print("=== Step 6: Fold Change ===")

fc = pd.DataFrame({
    "gene": counts["gene"],
    "normal_mean": counts[normals].mean(axis=1),
    "tumor_mean": counts[tumors].mean(axis=1),
})
fc["log2fc"] = np.log2(fc["tumor_mean"] / fc["normal_mean"])

print("Fold changes (first 10):")
fc_display = fc.head(10).copy()
fc_display["normal_mean"] = fc_display["normal_mean"].round(1)
fc_display["tumor_mean"] = fc_display["tumor_mean"].round(1)
fc_display["log2fc"] = fc_display["log2fc"].round(2)
print(fc_display.to_string(index=False))
print()

# ── Step 7: Differential Expression (t-test per gene) ────────────────────

print("=== Step 7: Differential Expression ===")

de_results = []
for _, row in counts.iterrows():
    normal_vals = [row[s] for s in normals]
    tumor_vals = [row[s] for s in tumors]
    t_stat, pvalue = stats.ttest_ind(tumor_vals, normal_vals)
    log2fc = np.log2(np.mean(tumor_vals) / np.mean(normal_vals))
    de_results.append({
        "gene": row["gene"],
        "log2fc": round(log2fc, 2),
        "pvalue": pvalue,
        "mean_ctrl": round(np.mean(normal_vals), 1),
        "mean_treat": round(np.mean(tumor_vals), 1),
    })

de = pd.DataFrame(de_results)

# Multiple testing correction (Benjamini-Hochberg)
reject, padj, _, _ = multipletests(de["pvalue"], method="fdr_bh")
de["padj"] = padj

de = de.sort_values("pvalue")
print(f"DE results: {len(de)} genes")
print(de.head(5).to_string(index=False))
print()

# ── Step 8: Filter Significant Results ───────────────────────────────────

print("=== Step 8: Significant DE Genes ===")

significant = de[(de["padj"] < 0.05) & (de["log2fc"].abs() > 1.0)]
significant = significant.sort_values("padj")

print(f"Significant DE genes (|log2FC| > 1, padj < 0.05): {len(significant)}")
print(significant.to_string(index=False))

up = len(significant[significant["log2fc"] > 0])
down = len(significant[significant["log2fc"] < 0])
print(f"Upregulated in tumor: {up}")
print(f"Downregulated in tumor: {down}")
print()

# ── Step 9: Multiple Testing Correction Demo ─────────────────────────────

print("=== Step 9: Multiple Testing Correction ===")

raw_pvals = [0.001, 0.01, 0.03, 0.04, 0.049, 0.06, 0.1]
_, adj, _, _ = multipletests(raw_pvals, method="fdr_bh")
print("Raw vs Adjusted p-values:")
for raw, adjusted in zip(raw_pvals, adj):
    print(f"  {raw} -> {round(adjusted, 4)}")
print()

# ── Step 10: Export Results ──────────────────────────────────────────────

print("=== Step 10: Export Results ===")

significant.to_csv("results/significant_genes.csv", index=False)
print(f"Saved {len(significant)} significant genes to results/significant_genes.csv")

fc.to_csv("results/fold_changes.csv", index=False)
print(f"Saved fold changes for {len(fc)} genes to results/fold_changes.csv")

print("\n=== Pipeline complete ===")
