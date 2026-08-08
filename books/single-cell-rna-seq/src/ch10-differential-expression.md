# Differential Expression Without Pseudoreplication

## Two different marker questions

**Cluster marker analysis** asks which genes distinguish groups of cells in the
current dataset. **Condition differential expression** asks whether expression
changes consistently across independent samples.

The first can help annotation. The second supports a population-level claim and
requires biological replication.

## Why cell-level tests can fail

Cells from one donor are correlated. If each cell is treated as an independent
replicate, a method can report extremely small p-values for donor-specific
differences. The
[Squair et al. benchmark](https://www.nature.com/articles/s41467-021-25960-2)
showed that methods accounting for biological replication, including
pseudobulk approaches, better controlled false discoveries.

## Pseudobulk in plain language

For each sample and cell type:

1. return to raw counts;
2. sum counts across cells;
3. create one count profile per biological sample and cell type;
4. use a replicate-aware bulk RNA-seq model with the study design;
5. report effect sizes, uncertainty, and multiple-testing correction.

If eight patients each have monocytes, the monocyte comparison has up to eight
independent patient profiles, not thousands of independent monocytes.

## Build the table in BioLang

`sc.pseudobulk()` does steps 1 to 3. It sums **raw** counts, not normalized
values, within each (cluster, sample) pair. The teaching fixture has no donors,
so assign synthetic ones to see the shape:

> Requires CLI: this example imports the package and reads local files.

```biolang
import "singlecell" as sc

let obj = sc.standard(
    sc.load("ctrl_raw"),
    resolution: 0.5, n_hvg: 100, k: 15,
    min_genes: 20, max_genes: 2500, max_pct_mito: 5.0,
    min_cells: 3, target: 10000.0, quiet: true
)

# Stand-in donor labels. In a real study these come from your sample sheet.
let donors = range(0, obj.n_cells) |> map(|i| "donor_" + str(i % 4))

let panel = sc.pseudobulk(obj, donors)
println(colnames(panel))
println(str(nrow(panel)) + " genes x " + str(len(colnames(panel))) + " profiles")
```

Each column is one independent profile named `<cluster>__<sample>`, and each row
is a gene in `obj.genes` order. Four clusters across four donors gives sixteen
columns — the real sample size for a condition contrast, against 220 cells.

Counts stay sparse until they are summed, so this does not densify the matrix.

## Exploratory paired analysis and the formal boundary

`sc.marker_table()` performs cluster-to-cluster exploratory marker comparison.
Its `log2fc` is positive when a gene is higher in the first cluster, matching
Seurat's `FindMarkers` and scanpy's `rank_genes_groups`. It is not a replacement
for a pseudobulk negative-binomial model such as those used by edgeR or DESeq2.

BioLang includes `sc.pseudobulk_profiles()` and
`sc.paired_pseudobulk_de()` for a transparent donor-paired log2-CPM analysis.
It also provides volcano, MA, paired-donor, and pseudobulk-PCA plots. This is
appropriate for teaching, exploration, and cross-checking effect direction.

Formal count inference remains an explicit handoff. Write the raw panel out and
invoke a validated R, Python, container, or workflow step using DESeq2, edgeR,
or another suitable negative-binomial method. For paired samples, represent the
pairing in the design, for example `~ donor + condition`. Keep this boundary
explicit in the report.

A project handoff might look like:

```text
results/pseudobulk_counts.tsv
results/sample_metadata.tsv
scripts/run_deseq2.R
results/differential_expression.tsv
validation/session_info.txt
```

## Questions for every DE table

- What is the independent biological unit?
- How many units are in each condition?
- Was pairing represented?
- Were batches and major covariates included?
- Were raw counts used by a suitable count model?
- Was the cell type defined without using the tested condition in a circular
  way?
- Are effect sizes meaningful, not merely significant?
- Were results stable to QC and annotation sensitivity analyses?

## For clinicians

A differentially expressed gene is an association in a specified dataset. It
does not establish causality, drug response, prognosis, or diagnostic accuracy.
Those require separate study designs and validation.
