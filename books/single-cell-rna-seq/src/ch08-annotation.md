# Maps, Markers, and Cell Types

## Cluster is not cell type

A cluster is an algorithmic community in a selected graph. A cell type is a
biological interpretation supported by multiple lines of evidence. One cell
type can occupy several states or clusters; one cluster can contain several
types.

## Visualize the partition

> Requires CLI: this example imports the package and writes a file.

```biolang
import "singlecell" as sc

let obj = sc.standard(
    sc.load("nsclc_like"),
    resolution: 0.5, n_hvg: 100, k: 15,
    min_genes: 20, max_genes: 2500, max_pct_mito: 5.0,
    min_cells: 3, target: 10000.0, quiet: true
)

write_text("umap.svg", sc.plot_umap(obj, "Teaching populations"))
sc.plot_proportions(obj)
```

The SVG is a view of the result, not the result itself. Keep the parameters,
object summary, and cell-level assignments.

## Find candidate markers

`marker_table()` compares two clusters:

> Requires CLI: this example imports the package.

```biolang
import "singlecell" as sc

let obj = sc.standard(
    sc.load("nsclc_like"),
    resolution: 0.5, n_hvg: 100, k: 15,
    min_genes: 20, max_genes: 2500, max_pct_mito: 5.0,
    min_cells: 3, target: 10000.0, quiet: true
)
let markers = sc.marker_table(obj, 0, 1)
    |> filter(|x| x.significant)
    |> sort(|a, b| if a.log2fc > b.log2fc { -1 } else { 1 })
println(head(table(markers), 12))
```

`log2fc` is positive when the gene is higher in the **first** cluster, matching
Seurat's `FindMarkers(ident.1 = 0, ident.2 = 1)` and scanpy's
`rank_genes_groups`. Values are on the expression scale, not the log-normalized
scale, so they are comparable to a Seurat `avg_log2FC`.

`pct_a` and `pct_b` give the fraction of cells in each cluster with a nonzero
value. A gene at `pct_a = 1.0, pct_b = 0.04` is specific; the same fold change
at `pct_a = 0.3, pct_b = 0.1` describes a minority of the cluster and is a much
weaker identity claim. Means alone hide that difference.

Marker ranking is a hypothesis generator. Review effect size, fraction
expressing, specificity across all clusters, known biology, and technical
covariates. A tiny p-value with a negligible effect is not a useful identity.
This comparison is one cluster against one other; a gene distinguishing 0 from 1
may be shared with cluster 2.

## Annotate with positive and negative evidence

For each proposed label, record:

| Evidence | Example |
|---|---|
| Positive markers | Several expected genes are enriched |
| Negative markers | Incompatible lineage markers are absent |
| State markers | Stress, cycle, interferon, or activation genes |
| Reference mapping | Agreement with an appropriate tissue reference |
| Orthogonal evidence | Protein, morphology, spatial location, or sorting |
| Uncertainty | Broad label or `unknown` when evidence conflicts |

Do not force every cluster into a precise label. "T cell" can be more honest
than "exhausted tissue-resident memory CD8 T cell" when evidence is limited.

Automated references provide a starting point, not ground truth. 10x notes that
its reference-based annotation may perform poorly on cancers and cell lines
that are not well represented in the reference
[model](https://www.10xgenomics.com/support/software/cell-ranger/latest/algorithms-overview/cr-cell-annotation-algorithm).

## Visualize marker panels

> Requires CLI: this example imports the package and writes files.

```biolang
write_text("top_markers.svg", sc.plot_markers(obj, 5))
write_text(
    "marker_dotplot.svg",
    sc.expr_dotplot(obj, ["MARK0_001", "MARK1_001", "MARK2_001", "MARK3_001"])
)
```

On a real immune dataset, use coherent panels rather than one famous marker.
Check that marker genes use the same naming convention as the matrix.
