# The Whole Workflow

*Follows HBC lesson 14 (scRNA-seq workflow).*

The course closes by writing the whole analysis out in one place, and it is the
most useful lesson in the set. Individual steps make sense in isolation; the
shape of the thing only becomes visible when you see it end to end.

## Written out explicitly

```biolang
import "singlecell" as sc

let obj = sc.load("nsclc_like")
    |> sc.filter_genes(3)              # gene seen in >= 3 cells
    |> sc.filter_cells(20, 2500, 5.0)  # min genes, max genes, max % mito
    |> sc.normalize(10000.0)           # CP10K then log1p
    |> sc.variable_genes(50)           # HVGs to focus PCA
    |> sc.run_pca(20)                  # linear dimensionality reduction
    |> sc.neighbors(15)                # kNN graph, k = 15
    |> sc.cluster_leiden(15, 0.5)      # communities at resolution 0.5

println("cells: " + str(obj.n_cells))
println("clusters: " + str(sc.get_clusters(obj) |> unique |> len))
println(sc.plot_umap(obj, "clusters"))
```

Nine lines. Every parameter that shapes the result is visible in them, which is
the property to preserve — the numbers *are* the analysis, and hiding them
inside a function makes the analysis unreviewable.

## Or with the decisions printed for you

`sc.standard` runs exactly that pipeline and reports what it did:

```biolang
import "singlecell" as sc

let obj = sc.load("nsclc_like")
    |> sc.standard(nil, 50, 15, 20, 2500, 5.0)
```

which prints:

```text
sc.standard — equivalent explicit pipeline:

    obj |> sc.filter_genes(3)
        |> sc.filter_cells(20, 2500, 5.0)
        |> sc.normalize(10000.0)
        |> sc.variable_genes(50)
        |> sc.run_pca()
        |> sc.neighbors(15)
        |> sc.cluster_leiden(15, 0.5)

  265 cells in -> 220 kept -> 4 clusters

  parameter      value      source     note
  min_cells      3          [default]  gene kept if seen in >= this many cells
  min_genes      20         [set]      cell kept if it has >= this many genes
  max_genes      2500       [set]      upper cut, catches doublets
  max_pct_mito   5.0        [set]      needs gene SYMBOLS to work (MT- prefix)
  target         10000.0    [default]  counts-per-cell before log1p
  n_hvg          50         [set]      variable genes kept
  k              15         [set]      neighbours in the kNN graph
  resolution     0.5        [default]  drives cluster count — tune this first
```

Note what this does and does not do. It is **not** a black box that hides the
pipeline — it prints the explicit form, marks which parameters you set and which
fell back to defaults, and tells you the one to tune first. Every decision stays
inspectable in `obj.decisions`.

That distinction is the whole design argument. A convenience function that
concealed these numbers would produce results nobody could review. One that
prints them teaches the pipeline while running it, and you can graduate to the
explicit form the moment you need to deviate.

## Checking it

```biolang
import "singlecell" as sc

let obj = sc.load("nsclc_like")
    |> sc.standard(nil, 50, 15, 20, 2500, 5.0, nil, nil, true)

println(sc.cluster_diagnostics(obj))
```

The fixture was built with four populations, and the pipeline recovers four
clusters. That agreement is the point of using synthetic data to learn on: you
know the answer, so you can tell whether the method found it. On real data you
have no such check, which is why the diagnostics in
[Clustering](hbc-05-clustering.md) matter more there, not less.

## What the workflow does not include

Being explicit about the boundary, because a pipeline that runs to completion
invites the belief that it is finished.

- **Annotation is not in it.** The clusters are numbered. Naming them is manual
  and needs biology the data cannot supply.
- **Differential expression between conditions is not in it.** That needs
  replicates and a pseudobulk design — see
  [Differential Expression Without Pseudoreplication](ch10-differential-expression.md).
  Testing across cells rather than samples treats cells from one donor as
  independent replicates, which they are not, and produces confidently wrong
  p-values.
- **Doublet detection is not in it.** `sc.doublets` and `sc.flag_doublets`
  exist; add them if your loading concentration was high.
- **Integration is not in it.** Add `sc.integrate` only if you have multiple
  samples that need it — see [Integration](hbc-04-integration.md).

## Where to go next

You have now walked the HBC curriculum. The rest of this book goes past where
the course stops:

- [Trajectories, Cell Cycle, and Doublets](ch11-cell-state.md) — continuous
  processes rather than discrete types
- [A Tumor Microenvironment Case Study](ch12-case-study.md) — the whole thing on
  a real biological question
- [Validate with Scanpy and Seurat](ch13-validation.md) — checking BioLang's
  numbers against the reference implementations
- [Failure Modes and Review Checklist](ch15-failure-modes.md) — what to check
  before you believe your own result

And read the course itself, at
<https://hbctraining.github.io/Intro-to-scRNAseq/>. It teaches the same material
in R with Seurat, which remains the implementation everything else is measured
against. Credit and licence are on the
[Attribution and licence](hbc-attribution.md) page.
