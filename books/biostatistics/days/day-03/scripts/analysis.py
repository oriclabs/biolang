# Day 3: Distributions
import numpy as np
import pandas as pd
from scipy import stats

normal = pd.read_csv("normal.csv")["value"]
lognorm = pd.read_csv("lognormal.csv")["value"]
poisson = pd.read_csv("poisson.csv")["value"]

# Fit normal
mu, sigma = stats.norm.fit(normal)
print(f"Normal fit: mu={mu:.3f}, sigma={sigma:.3f}")
_, p = stats.shapiro(normal[:500])
print(f"Shapiro-Wilk p={p:.4f}")

# Fit log-normal
s, loc, scale = stats.lognorm.fit(lognorm, floc=0)
print(f"\nLog-normal fit: s={s:.3f}, scale={scale:.3f}")
_, p = stats.shapiro(np.log(lognorm)[:500])
print(f"Shapiro-Wilk on log(x): p={p:.4f}")

# Fit Poisson
lam = poisson.mean()
print(f"\nPoisson fit: lambda={lam:.3f}")
print(f"Variance/Mean ratio: {poisson.var() / poisson.mean():.3f} (expect ~1)")

# Kolmogorov-Smirnov tests
_, p_norm = stats.kstest(normal, "norm", args=(mu, sigma))
print(f"\nKS test (normal): p={p_norm:.4f}")
