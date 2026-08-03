# Quality Control

## Why QC exists

Quality control asks whether each barcode is a plausible measurement of one
useful cell. It is not a ritual for applying universal thresholds. Tissue,
protocol, chemistry, and biological state all change the distributions.

Three common per-cell metrics are:

- `total_counts`: total observed UMIs;
- `n_genes`: number of genes with a nonzero count;
- `pct_mito`: percentage of counts assigned to mitochondrial genes.

Low totals or few detected genes can indicate empty droplets or damaged cells.
Very high totals or gene counts can indicate doublets. High mitochondrial
fraction can indicate stress or membrane damage, but it can also be normal in
some tissues.

## Inspect before filtering

> Requires CLI: this example imports the package and reads local files.

```biolang
import "singlecell" as sc

let raw = sc.load("nsclc_like") |> sc.qc()
println(head(raw.cell_qc_table, 8))
println(head(raw.gene_qc_table, 8))
```

Plot or summarize each metric by sample. A global threshold can unfairly remove
one sample when library depth differs.

## Filter with stated reasons

The teaching matrix has only 168 genes, so its minimum is intentionally much
lower than a real whole-transcriptome matrix:

> Requires CLI: this example imports the package and reads local files.

```biolang
import "singlecell" as sc

let raw = sc.load("nsclc_like")
let clean = raw
    |> sc.filter_genes(3)
    |> sc.filter_cells(20, 2500, 5.0)

println("before: " + str(raw.n_cells))
println("after:  " + str(clean.n_cells))
```

For a real matrix with roughly 20,000 to 35,000 genes, a starting exploratory
range might be 200 to 5,000 detected genes and a tissue-appropriate
mitochondrial threshold. These are starting points, not standards.

## Empty droplets and ambient RNA

A filtered Cell Ranger matrix has already undergone cell calling, but it can
still contain borderline barcodes. Conversely, aggressive filtering can discard
small, low-RNA cell types. The
[EmptyDrops paper](https://pmc.ncbi.nlm.nih.gov/articles/PMC6431044/)
formalized a test against the ambient RNA profile and showed why total-count
thresholds alone can miss biologically meaningful cells.

BioLang's current package provides metric-based filtering, not an EmptyDrops
implementation. If raw droplet cell calling is scientifically important, run a
validated cell-calling method upstream and record it.

## Doublets

`sc.doublets()` simulates mixtures and assigns a score; `sc.flag_doublets()`
applies a threshold:

> Requires CLI: this example imports the package.

```biolang
import "singlecell" as sc

let scored = clean |> sc.normalize() |> sc.variable_genes(100) |> sc.doublets(500)
let flagged = scored |> sc.flag_doublets(0.5)
println(flagged.is_doublet |> filter(|x| x) |> len)
```

A computational flag is evidence, not certainty. The
[Scrublet study](https://pmc.ncbi.nlm.nih.gov/articles/PMC6625319/)
describes how simulated doublets and nearest neighbors can identify hybrid
profiles.

## What can fool you

- Mitochondrial gene recognition depends on gene symbols such as `MT-...`.
- Removing all high-count cells can erase a genuinely large cell type.
- Filtering samples separately and then merging is often safer than one global
  cutoff.
- A low-quality cluster should not automatically be relabeled as a novel type.
- Report cells before and after every filter, per sample.
