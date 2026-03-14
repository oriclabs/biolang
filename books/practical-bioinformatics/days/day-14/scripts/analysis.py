#!/usr/bin/env python3
"""Day 14: Statistics for Bioinformatics — Python equivalent."""

import numpy as np
import pandas as pd
from scipy import stats
from statsmodels.stats.multitest import multipletests

# Step 1: Descriptive statistics
expression = np.array([5.2, 8.1, 3.4, 6.7, 4.1, 9.3, 7.5, 2.8])
print(f"Mean:     {np.mean(expression):.2f}")
print(f"Median:   {np.median(expression):.2f}")
print(f"Stdev:    {np.std(expression, ddof=1):.2f}")
print(f"Variance: {np.var(expression, ddof=1):.2f}")
print(f"Min:      {np.min(expression)}")
print(f"Max:      {np.max(expression)}")
print(f"Range:    {np.max(expression) - np.min(expression)}")
print(f"Q25:      {np.percentile(expression, 25):.2f}")
print(f"Q75:      {np.percentile(expression, 75):.2f}")
print()

# Load experiment data
data = pd.read_csv("data/experiment.csv")
print(data.describe())
print()

# Step 2: t-tests
normal = [5.2, 4.8, 5.1, 4.9, 5.3]
tumor = [8.1, 7.9, 8.5, 7.6, 8.3]
t_stat, p_val = stats.ttest_ind(normal, tumor)
print(f"Two-sample t-test: t={t_stat:.3f}, p={p_val}")

before = [10.2, 8.5, 12.1, 9.8, 11.3]
after = [7.1, 6.2, 8.5, 7.0, 8.8]
t_stat, p_val = stats.ttest_rel(before, after)
print(f"Paired t-test: p={p_val:.4f}")

observed = [2.1, 1.9, 2.3, 2.0, 2.2]
t_stat, p_val = stats.ttest_1samp(observed, 2.0)
print(f"One-sample t-test: p={p_val:.4f}")
print()

# Step 3: Wilcoxon
control = [1.2, 3.5, 2.1, 4.8, 1.5]
treated = [5.2, 8.1, 6.3, 9.5, 7.2]
stat, p_val = stats.mannwhitneyu(control, treated, alternative="two-sided")
print(f"Wilcoxon/Mann-Whitney p={p_val:.4f}")
print()

# Step 4: ANOVA
ctrl = [5.0, 4.8, 5.2, 4.9]
low_dose = [6.5, 7.1, 6.8, 6.3]
high_dose = [9.2, 8.8, 9.5, 9.0]
f_stat, p_val = stats.f_oneway(ctrl, low_dose, high_dose)
print(f"ANOVA: F={f_stat:.2f}, p={p_val}")
print()

# Step 5: Correlation
gene_a = np.array([2.1, 3.5, 4.2, 5.8, 6.1, 7.3])
gene_b = np.array([1.8, 3.2, 3.9, 5.5, 6.4, 7.0])
r, _ = stats.pearsonr(gene_a, gene_b)
print(f"Pearson r: {r:.3f}")
rho, p = stats.spearmanr(gene_a, gene_b)
print(f"Spearman rho: {rho:.3f}")
tau, p = stats.kendalltau(gene_a, gene_b)
print(f"Kendall tau: {tau:.3f}")
print()

# Step 6: Linear regression
x = np.array([1.0, 2.0, 3.0, 4.0, 5.0, 6.0])
y = np.array([2.1, 3.9, 6.2, 7.8, 10.1, 12.3])
slope, intercept, r_value, p_value, std_err = stats.linregress(x, y)
print(f"Slope: {slope:.3f}")
print(f"Intercept: {intercept:.3f}")
print(f"R-squared: {r_value**2:.3f}")
print(f"p-value: {p_value}")
print()

# Step 7: Multiple testing
raw_pvals = [0.001, 0.005, 0.01, 0.03, 0.04, 0.049, 0.06, 0.1, 0.5, 0.9]
reject_bh, bh_adj, _, _ = multipletests(raw_pvals, method="fdr_bh")
reject_bonf, bonf_adj, _, _ = multipletests(raw_pvals, method="bonferroni")
print("Raw       | BH        | Bonferroni")
for raw, bh, bonf in zip(raw_pvals, bh_adj, bonf_adj):
    print(f"{raw}    | {bh:.4f}   | {bonf:.4f}")
print()

# Step 8: Categorical tests
chi2, p, dof, expected = stats.chi2_contingency([[30, 10], [15, 25]])
print(f"Chi-square: stat={chi2:.2f}, p={p:.4f}")
odds_ratio, p = stats.fisher_exact([[8, 2], [1, 9]])
print(f"Fisher's exact: p={p:.4f}")
print()

# Step 9: Full experiment analysis
control_cols = ["control_1", "control_2", "control_3"]
treated_cols = ["treated_1", "treated_2", "treated_3"]

results = []
for _, row in data.iterrows():
    ctrl_vals = row[control_cols].values.astype(float)
    trt_vals = row[treated_cols].values.astype(float)
    ctrl_mean = np.mean(ctrl_vals)
    trt_mean = np.mean(trt_vals)
    fc = trt_mean / ctrl_mean
    log2fc = np.log2(fc)
    t_stat, p_val = stats.ttest_ind(ctrl_vals, trt_vals)
    results.append({
        "gene": row["gene"],
        "ctrl_mean": round(ctrl_mean, 1),
        "trt_mean": round(trt_mean, 1),
        "log2fc": round(log2fc, 2),
        "pvalue": p_val,
    })

res_df = pd.DataFrame(results)
reject, adj_p, _, _ = multipletests(res_df["pvalue"], method="fdr_bh")
res_df["adj_pvalue"] = adj_p
print(res_df.to_string(index=False))
sig = res_df[res_df["adj_pvalue"] < 0.05]
print(f"\nSignificant: {len(sig)} ({(sig['log2fc'] > 0).sum()} up, {(sig['log2fc'] < 0).sum()} down)")
