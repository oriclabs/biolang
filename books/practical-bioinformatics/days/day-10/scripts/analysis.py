#!/usr/bin/env python3
"""Day 10: Table Analysis Pipeline (Python/pandas equivalent)

Run init.bl first to generate data files, then:
    pip install -r python/requirements.txt
    python scripts/analysis.py
"""

import pandas as pd
import numpy as np
import os

# ── Step 1: Load data ───────────────────────────────────────────────

expr = pd.read_csv("data/expression.csv")
gene_info = pd.read_csv("data/gene_info.csv")

print(f"Expression data: {len(expr)} genes x {len(expr.columns)} columns")
print(f"Columns: {list(expr.columns)}")
print(f"Gene info: {len(gene_info)} annotations")
print()

# ── Step 2: Explore ─────────────────────────────────────────────────

print("=== First 5 rows ===")
print(expr.head(5).to_string(index=False))
print()

print("=== Summary statistics ===")
print(expr.describe().to_string())
print()

# ── Step 3: Add derived columns ─────────────────────────────────────

analyzed = expr.copy()
analyzed["significant"] = analyzed["padj"] < 0.05
analyzed["direction"] = np.where(analyzed["log2fc"] > 0, "up", "down")
analyzed["neg_log_p"] = -np.log10(analyzed["pval"])

print("=== With derived columns (first 5) ===")
print(analyzed.head(5).to_string(index=False))
print()

# ── Step 4: Filter significant genes ────────────────────────────────

sig_genes = analyzed[analyzed["significant"]]
print(f"Significant genes: {len(sig_genes)} of {len(analyzed)}")

sig_up = analyzed[(analyzed["significant"]) & (analyzed["direction"] == "up")]
sig_down = analyzed[(analyzed["significant"]) & (analyzed["direction"] == "down")]
print(f"  Upregulated: {len(sig_up)}")
print(f"  Downregulated: {len(sig_down)}")
print()

# ── Step 5: Count by category ──────────────────────────────────────

print("=== Genes per chromosome ===")
chr_counts = analyzed["chr"].value_counts().reset_index()
chr_counts.columns = ["chr", "count"]
print(chr_counts.to_string(index=False))
print()

print("=== Genes by biotype ===")
biotype_counts = analyzed["biotype"].value_counts().reset_index()
biotype_counts.columns = ["biotype", "count"]
print(biotype_counts.to_string(index=False))
print()

# ── Step 6: Group and summarize ─────────────────────────────────────

print("=== Significant genes by direction ===")
direction_summary = (
    sig_genes.groupby("direction")
    .agg(
        count=("gene", "count"),
        mean_fc=("log2fc", "mean"),
        min_padj=("padj", "min"),
    )
    .reset_index()
)
print(direction_summary.to_string(index=False))
print()

# ── Step 7: Join with annotations ───────────────────────────────────

annotated = analyzed.merge(gene_info, on="gene", how="left")
print(f"Annotated table: {len(annotated)} rows x {len(annotated.columns)} columns")

# Check for missing annotations
missing = analyzed[~analyzed["gene"].isin(gene_info["gene"])]
print(f"Genes without annotations: {len(missing)}")
print()

# ── Step 8: Top 10 most significant ─────────────────────────────────

print("=== Top 10 most significant genes ===")
top10 = (
    annotated[annotated["significant"]]
    .sort_values("padj")
    .head(10)[["gene", "log2fc", "padj", "direction", "pathway"]]
)
print(top10.to_string(index=False))
print()

# ── Step 9: Pathway summary ─────────────────────────────────────────

print("=== Pathway summary (significant genes only) ===")
pathway_summary = (
    annotated[annotated["significant"]]
    .groupby("pathway")
    .agg(n_genes=("gene", "count"), mean_fc=("log2fc", "mean"))
    .reset_index()
)
print(pathway_summary.to_string(index=False))
print()

# ── Step 10: Pivot demonstration ────────────────────────────────────

print("=== Pivot: long to wide ===")
long_data = pd.DataFrame(
    {
        "gene": ["BRCA1", "BRCA1", "TP53", "TP53", "EGFR", "EGFR"],
        "sample": ["Control", "Treated", "Control", "Treated", "Control", "Treated"],
        "expression": [5.2, 8.1, 3.4, 7.6, 2.1, 9.3],
    }
)

print("Long format:")
print(long_data.to_string(index=False))

wide_data = long_data.pivot(index="gene", columns="sample", values="expression").reset_index()
wide_data.columns.name = None
print("Wide format:")
print(wide_data.to_string(index=False))

back_to_long = wide_data.melt(
    id_vars=["gene"], value_vars=["Control", "Treated"],
    var_name="sample", value_name="expression"
)
print("Back to long:")
print(back_to_long.to_string(index=False))
print()

# ── Step 11: Window functions ───────────────────────────────────────

print("=== Ranked by significance ===")
ranked = sig_genes.sort_values("padj").reset_index(drop=True)
ranked["row_number"] = range(1, len(ranked) + 1)
print(ranked[["gene", "log2fc", "padj", "row_number"]].head(5).to_string(index=False))
print()

# ── Step 12: Export results ─────────────────────────────────────────

os.makedirs("results", exist_ok=True)
annotated.to_csv("results/annotated_results.csv", index=False)
print("Saved: results/annotated_results.csv")
print("Done!")
