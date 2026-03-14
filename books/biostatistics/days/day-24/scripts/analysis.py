# Day 24: Bayesian Statistics
import numpy as np
from scipy import stats
import pandas as pd

# Variant carrier frequency: Beta-Binomial model
k, n = 3, 1000
# Prior: Beta(1, 1) = Uniform
# Posterior: Beta(1+k, 1+n-k) = Beta(4, 998)
a_post, b_post = 1 + k, 1 + n - k
posterior = stats.beta(a_post, b_post)

print("=== Bayesian Variant Frequency ===")
print(f"Observed: {k}/{n} = {k/n:.4f}")
print(f"Posterior: Beta({a_post}, {b_post})")
print(f"Posterior mean:   {posterior.mean():.5f}")
print(f"Posterior median: {posterior.median():.5f}")
print(f"95% credible interval: ({posterior.ppf(0.025):.5f}, {posterior.ppf(0.975):.5f})")

# Informative prior: Beta(2, 500) — prior belief ~0.004
a_prior, b_prior = 2, 500
a_post2 = a_prior + k
b_post2 = b_prior + n - k
post2 = stats.beta(a_post2, b_post2)
print(f"\nWith informative prior Beta(2,500):")
print(f"Posterior: Beta({a_post2}, {b_post2})")
print(f"Mean: {post2.mean():.5f}")
print(f"95% CI: ({post2.ppf(0.025):.5f}, {post2.ppf(0.975):.5f})")

# Sequential Bayesian updating with diagnostic test
diag = pd.read_csv("bayesian_diagnostic.csv")
print("\n=== Sequential Bayesian Updating (Prevalence) ===")
a, b = 1, 1  # start uniform
for i, row in diag.iterrows():
    if row.test_positive:
        a += 1
    else:
        b += 1
print(f"After {len(diag)} tests: Beta({a}, {b})")
print(f"Estimated prevalence: {a/(a+b):.3f}")
