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
    |> sc.run_pca(30)
    |> sc.neighbors(15)
    |> sc.cluster_leiden(15, 0.5)

println(sc.summary(result))
write_text("umap.svg", sc.plot_umap(result))
```

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

The standard analysis order is:

1. `filter_genes` and `filter_cells`
2. `normalize`
3. `variable_genes`
4. `run_pca`
5. `neighbors`
6. `cluster_leiden`

PCA centers features mathematically while operating on CSR values, so the
workflow does not need to create a dense, centered expression matrix.
`neighbors` always uses PCA scores, or an integrated PCA embedding when one is
present. `cluster_leiden` consumes the stored sparse graph rather than rebuilding
neighbors.

## Tests

Run the native runtime coverage:

```text
cargo test -p bl-runtime --test singlecell_tests
```

Run the package-level behavior test from the repository `packages` directory:

```text
cd packages
../target/debug/bl run singlecell/tests/pipeline.bl
../target/debug/bl run singlecell/tests/advanced.bl
```

The `validation` directory contains optional Scanpy and Seurat reference
workflows. Those tools are not package dependencies. They are installed and run
in separate Python or R environments so the BioLang package remains lightweight.

## Method boundaries

- `highly_variable_genes` ranks genes by dispersion (CV squared) globally,
  without binning by mean expression the way Scanpy's `seurat` flavor and
  Seurat VST do. Selection therefore leans toward low-expression genes and the
  set is not expected to equal those flavors.
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
- `scale` is dense-only and raises on a sparse object rather than quietly
  materializing every zero. Use `run_pca`, which centers without densifying.
- AnnData Zarr is read and written natively while preserving CSR sparsity.
  The native interchange currently covers `X` and the `obs`/`var` index names;
  arbitrary AnnData metadata columns and auxiliary layers are not yet copied.
  Direct `.h5ad` I/O is not built into the runtime; convert it with
  Python/anndata or a configured container, then use `read_anndata` on the
  resulting `.zarr` store.
