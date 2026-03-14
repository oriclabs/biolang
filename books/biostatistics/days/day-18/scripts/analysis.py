# Day 18: Experimental Design and Power Analysis
import pandas as pd
import numpy as np
from statsmodels.stats.power import TTestIndPower

data = pd.read_csv("power_simulation.csv")

# Empirical power from simulations
print("=== Empirical Power (from simulation) ===")
for n in sorted(data.sample_size.unique()):
    subset = data[data.sample_size == n]
    power = subset.significant.mean()
    print(f"  n={n:3d}: power={power:.3f}")

# Analytical power calculation
print("\n=== Analytical Power (statsmodels) ===")
analysis = TTestIndPower()
for n in [5, 10, 20, 50]:
    power = analysis.solve_power(effect_size=0.8, nobs1=n, alpha=0.05)
    print(f"  n={n:3d}: power={power:.3f}")

# Required sample size for 80% power
n_needed = analysis.solve_power(effect_size=0.8, power=0.8, alpha=0.05)
print(f"\nRequired n per group for 80% power (d=0.8): {int(np.ceil(n_needed))}")

# Sensitivity: minimum detectable effect at n=20
min_d = analysis.solve_power(nobs1=20, power=0.8, alpha=0.05)
print(f"Min detectable effect at n=20, 80% power: d={min_d:.3f}")
