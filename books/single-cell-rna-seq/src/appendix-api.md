# BioLang Single-Cell API

Import the package with:

> Requires CLI: package imports are not available in the browser runner.

```biolang
import "singlecell" as sc
```

BioLang resolves this name through the package's `[lib].entry` declaration in
`biolang.toml`.

## Object creation and access

| Function | Purpose |
|---|---|
| `sc.load(path)` | Read a 10x MEX directory into a sparse object |
| `sc.from_matrix(matrix, genes, barcodes)` | Build an object from a dense cells-by-genes matrix |
| `sc.summary(obj)` | Return dimensions and pipeline-stage flags |
| `sc.n_cells`, `sc.n_genes` | Return current dimensions |
| `sc.get_matrix` | Return raw counts |
| `sc.get_norm_matrix` | Return normalized values |
| `sc.get_genes`, `sc.get_barcodes` | Return identifiers |
| `sc.get_hvg_genes` | Return selected HVG names |
| `sc.get_clusters` | Return cluster labels |
| `sc.get_pseudotime` | Return per-cell pseudotime |
| `sc.get_doublet_scores` | Return per-cell scores |

## QC and preprocessing

| Function | Purpose |
|---|---|
| `sc.qc(obj)` | Add cell and gene QC tables |
| `sc.filter_cells(obj, min_genes, max_genes, max_pct_mito)` | Subset cells |
| `sc.filter_genes(obj, min_cells)` | Subset genes |
| `sc.normalize(obj, target)` | Total-count normalization plus log1p |
| `sc.sctransform(obj, n_features)` | Regularized negative-binomial residual transform; `n_features` caps it at the top genes by residual variance |
| `sc.variable_genes(obj, n)` | Select genes by dispersion — log-normalized values only, **not** SCTransform residuals |
| `sc.scale(obj, clip)` | Dense mean-center and standardize, clipping z-scores at `clip` (default 10); raises on sparse objects |
| `sc.run_pca(obj, n_pcs)` | Sparse-aware PCA |
| `sc.neighbors(obj, k)` | Exact kNN graph on PCA or integrated embedding |
| `sc.cluster(obj, k)` | k-means alternative |
| `sc.cluster_leiden(obj, k, resolution)` | Leiden clustering on stored graph |
| `sc.standard(obj, resolution:, n_hvg:, k:, min_genes:, max_genes:, max_pct_mito:, min_cells:, target:, quiet:)` | Run and report the standard workflow |
| `sc.sweep(obj, resolutions:, n_hvg:, k:)` | Compare clustering resolutions |

## Annotation and state

| Function | Purpose |
|---|---|
| `sc.marker_table(obj, a, b)` | Exploratory markers for cluster A vs B; `log2fc > 0` means higher in A |
| `sc.pseudobulk(obj, sample_ids)` | Sum raw counts per (cluster, sample) for replicate-aware DE |
| `sc.pseudobulk_profiles(obj, donors, conditions, cell_types, min_cells:)` | Build raw and log-CPM donor-condition-cell-type profiles |
| `sc.paired_pseudobulk_de(obj, donors, conditions, cell_types, target_type:, condition_a:, condition_b:, min_cells:)` | Exploratory paired donor-level condition test; `log2fc > 0` means higher in `condition_b` |
| `sc.donor_gene_table(obj, gene, donors, conditions, cell_types, target_type:, condition_a:, condition_b:, min_cells:)` | Return per-donor values for one gene and two conditions |
| `sc.composition_table(...)` | Return donor-condition cell-type fractions, including zeros |
| `sc.paired_composition_test(donors, conditions, cell_types, condition_a:, condition_b:, min_cells:)` | Test paired donor-level composition changes |
| `sc.cluster_diagnostics(obj)` | Per-cell and mean silhouette; nil when undefined. **Quadratic in cell count** |
| `sc.cluster_stability(obj, resolutions:, k:)` | Sweep Leiden resolution with silhouette and ARI; `ari_previous` is nil on the first row |
| `sc.ranked_genes(de_results)` | Prepare signed `gene, score` input for `gsea()` |
| `sc.cell_cycle(obj, s_genes, g2m_genes)` | Score cell-cycle phase |
| `sc.gene_module_score(obj, gene_indices)` | Average selected genes per cell |
| `sc.doublets(obj, n_sim)` | Simulate mixtures and score doublets |
| `sc.flag_doublets(obj, threshold)` | Add boolean doublet flags |

## Multiple samples and trajectories

| Function | Purpose |
|---|---|
| `sc.merge(a, b, batch_a, batch_b)` | Merge gene-aligned objects |
| `sc.integrate(obj, batch_ids)` | Center batches in PCA space |
| `sc.integrate_from_column(obj, batch_col)` | Integrate from a top-level field such as `batch_ids` |
| `sc.pseudotime(obj, start_cell)` | Graph distance from a root cell |
| `sc.order_by_pseudotime(obj)` | Return cell order |
| `sc.pseudotime_bins(pt, n_bins)` | Assign relative bins |
| `sc.smooth_along_pseudotime(obj, gene_idx, n_bins)` | Bin gene expression along order |

## Visualization

| Function | Return |
|---|---|
| `sc.plot_umap(obj, title)` | SVG string |
| `sc.plot_pca(obj, title)` | SVG string |
| `sc.plot_feature(obj, gene, title)` | SVG string |
| `sc.plot_violin(obj, gene)` | SVG string |
| `sc.plot_markers(obj, n)` | SVG string |
| `sc.expr_dotplot(obj, genes, title)` | SVG string |
| `sc.plot_elbow(obj, n_pcs)` | SVG string |
| `sc.plot_proportions(obj, groups)` | SVG string |
| `sc.plot_qc_violin(obj, title)` | SVG string |
| `sc.plot_qc_scatter(obj, title)` | SVG string |
| `sc.plot_qc_dashboard(obj, title)` | SVG string |
| `sc.plot_embedding(obj, groups, title)` | SVG string |
| `sc.plot_feature_split(obj, gene, groups, title)` | SVG string |
| `sc.plot_group_heatmap(obj, genes, groups, title)` | SVG string |
| `sc.plot_volcano(results, title, fc, padj)` | SVG string |
| `sc.plot_ma(results, title, fc, padj)` | SVG string |
| `sc.plot_donor_expression(obj, gene, donors, conditions, cell_types, target_type:, condition_a:, condition_b:, min_cells:)` | SVG string |
| `sc.plot_pseudobulk_pca(obj, donors, conditions, cell_types, target_type:, min_cells:)` | SVG string |
| `sc.plot_composition(...)` | SVG string |
| `sc.plot_de_overlap(result_sets, names, title)` | SVG string |
| `sc.plot_enrichment(results, title)` | SVG string |
| `sc.plot_cluster_stability(stability, title)` | SVG string |
| `sc.plot_silhouette(diagnostics, title)` | SVG string |

## Calling convention

Optional parameters are passed by name: `sc.standard(obj, resolution: 1.0)`,
`sc.paired_pseudobulk_de(obj, donors, conditions, cell_types, target_type: "T",
condition_a: "control", condition_b: "treated")`. Positional calls still work,
but `sc.standard()` takes eight numbers and `paired_pseudobulk_de()` takes three
consecutive strings — transposing two of them yields a different analysis with
no error. Name them.

## Important boundaries

- Exact kNN is currently quadratic in cell count.
- `cluster_diagnostics()` is also quadratic, and much more expensive in
  practice: roughly 23 seconds for 220 cells, growing as n². `cluster_stability()`
  runs it once per resolution. Subsample before using either on a real dataset.
- `cluster_diagnostics()` returns `mean_score: nil` when a silhouette is
  undefined (one cluster, or every cell its own), and `cluster_stability()`
  returns `ari_previous: nil` on the first row. Neither substitutes a number
  that would read as a real result.
- `gsea()` p-values come from a permutation test. It uses a fixed default seed
  so runs reproduce; pass `seed:` to vary it, and record the value.
- `scale()` is dense-only and raises on an object loaded from a 10x directory,
  because centering would materialize every zero. Use `run_pca()`.
- HVG ranking is global CV² with no binning by mean expression, so it favors
  low-expression genes more than Scanpy's `seurat` flavor or Seurat VST.
- Integration is PCA batch centering, not a full atlas-mapping method.
- `marker_table()` is exploratory and cell-level. It is not replicate-aware
  condition DE — use `pseudobulk()` and a count model for that.
- `paired_pseudobulk_de()` uses a paired test on log2 CPM for transparent
  exploration. Publication-grade count inference should use the raw
  pseudobulk profiles with a validated negative-binomial model and explicit
  study design.
- `sctransform()` returns a dense matrix; Pearson residuals are nonzero where
  counts were zero, so there is no sparse result to preserve. Pass `n_features`
  to compute only the genes that survive selection — the memory difference is
  large enough to decide whether a run completes.
- `variable_genes()` is for log-normalized values. Ranking centred residuals by
  variance over squared mean is not meaningful, and doing it silently produced a
  published cluster count that turned out to be a coincidence.
- AnnData Zarr metadata interchange is currently limited.
