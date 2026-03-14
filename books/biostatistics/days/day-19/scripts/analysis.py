# Day 19: Effect Sizes
import pandas as pd
import numpy as np
from scipy import stats

data = pd.read_csv("effect_sizes.csv")
variants = pd.read_csv("variant_association.csv")

# Cohen's d for each gene
print("=== Cohen's d per Gene ===")
for gene in data.gene.unique():
    gdata = data[data.gene == gene]
    ctrl = gdata[gdata.group == "Control"]["value"]
    trt = gdata[gdata.group == "Treatment"]["value"]
    pooled_sd = np.sqrt(((len(ctrl)-1)*ctrl.std()**2 + (len(trt)-1)*trt.std()**2) /
                        (len(ctrl)+len(trt)-2))
    d = (trt.mean() - ctrl.mean()) / pooled_sd
    _, p = stats.ttest_ind(trt, ctrl)
    n = len(ctrl)
    print(f"  {gene:6s}: d={d:5.2f}, p={p:.4f}, n={n}")

# Odds ratios for variants
print("\n=== Variant Odds Ratios ===")
for _, row in variants.iterrows():
    a, b, c, d_val = row.case_exposed, row.case_unexposed, row.control_exposed, row.control_unexposed
    odds_ratio = (a * d_val) / (b * c)
    se_log_or = np.sqrt(1/a + 1/b + 1/c + 1/d_val)
    ci_lo = np.exp(np.log(odds_ratio) - 1.96 * se_log_or)
    ci_hi = np.exp(np.log(odds_ratio) + 1.96 * se_log_or)
    print(f"  {row.variant}: OR={odds_ratio:.2f} ({ci_lo:.2f}-{ci_hi:.2f})")
