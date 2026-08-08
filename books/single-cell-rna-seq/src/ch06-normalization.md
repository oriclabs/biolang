# Normalize and Select Signal

## Why library size matters

One cell may have 20,000 observed UMIs and another 5,000. Directly comparing raw
counts makes deeply sampled cells look globally more active. A common
exploratory transformation divides each cell by its total, multiplies by a
target such as 10,000, and applies `log(1 + x)`.

> Requires CLI: this example imports the package.

```biolang
import "singlecell" as sc

let obj = sc.load("ctrl_raw")
    |> sc.filter_genes(3)
    |> sc.filter_cells(250, 100000, 20.0)
    |> sc.normalize(10000.0)

println(sc.summary(obj))
```

Raw counts remain in `layers.counts`; normalized values are in `norm_matrix`
and `layers.lognorm`.

## What normalization does not do

Normalization does not:

- recover transcripts that were not captured;
- remove batch effects;
- prove that a gene is differentially expressed;
- make every cell biologically comparable;
- replace a replicate-aware statistical model.

It creates a more useful representation for exploration and many downstream
geometric methods.

## Highly variable genes

Thousands of genes vary little or mostly add noise. Highly variable gene (HVG)
selection retains a subset whose variation can help describe cell-to-cell
structure:

> Requires CLI: this example imports the package.

```biolang
import "singlecell" as sc

let obj = sc.load("ctrl_raw")
    |> sc.filter_genes(3)
    |> sc.filter_cells(250, 100000, 20.0)
    |> sc.normalize()
    |> sc.variable_genes(2000)

println(take(obj.hvg_genes, 12))
```

BioLang currently ranks genes using dispersion (coefficient of variation
squared) across all genes at once. Scanpy's `seurat` flavor and Seurat's VST
first bin genes by mean expression and rank dispersion within each bin, which
compensates for the fact that low-mean genes have higher dispersion for purely
sampling reasons. BioLang's unbinned ranking therefore leans further toward
low-expression genes. Exact HVG lists are not expected to match across tools;
compare downstream structure and marker behavior, not gene-list identity.

## SCTransform

`sc.sctransform()` provides regularized negative-binomial residuals as an
alternative transformation:

> Requires CLI: this example imports the package.

```biolang
import "singlecell" as sc

let transformed = sc.load("ctrl_raw")
    |> sc.filter_genes(3)
    |> sc.filter_cells(250, 100000, 20.0)
    |> sc.sctransform()

println(sc.summary(transformed))
```

Note that the result is dense. Pearson residuals are nonzero where counts were
zero, so unlike `normalize()` this does not preserve sparsity — check the cell
and gene counts before running it on a large matrix.

Do not mix normalization strategies without a reason. Record which matrix each
later method used.

## Decision record

For this step, save:

- the normalization method and target;
- whether raw counts were preserved;
- the HVG method and number selected;
- genes deliberately excluded, such as mitochondrial or ribosomal sets;
- plots or summaries used to justify the choice.
