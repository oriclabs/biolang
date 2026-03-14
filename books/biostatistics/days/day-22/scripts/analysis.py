# Day 22: Clustering
import pandas as pd
import numpy as np
from sklearn.cluster import KMeans, AgglomerativeClustering, DBSCAN
from sklearn.preprocessing import StandardScaler
from sklearn.metrics import adjusted_rand_score, silhouette_score

data = pd.read_csv("tumor_expression.csv")
gene_cols = [c for c in data.columns if c.startswith("gene_")]
X = StandardScaler().fit_transform(data[gene_cols].values)
true_labels = data["true_subtype"].values

# K-means
km = KMeans(n_clusters=4, random_state=42, n_init=10)
km_labels = km.fit_predict(X)
print("=== K-Means (k=4) ===")
print(f"ARI: {adjusted_rand_score(true_labels, km_labels):.3f}")
print(f"Silhouette: {silhouette_score(X, km_labels):.3f}")

# Hierarchical
hc = AgglomerativeClustering(n_clusters=4)
hc_labels = hc.fit_predict(X)
print(f"\n=== Hierarchical (k=4) ===")
print(f"ARI: {adjusted_rand_score(true_labels, hc_labels):.3f}")
print(f"Silhouette: {silhouette_score(X, hc_labels):.3f}")

# Elbow method
print("\n=== Elbow Method ===")
for k in [2, 3, 4, 5, 6]:
    km_k = KMeans(n_clusters=k, random_state=42, n_init=10).fit(X)
    sil = silhouette_score(X, km_k.labels_)
    print(f"  k={k}: inertia={km_k.inertia_:.0f}, silhouette={sil:.3f}")
