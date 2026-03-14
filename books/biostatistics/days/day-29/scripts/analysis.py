# Day 29: Capstone — Differential Expression Analysis
import pandas as pd
import numpy as np
from scipy import stats
from statsmodels.stats.multitest import multipletests

counts = pd.read_csv("counts.csv", index_col=0)
meta = pd.read_csv("sample_metadata.csv")
gene_info = pd.read_csv("gene_info.csv")

ctrl_samples = meta[meta.condition == "Control"]["sample"].tolist()
trt_samples = meta[meta.condition == "Treated"]["sample"].tolist()

# Filter low-count genes
row_means = counts.mean(axis=1)
keep = row_means >= 10
counts_filt = counts[keep]
print(f"Genes after filtering: {len(counts_filt)} / {len(counts)}")

# Simple log2 fold change + t-test (educational, not production DE)
results = []
for gene in counts_filt.index:
    ctrl_vals = np.log2(counts_filt.loc[gene, ctrl_samples].values + 1)
    trt_vals = np.log2(counts_filt.loc[gene, trt_samples].values + 1)
    lfc = trt_vals.mean() - ctrl_vals.mean()
    t_stat, p_val = stats.ttest_ind(trt_vals, ctrl_vals)
    results.append({"gene": gene, "log2fc": lfc, "pvalue": p_val})

res = pd.DataFrame(results)
_, res["padj"], _, _ = multipletests(res.pvalue, method="fdr_bh")

# Summary
sig = res[(res.padj < 0.05) & (res.log2fc.abs() > 1)]
print(f"\n=== DE Results ===")
print(f"Significant (FDR<0.05, |LFC|>1): {len(sig)}")
print(f"  Up-regulated:   {(sig.log2fc > 0).sum()}")
print(f"  Down-regulated: {(sig.log2fc < 0).sum()}")

# Compare with truth
truth = gene_info.set_index("gene")
merged = res.merge(truth, left_on="gene", right_index=True)
called_de = merged.padj < 0.05
true_de = merged.true_de == 1
tp = (called_de & true_de).sum()
fp = (called_de & ~true_de).sum()
fn = (~called_de & true_de).sum()
print(f"\nTP={tp}, FP={fp}, FN={fn}")
print(f"FDR: {fp/(tp+fp):.3f}, Sensitivity: {tp/(tp+fn):.3f}")

# Top DE genes
print("\nTop 10 DE genes by adjusted p-value:")
print(res.nsmallest(10, "padj")[["gene", "log2fc", "pvalue", "padj"]].to_string(index=False))
