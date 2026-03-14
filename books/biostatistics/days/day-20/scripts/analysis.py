# Day 20: Batch Effects
import pandas as pd
import numpy as np
from sklearn.decomposition import PCA
from sklearn.preprocessing import StandardScaler

data = pd.read_csv("batch_expression.csv")
gene_cols = [c for c in data.columns if c.startswith("gene_")]
X = data[gene_cols].values

# PCA before correction
scaler = StandardScaler()
X_scaled = scaler.fit_transform(X)
pca = PCA(n_components=5)
pcs = pca.fit_transform(X_scaled)

print("=== PCA Before Batch Correction ===")
print(f"PC1 explains {pca.explained_variance_ratio_[0]*100:.1f}% variance")
print(f"PC2 explains {pca.explained_variance_ratio_[1]*100:.1f}% variance")

# Check PC1 association with batch vs condition
from scipy import stats
for label, col in [("Batch", "batch"), ("Condition", "condition")]:
    groups = [pcs[data[col] == g, 0] for g in data[col].unique()]
    f_stat, p = stats.f_oneway(*groups)
    print(f"PC1 ~ {label}: F={f_stat:.2f}, p={p:.4e}")

# Simple batch correction: subtract batch means
X_corrected = X.copy()
for batch in data.batch.unique():
    mask = data.batch == batch
    batch_mean = X[mask].mean(axis=0)
    grand_mean = X.mean(axis=0)
    X_corrected[mask] = X[mask] - batch_mean + grand_mean

# PCA after correction
X_corr_scaled = scaler.fit_transform(X_corrected)
pcs_corr = pca.fit_transform(X_corr_scaled)
print(f"\n=== PCA After Batch Correction ===")
print(f"PC1 explains {pca.explained_variance_ratio_[0]*100:.1f}% variance")
