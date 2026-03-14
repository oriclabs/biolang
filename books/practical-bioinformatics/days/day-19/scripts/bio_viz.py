#!/usr/bin/env python3
"""Day 19: Biological Data Visualization — Python equivalent.

Requires: matplotlib, seaborn, pandas, numpy, scipy, scikit-learn, lifelines
See ../python/requirements.txt for installation.
"""

import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
import seaborn as sns
from scipy import stats
from sklearn.decomposition import PCA
from sklearn.metrics import roc_curve, auc
from lifelines import KaplanMeierFitter
import os

os.makedirs("figures", exist_ok=True)

print("=" * 60)
print("Day 19: Biological Data Visualization (Python)")
print("=" * 60)

# ── Section 1: GWAS Visualization ────────────────────────────────────

print("\n── GWAS Visualization ──\n")

gwas = pd.read_csv("data/gwas.csv")
print(f"Loaded GWAS data: {len(gwas)} variants")

# Manhattan plot
fig, ax = plt.subplots(figsize=(14, 5))
chrom_order = [f"chr{i}" for i in range(1, 23)] + ["chrX"]
gwas["chrom"] = pd.Categorical(gwas["chrom"], categories=chrom_order, ordered=True)
gwas = gwas.sort_values(["chrom", "pos"])
gwas["neglog10p"] = -np.log10(gwas["pvalue"])

# Assign x positions with chromosome offsets
chrom_offsets = {}
cumulative = 0
for chrom in chrom_order:
    chrom_offsets[chrom] = cumulative
    chrom_data = gwas[gwas["chrom"] == chrom]
    if len(chrom_data) > 0:
        cumulative += chrom_data["pos"].max() + 1e6

gwas["genome_pos"] = gwas.apply(
    lambda row: chrom_offsets.get(row["chrom"], 0) + row["pos"], axis=1
)

colors = ["#1f77b4", "#aec7e8"]
for i, chrom in enumerate(chrom_order):
    subset = gwas[gwas["chrom"] == chrom]
    ax.scatter(subset["genome_pos"], subset["neglog10p"],
               c=colors[i % 2], s=10, alpha=0.7)

ax.axhline(y=-np.log10(5e-8), color="red", linestyle="--", linewidth=0.8)
ax.set_xlabel("Chromosome")
ax.set_ylabel("-log10(p-value)")
ax.set_title("Genome-Wide Association Study")
plt.tight_layout()
plt.savefig("figures/manhattan.png", dpi=150)
plt.close()
print("Saved figures/manhattan.png")

# QQ plot
observed = -np.log10(np.sort(gwas["pvalue"].values))
expected = -np.log10(np.linspace(1 / len(observed), 1, len(observed)))

fig, ax = plt.subplots(figsize=(6, 6))
ax.scatter(expected, observed, s=10, alpha=0.7)
lims = [0, max(max(expected), max(observed)) + 0.5]
ax.plot(lims, lims, "r--", linewidth=0.8)
ax.set_xlabel("Expected -log10(p)")
ax.set_ylabel("Observed -log10(p)")
ax.set_title("QQ Plot — Observed vs Expected")
plt.tight_layout()
plt.savefig("figures/qq_plot.png", dpi=150)
plt.close()
print("Saved figures/qq_plot.png")

# ── Section 2: Expression Visualization ──────────────────────────────

print("\n── Expression Visualization ──\n")

# Violin plot
groups_data = {
    "control": [5.2, 4.8, 5.1, 4.9, 5.3, 5.0, 4.7, 5.4],
    "low_dose": [6.5, 7.1, 6.8, 6.3, 7.0, 6.6, 6.9, 7.2],
    "high_dose": [9.2, 8.8, 9.5, 9.0, 8.6, 9.3, 8.9, 9.1],
}
df_violin = pd.DataFrame([
    {"group": k, "value": v}
    for k, vals in groups_data.items()
    for v in vals
])

fig, ax = plt.subplots(figsize=(8, 5))
sns.violinplot(data=df_violin, x="group", y="value", ax=ax)
ax.set_title("Expression by Treatment Group")
plt.tight_layout()
plt.savefig("figures/violin.png", dpi=150)
plt.close()
print("Saved figures/violin.png")

# Density plot
values = [2.1, 3.5, 4.2, 5.8, 6.1, 7.3, 3.8, 5.5, 4.9, 6.7, 3.2, 5.1, 4.5, 6.0, 7.8]
fig, ax = plt.subplots(figsize=(8, 5))
sns.kdeplot(values, ax=ax, fill=True)
ax.set_title("Expression Density")
plt.tight_layout()
plt.savefig("figures/density.png", dpi=150)
plt.close()
print("Saved figures/density.png")

# PCA plot
expr = pd.read_csv("data/expression_matrix.csv", index_col=0)
pca = PCA(n_components=2)
pcs = pca.fit_transform(expr.T)  # transpose: samples as rows
labels = expr.columns

fig, ax = plt.subplots(figsize=(8, 6))
colors_pca = ["blue"] * 4 + ["red"] * 4
ax.scatter(pcs[:, 0], pcs[:, 1], c=colors_pca, s=80)
for i, label in enumerate(labels):
    ax.annotate(label, (pcs[i, 0], pcs[i, 1]), fontsize=8)
ax.set_xlabel(f"PC1 ({pca.explained_variance_ratio_[0]:.1%})")
ax.set_ylabel(f"PC2 ({pca.explained_variance_ratio_[1]:.1%})")
ax.set_title("PCA — Sample Clustering")
plt.tight_layout()
plt.savefig("figures/pca.png", dpi=150)
plt.close()
print("Saved figures/pca.png")

# Clustered heatmap
g = sns.clustermap(expr.T, figsize=(10, 6), cmap="viridis")
g.fig.suptitle("Hierarchical Clustering", y=1.02)
g.savefig("figures/clustered_heatmap.png", dpi=150)
plt.close()
print("Saved figures/clustered_heatmap.png")

# ── Section 3: Clinical Visualization ────────────────────────────────

print("\n── Clinical Visualization ──\n")

# Kaplan-Meier
km_data = pd.DataFrame([
    {"time": 12, "event": 1}, {"time": 24, "event": 1}, {"time": 36, "event": 0},
    {"time": 8, "event": 1}, {"time": 48, "event": 0}, {"time": 15, "event": 1},
    {"time": 30, "event": 0}, {"time": 20, "event": 1}, {"time": 42, "event": 0},
    {"time": 6, "event": 1},
])

fig, ax = plt.subplots(figsize=(8, 5))
kmf = KaplanMeierFitter()
kmf.fit(km_data["time"], event_observed=km_data["event"])
kmf.plot_survival_function(ax=ax)
ax.set_title("Overall Survival")
ax.set_xlabel("Time (months)")
ax.set_ylabel("Survival probability")
plt.tight_layout()
plt.savefig("figures/kaplan_meier.png", dpi=150)
plt.close()
print("Saved figures/kaplan_meier.png")

# ROC curve
pred_data = pd.DataFrame([
    {"score": 0.9, "label": 1}, {"score": 0.8, "label": 1}, {"score": 0.7, "label": 0},
    {"score": 0.6, "label": 1}, {"score": 0.5, "label": 0}, {"score": 0.4, "label": 0},
    {"score": 0.3, "label": 0}, {"score": 0.2, "label": 1}, {"score": 0.1, "label": 0},
])

fpr, tpr, _ = roc_curve(pred_data["label"], pred_data["score"])
roc_auc = auc(fpr, tpr)

fig, ax = plt.subplots(figsize=(6, 6))
ax.plot(fpr, tpr, linewidth=2, label=f"AUC = {roc_auc:.2f}")
ax.plot([0, 1], [0, 1], "k--", linewidth=0.8)
ax.set_xlabel("False Positive Rate")
ax.set_ylabel("True Positive Rate")
ax.set_title("Classifier Performance")
ax.legend()
plt.tight_layout()
plt.savefig("figures/roc_curve.png", dpi=150)
plt.close()
print("Saved figures/roc_curve.png")

# Forest plot — manual with matplotlib
studies_data = [
    {"study": "Smith 2020", "effect": 1.5, "ci_lower": 1.1, "ci_upper": 2.0},
    {"study": "Jones 2021", "effect": 1.8, "ci_lower": 1.3, "ci_upper": 2.5},
    {"study": "Chen 2022", "effect": 1.2, "ci_lower": 0.8, "ci_upper": 1.8},
    {"study": "Patel 2023", "effect": 1.6, "ci_lower": 1.2, "ci_upper": 2.1},
]

fig, ax = plt.subplots(figsize=(8, 4))
for i, s in enumerate(studies_data):
    ax.errorbar(s["effect"], i, xerr=[[s["effect"] - s["ci_lower"]], [s["ci_upper"] - s["effect"]]],
                fmt="o", color="black", capsize=3)
    ax.text(0.5, i, s["study"], va="center", fontsize=9)
ax.axvline(x=1.0, color="gray", linestyle="--", linewidth=0.8)
ax.set_yticks(range(len(studies_data)))
ax.set_yticklabels([""] * len(studies_data))
ax.set_xlabel("Effect Size")
ax.set_title("Meta-Analysis: Gene X Association")
plt.tight_layout()
plt.savefig("figures/forest_plot.png", dpi=150)
plt.close()
print("Saved figures/forest_plot.png")

# ── Section 4: Sequence visualization ────────────────────────────────

print("\n── Sequence Visualization ──\n")

# Sequence logo — requires logomaker
try:
    import logomaker
    seqs = [
        "TATAAAGC", "TATAATGC", "TATAAAGC", "TATAATGC",
        "TATAAAGC", "TATAATGC", "TATAAAGC", "TATAATGC",
    ]
    counts = logomaker.alignment_to_matrix(seqs, to_type="counts")
    info = logomaker.alignment_to_matrix(seqs, to_type="information")

    fig, ax = plt.subplots(figsize=(8, 3))
    logomaker.Logo(info, ax=ax)
    ax.set_title("TATA Box Motif")
    ax.set_ylabel("Information (bits)")
    plt.tight_layout()
    plt.savefig("figures/sequence_logo.png", dpi=150)
    plt.close()
    print("Saved figures/sequence_logo.png")
except ImportError:
    print("Skipping sequence logo (install logomaker: pip install logomaker)")

print("\n" + "=" * 60)
print("Python visualization complete.")
print("=" * 60)
