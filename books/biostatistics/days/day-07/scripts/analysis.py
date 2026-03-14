# Day 7: Hypothesis Testing
import numpy as np
import pandas as pd
from scipy import stats

bio = pd.read_csv("biomarker.csv")
null_z = pd.read_csv("null_simulation.csv")["z_statistic"]

ad = bio[bio.group == "AD"]["biomarker_ngml"]
ctrl = bio[bio.group == "Control"]["biomarker_ngml"]

# Manual z-test (known population parameters for illustration)
mean_diff = ad.mean() - ctrl.mean()
se = np.sqrt(ad.var()/len(ad) + ctrl.var()/len(ctrl))
z_obs = mean_diff / se
p_val = 2 * (1 - stats.norm.cdf(abs(z_obs)))

print("=== Two-Sample Z-Test ===")
print(f"AD mean:      {ad.mean():.2f}")
print(f"Control mean: {ctrl.mean():.2f}")
print(f"Difference:   {mean_diff:.2f}")
print(f"Z-statistic:  {z_obs:.3f}")
print(f"P-value:      {p_val:.4f}")
print(f"Reject H0 at alpha=0.05? {'Yes' if p_val < 0.05 else 'No'}")

# Null simulation: what fraction exceed our z?
exceed = (np.abs(null_z) >= abs(z_obs)).mean()
print(f"\nNull simulation: {exceed:.3f} of z-stats exceed |{z_obs:.2f}|")
