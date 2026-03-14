# Day 8: t-Tests
import pandas as pd
from scipy import stats

expr = pd.read_csv("expression.csv")
paired = pd.read_csv("paired_treatment.csv")

# Independent t-tests per gene
print("=== Independent Two-Sample t-Tests ===")
for gene in expr.gene.unique():
    gdata = expr[expr.gene == gene]
    tumor = gdata[gdata.group == "Tumor"]["expression"]
    normal = gdata[gdata.group == "Normal"]["expression"]
    t_stat, p_val = stats.ttest_ind(tumor, normal)
    fc = tumor.mean() / normal.mean()
    print(f"{gene:8s}: t={t_stat:6.2f}, p={p_val:.4f}, FC={fc:.2f}")

# Paired t-test
print("\n=== Paired t-Test (Before/After Treatment) ===")
t_stat, p_val = stats.ttest_rel(paired["before"], paired["after"])
diff = paired["before"] - paired["after"]
print(f"Mean before:  {paired['before'].mean():.1f}")
print(f"Mean after:   {paired['after'].mean():.1f}")
print(f"Mean diff:    {diff.mean():.1f}")
print(f"t-statistic:  {t_stat:.3f}")
print(f"p-value:      {p_val:.4f}")
