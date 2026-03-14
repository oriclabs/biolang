# Day 10: ANOVA
import pandas as pd
from scipy import stats

dose = pd.read_csv("dose_response.csv")
tissue = pd.read_csv("tissue_expression.csv")

# One-way ANOVA: dose-response
groups = [g["tumor_volume_mm3"].values for _, g in dose.groupby("dose")]
f_stat, p_val = stats.f_oneway(*groups)
print("=== One-Way ANOVA (Dose-Response) ===")
for d in dose.dose.unique():
    vals = dose[dose.dose == d]["tumor_volume_mm3"]
    print(f"  {d:10s}: mean={vals.mean():.1f}, sd={vals.std():.1f}")
print(f"F-statistic: {f_stat:.2f}")
print(f"P-value:     {p_val:.4e}")

# Tissue expression ANOVA
groups_t = [g["expression"].values for _, g in tissue.groupby("tissue")]
f_t, p_t = stats.f_oneway(*groups_t)
print("\n=== One-Way ANOVA (Tissue Expression) ===")
for t in tissue.tissue.unique():
    vals = tissue[tissue.tissue == t]["expression"]
    print(f"  {t:8s}: mean={vals.mean():.2f}")
print(f"F-statistic: {f_t:.2f}")
print(f"P-value:     {p_t:.4e}")
