#!/usr/bin/env python3
"""Day 15: Publication-Quality Visualization — Python equivalent."""

import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
import matplotlib
import os

matplotlib.use("Agg")  # Non-interactive backend for SVG output

os.makedirs("figures", exist_ok=True)

# ── Load data ──────────────────────────────────────────────────────

de = pd.read_csv("data/de_results.csv")
print(f"Loaded DE results: {len(de)} genes")

gwas = pd.read_csv("data/gwas_results.csv")
print(f"Loaded GWAS results: {len(gwas)} variants")

# ── Scatter plot ───────────────────────────────────────────────────

x = [1.0, 2.0, 3.0, 4.0, 5.0]
y = [2.1, 3.9, 6.2, 7.8, 10.1]

fig, ax = plt.subplots(figsize=(6, 4))
ax.scatter(x, y, color="steelblue")
ax.set_title("Gene Expression Correlation")
ax.set_xlabel("x")
ax.set_ylabel("y")
plt.tight_layout()
plt.savefig("figures/scatter.svg", format="svg")
plt.close()
print("Saved figures/scatter.svg")

# ── Histogram ──────────────────────────────────────────────────────

values = [2.1, 3.5, 4.2, 5.8, 6.1, 7.3, 3.8, 5.5, 4.9, 6.7, 3.2, 5.1]

fig, ax = plt.subplots(figsize=(6, 4))
ax.hist(values, bins=6, color="steelblue", edgecolor="white")
ax.set_title("Expression Distribution")
ax.set_xlabel("Expression")
ax.set_ylabel("Count")
plt.tight_layout()
plt.savefig("figures/histogram.svg", format="svg")
plt.close()
print("Saved figures/histogram.svg")

# ── Bar chart ──────────────────────────────────────────────────────

categories = ["SNP", "Insertion", "Deletion", "MNV"]
counts = [3500, 450, 520, 30]

fig, ax = plt.subplots(figsize=(6, 4))
ax.bar(categories, counts, color="steelblue")
ax.set_title("Variant Types")
ax.set_ylabel("Count")
plt.tight_layout()
plt.savefig("figures/bar_chart.svg", format="svg")
plt.close()
print("Saved figures/bar_chart.svg")

# ── Boxplot ────────────────────────────────────────────────────────

control = [5.2, 4.8, 5.1, 4.9, 5.3, 5.0]
treated = [8.1, 7.9, 8.5, 7.6, 8.3, 8.0]
resistant = [5.5, 5.3, 5.8, 5.1, 5.6, 5.4]

fig, ax = plt.subplots(figsize=(6, 4))
ax.boxplot([control, treated, resistant], labels=["control", "treated", "resistant"])
ax.set_title("Expression by Group")
ax.set_ylabel("Expression")
plt.tight_layout()
plt.savefig("figures/boxplot.svg", format="svg")
plt.close()
print("Saved figures/boxplot.svg")

# ── Volcano plot ───────────────────────────────────────────────────

de["-log10p"] = -np.log10(de["padj"])

fig, ax = plt.subplots(figsize=(8, 6))
colors = []
for _, row in de.iterrows():
    if row["padj"] < 0.05 and row["log2fc"] > 1.0:
        colors.append("red")
    elif row["padj"] < 0.05 and row["log2fc"] < -1.0:
        colors.append("blue")
    else:
        colors.append("grey")

ax.scatter(de["log2fc"], de["-log10p"], c=colors, alpha=0.7)
ax.axhline(-np.log10(0.05), ls="--", color="grey", alpha=0.5)
ax.axvline(1.0, ls="--", color="grey", alpha=0.5)
ax.axvline(-1.0, ls="--", color="grey", alpha=0.5)
ax.set_xlabel("log2 Fold Change")
ax.set_ylabel("-log10(adjusted p-value)")
ax.set_title("A) Differential Expression")
plt.tight_layout()
plt.savefig("figures/fig1_volcano.svg", format="svg")
plt.close()
print("Saved figures/fig1_volcano.svg")

# ── MA plot ────────────────────────────────────────────────────────

fig, ax = plt.subplots(figsize=(8, 6))
colors_ma = ["red" if p < 0.05 else "grey" for p in de["padj"]]
ax.scatter(np.log10(de["baseMean"]), de["log2fc"], c=colors_ma, alpha=0.7)
ax.axhline(0, ls="--", color="grey", alpha=0.5)
ax.set_xlabel("log10(baseMean)")
ax.set_ylabel("log2 Fold Change")
ax.set_title("B) MA Plot")
plt.tight_layout()
plt.savefig("figures/fig2_ma.svg", format="svg")
plt.close()
print("Saved figures/fig2_ma.svg")

# ── Summary ────────────────────────────────────────────────────────

up = len(de[(de["padj"] < 0.05) & (de["log2fc"] > 1.0)])
down = len(de[(de["padj"] < 0.05) & (de["log2fc"] < -1.0)])
ns = len(de) - up - down

print(f"\nUpregulated (padj<0.05, log2FC>1):   {up}")
print(f"Downregulated (padj<0.05, log2FC<-1): {down}")
print(f"Not significant:                      {ns}")
print("\nDone.")
