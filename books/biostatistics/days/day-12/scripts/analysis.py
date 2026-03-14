# Day 12: Multiple Testing Correction
import pandas as pd
import numpy as np
from statsmodels.stats.multitest import multipletests

de = pd.read_csv("de_results.csv")
pvals = de["pvalue"].values

# Bonferroni
reject_bonf, padj_bonf, _, _ = multipletests(pvals, method="bonferroni")
# Benjamini-Hochberg
reject_bh, padj_bh, _, _ = multipletests(pvals, method="fdr_bh")

de["p_bonferroni"] = padj_bonf
de["p_bh"] = padj_bh

# Compare methods
print("=== Multiple Testing Correction ===")
print(f"Total genes: {len(de)}")
print(f"True DE:     {de.true_de.sum()}")
print(f"\nRaw p < 0.05:       {(pvals < 0.05).sum()}")
print(f"Bonferroni < 0.05:  {reject_bonf.sum()}")
print(f"BH FDR < 0.05:      {reject_bh.sum()}")

# Confusion matrix for BH
tp = ((padj_bh < 0.05) & (de.true_de == 1)).sum()
fp = ((padj_bh < 0.05) & (de.true_de == 0)).sum()
fn = ((padj_bh >= 0.05) & (de.true_de == 1)).sum()
tn = ((padj_bh >= 0.05) & (de.true_de == 0)).sum()
print(f"\nBH FDR confusion: TP={tp}, FP={fp}, FN={fn}, TN={tn}")
print(f"Observed FDR: {fp/(tp+fp):.3f}")
print(f"Power (TPR):  {tp/(tp+fn):.3f}")
