Reference outputs generated from Seurat's MIT-licensed implementations.

Seurat        5.5.1  (MIT + file LICENSE)
SeuratObject  5.4.0  (MIT + file LICENSE)
R             4.5.2

snn_input.csv / snn_expected.csv
  200 cells x 10 dims, seed 20260810. Neighbours ranked by exact distance,
  k = 20 including self, then Seurat:::ComputeSNN(prune = 1/15).
  Edges are the upper triangle only, sorted by (i, j), 0-based.

lognorm_counts.csv / lognorm_expected.csv
  60 genes x 40 cells, seed 4242, Seurat::LogNormalize(scale.factor = 10000).
  Stored genes x cells, matching Seurat's orientation.

Regenerate with validation/single-cell/gen_fixtures.R in biolang-workflows.
No Seurat source is copied here - these are reference outputs only.
