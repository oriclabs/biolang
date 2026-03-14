# Day 5: Sampling and the Central Limit Theorem
import numpy as np
import pandas as pd

pop = pd.read_csv("population.csv")["allele_freq"].values
samples = pd.read_csv("sample_means.csv")

print("=== Population ===")
print(f"N: {len(pop)}, Mean: {pop.mean():.4f}, SD: {pop.std():.4f}")

print("\n=== Sample Means (n=20) ===")
m20 = samples["mean_n20"]
print(f"Mean of means: {m20.mean():.4f}")
print(f"SD of means:   {m20.std():.4f}")
print(f"Expected SE:   {pop.std() / np.sqrt(20):.4f}")

print("\n=== Sample Means (n=200) ===")
m200 = samples["mean_n200"]
print(f"Mean of means: {m200.mean():.4f}")
print(f"SD of means:   {m200.std():.4f}")
print(f"Expected SE:   {pop.std() / np.sqrt(200):.4f}")

# Demonstrate CLT: larger n -> tighter distribution
print(f"\nSD ratio (n20/n200): {m20.std() / m200.std():.2f}")
print(f"Expected ratio sqrt(200/20): {np.sqrt(200/20):.2f}")
