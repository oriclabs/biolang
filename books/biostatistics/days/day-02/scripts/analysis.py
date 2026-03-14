# Day 2: Descriptive Statistics
import numpy as np
import pandas as pd

# Load data
quality = pd.read_csv("quality_scores.csv")
expression = pd.read_csv("expression.csv")

# Quality score descriptives
qs = quality["quality_score"]
print("=== FASTQ Quality Scores ===")
print(f"N:      {len(qs)}")
print(f"Mean:   {qs.mean():.2f}")
print(f"Median: {qs.median():.2f}")
print(f"SD:     {qs.std():.2f}")
print(f"IQR:    {qs.quantile(0.75) - qs.quantile(0.25):.2f}")
print(f"Min:    {qs.min()}, Max: {qs.max()}")
print(f"Skew:   {qs.skew():.3f}")
print(f"Kurt:   {qs.kurtosis():.3f}")

# Expression descriptives
expr = expression["expression"]
print("\n=== Gene Expression ===")
print(f"N:      {len(expr)}")
print(f"Mean:   {expr.mean():.2f}")
print(f"Median: {expr.median():.2f}")
print(f"SD:     {expr.std():.2f}")
print(f"Range:  {expr.min():.2f} - {expr.max():.2f}")

# Log-transformed
log_expr = np.log2(expr + 1)
print(f"\nLog2 Mean:   {log_expr.mean():.2f}")
print(f"Log2 Median: {log_expr.median():.2f}")
print(f"Log2 SD:     {log_expr.std():.2f}")
