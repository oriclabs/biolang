# Day 21: PCA
import pandas as pd
import numpy as np
from sklearn.decomposition import PCA
from sklearn.preprocessing import StandardScaler

data = pd.read_csv("single_cell.csv")
gene_cols = [c for c in data.columns if c.startswith("gene_")]
X = data[gene_cols].values
labels = data["true_cluster"].values

# PCA
scaler = StandardScaler()
X_scaled = scaler.fit_transform(X)
pca = PCA(n_components=10)
pcs = pca.fit_transform(X_scaled)

print("=== PCA on Single-Cell Data ===")
print(f"Cells: {X.shape[0]}, Genes: {X.shape[1]}")
for i in range(5):
    print(f"  PC{i+1}: {pca.explained_variance_ratio_[i]*100:.1f}%")
print(f"  Total (PC1-5): {sum(pca.explained_variance_ratio_[:5])*100:.1f}%")

# Cluster separation in PC space
from scipy import stats
print("\n=== Cluster Separation ===")
for pc in range(3):
    groups = [pcs[labels == c, pc] for c in [1, 2, 3]]
    f_stat, p = stats.f_oneway(*groups)
    print(f"  PC{pc+1} ~ cluster: F={f_stat:.1f}, p={p:.4e}")

# Top loading genes for PC1
loadings = pca.components_[0]
top_genes = np.argsort(np.abs(loadings))[-10:][::-1]
print("\nTop 10 genes for PC1:")
for idx in top_genes:
    print(f"  {gene_cols[idx]}: {loadings[idx]:.4f}")
