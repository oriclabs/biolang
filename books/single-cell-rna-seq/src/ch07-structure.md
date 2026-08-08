# PCA, Neighbors, and Clusters

## From thousands of genes to a map

Each cell begins as a point in a space with one dimension per gene. This space
is noisy and difficult to inspect. The standard workflow builds progressively
simpler representations:

1. **PCA** finds directions that explain large patterns of variation.
2. A **k-nearest-neighbor graph** connects similar cells in PCA space.
3. **Leiden clustering** partitions the graph into connected communities.
4. **UMAP** produces a two-dimensional visualization.

These are related but distinct. UMAP coordinates are for visualization; Leiden
uses the neighbor graph. Clusters should not be defined by drawing shapes on
the UMAP.

## Build the structure

> Requires CLI: this example imports the package and reads local files.

```biolang
import "singlecell" as sc

let obj = sc.load("ctrl_raw")
    |> sc.filter_genes(3)
    |> sc.filter_cells(250, 100000, 20.0)
    |> sc.normalize()
    |> sc.variable_genes(2000)
    |> sc.run_pca(20)
    |> sc.neighbors(15)
    |> sc.cluster_leiden(15, 0.5)

println("clusters: " + str(obj.clusters |> unique |> len))
write_text("elbow.svg", sc.plot_elbow(obj, 15))
```

BioLang's sparse PCA centers features mathematically without materializing a
dense centered expression matrix. The compact cell-by-component scores are
dense.

## Parameters are scientific decisions

`n_pcs` controls how much structure reaches the graph. `k` controls
neighborhood scale. `resolution` controls cluster granularity. A higher
resolution often yields more clusters, but no single value is automatically
correct.

Compare several resolutions:

> Requires CLI: this example imports the package.

```biolang
import "singlecell" as sc

let base = sc.load("ctrl_raw")
    |> sc.filter_genes(3)
    |> sc.filter_cells(250, 100000, 20.0)

let trials = sc.sweep(base, resolutions: [0.2, 0.5, 0.8, 1.2], n_hvg: 100, k: 15)
println(trials |> map(|x| {resolution: x.resolution, clusters: x.n_clusters}))
```

The [Leiden algorithm](https://www.nature.com/articles/s41598-019-41695-z)
was designed to avoid poorly connected communities that can arise with
Louvain-style optimization. That guarantee does not make every community a
biological cell type.

## The one-call workflow

`sc.standard()` makes defaults visible and returns them in `decisions`:

> Requires CLI: this example imports the package.

```biolang
import "singlecell" as sc

let raw = sc.load("ctrl_raw")

# Pass only what you mean to change; the rest stay at their defaults and are
# reported as [default] so you can see what you have not thought about.
let quick = sc.standard(raw, resolution: 0.5, min_genes: 20, max_pct_mito: 5.0)

# Or state every decision explicitly for a final analysis.
let result = sc.standard(
    raw,
    resolution: 0.5, n_hvg: 100, k: 15,
    min_genes: 20, max_genes: 2500, max_pct_mito: 5.0,
    min_cells: 3, target: 10000.0
)
println(result.decisions)
```

Name the arguments. `sc.standard(raw, 0.5, 100, 15, 20, 2500, 5.0, 3, 10000.0)`
is also valid, but nobody reading it can tell which number is the mitochondrial
cutoff and which is the neighbourhood size — and swapping two of them silently
produces a different, plausible-looking answer. Naming them is the same
discipline the `decisions` table exists to enforce.

Use this to begin exploring, then write the explicit pipeline for a final
analysis.

## What can fool you

- A batch can form a clean cluster.
- A doublet can sit between two cell types.
- UMAP distances and empty spaces are not calibrated biological distances.
- Different PCA signs are mathematically equivalent.
- Numeric cluster IDs are arbitrary and can change between implementations.
