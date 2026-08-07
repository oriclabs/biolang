# The Biology and the Matrix

*Follows HBC lessons 01 (Intro to scRNA-seq) and 02 (Generation of the count matrix).*

## What the experiment buys you

Bulk RNA-seq measures an average. Take a gram of tumor, grind it, sequence the
RNA, and you learn what the average cell in that gram was transcribing. The
average is a real number, and it is often the wrong one.

The classic failure: suppose gene A and gene B are positively correlated within
every cell type, but the cell types themselves sit at different overall levels.
Average across the mixture and the correlation can invert. You will confidently
report that A suppresses B when in every actual cell it does the opposite. This
is Simpson's paradox with a pipette, and it is not a rare edge case — it is the
default hazard whenever a tissue contains more than one kind of cell, which is
every tissue.

Single-cell RNA-seq measures cells separately, so the grouping that the average
destroyed is still there to be recovered. That buys you:

- which cell types are present, and in what proportion
- rare populations that a bulk average dilutes below detection
- how expression changes along a differentiation trajectory
- differential expression **within a cell type** between conditions

The last one is the one that changes conclusions most often. "Gene X is up in
disease" is a different claim from "gene X is up in the macrophages of disease
patients", and only the second one tells you where to look next.

## What it costs you

Four things go wrong that do not go wrong in bulk. Each gets a chapter later;
here is the shape of each.

**The data is large.** Tens of thousands of cells by twenty thousand genes.
Stored densely that is a matrix with hundreds of millions of entries, most of
them zero. This is why the count matrix is stored sparsely and why BioLang's
`sc.load` returns a sparse matrix rather than a list of lists.

**Sequencing per cell is shallow.** Droplet methods detect perhaps 10–50% of the
transcriptome in any given cell. So a zero in the matrix is ambiguous: the gene
was off, or the gene was on and you missed it. You cannot tell which from the
number alone.

This ambiguity is the single most important fact about the data type, and it is
worth being careful about the vocabulary. scRNA-seq data is often called
*zero-inflated*, meaning it has more zeros than a count model would predict.
Recent analyses argue that it mostly does not — the zeros are about what you
would expect given the sequencing depth, and a plain negative binomial handles
them. The zeros are real dropouts, not evidence of a second process. This
matters because "zero-inflated" invites you to add a machine for it, and the
added machine can do harm.

**Biological variation you did not ask about.** Transcription is bursty; a gene
that is "on" is not transcribing continuously, so harvest time decides whether
you catch it. Cells cycle. Cells respond to their neighbours. Cell identity is
sometimes a gradient rather than a category. All of this is real biology and
none of it is the biology you are studying, and it lands in the same matrix.

**Technical variation.** Capture efficiency differs per cell. Amplification is
not uniform across transcripts. Libraries differ in quality. And batches differ
from each other, sometimes more than the conditions do — which is why
integration gets two lessons later on, and why the study design chapter earlier
in this book insists that you never confound a batch with a condition.

## Where the matrix comes from

BioLang starts one step downstream of the sequencer. It is worth knowing what
that step did.

In a droplet experiment, each droplet ideally captures one cell and one bead.
The bead carries millions of oligos, and every oligo on a given bead shares a
**cell barcode** — so everything sequenced from that droplet is stamped with the
same identifier. Each individual oligo also carries a **unique molecular
identifier (UMI)**, a short random sequence that differs between oligos on the
same bead.

The UMI is the clever part. PCR amplifies some molecules more than others, so
read counts are a distorted measure of how much RNA was there. But two reads
carrying the same UMI *and* the same cell barcode *and* the same gene came from
one original molecule. Collapse them, count the distinct UMIs, and you have
counted molecules instead of reads. Amplification bias largely disappears.

Cell Ranger (or an equivalent) does the demultiplexing, barcode correction,
alignment, and UMI collapsing, and writes three files:

- `barcodes.tsv` — one cell barcode per line, the matrix columns
- `features.tsv` — one gene per line, the matrix rows
- `matrix.mtx` — the non-zero counts, in Matrix Market format

**BioLang does not run Cell Ranger, and neither does Seurat.** That work is
upstream of both. What BioLang does is read its output:

```biolang
import "singlecell" as sc

# A 10x MEX directory: barcodes.tsv, features.tsv, matrix.mtx (.gz is fine).
# Here we use the bundled demo data instead of a path on disk.
let raw = sc.load("nsclc_like")

println("cells: " + str(raw.n_cells))
println("genes: " + str(raw.n_genes))
println("first genes:    " + str(sc.get_genes(raw) |> take(5)))
println("first barcodes: " + str(sc.get_barcodes(raw) |> take(3)))
```

The object you get back is a plain BioLang record — `matrix`, `genes`,
`barcodes`, `n_cells`, `n_genes`. Every later step adds fields to it rather than
replacing it, so you can always look inside and see what a step did. There is no
opaque class here; `keys(raw)` will tell you the whole story.

## What a count actually means

One entry of that matrix is: *the number of distinct mRNA molecules from gene G
that were captured, reverse transcribed, amplified, sequenced, and successfully
assigned to cell C.*

Every verb in that sentence can fail, and each one fails at a rate that varies
between cells. That is the honest reading, and holding onto it is what keeps you
from over-interpreting a small number later. A count of zero is weak evidence of
absence. A count of one is weak evidence of anything.

## Next

The matrix as loaded contains cells that are not cells — empty droplets that
caught ambient RNA, dying cells leaking their contents, and droplets that caught
two cells at once. Sorting those out is [Quality Control](hbc-02-quality-control.md).
