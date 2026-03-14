# Day 13: Correlation
import pandas as pd
import numpy as np
from scipy import stats

genes = pd.read_csv("gene_correlation.csv")
anscombe = pd.read_csv("anscombe.csv")

# Pearson and Spearman correlations
print("=== Gene Correlations ===")
for pair in [("brca1","bard1"), ("brca1","proliferation"), ("bard1","proliferation")]:
    r, p = stats.pearsonr(genes[pair[0]], genes[pair[1]])
    rho, p_s = stats.spearmanr(genes[pair[0]], genes[pair[1]])
    print(f"{pair[0]:15s} vs {pair[1]:15s}: Pearson r={r:.3f} (p={p:.4f}), Spearman rho={rho:.3f}")

# Partial correlation: BRCA1-BARD1 controlling for proliferation
from numpy.linalg import inv
cols = ["brca1", "bard1", "proliferation"]
R = genes[cols].corr().values
P = -inv(R)
partial_r = -P[0,1] / np.sqrt(P[0,0] * P[1,1])
print(f"\nPartial r(BRCA1,BARD1 | proliferation): {partial_r:.3f}")

# Anscombe's quartet — same stats, different data
print("\n=== Anscombe's Quartet ===")
for i, ycol in enumerate(["y1","y2","y3","y4"], 1):
    xcol = "x4" if i == 4 else "x"
    r, _ = stats.pearsonr(anscombe[xcol], anscombe[ycol])
    print(f"Set {i}: mean_x={anscombe[xcol].mean():.1f}, mean_y={anscombe[ycol].mean():.2f}, r={r:.3f}")
