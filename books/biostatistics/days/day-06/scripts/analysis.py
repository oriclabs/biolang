# Day 6: Confidence Intervals
import numpy as np
import pandas as pd
from scipy import stats

# IC50 confidence interval
ic50 = pd.read_csv("ic50.csv")["ic50_uM"]
n = len(ic50)
mean_ic50 = ic50.mean()
se = ic50.std(ddof=1) / np.sqrt(n)
ci95 = stats.t.interval(0.95, df=n-1, loc=mean_ic50, scale=se)
print("=== IC50 (t-interval) ===")
print(f"Mean: {mean_ic50:.2f} uM")
print(f"SE:   {se:.2f}")
print(f"95% CI: ({ci95[0]:.2f}, {ci95[1]:.2f})")

# Vaccine trial: proportion CI (Wilson score)
vaccine = pd.read_csv("vaccine_trial.csv")["responded"]
p_hat = vaccine.mean()
n_v = len(vaccine)
z = 1.96
# Wilson score interval
denom = 1 + z**2 / n_v
center = (p_hat + z**2 / (2*n_v)) / denom
margin = z * np.sqrt(p_hat*(1-p_hat)/n_v + z**2/(4*n_v**2)) / denom
print(f"\n=== Vaccine Efficacy ===")
print(f"p-hat: {p_hat:.3f}, n={n_v}")
print(f"Wilson 95% CI: ({center - margin:.3f}, {center + margin:.3f})")
