#!/usr/bin/env python3
"""Day 16: Pathway and Enrichment Analysis — Python equivalent.

Requires: pip install gseapy pandas matplotlib
Data: run init.bl first to generate data/ files.
"""

import pandas as pd
import gseapy as gp
from collections import defaultdict

# ── Step 1: Load Gene Sets (GMT) ───────────────────────────────────────

print("=== Step 1: Load Gene Sets ===\n")

gene_sets = {}
with open("data/hallmark.gmt") as f:
    for line in f:
        fields = line.strip().split("\t")
        if len(fields) >= 3:
            name = fields[0]
            genes = [g for g in fields[2:] if g]
            gene_sets[name] = genes

print(f"Gene sets loaded: {len(gene_sets)}")
dna_repair = gene_sets["HALLMARK_DNA_REPAIR"]
print(f"DNA repair genes: {len(dna_repair)}")
print(f"First 5: {dna_repair[:5]}")

# ── Step 2: Over-Representation Analysis (ORA) ────────────────────────

print("\n=== Step 2: Over-Representation Analysis ===\n")

de = pd.read_csv("data/de_results.csv")
print(f"Total genes in DE results: {len(de)}")

sig = de[(de["padj"] < 0.05) & (de["log2fc"].abs() > 1.0)]
sig_genes = sig["gene"].tolist()
print(f"Significant DE genes: {len(sig_genes)}")

# Manual ORA using scipy
from scipy.stats import hypergeom

bg_size = 20000
results = []
for set_name, set_genes in gene_sets.items():
    overlap = [g for g in sig_genes if g in set_genes]
    k = len(overlap)
    K = len(set_genes)
    n = len(sig_genes)
    # P(X >= k) using survival function
    pval = hypergeom.sf(k - 1, bg_size, K, n) if k > 0 else 1.0
    results.append({
        "term": set_name,
        "overlap": k,
        "p_value": pval,
        "genes": ",".join(overlap),
    })

results_df = pd.DataFrame(results).sort_values("p_value")

# BH correction
from statsmodels.stats.multitest import multipletests
_, fdr, _, _ = multipletests(results_df["p_value"], method="fdr_bh")
results_df["fdr"] = fdr

sig_ora = results_df[results_df["fdr"] < 0.05]
print(f"Significant terms (FDR < 0.05): {len(sig_ora)}")
print(sig_ora.to_string(index=False))

# ── Step 3: GSEA ──────────────────────────────────────────────────────

print("\n=== Step 3: Gene Set Enrichment Analysis ===\n")

ranked = pd.read_csv("data/ranked_genes.csv")
print(f"Ranked genes loaded: {len(ranked)}")

# gseapy prerank
rnk = ranked.set_index("gene")["score"]
try:
    gsea_res = gp.prerank(
        rnk=rnk,
        gene_sets=gene_sets,
        permutation_num=1000,
        no_plot=True,
        seed=42,
    )
    gsea_df = gsea_res.res2d
    gsea_sig = gsea_df[gsea_df["FDR q-val"].astype(float) < 0.25]
    print(f"Significant terms (FDR < 0.25): {len(gsea_sig)}")
    print(gsea_sig[["Term", "ES", "NES", "NOM p-val", "FDR q-val"]].to_string(index=False))
except Exception as e:
    print(f"GSEA failed (gseapy not installed?): {e}")

# ── Step 4: Export ─────────────────────────────────────────────────────

print("\n=== Step 4: Export ===\n")

sig_ora.to_csv("results/ora_results_py.csv", index=False)
print("Saved results/ora_results_py.csv")

print("\n=== Analysis complete ===")
