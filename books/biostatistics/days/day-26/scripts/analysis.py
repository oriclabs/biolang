# Day 26: Meta-Analysis
import pandas as pd
import numpy as np
from scipy import stats

data = pd.read_csv("gwas_meta.csv")

# Fixed-effect inverse-variance weighted meta-analysis
betas = data["beta"].values
ses = data["se"].values
weights = 1 / ses**2

beta_fe = np.sum(weights * betas) / np.sum(weights)
se_fe = 1 / np.sqrt(np.sum(weights))
z_fe = beta_fe / se_fe
p_fe = 2 * stats.norm.sf(abs(z_fe))

print("=== Fixed-Effect Meta-Analysis ===")
print(f"Combined beta: {beta_fe:.5f}")
print(f"Combined SE:   {se_fe:.5f}")
print(f"Z-statistic:   {z_fe:.3f}")
print(f"P-value:       {p_fe:.4e}")
print(f"95% CI: ({beta_fe - 1.96*se_fe:.5f}, {beta_fe + 1.96*se_fe:.5f})")

# Heterogeneity (Cochran's Q)
Q = np.sum(weights * (betas - beta_fe)**2)
df = len(betas) - 1
p_het = 1 - stats.chi2.cdf(Q, df)
I2 = max(0, (Q - df) / Q * 100)
print(f"\n=== Heterogeneity ===")
print(f"Q = {Q:.2f}, df={df}, p={p_het:.4f}")
print(f"I-squared: {I2:.1f}%")

# Random-effects (DerSimonian-Laird)
tau2 = max(0, (Q - df) / (np.sum(weights) - np.sum(weights**2)/np.sum(weights)))
w_re = 1 / (ses**2 + tau2)
beta_re = np.sum(w_re * betas) / np.sum(w_re)
se_re = 1 / np.sqrt(np.sum(w_re))
print(f"\n=== Random-Effects ===")
print(f"tau2: {tau2:.6f}")
print(f"Combined beta: {beta_re:.5f} (SE={se_re:.5f})")

# Forest plot data
print("\n=== Forest Plot Data ===")
for _, row in data.iterrows():
    ci_lo = row.beta - 1.96 * row.se
    ci_hi = row.beta + 1.96 * row.se
    print(f"  {row.study:15s}: {row.beta:.5f} ({ci_lo:.5f}, {ci_hi:.5f})")
