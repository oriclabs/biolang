Reference outputs generated from Seurat's MIT-licensed implementations.

Seurat        5.5.1  (MIT + file LICENSE)
SeuratObject  5.4.0  (MIT + file LICENSE)
R             4.5.2

snn_input.csv / snn_expected.csv
  200 cells x 10 dims, seed 20260810. Neighbours ranked by exact distance,
  k = 20 including self, then Seurat:::ComputeSNN(prune = 1/15).
  Edges are the upper triangle only, sorted by (i, j), 0-based.

louvain_expected.csv
  Seurat:::RunModularityClustering over snn_expected.csv with modularity = 1,
  resolution = 0.8, algorithm = 1, n.start = 10, n.iter = 10, and seed = 0.

lognorm_counts.csv / lognorm_expected.csv
  60 genes x 40 cells, seed 4242, Seurat::LogNormalize(scale.factor = 10000).
  Stored genes x cells, matching Seurat's orientation.

Regenerate SNN/log-normalization with validation/single-cell/gen_fixtures.R and
Louvain with validation/single-cell/seurat_louvain_fixture.R in
biolang-workflows.
No Seurat source is copied here - these are reference outputs only.
