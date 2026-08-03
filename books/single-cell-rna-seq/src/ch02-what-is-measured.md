# What the Machine Measures

## From tissue to counts

A common droplet workflow follows this path:

1. A tissue or blood sample is separated into cells or nuclei.
2. Individual cells enter droplets with barcoded beads.
3. RNA is copied into DNA while receiving a **cell barcode** and a **unique
   molecular identifier** (UMI).
4. The DNA is sequenced.
5. Reads are aligned or assigned to genes.
6. Reads with the same cell barcode, gene, and corrected UMI are collapsed into
   one observed molecule count.

10x Genomics describes Cell Ranger as performing alignment, filtering, barcode
processing, and UMI counting to produce a feature-barcode matrix. A UMI is used
to reduce the effect of sequencing the same captured molecule many times. See
the [Cell Ranger overview](https://www.10xgenomics.com/support/software/cell-ranger/latest/getting-started/cr-what-is-cell-ranger)
and [gene-expression algorithm](https://www.10xgenomics.com/support/software/cell-ranger/latest/algorithms-overview/cr-gex-algorithm).

## Barcode is not identity

A barcode identifies a captured droplet, not a known biological cell type.
Several complications follow:

- An empty droplet can contain ambient RNA.
- One droplet can capture two or more cells, producing a **doublet** or
  **multiplet**.
- A damaged cell can lose cytoplasmic RNA.
- Two samples can use overlapping barcode sequences.
- A nucleus and a whole cell measure different RNA compartments.

The count matrix therefore begins as a set of candidate cellular libraries, not
perfect cells.

## Why so many zeros?

A zero can mean at least three things:

1. the gene was inactive;
2. the transcript existed but was not captured;
3. the transcript was captured but not confidently assigned.

This is why absence of a marker in one cell is weak evidence. Patterns across
many genes and cells are more reliable.

## The 10x MEX directory

BioLang reads the common Matrix Exchange layout:

```text
filtered_feature_bc_matrix/
  matrix.mtx.gz
  features.tsv.gz
  barcodes.tsv.gz
```

`matrix.mtx.gz` stores only nonzero entries. `features.tsv.gz` names genes or
other features. `barcodes.tsv.gz` names cell barcodes. The file matrix is
typically genes by cells; `sc.load()` exposes it as cells by genes.

## Raw, filtered, and normalized data

- **Raw counts** are observed UMI counts before downstream transformation.
- A **filtered matrix** contains barcodes selected by a cell-calling process.
- **Normalized values** make cells with different library sizes more
  comparable for exploration.
- **Scaled values** center and standardize genes for some algorithms.

Never replace raw counts without recording what happened. Count-based
statistical models may need them later.

## Checkpoint

For every matrix you receive, ask: Is it raw or normalized? Are rows genes or
cells? How were cells called? Are gene symbols unique? Which genome annotation
was used? Which sample produced each barcode?
