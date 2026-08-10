# Neighbors, clusters, and UMAP

The corrected PCA embedding becomes a shared-nearest-neighbor (SNN) graph. The
course settings use 40 dimensions, 20 nearest neighbors, Louvain clustering,
and resolution 0.8.

```biolang
let graph = sc.neighbors(corrected, 20, true, 40)
let clustered = sc.cluster_louvain(graph, 20, 0.8)
```

An all-pairs distance matrix is too large for 29,629 cells. When a compatible
GPU is available, BioLang scans distances in bounded batches and performs the
top-k reduction on the device, returning only the selected neighbors. This
avoids allocating the quadratic matrix. The CPU fallback builds deterministic
random-projection trees, gathers candidates from nearby leaves, and reranks
those candidates with exact distances. The SNN graph weights an edge by the
overlap between the two cells' neighbor sets.

Louvain optimizes graph modularity. Resolution controls how strongly the
objective favors smaller communities; it is not a requested number of
clusters. Therefore it would be misleading to tune resolution until BioLang
happens to print the historical HBC count.

```biolang
let embedded = sc.run_umap(clustered, 40, 30, 0.3, 123456)
```

BioLang's UMAP starts from the selected-neighbor graph and performs sparse
spectral initialization and low-dimensional optimization. UMAP axes have no
direct interpretation, and coordinate equality is not a useful test. Even
after allowing rotations and reflections, local-neighborhood preservation is
the stronger comparison. Against the current Seurat oracle, the measured GPU
run's approximate integrated-PC 15-neighbor Jaccard was 0.0648. Its exact
two-dimensional UMAP 15-neighbor Jaccard was 0.0159. The former shows that
integration is the larger upstream gap; the latter means these UMAPs should be
treated as BioLang's views of the data rather than reproductions of the HBC
figure. The CPU backend was not rerun after the anchor/PCA dimension correction.
