# Day 22: Clustering — Finding Structure in Omics Data

<div class="day-meta">
<span class="badge">Day 22 of 30</span>
<span class="badge">Prerequisites: Days 2-3, 13, 21</span>
<span class="badge">~60 min reading</span>
<span class="badge">Unsupervised Learning</span>
</div>

## The Problem

You are part of a cancer genomics consortium. Five hundred tumor samples have been profiled with RNA-seq, measuring the expression of 18,000 genes in each. The pathologist has classified these tumors into three histological subtypes based on what she sees under the microscope. But molecular data often reveals finer distinctions invisible to the eye.

Your task: find natural groupings in the gene expression data — without peeking at the pathologist's labels. If the molecular subtypes align with the histological ones, confidence in the classification increases. If the data reveals additional subtypes, you may have discovered clinically distinct groups that respond differently to treatment. In breast cancer, this is exactly how the PAM50 molecular subtypes were discovered — and they now guide treatment decisions for millions of patients worldwide.

But there is a danger lurking. Clustering algorithms always find clusters, even in random noise. The critical question is not "can I find groups?" but "are the groups real?"

## What Is Clustering?

Clustering is unsupervised grouping: divide observations into sets such that observations within a set are more similar to each other than to observations in other sets.

Unlike classification (supervised learning), clustering has no labels to learn from. You are not training the algorithm to distinguish "tumor" from "normal." You are asking the algorithm to discover structure on its own.

Think of it as sorting a pile of coins. If you have pennies, nickels, dimes, and quarters, the task is straightforward — there are obvious groupings by size and color. But if someone hands you a pile of irregularly shaped pebbles and says "sort these into groups," you must decide what "similar" means. By weight? By color? By texture? Different definitions of similarity lead to different groupings. This subjectivity is both the power and the peril of clustering.

> **Key insight:** Clustering is a tool for exploration, not proof. It generates hypotheses about structure in your data. Validating those hypotheses requires independent evidence — clinical outcomes, functional assays, or replication in new datasets.

## K-Means Clustering

K-means is the simplest and most widely used clustering algorithm. It partitions data into exactly *k* groups by iterating two steps.

### The Algorithm

1. **Choose k** (the number of clusters you want).
2. **Initialize** k random "centroids" (cluster centers).
3. **Assign** each data point to the nearest centroid.
4. **Update** each centroid to the mean of its assigned points.
5. **Repeat** steps 3-4 until assignments stop changing.

That is it. The algorithm converges quickly, typically in 10-20 iterations.

### Properties

| Property | K-means |
|---|---|
| Shape of clusters | Spherical (roughly equal-sized blobs) |
| Requires k in advance | Yes |
| Handles outliers | Poorly — outliers pull centroids |
| Handles unequal cluster sizes | Poorly — tends to split large clusters |
| Speed | Very fast, even on large datasets |
| Deterministic | No — depends on random initialization |

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="500" viewBox="0 0 680 500" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="28" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">K-Means Iteration (k=3)</text>
  <!-- Panel 1: Random initialization -->
  <rect x="15" y="45" width="200" height="200" rx="6" fill="white" stroke="#e5e7eb"/>
  <text x="115" y="65" text-anchor="middle" font-size="12" font-weight="bold" fill="#1e293b">1. Random Centroids</text>
  <!-- Data points (grey - unassigned) -->
  <circle cx="55" cy="100" r="4" fill="#9ca3af"/><circle cx="70" cy="115" r="4" fill="#9ca3af"/>
  <circle cx="45" cy="120" r="4" fill="#9ca3af"/><circle cx="65" cy="105" r="4" fill="#9ca3af"/>
  <circle cx="80" cy="110" r="4" fill="#9ca3af"/><circle cx="50" cy="108" r="4" fill="#9ca3af"/>
  <circle cx="140" cy="150" r="4" fill="#9ca3af"/><circle cx="155" cy="165" r="4" fill="#9ca3af"/>
  <circle cx="160" cy="145" r="4" fill="#9ca3af"/><circle cx="145" cy="158" r="4" fill="#9ca3af"/>
  <circle cx="150" cy="140" r="4" fill="#9ca3af"/><circle cx="135" cy="155" r="4" fill="#9ca3af"/>
  <circle cx="85" cy="195" r="4" fill="#9ca3af"/><circle cx="95" cy="210" r="4" fill="#9ca3af"/>
  <circle cx="75" cy="205" r="4" fill="#9ca3af"/><circle cx="100" cy="195" r="4" fill="#9ca3af"/>
  <circle cx="90" cy="215" r="4" fill="#9ca3af"/><circle cx="80" cy="200" r="4" fill="#9ca3af"/>
  <!-- Random centroids (X marks) -->
  <text x="100" y="100" text-anchor="middle" font-size="18" font-weight="bold" fill="#2563eb">+</text>
  <text x="170" y="200" text-anchor="middle" font-size="18" font-weight="bold" fill="#dc2626">+</text>
  <text x="130" y="120" text-anchor="middle" font-size="18" font-weight="bold" fill="#16a34a">+</text>
  <text x="115" y="238" text-anchor="middle" font-size="10" fill="#6b7280">Centroids placed randomly</text>

  <!-- Panel 2: Points assigned to nearest centroid -->
  <rect x="240" y="45" width="200" height="200" rx="6" fill="white" stroke="#e5e7eb"/>
  <text x="340" y="65" text-anchor="middle" font-size="12" font-weight="bold" fill="#1e293b">2. Assign to Nearest</text>
  <!-- Blue cluster -->
  <circle cx="280" cy="100" r="4" fill="#3b82f6"/><circle cx="295" cy="115" r="4" fill="#3b82f6"/>
  <circle cx="270" cy="120" r="4" fill="#3b82f6"/><circle cx="290" cy="105" r="4" fill="#3b82f6"/>
  <circle cx="305" cy="110" r="4" fill="#3b82f6"/><circle cx="275" cy="108" r="4" fill="#3b82f6"/>
  <!-- Green cluster -->
  <circle cx="365" cy="150" r="4" fill="#22c55e"/><circle cx="380" cy="165" r="4" fill="#22c55e"/>
  <circle cx="385" cy="145" r="4" fill="#22c55e"/><circle cx="370" cy="158" r="4" fill="#22c55e"/>
  <circle cx="375" cy="140" r="4" fill="#22c55e"/><circle cx="360" cy="155" r="4" fill="#22c55e"/>
  <!-- Red cluster -->
  <circle cx="310" cy="195" r="4" fill="#ef4444"/><circle cx="320" cy="210" r="4" fill="#ef4444"/>
  <circle cx="300" cy="205" r="4" fill="#ef4444"/><circle cx="325" cy="195" r="4" fill="#ef4444"/>
  <circle cx="315" cy="215" r="4" fill="#ef4444"/><circle cx="305" cy="200" r="4" fill="#ef4444"/>
  <!-- Centroids still at old positions -->
  <text x="325" y="100" text-anchor="middle" font-size="18" font-weight="bold" fill="#2563eb">+</text>
  <text x="395" y="200" text-anchor="middle" font-size="18" font-weight="bold" fill="#dc2626">+</text>
  <text x="355" y="120" text-anchor="middle" font-size="18" font-weight="bold" fill="#16a34a">+</text>
  <text x="340" y="238" text-anchor="middle" font-size="10" fill="#6b7280">Colors show assignments</text>

  <!-- Panel 3: Centroids move to means -->
  <rect x="465" y="45" width="200" height="200" rx="6" fill="white" stroke="#e5e7eb"/>
  <text x="565" y="65" text-anchor="middle" font-size="12" font-weight="bold" fill="#1e293b">3. Update Centroids</text>
  <!-- Blue cluster -->
  <circle cx="505" cy="100" r="4" fill="#3b82f6"/><circle cx="520" cy="115" r="4" fill="#3b82f6"/>
  <circle cx="495" cy="120" r="4" fill="#3b82f6"/><circle cx="515" cy="105" r="4" fill="#3b82f6"/>
  <circle cx="530" cy="110" r="4" fill="#3b82f6"/><circle cx="500" cy="108" r="4" fill="#3b82f6"/>
  <!-- Green cluster -->
  <circle cx="590" cy="150" r="4" fill="#22c55e"/><circle cx="605" cy="165" r="4" fill="#22c55e"/>
  <circle cx="610" cy="145" r="4" fill="#22c55e"/><circle cx="595" cy="158" r="4" fill="#22c55e"/>
  <circle cx="600" cy="140" r="4" fill="#22c55e"/><circle cx="585" cy="155" r="4" fill="#22c55e"/>
  <!-- Red cluster -->
  <circle cx="535" cy="195" r="4" fill="#ef4444"/><circle cx="545" cy="210" r="4" fill="#ef4444"/>
  <circle cx="525" cy="205" r="4" fill="#ef4444"/><circle cx="550" cy="195" r="4" fill="#ef4444"/>
  <circle cx="540" cy="215" r="4" fill="#ef4444"/><circle cx="530" cy="200" r="4" fill="#ef4444"/>
  <!-- Centroids at cluster centers now -->
  <text x="511" y="113" text-anchor="middle" font-size="20" font-weight="bold" fill="#2563eb">+</text>
  <text x="538" y="207" text-anchor="middle" font-size="20" font-weight="bold" fill="#dc2626">+</text>
  <text x="598" y="155" text-anchor="middle" font-size="20" font-weight="bold" fill="#16a34a">+</text>
  <text x="565" y="238" text-anchor="middle" font-size="10" fill="#6b7280">Centroids at cluster means</text>

  <!-- Arrows between panels -->
  <text x="227" y="150" text-anchor="middle" font-size="22" fill="#9ca3af">&rarr;</text>
  <text x="452" y="150" text-anchor="middle" font-size="22" fill="#9ca3af">&rarr;</text>

  <!-- Legend -->
  <rect x="15" y="258" width="650" height="28" rx="4" fill="#f1f5f9"/>
  <text x="30" y="276" font-size="18" font-weight="bold" fill="#2563eb">+</text>
  <text x="46" y="276" font-size="11" fill="#6b7280">Blue centroid</text>
  <text x="160" y="276" font-size="18" font-weight="bold" fill="#dc2626">+</text>
  <text x="176" y="276" font-size="11" fill="#6b7280">Red centroid</text>
  <text x="280" y="276" font-size="18" font-weight="bold" fill="#16a34a">+</text>
  <text x="296" y="276" font-size="11" fill="#6b7280">Green centroid</text>
  <text x="420" y="276" font-size="11" fill="#6b7280">Repeat steps 2-3 until convergence</text>

  <!-- Dendrogram section -->
  <text x="340" y="310" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">Hierarchical Clustering: Dendrogram</text>
  <!-- Leaf labels -->
  <text x="60" y="488" text-anchor="middle" font-size="10" fill="#6b7280">S1</text>
  <text x="120" y="488" text-anchor="middle" font-size="10" fill="#6b7280">S2</text>
  <text x="180" y="488" text-anchor="middle" font-size="10" fill="#6b7280">S3</text>
  <text x="240" y="488" text-anchor="middle" font-size="10" fill="#6b7280">S4</text>
  <text x="330" y="488" text-anchor="middle" font-size="10" fill="#6b7280">S5</text>
  <text x="400" y="488" text-anchor="middle" font-size="10" fill="#6b7280">S6</text>
  <text x="490" y="488" text-anchor="middle" font-size="10" fill="#6b7280">S7</text>
  <text x="560" y="488" text-anchor="middle" font-size="10" fill="#6b7280">S8</text>
  <!-- Vertical lines from leaves -->
  <line x1="60" y1="475" x2="60" y2="445" stroke="#3b82f6" stroke-width="2"/>
  <line x1="120" y1="475" x2="120" y2="445" stroke="#3b82f6" stroke-width="2"/>
  <line x1="180" y1="475" x2="180" y2="430" stroke="#3b82f6" stroke-width="2"/>
  <line x1="240" y1="475" x2="240" y2="430" stroke="#3b82f6" stroke-width="2"/>
  <line x1="330" y1="475" x2="330" y2="440" stroke="#dc2626" stroke-width="2"/>
  <line x1="400" y1="475" x2="400" y2="440" stroke="#dc2626" stroke-width="2"/>
  <line x1="490" y1="475" x2="490" y2="435" stroke="#16a34a" stroke-width="2"/>
  <line x1="560" y1="475" x2="560" y2="435" stroke="#16a34a" stroke-width="2"/>
  <!-- First merge level: S1+S2, S3+S4, S5+S6, S7+S8 -->
  <line x1="60" y1="445" x2="120" y2="445" stroke="#3b82f6" stroke-width="2"/>
  <line x1="180" y1="430" x2="240" y2="430" stroke="#3b82f6" stroke-width="2"/>
  <line x1="330" y1="440" x2="400" y2="440" stroke="#dc2626" stroke-width="2"/>
  <line x1="490" y1="435" x2="560" y2="435" stroke="#16a34a" stroke-width="2"/>
  <!-- Second merge: (S1,S2)+(S3,S4), (S5,S6)+(S7,S8) -->
  <line x1="90" y1="445" x2="90" y2="400" stroke="#3b82f6" stroke-width="2"/>
  <line x1="210" y1="430" x2="210" y2="400" stroke="#3b82f6" stroke-width="2"/>
  <line x1="90" y1="400" x2="210" y2="400" stroke="#3b82f6" stroke-width="2"/>
  <line x1="365" y1="440" x2="365" y2="395" stroke="#dc2626" stroke-width="2"/>
  <line x1="525" y1="435" x2="525" y2="395" stroke="#16a34a" stroke-width="2"/>
  <line x1="365" y1="395" x2="525" y2="395" stroke="#7c3aed" stroke-width="2"/>
  <!-- Third merge: all together -->
  <line x1="150" y1="400" x2="150" y2="350" stroke="#7c3aed" stroke-width="2"/>
  <line x1="445" y1="395" x2="445" y2="350" stroke="#7c3aed" stroke-width="2"/>
  <line x1="150" y1="350" x2="445" y2="350" stroke="#7c3aed" stroke-width="2"/>
  <!-- Cut line -->
  <line x1="20" y1="410" x2="650" y2="410" stroke="#dc2626" stroke-width="1.5" stroke-dasharray="8,4"/>
  <text x="630" y="405" text-anchor="end" font-size="11" font-weight="bold" fill="#dc2626">Cut here &rarr; 3 clusters</text>
  <!-- Height axis label -->
  <text x="18" y="420" font-size="10" fill="#9ca3af">Height</text>
  <!-- Cluster color bars at bottom -->
  <rect x="40" y="493" width="220" height="4" rx="2" fill="#3b82f6"/>
  <rect x="310" y="493" width="110" height="4" rx="2" fill="#dc2626"/>
  <rect x="470" y="493" width="110" height="4" rx="2" fill="#16a34a"/>
</svg>
</div>

### The Initialization Problem

Because k-means starts with random centroids, different runs can give different results. A bad initialization might converge to a suboptimal solution. The standard fix is to run k-means multiple times (say 10-50) with different random seeds and keep the solution with the lowest total within-cluster variance.

```bio
set_seed(42)
# K-means clustering on PCA-reduced expression data
let result = pca(expression_matrix)
let pca_scores = result.scores |> take_cols(0..20)  # first 20 PCs

let clusters = kmeans(pca_scores, 4)
print("Cluster sizes: " + str(clusters.sizes))
print("Total within-cluster variance: " + str(clusters.total_within_ss))
```

## Choosing k: The Elbow Method

The hardest part of k-means is choosing k. If you pick k = 500, every tumor gets its own cluster — perfect within-cluster similarity but meaningless. If you pick k = 1, everything is in one group — also meaningless. The right k is somewhere in between.

The **elbow method** runs k-means for k = 1, 2, 3, ..., K and plots the total within-cluster sum of squares (WCSS) against k. As k increases, WCSS always decreases (more clusters = tighter fits). The "elbow" — where the rate of decrease sharply levels off — suggests the best k.

```bio
set_seed(42)
# Elbow plot to find optimal k
let wcss = seq(1, 15) |> map(|k| {
  let cl = kmeans(pca_scores, k)
  {k: k, within_ss: cl.total_within_ss}
}) |> to_table()
plot(wcss, {type: "line", x: "k", y: "within_ss",
  title: "Elbow Plot — Optimal Number of Clusters"})
```

> **Common pitfall:** The elbow is often ambiguous. Real data rarely shows a sharp bend. If the elbow plot suggests k could be 3, 4, or 5, use biological knowledge or validation metrics (like silhouette scores) to decide.

## Hierarchical Clustering

Hierarchical clustering builds a tree (dendrogram) of nested clusters rather than producing a flat partition. It does not require you to choose k in advance — you can cut the tree at any height to get the desired number of clusters.

### Agglomerative (Bottom-Up) Algorithm

1. Start with each sample as its own cluster (500 clusters for 500 tumors).
2. Find the two closest clusters and merge them (now 499 clusters).
3. Repeat until everything is in one cluster.
4. The dendrogram records every merge and the distance at which it occurred.

### Linkage Methods

"Distance between clusters" is ambiguous when clusters contain multiple points. The linkage method resolves this:

| Linkage | Distance between clusters A and B | Tendency |
|---|---|---|
| Single | Minimum distance between any pair | Produces long, chained clusters |
| Complete | Maximum distance between any pair | Produces compact, equal-sized clusters |
| Average | Mean pairwise distance | Compromise between single and complete |
| Ward | Increase in total variance when merged | Minimizes within-cluster variance (like k-means) |

Ward linkage is the most common choice in genomics because it tends to produce balanced, compact clusters similar to k-means.

```bio
# Hierarchical clustering with Ward linkage
let hc = hclust(pca_scores, "ward")

# Cut tree at k=4 clusters
let labels = hc |> cut_tree(4)
print("Hierarchical cluster sizes: " + str(table_counts(labels)))
```

### The Dendrogram

The dendrogram is one of the most informative visualizations in genomics. The height of each merge indicates how dissimilar the merged clusters were. A long vertical line before a merge suggests a clear separation; short lines suggest gradual transitions.

```bio
# Dendrogram with colored clusters
dendrogram(hc, {k: 4, title: "Tumor Expression — Hierarchical Clustering"})
```

## DBSCAN: Density-Based Clustering

DBSCAN (Density-Based Spatial Clustering of Applications with Noise) takes a fundamentally different approach. Instead of assuming spherical clusters, it finds regions of high density separated by regions of low density. It also identifies outliers — points in low-density regions that do not belong to any cluster.

### The Algorithm

1. For each point, count neighbors within distance epsilon.
2. If a point has at least min_samples neighbors, it is a "core point."
3. Connect core points that are within epsilon of each other.
4. Connected components of core points form clusters.
5. Non-core points within epsilon of a core point join that cluster.
6. Remaining points are labeled as noise (outliers).

### Properties

| Property | DBSCAN |
|---|---|
| Shape of clusters | Any shape (can find crescents, rings, etc.) |
| Requires k in advance | No — finds k automatically |
| Handles outliers | Excellent — explicitly labels them |
| Parameters | epsilon (neighborhood radius), min_samples |
| Handles varying density | Poorly — one epsilon for all regions |

```bio
set_seed(42)
# DBSCAN on PCA scores
let db = dbscan(pca_scores, 2.5, 10)
print("Clusters found: " + str(db.n_clusters))
print("Noise points: " + str(db.n_noise))
```

> **Key insight:** DBSCAN is excellent for scRNA-seq data where clusters are irregularly shaped and outlier cells (doublets, dying cells) should be flagged rather than forced into a cluster. K-means forces every point into a cluster — DBSCAN does not.

## Silhouette Scores: Validating Clusters

How do you know your clusters are real? The silhouette score provides internal validation. For each point, it measures:

- **a** = mean distance to other points in the same cluster
- **b** = mean distance to points in the nearest other cluster
- **Silhouette = (b - a) / max(a, b)**

<div style="text-align: center; margin: 2em 0;">
<svg width="650" height="300" viewBox="0 0 650 300" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="325" y="28" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">Silhouette Score: Measuring Cluster Quality</text>
  <!-- Cluster A (blue) -->
  <circle cx="140" cy="160" r="5" fill="#3b82f6"/><circle cx="160" cy="145" r="5" fill="#3b82f6"/>
  <circle cx="125" cy="150" r="5" fill="#3b82f6"/><circle cx="155" cy="170" r="5" fill="#3b82f6"/>
  <circle cx="145" cy="175" r="5" fill="#3b82f6"/>
  <!-- Target point (highlighted) -->
  <circle cx="180" cy="158" r="7" fill="#2563eb" stroke="#1e293b" stroke-width="2"/>
  <text x="180" y="140" text-anchor="middle" font-size="11" font-weight="bold" fill="#1e293b">Point X</text>
  <!-- Cluster B (red) -->
  <circle cx="380" cy="130" r="5" fill="#ef4444"/><circle cx="400" cy="145" r="5" fill="#ef4444"/>
  <circle cx="395" cy="120" r="5" fill="#ef4444"/><circle cx="415" cy="135" r="5" fill="#ef4444"/>
  <circle cx="390" cy="150" r="5" fill="#ef4444"/>
  <!-- Cluster C (green) -->
  <circle cx="350" cy="240" r="5" fill="#22c55e"/><circle cx="370" cy="250" r="5" fill="#22c55e"/>
  <circle cx="340" cy="255" r="5" fill="#22c55e"/><circle cx="365" cy="235" r="5" fill="#22c55e"/>
  <!-- Arrow: a = distance to own cluster -->
  <line x1="180" y1="162" x2="148" y2="168" stroke="#2563eb" stroke-width="2" stroke-dasharray="4,3"/>
  <text x="105" y="190" font-size="12" font-weight="bold" fill="#2563eb">a = 28</text>
  <text x="105" y="204" font-size="10" fill="#2563eb">(own cluster)</text>
  <!-- Arrow: b = distance to nearest other cluster -->
  <line x1="185" y1="155" x2="375" y2="138" stroke="#dc2626" stroke-width="2" stroke-dasharray="4,3"/>
  <text x="280" y="125" font-size="12" font-weight="bold" fill="#dc2626">b = 215</text>
  <text x="280" y="139" font-size="10" fill="#dc2626">(nearest other)</text>
  <!-- Formula -->
  <rect x="440" y="185" width="195" height="80" rx="6" fill="white" stroke="#e5e7eb"/>
  <text x="538" y="210" text-anchor="middle" font-size="12" font-weight="bold" fill="#1e293b">Silhouette Score</text>
  <text x="538" y="232" text-anchor="middle" font-size="13" fill="#7c3aed">s = (b - a) / max(a, b)</text>
  <text x="538" y="254" text-anchor="middle" font-size="12" fill="#16a34a">s = (215 - 28) / 215 = 0.87</text>
  <!-- Cluster labels -->
  <text x="140" y="105" text-anchor="middle" font-size="12" fill="#3b82f6" font-weight="bold">Cluster A</text>
  <text x="395" y="100" text-anchor="middle" font-size="12" fill="#ef4444" font-weight="bold">Cluster B</text>
  <text x="355" y="225" text-anchor="middle" font-size="12" fill="#22c55e" font-weight="bold">Cluster C</text>
  <!-- Interpretation -->
  <rect x="15" y="270" width="620" height="22" rx="4" fill="#f1f5f9"/>
  <text x="325" y="286" text-anchor="middle" font-size="11" fill="#6b7280">Close to +1 = well-clustered | Near 0 = on boundary | Negative = probably misassigned</text>
</svg>
</div>

The score ranges from -1 to +1:

| Score | Interpretation |
|---|---|
| +1 | Point is far from neighboring clusters — excellent clustering |
| 0 | Point is on the boundary between clusters |
| -1 | Point is probably in the wrong cluster |

Average silhouette scores:

| Average Score | Quality |
|---|---|
| 0.71 - 1.00 | Strong structure |
| 0.51 - 0.70 | Reasonable structure |
| 0.26 - 0.50 | Weak structure, possibly artificial |
| < 0.25 | No substantial structure found |

```bio
set_seed(42)
# Silhouette analysis for different k
for k in 2..8 {
  let cl = kmeans(pca_scores, k)
  let sil = cl.silhouette
  print("k=" + str(k) + " silhouette: " + str(round(sil, 3)))
}
```

> **Common pitfall:** A high silhouette score does not prove biological relevance. Random data with well-separated clusters in PCA space can have high silhouette scores. Always validate clusters against independent information — clinical outcomes, known markers, or held-out data.

## Clustering Always Finds Clusters — Even in Noise

This is the single most important warning about clustering. Run k-means with k = 3 on completely random data, and it will dutifully return three clusters. The clusters will look somewhat real in a scatter plot. But they are meaningless.

```bio
set_seed(42)
# DANGER: Clustering random noise
let noise = table(500, 20, "rnorm")
let fake_clusters = kmeans(noise, 3)
let noise_pca = pca(noise)

# These clusters are pure noise — but the plot looks convincing!
scatter(noise_pca.scores[0], noise_pca.scores[1])

# Silhouette score will be low
let sil = fake_clusters.silhouette
print("Silhouette on noise: " + str(round(sil, 3)))  # ~0.1-0.2
```

Always compare your clustering against a null — random data of the same dimensions and sample size. If the silhouette score on real data is not substantially higher than on random data, the clusters may not be real.

## Heatmaps with Clustering

The clustered heatmap is the workhorse visualization of genomics. It shows expression values as colors, with both rows (genes) and columns (samples) reordered by hierarchical clustering. Similar genes are placed next to similar genes; similar samples next to similar samples.

```bio
# Clustered heatmap of top variable genes
let top_genes = variance_per_column(expression_matrix)
  |> sort_by(|x| -x.value)
  |> take(100)
  |> map(|x| x.name)

let sub_matrix = expression_matrix |> select_cols(top_genes)

heatmap(sub_matrix, {cluster_rows: true, cluster_cols: true,
  color_scale: "viridis",
  title: "Top 100 Variable Genes — Clustered Heatmap"})
```

## Clustering in BioLang — Complete Pipeline

```bio
set_seed(42)
# Full clustering analysis on simulated tumor expression data

# Simulate 500 tumors with 4 molecular subtypes
let n_per_group = 125
let n_genes = 200

let subtype_a = table(n_per_group, n_genes, "rnorm")
let subtype_b = table(n_per_group, n_genes, "rnorm")
let subtype_c = table(n_per_group, n_genes, "rnorm")
let subtype_d = table(n_per_group, n_genes, "rnorm")

# Each subtype has distinct gene blocks elevated
for i in 0..n_per_group {
  for j in 0..50 { subtype_a[i][j] = subtype_a[i][j] + 3.0 }
  for j in 50..100 { subtype_b[i][j] = subtype_b[i][j] + 3.0 }
  for j in 100..150 { subtype_c[i][j] = subtype_c[i][j] + 3.0 }
  for j in 150..200 { subtype_d[i][j] = subtype_d[i][j] + 3.0 }
}

let expr = rbind(subtype_a, subtype_b, subtype_c, subtype_d)
let true_labels = repeat("A", 125) + repeat("B", 125) + repeat("C", 125) + repeat("D", 125)

# 1. PCA for visualization and dimensionality reduction
let pca_result = pca(expr)
scatter(pca_result.scores[0], pca_result.scores[1])

# 2. K-means clustering
let km = kmeans(expr, 4)
scatter(pca_result.scores[0], pca_result.scores[1])

# 3. Elbow plot
let wcss = seq(1, 10) |> map(|k| {
  let cl = kmeans(expr, k)
  {k: k, within_ss: cl.total_within_ss}
}) |> to_table()
plot(wcss, {type: "line", x: "k", y: "within_ss",
  title: "Elbow Plot"})

# 4. Silhouette analysis
for k in 2..8 {
  let cl = kmeans(expr, k)
  let sil = cl.silhouette
  print("k=" + str(k) + " silhouette=" + str(round(sil, 3)))
}

# 5. Hierarchical clustering
let hc = hclust(expr, "ward")
dendrogram(hc, {k: 4, title: "Dendrogram — Ward Linkage"})

let hc_labels = hc |> cut_tree(4)
scatter(pca_result.scores[0], pca_result.scores[1])

# 6. DBSCAN
let db = dbscan(expr, 8.0, 10)
print("DBSCAN found " + str(db.n_clusters) + " clusters, " + str(db.n_noise) + " noise")

# 7. Compare clustering to true labels
print("\nK-means vs true labels:")
print(cross_tabulate(km.labels, true_labels))

print("\nHierarchical vs true labels:")
print(cross_tabulate(hc_labels, true_labels))

# 8. Clustered heatmap
heatmap(expr, {cluster_rows: true, cluster_cols: true,
  color_scale: "red_blue",
  title: "Expression Heatmap — K-means Clusters"})
```

**Python:**

```python
from sklearn.cluster import KMeans, DBSCAN, AgglomerativeClustering
from sklearn.metrics import silhouette_score
from scipy.cluster.hierarchy import dendrogram, linkage
import seaborn as sns

# K-means
km = KMeans(n_clusters=4, n_init=25, random_state=42)
km_labels = km.fit_predict(expr)

# Elbow plot
wcss = [KMeans(n_clusters=k, n_init=25).fit(expr).inertia_ for k in range(1, 10)]
plt.plot(range(1, 10), wcss, 'bo-')
plt.show()

# Silhouette
for k in range(2, 8):
    km_k = KMeans(n_clusters=k, n_init=25).fit(expr)
    print(f"k={k} silhouette={silhouette_score(expr, km_k.labels_):.3f}")

# Hierarchical
Z = linkage(expr, method='ward')
dendrogram(Z)
plt.show()

# Heatmap
sns.clustermap(expr, method='ward', cmap='RdBu_r')
plt.show()
```

**R:**

```r
# K-means
km <- kmeans(expr, centers = 4, nstart = 25)
table(km$cluster, true_labels)

# Elbow plot
wcss <- sapply(1:9, function(k) kmeans(expr, k, nstart=25)$tot.withinss)
plot(1:9, wcss, type="b", xlab="k", ylab="Total within SS")

# Hierarchical
hc <- hclust(dist(expr), method = "ward.D2")
plot(hc)
cutree(hc, k = 4)

# Heatmap
library(pheatmap)
pheatmap(expr, clustering_method = "ward.D2", show_rownames = FALSE)

# Silhouette
library(cluster)
for (k in 2:7) {
  cl <- kmeans(expr, k, nstart=25)
  cat(sprintf("k=%d silhouette=%.3f\n", k, mean(silhouette(cl$cluster, dist(expr))[,3])))
}
```

## Exercises

1. **Three subtypes.** The pathologist says 3 subtypes, but your elbow plot suggests 4 or 5. Simulate 500 tumors with 5 true subtypes (100 each). Run k-means for k = 3, 4, 5, 6. Use silhouette scores and cross-tabulation against true labels. Which k best recovers the truth?

```bio
# Your code: simulate 5 subtypes, test multiple k values
```

2. **Linkage comparison.** Run hierarchical clustering on the same data with single, complete, average, and Ward linkage. Cut each at k = 5. Which linkage best recovers the true labels? Visualize the four dendrograms.

```bio
# Your code: four linkage methods, compare cross-tabulations
```

3. **DBSCAN tuning.** On the 5-subtype data, try DBSCAN with epsilon values from 3 to 15 (step 1) and min_samples from 5 to 20. Find the combination that gives 5 clusters with the fewest noise points.

```bio
# Your code: grid search over epsilon and min_samples
```

4. **Noise test.** Generate pure random data (500 samples x 200 genes, no structure). Run k-means with k = 4. Plot the result. Calculate the silhouette score. Now compare to the silhouette score from the structured data. What is the difference?

```bio
# Your code: cluster noise, compare silhouette to real data
```

5. **Heatmap with annotation.** Create a clustered heatmap of the top 50 most variable genes, with a color bar showing both k-means cluster assignment and true subtype. Do the two agree?

```bio
# Your code: select top variable genes, heatmap with dual annotation
```

## Key Takeaways

- Clustering is unsupervised grouping — it finds structure without labels, making it essential for discovering molecular subtypes, cell populations, and other hidden patterns.
- K-means is fast and simple but requires choosing k in advance, assumes spherical clusters, and is sensitive to initialization.
- The elbow method and silhouette scores help choose k, but neither is definitive — biological knowledge must guide the final decision.
- Hierarchical clustering produces a dendrogram showing nested relationships; Ward linkage is the default choice for genomics.
- DBSCAN finds arbitrarily shaped clusters and identifies outliers, making it valuable for scRNA-seq data, but requires tuning epsilon and min_samples.
- Silhouette scores validate clustering quality: above 0.5 is reasonable, above 0.7 is strong, but always compare against random data.
- Clustering always finds clusters, even in noise. Never trust clusters without validation against independent evidence.
- Clustered heatmaps are the standard visualization for gene expression patterns across samples.

## What's Next

PCA and clustering assume that the data you have is large enough to draw conclusions. But what if your dataset is tiny — six mice per group, far too few to verify normality or trust asymptotic theory? Tomorrow, we meet resampling methods: bootstrap and permutation tests. These techniques let your data speak for itself, building confidence intervals and testing hypotheses without any distributional assumptions, by literally reshuffling the data thousands of times.
