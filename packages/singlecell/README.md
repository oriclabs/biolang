# BioLang singlecell

`singlecell` provides a reproducible single-cell RNA-seq workflow for 10x
Genomics matrices. The package keeps count and normalized matrices sparse,
materializes only compact PCA scores, and stores analysis results in one record.

## Quick start

```biolang
import "singlecell" as sc

let cells = sc.load("filtered_feature_bc_matrix")
let result = cells
    |> sc.filter_genes(3)
    |> sc.filter_cells(200, 5000, 20.0)
    |> sc.normalize(10000.0)
    |> sc.variable_genes(2000)
    |> sc.scale()
    |> sc.run_pca(30)
    |> sc.neighbors(20, true, 10)
    |> sc.cluster_louvain(20, 0.8)
    |> sc.run_umap()

println(sc.summary(result))
write_text("umap.svg", sc.dim_plot(result, nil, "UMAP", true))
```

## R-familiar plots

The presentation layer uses familiar Seurat concepts without requiring R:

```biolang
write_text("DimPlot.svg", sc.dim_plot(result, nil, "UMAP", true))
write_text("FeaturePlot.svg", sc.feature_plot(result, "MS4A1"))
write_text("VlnPlot.svg", sc.vln_plot(result, "MS4A1"))
write_text("DotPlot.svg", sc.dot_plot(result, ["CD3D", "NKG7", "MS4A1", "LYZ"]))
write_text("DoHeatmap.svg", sc.do_heatmap(result, 5))
```

These are semantically aligned rather than pixel-for-pixel copies. `dim_plot`
uses an R/ggplot-like discrete palette and optional median-position labels;
`feature_plot` uses the familiar light-grey-to-blue scale. `dot_plot` maps
circle area to percent detected and colour to per-gene standardized average
expression. Above 5,000 cells the point layer is automatically rasterised,
while titles, axes, labels, and legends remain vector SVG.

The package also includes donor-aware pseudobulk exploration, paired
composition tests, cluster diagnostics, and an SVG plot gallery:

```text
bl run singlecell/examples/advanced_analysis.bl
```

This writes QC, split-feature, donor-pair, pseudobulk-PCA, volcano, MA,
composition, stability, silhouette, and grouped heatmap figures.

Installed packages retain this package's complete `examples` directory. List or
copy it into an independent working directory without cloning the BioLang
repository:

```text
bl examples singlecell
bl examples singlecell --copy singlecell-examples
```

From a source checkout, the same command accepts the local package path:

```text
bl examples packages/singlecell --copy singlecell-examples
```

`sc.load()` reads `matrix.mtx[.gz]`, `features.tsv[.gz]`, and
`barcodes.tsv[.gz]` directly into a cells-by-genes CSR matrix. The returned
record contains:

- `matrix`: raw sparse counts
- `layers.counts`: the raw-count layer
- `obs`: cell metadata, initially the barcode
- `var`: gene metadata, initially the gene symbol
- `genes`, `barcodes`, `n_cells`, and `n_genes`

Filtering synchronizes matrices, layers, metadata tables, names, and dimensions.
Gene filtering invalidates downstream reductions and graphs. Cell filtering
subsets cell-level results and invalidates the neighbor graph.

## Pipeline contract

`sc.standard()` records the compatibility profile `seurat_5_5_1` and runs:

1. `filter_genes` and `filter_cells`
2. `normalize`
3. `variable_genes`
4. `scale` selected variable features
5. `run_pca`
6. `neighbors` on the first 10 PCs, building an SNN graph
7. `cluster_louvain`
8. `run_umap`

Only selected variable features are materialized by `scale`, so the workflow
does not create a dense full-transcriptome matrix.
`neighbors` always uses PCA scores, or an integrated PCA embedding when one is
present. `cluster_leiden` consumes the stored sparse graph rather than rebuilding
neighbors.

## Tests

Run the native runtime coverage:

```text
cargo test -p bl-runtime --test singlecell_tests
cargo test -p bl-runtime --test anchor_integration_tests
```

Run the package-level behavior test from the repository `packages` directory:

```text
cd packages
../target/debug/bl run singlecell/tests/pipeline.bl
../target/debug/bl run singlecell/tests/advanced.bl
../target/debug/bl run singlecell/tests/integration_anchors.bl
```

The `validation` directory contains optional reference workflows. Validation
tools are not linked, imported, packaged as runtime dependencies, or used to
generate BioLang source. Run them only in an isolated environment.

## Method boundaries

- `highly_variable_genes(..., "vst")` fits the mean-variance trend on raw
  counts and ranks standardized variance. It is the standard profile used by
  `variable_genes`; the legacy dispersion selector remains available only by
  requesting it explicitly.
- `find_integration_anchors` independently implements the published CCA/RPCA,
  mutual-neighbour, and shared-neighbour scoring design. `integrate_data`
  applies locally weighted anchor corrections and preserves raw RNA counts.
- UMAP follows the published fuzzy-neighbour graph, smooth-kNN calibration,
  spectral initialization, and negative-sampling objective. It is deterministic
  for a seed, but raw coordinates are not an interoperability contract: rotation,
  reflection, and optimizer details can change coordinates without changing the
  biological neighbourhoods.
- `marker_table` reports `log2fc > 0` when a gene is higher in the first
  cluster, matching `FindMarkers(ident.1, ident.2)`, and computes it on the
  expression scale (expm1 of the log-normalized means) so it is comparable to
  Seurat's `avg_log2FC`. It also returns `pct_a`/`pct_b` detection rates.
- `marker_table` is cell-level and exploratory. For condition contrasts use
  `pseudobulk(obj, sample_ids)`, which sums raw counts per (cluster, sample)
  straight from the CSR nonzeros, and hand the panel to a count model.
- `paired_pseudobulk_de` pairs donor profiles and tests log2 CPM for transparent
  exploration. It is cross-checked against SciPy and R, but it is not a
  negative-binomial count model. Use exported raw profiles with DESeq2, edgeR,
  or another validated method for formal inference.
- `paired_pseudobulk_de` reports `log2fc > 0` when a gene is higher in
  `condition_b`, and `statistic` shares that sign. Note this is the opposite
  anchor from `marker_table`, whose positive direction is the *first* argument —
  a condition contrast reads naturally as "b relative to a", a cluster contrast
  as "markers of a". Pass these by name so which is which is visible at the
  call site.
- `cluster_diagnostics` computes an exact silhouette: every cell against every
  other, O(n_cells² x n_components). Roughly 23 seconds for 220 cells x 50 PCs
  and quadratic from there; `cluster_stability` runs it once per resolution.
  Subsample before using either on a real dataset.
- `cluster_diagnostics` returns `mean_score: nil` when a silhouette is not
  defined for the partition, and `cluster_stability` returns
  `ari_previous: nil` on the first row. 0.0 and 1.0 are both legitimate values,
  so neither is used as a "not applicable" placeholder.
- `sctransform` returns a dense matrix — Pearson residuals are nonzero where
  counts were zero, so there is no sparse result to preserve.
- PCA signs can differ between implementations; compare explained variance,
  pairwise structure, or downstream labels rather than raw signs.
- The current exact k-nearest-neighbor search uses quadratic time in the number
  of cells. The graph storage is sparse, but very large atlases should use a
  sampled workflow or a remote backend until approximate-neighbor indexing is
  available.
- Leiden cluster numeric IDs are arbitrary. Compare partitions with ARI or NMI.
- `scale` accepts sparse input but materializes only the selected variable
  features, using sample standard deviation and the standard clipping value 10.
- AnnData Zarr is read and written natively while preserving CSR sparsity.
  The native interchange currently covers `X` and the `obs`/`var` index names;
  arbitrary AnnData metadata columns and auxiliary layers are not yet copied.
  Direct `.h5ad` I/O is not built into the runtime; convert it with
  Python/anndata or a configured container, then use `read_anndata` on the
  resulting `.zarr` store.
