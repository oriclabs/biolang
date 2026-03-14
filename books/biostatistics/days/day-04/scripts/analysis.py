# Day 4: Probability
import pandas as pd
import numpy as np
from scipy import stats

# Hardy-Weinberg
hw = pd.read_csv("hardy_weinberg.csv")
counts = hw["genotype"].value_counts()
n = len(hw)
print("=== Hardy-Weinberg ===")
print(counts)

# Expected frequencies under HWE with p=0.3
p = 0.3
q = 1 - p
expected = {"AA": p**2 * n, "Aa": 2*p*q * n, "aA": 0, "aa": q**2 * n}
# Combine Aa and aA
obs_AA = counts.get("AA", 0)
obs_het = counts.get("Aa", 0) + counts.get("aA", 0)
obs_aa = counts.get("aa", 0)
exp_arr = [p**2 * n, 2*p*q * n, q**2 * n]
obs_arr = [obs_AA, obs_het, obs_aa]
chi2, pval = stats.chisquare(obs_arr, f_exp=exp_arr)
print(f"\nChi-square HWE test: chi2={chi2:.3f}, p={pval:.4f}")

# Diagnostic test: Bayes' theorem
diag = pd.read_csv("diagnostic_test.csv")
tp = ((diag.disease == 1) & (diag.test_positive == 1)).sum()
fp = ((diag.disease == 0) & (diag.test_positive == 1)).sum()
fn = ((diag.disease == 1) & (diag.test_positive == 0)).sum()
tn = ((diag.disease == 0) & (diag.test_positive == 0)).sum()
print("\n=== Diagnostic Test ===")
print(f"TP={tp}, FP={fp}, FN={fn}, TN={tn}")
ppv = tp / (tp + fp) if (tp + fp) > 0 else 0
npv = tn / (tn + fn) if (tn + fn) > 0 else 0
print(f"PPV (precision): {ppv:.4f}")
print(f"NPV: {npv:.4f}")
