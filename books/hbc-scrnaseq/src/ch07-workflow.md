# The Whole Workflow

*Follows HBC lesson 14 (scRNA-seq workflow).*

The course closes by writing the whole analysis out in one place, and it is the
most useful lesson in the set. Individual steps make sense in isolation; the
shape of the thing only becomes visible end to end.

## One sample, written out

```biolang
import "singlecell" as sc

let obj = sc.load("ctrl_raw")
    |> sc.filter_genes(3)              # gene kept if seen in >= 3 cells
    |> sc.filter_cells(250, 100000, 20.0)  # min genes, max genes, max % mito
    |> sc.normalize(10000.0)           # CP10K then log1p
    |> sc.variable_genes(2000)         # HVGs, to focus PCA
    |> sc.run_pca(30)                  # linear dimensionality reduction
    |> sc.neighbors(15)                # kNN graph, k = 15
    |> sc.cluster_leiden(15, 0.5)      # communities at resolution 0.5

println("cells:    " + str(obj.n_cells))
println("clusters: " + str(sc.get_clusters(obj) |> unique |> len))
write_text("umap.svg", sc.plot_umap(obj, "PBMC control, UMAP by cluster"))
```

```text
cells:    15049
clusters: 11
```

Nine lines, and every parameter that shapes the result is visible in them. That
is the property to preserve — **the numbers are the analysis**, and hiding them
inside a function makes the analysis unreviewable.

## Or with the decisions printed for you

`sc.standard` runs that pipeline and reports what it did:

```biolang
import "singlecell" as sc

let obj = sc.load("ctrl_raw")
    |> sc.standard(nil, 2000, 15, 250, 100000, 20.0)
```

It prints the equivalent explicit pipeline, the cells in and out, and a table of
every parameter marked `[set]` or `[default]` with a note on what each does — and
tells you that resolution is the one to tune first.

This is **not** a black box. It prints the explicit form, so it teaches the
pipeline while running it, and every decision stays inspectable in
`obj.decisions`. A convenience function that concealed these numbers would
produce results nobody could review; one that prints them lets you graduate to
the explicit form the moment you need to deviate.

## Both samples

The version this dataset is really for:

```biolang
import "singlecell" as sc

# Cells are filtered per sample; genes must stay identical across the two for
# the merge, so gene filtering happens after it.
let ctrl = sc.load("ctrl_raw") |> sc.filter_cells(250, 100000, 20.0)
let stim = sc.load("stim_raw") |> sc.filter_cells(250, 100000, 20.0)

let merged = sc.merge(ctrl, stim, "ctrl", "stim")
    |> sc.filter_genes(3)
    |> sc.normalize()
    |> sc.variable_genes(2000)
    |> sc.run_pca(30)
    |> sc.neighbors(15)
    |> sc.cluster_leiden(15, 0.5)

println("merged: " + str(merged.n_cells) + " cells, " +
        str(sc.get_clusters(merged) |> unique |> len) + " clusters")

let corrected = sc.integrate(merged, merged.batch_ids)
```

```text
merged: 30043 cells, 14 clusters
```

> **The ordering is not cosmetic.** `filter_genes` before the merge leaves the
> two samples with different gene sets, and `sc.merge` refuses:
> `sc_merge_objects() requires identical genes in identical order`. Refusing is
> the right behaviour — quietly aligning by position would produce a matrix
> where column 400 means one gene in one half and another gene in the other.

## Checking it

```biolang
import "singlecell" as sc

let obj = sc.load("ctrl_raw")
    |> sc.filter_genes(3) |> sc.filter_cells(250, 100000, 20.0)
    |> sc.normalize() |> sc.variable_genes(2000) |> sc.run_pca(30)
    |> sc.neighbors(15) |> sc.cluster_leiden(15, 0.5)

println(sc.cluster_diagnostics(obj))
println(sc.cluster_stability(obj))
```

On real data you have no ground truth, so the diagnostics matter more, not less.
The checks that carried weight in this book were the ones where two independent
views had to agree: the marker dot plot and the UMAP topology both say clusters 1
and 9 are monocytes and 7 and 8 are B cells. Neither was told about the other.

## What the workflow does not include

Being explicit about the boundary, because a pipeline that runs to completion
invites the belief that it is finished.

- **Annotation is not in it.** The clusters are numbered. Naming them is manual
  and needs biology the data cannot supply — see
  [Markers and Annotation](ch06-markers.md).
- **Differential expression between conditions is not in it.** With ctrl and
  stim in hand it is the obvious next question, and it needs a pseudobulk design
  with replicates. Testing across cells treats cells from one sample as
  independent replicates, which they are not, and produces confidently wrong
  p-values.
- **Doublet detection is not in it.** `sc.doublets` and `sc.flag_doublets` exist;
  add them if your loading concentration was high.
- **Cell cycle regression is not in it.** `sc.cell_cycle` scores cells so you can
  check whether it matters before deciding to remove it.

## Timings

Measured on a laptop, single-threaded, on this dataset:

| Step | |
|---|---|
| Load one raw matrix (737,280 × 33,538) | 2 s |
| Filter to 15,049 cells | 5 s |
| Normalize → HVG → PCA → neighbours → cluster | 15 s |
| UMAP and plot | 25 s |
| Both samples merged (30,043 cells) | 40 s |
| Harmony on 30,043 cells | minutes |

Nothing here needs a cluster, but the two-sample chapters are not instant.

## Where to go next

Read the course itself, at <https://hbctraining.github.io/Intro-to-scRNAseq/>.
It teaches the same material in R with Seurat, which remains the implementation
everything else is measured against. Credit and licence are on
[Attribution and Licence](attribution.md), and the places BioLang and Seurat
part company are in [What Differs from the Course](differences.md).
