# Day 11: Categorical Data
import pandas as pd
from scipy import stats

snp = pd.read_csv("snp_case_control.csv")
hwe = pd.read_csv("hwe_test.csv")
rare = pd.read_csv("rare_variant.csv")

# Chi-square test: SNP case-control
ct = pd.crosstab(snp["status"], snp["genotype"])
print("=== Chi-Square (SNP Case-Control) ===")
print(ct)
chi2, p, dof, expected = stats.chi2_contingency(ct)
print(f"Chi2={chi2:.2f}, df={dof}, p={p:.4f}")

# Hardy-Weinberg test
counts = hwe["genotype_count"].value_counts().sort_index()
n = len(hwe)
p_hat = (2 * counts.get(2, 0) + counts.get(1, 0)) / (2 * n)
q_hat = 1 - p_hat
exp_hwe = [q_hat**2 * n, 2*p_hat*q_hat * n, p_hat**2 * n]
obs_hwe = [counts.get(0, 0), counts.get(1, 0), counts.get(2, 0)]
chi2_hwe, p_hwe = stats.chisquare(obs_hwe, f_exp=exp_hwe)
print(f"\n=== HWE Test ===")
print(f"p_hat={p_hat:.3f}, Chi2={chi2_hwe:.3f}, p={p_hwe:.4f}")

# Fisher's exact: rare variant
ct_rare = pd.crosstab(rare["status"], rare["carrier"])
print(f"\n=== Fisher's Exact (Rare Variant) ===")
print(ct_rare)
odds_ratio, p_fisher = stats.fisher_exact(ct_rare)
print(f"Odds ratio: {odds_ratio:.2f}, p={p_fisher:.4f}")
