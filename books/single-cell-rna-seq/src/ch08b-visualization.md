# Visualization: Every Plot with a Purpose

A plot should answer a stated question. Save the table or cell-level values
behind it, because the figure is a view rather than the complete result.

## Prepare one object

The examples in this chapter use one clustered object:

> Requires CLI: this example imports the package and reads local files.

> Requires CLI: this example imports the package.

```biolang
import "singlecell" as sc

let obj = sc.standard(
    sc.load("ctrl_raw"),
    resolution: 0.5, n_hvg: 100, k: 15,
    min_genes: 20, max_genes: 2500, max_pct_mito: 5.0,
    min_cells: 3, target: 10000.0, quiet: true
)
```

## QC distributions

Before filtering, inspect totals, detected genes, and mitochondrial percentage.
The current package provides QC tables; BioLang's general plotting builtins
render their columns:

> Requires CLI: this example imports the package and reads local files.

```biolang
let with_qc = sc.load("ctrl_raw") |> sc.qc()

println("Total counts")
println(hist(col(with_qc.cell_qc_table, "total_counts"), 20))
println("Genes detected")
println(hist(col(with_qc.cell_qc_table, "n_genes"), 20))
println("Mitochondrial percentage")
println(hist(col(with_qc.cell_qc_table, "pct_mito"), 20))
```

These ASCII histograms are useful in logs and remote jobs. For multiple samples,
split metrics by sample rather than hiding differences in one pooled
distribution.

For a connected SVG inspection surface:

> Requires CLI: this example imports the package and writes a local file.

```biolang
write_text("qc-dashboard.svg", sc.plot_qc_dashboard(sc.load("ctrl_raw")))
```

`plot_qc_violin()` and `plot_qc_scatter()` provide the two component views when
they are needed separately.

## PCA plot

> Requires CLI: this example imports the package and writes a local file.

```biolang
write_text("pca.svg", sc.plot_pca(obj, "PCA: major linear variation"))
```

Use PCA to inspect dominant linear structure and technical separation. A sample
forming its own region can indicate biology, batch, or quality. PCA axes have a
global mathematical meaning that UMAP axes do not, but their signs can flip
between implementations.

## Elbow plot

> Requires CLI: this example imports the package.

```biolang
write_text("elbow.svg", sc.plot_elbow(obj, 15))
```

The ordered ASCII bars show variance explained by each principal component.
Look for a gradual leveling rather than pretending there is always one exact
cutoff. Check whether later PCs contain coherent biology or mostly noise.

## UMAP cluster map

> Requires CLI: this example imports the package and writes a local file.

```biolang
write_text("umap.svg", sc.plot_umap(obj, "UMAP by Leiden cluster"))
```

Use UMAP to inspect local neighborhoods, mixing, and outliers. Do not interpret
axis values, island area, or long-range distance as calibrated biology. Label
the plot with the representation and parameters used.

Use arbitrary per-cell labels for condition, donor, cell type, batch, or phase.
The teaching fixture has one sample, so stand-in labels show the mechanism:

> Requires CLI: this example imports the package and writes a local file.

```biolang
# In a real analysis these come from your sample sheet, aligned to obj.barcodes.
let condition_labels = range(0, obj.n_cells)
    |> map(|i| if i % 2 == 0 { "control" } else { "treated" })

write_text(
    "umap-by-condition.svg",
    sc.plot_embedding(obj, condition_labels, "UMAP by condition")
)
```

## Feature plot

> Requires CLI: this example imports the package and writes a local file.

```biolang
write_text(
    "feature-LYZ.svg",
    sc.plot_feature(obj, "LYZ", "LYZ normalized expression")
)
```

A feature plot colors each UMAP point by one gene's expression. Use several
positive and negative markers. A few isolated high cells can be ambient RNA,
doublets, or genuine rare expression.

When comparing conditions, keep one colour scale across panels:

> Requires CLI: this example imports the package and writes a local file.

```biolang
write_text(
    "feature-split.svg",
    sc.plot_feature_split(obj, "LYZ", condition_labels)
)
```

## Violin plot

> Requires CLI: this example imports the package and writes a local file.

```biolang
write_text("violin-LYZ.svg", sc.plot_violin(obj, "LYZ"))
```

The violin compares a gene's normalized expression distribution across
clusters. Check both the expressing fraction and magnitude: a broad low signal
and a narrow high signal can have similar means.

## Marker heatmap

> Requires CLI: this example imports the package and writes a local file.

```biolang
write_text("marker-heatmap.svg", sc.plot_markers(obj, 5))
```

The heatmap selects genes with high cluster-vs-rest mean differences and shows
mean expression by cluster. It is a compact overview, not a formal
replicate-aware differential-expression result.

`plot_group_heatmap()` accepts any per-cell grouping and a chosen gene panel,
so the same view can compare cell types, conditions, donors, or cell-cycle
phases.

## Expression dot plot

> Requires CLI: this example imports the package and writes a local file.

```biolang
write_text(
    "marker-dotplot.svg",
    sc.expr_dotplot(
        obj,
        ["LYZ", "MS4A1", "CD3D", "GNLY"],
        "Candidate population markers"
    )
)
```

Circle size represents the fraction of cells expressing the gene; color
represents mean expression among expressing cells. This separates prevalence
from intensity. BioLang's general `dotplot` builtin is a sequence-comparison
plot and is unrelated.

## Proportion plot

> Requires CLI: this example imports the package.

```biolang
write_text("proportions.svg", sc.plot_proportions(obj))
```

This ordered ASCII chart counts cells per cluster or supplied group. Raw cell
fractions can be affected by capture, filtering, and sampling. Perform
sample-level compositional analysis before making population claims.

## Export all SVG plots

Every `plot_*` function returns an SVG string, with no exceptions — pass the
result to `write_text` to save it, or to `save_png` to rasterise it. The
advanced gallery and complete export workflow are in
[Advanced Analysis and Diagnostic Plots](./ch10b-advanced-analysis.md).

Every exported figure should be accompanied by:

- the BioLang source and version;
- input identity and filtering summary;
- plot title, groups, genes, and transformations;
- the values or assignments behind the figure;
- a caption stating what the plot can and cannot establish.
