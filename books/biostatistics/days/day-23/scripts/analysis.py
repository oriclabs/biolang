# Day 23: Resampling Methods
import pandas as pd
import numpy as np

np.random.seed(42)
data = pd.read_csv("small_sample.csv")
trt = data[data.group == "Treatment"]["value"].values
ctrl = data[data.group == "Control"]["value"].values
obs_diff = trt.mean() - ctrl.mean()

# Bootstrap CI for mean difference
n_boot = 10000
boot_diffs = []
for _ in range(n_boot):
    b_trt = np.random.choice(trt, size=len(trt), replace=True)
    b_ctrl = np.random.choice(ctrl, size=len(ctrl), replace=True)
    boot_diffs.append(b_trt.mean() - b_ctrl.mean())

boot_diffs = np.array(boot_diffs)
ci_lo, ci_hi = np.percentile(boot_diffs, [2.5, 97.5])
print("=== Bootstrap (10,000 resamples) ===")
print(f"Observed diff: {obs_diff:.2f}")
print(f"Bootstrap mean: {boot_diffs.mean():.2f}")
print(f"95% CI: ({ci_lo:.2f}, {ci_hi:.2f})")

# Permutation test
n_perm = 10000
combined = np.concatenate([trt, ctrl])
n_trt = len(trt)
perm_diffs = []
for _ in range(n_perm):
    perm = np.random.permutation(combined)
    perm_diffs.append(perm[:n_trt].mean() - perm[n_trt:].mean())

perm_diffs = np.array(perm_diffs)
p_perm = (np.abs(perm_diffs) >= abs(obs_diff)).mean()
print(f"\n=== Permutation Test (10,000 permutations) ===")
print(f"P-value: {p_perm:.4f}")
