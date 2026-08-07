# The Biology and the Matrix

*Follows HBC lessons 01 (Intro to scRNA-seq) and 02 (Generation of the count matrix).*

## What the experiment buys you

Bulk RNA-seq measures an average. Grind up a tissue, sequence the RNA, and you
learn what the average cell was transcribing. The average is a real number, and
it is often the wrong one.

The classic failure: suppose genes A and B are positively correlated *within*
every cell type, but the cell types sit at different overall levels. Average
across the mixture and the correlation can invert. You will confidently report
that A suppresses B when in every actual cell it does the opposite. This is
Simpson's paradox with a pipette, and it is not an edge case — it is the default
hazard whenever a tissue contains more than one kind of cell, which is every
tissue.

Single-cell RNA-seq measures cells separately, so the grouping the average
destroyed is still there to recover. That buys you which cell types are present
and in what proportion, rare populations a bulk average dilutes away, how
expression shifts along a differentiation trajectory, and — the one that changes
conclusions most often — differential expression **within a cell type** between
conditions. "Gene X is up in disease" is a different claim from "gene X is up in
the monocytes of treated patients", and only the second tells you where to look
next.

That last distinction is exactly what this dataset is built for: the same
patients' cells, control and interferon-stimulated.

## What it costs you

**The data is large.** This one is 33,538 genes by 737,280 droplets per sample.
Stored densely that is 24 billion entries; stored sparsely it is about 10
million. `sc.load` returns a sparse matrix for that reason.

**Sequencing per cell is shallow.** Droplet methods detect perhaps 10–50% of the
transcriptome in a given cell, so a zero is ambiguous: the gene was off, or it
was on and you missed it. You cannot tell which from the number.

This ambiguity is the single most important fact about the data type. The
vocabulary is worth being careful about: scRNA-seq data is often called
*zero-inflated*, meaning more zeros than a count model predicts. Recent analyses
argue it mostly is not — the zeros are about what sequencing depth would
predict, and a plain negative binomial handles them. They are real dropouts, not
evidence of a second process. This matters because "zero-inflated" invites you
to add machinery for it, and the machinery can do harm.

**Biological variation you did not ask about.** Transcription is bursty. Cells
cycle. Cells respond to neighbours. Identity is sometimes a gradient rather than
a category. All real biology, none of it the biology you are studying, all of it
in the same matrix.

**Technical variation.** Capture efficiency differs per cell, amplification is
not uniform, libraries differ in quality, and batches differ from each other —
sometimes more than the conditions do. That is why integration gets two lessons.

## Where the matrix comes from

BioLang starts one step downstream of the sequencer. Worth knowing what that
step did.

In a droplet experiment each droplet ideally captures one cell and one bead. The
bead carries millions of oligos sharing a **cell barcode**, so everything
sequenced from that droplet is stamped with the same identifier. Each oligo also
carries a **unique molecular identifier (UMI)**, a short random sequence
differing between oligos on the same bead.

The UMI is the clever part. PCR amplifies some molecules more than others, so
read counts are a distorted measure of how much RNA was there. But two reads
sharing a UMI *and* a cell barcode *and* a gene came from one original molecule.
Collapse them, count distinct UMIs, and you have counted molecules instead of
reads. Amplification bias largely disappears.

Cell Ranger does the demultiplexing, barcode correction, alignment and UMI
collapsing, and writes three files: `barcodes.tsv` (the columns), `features.tsv`
(the rows) and `matrix.mtx` (the non-zero counts).

**BioLang does not run Cell Ranger, and neither does Seurat.** That work is
upstream of both. What BioLang does is read its output:

```biolang
import "singlecell" as sc

let ctrl = sc.load("ctrl_raw")

println("droplets: " + str(ctrl.n_cells))
println("genes:    " + str(ctrl.n_genes))
println("first genes: " + str(sc.get_genes(ctrl) |> take(5)))
```

```text
droplets: 737280
genes:    33538
first genes: [MIR1302-2HG, FAM138A, OR4F5, AL627309.1, AL627309.3]
```

Two things to notice. The gene names are real symbols, which is what makes
annotation possible later. And 737,280 is every barcode the chemistry can
produce — the overwhelming majority are empty droplets that caught only ambient
RNA. The matrix as loaded is mostly nothing.

The object is a plain BioLang record — `matrix`, `genes`, `barcodes`,
`n_cells`, `n_genes`. Later steps add fields rather than replacing it, so
`keys(obj)` always tells you what has happened.

## What a count actually means

One entry is: *the number of distinct mRNA molecules from gene G that were
captured, reverse transcribed, amplified, sequenced, and successfully assigned
to barcode C.*

Every verb can fail, and each fails at a rate that varies between cells. Holding
onto that is what stops you over-reading a small number later. A count of zero
is weak evidence of absence. A count of one is weak evidence of anything.

## Next

Separating the cells from the empty droplets is
[Quality Control](ch02-quality-control.md).
