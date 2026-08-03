# Trajectories, Cell Cycle, and Doublets

## Cell type and cell state

A type describes relatively stable identity; a state describes a more
temporary program such as activation, stress, proliferation, or response to
interferon. State programs can appear across several types and can dominate
clustering.

## Score a gene program

BioLang can average a selected gene set per cell:

> Requires CLI: this example imports the package.

```biolang
import "singlecell" as sc

let obj = sc.standard(
    sc.load("nsclc_like"),
    resolution: 0.5, n_hvg: 100, k: 15,
    min_genes: 20, max_genes: 2500, max_pct_mito: 5.0,
    min_cells: 3, target: 10000.0, quiet: true
)
let marker_indices = obj.hvg
    |> take(8)
let scored = sc.gene_module_score(obj, marker_indices)
println(take(scored.module_scores, 10))
```

A module score depends on gene-set quality and the matrix used. It is not an
assay measurement or pathway activation proof.

## Cell cycle

`sc.cell_cycle()` accepts S-phase and G2/M gene lists and returns per-cell
scores and phases. Use organism-appropriate genes. Decide whether cycle is a
nuisance, the biological question, or both before regressing or filtering it.
Removing a genuine proliferating population would erase biology.

## Pseudotime

Pseudotime orders cells along a graph from a chosen start cell:

> Requires CLI: this example imports the package.

```biolang
let ordered = obj |> sc.pseudotime(0)
println(take(ordered.pseudotime, 12) |> map(|v| round(v, 3)))
println(take(sc.order_by_pseudotime(ordered).barcodes, 6))
println(take(sc.pseudotime_bins(ordered.pseudotime, 6), 12))
```

`order_by_pseudotime` returns a new object whose cells are sorted, not a list of
positions, so read its `barcodes` to see the order.

The result is a relative graph distance, not clock time. Changing the root,
neighbors, or included cells can change the order. A branch may represent
different lineages, activation, cell cycle, or an artifact.

Use trajectory analysis when biology plausibly contains a continuum and when
intermediate cells were sampled. Support direction with known markers,
experimental time points, lineage tracing, RNA velocity, or another independent
source. Do not infer causality from an arrow drawn on UMAP.

## Doublets as false intermediates

Doublets often express markers from two lineages and appear between them. This
can resemble a transitional state. Before naming a bridge population:

- inspect doublet scores;
- check total counts and detected genes;
- examine incompatible lineage markers;
- compare samples and expected loading rate;
- seek orthogonal evidence.

The honest label `uncertain` is preferable to a memorable but unsupported new
cell state.
