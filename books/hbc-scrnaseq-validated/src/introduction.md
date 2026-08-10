# Introduction

This book develops a control-versus-stimulated PBMC analysis in BioLang. It
follows the scientific stages taught by the Harvard Chan Bioinformatics Core
(HBC): sparse count loading, cell and gene quality control, variance
stabilization, sample alignment, dimensional reduction, graph clustering,
marker discovery, and biological interpretation.

The measured implementation was written independently from published algorithm
descriptions. BioLang does not link to or execute R packages. Seurat itself is
MIT-licensed, however, so exact-parity work may inspect or port its covered R
and C++ files while preserving their copyright and licence notice. Copyleft
dependency implementations remain outside BioLang and are used only as
external result oracles. The validation-only scripts are excluded from every
BioLang build and runtime path. A compact, hashed evidence snapshot is
committed with this book; bulky generated PC files remain reproducible, ignored
artifacts.

The executable source of truth is
[`hbc_course_validation.bln`](../../../packages/singlecell/examples/hbc_course_validation.bln).
The book explains that notebook and records its validation outcome. If prose
and executable code ever disagree, the notebook wins.

## What "matching" means

Some outcomes must be exact: the retained barcodes, number of genes, and other
deterministic filtering checkpoints. Other outcomes cannot sensibly be compared
by raw numbers alone. Cluster labels are arbitrary; UMAP coordinates may rotate
or reflect; and different, independently implemented integration algorithms do
not have identical objectives. Those stages are compared with invariant
metrics and marker programs.

This distinction matters here. BioLang exactly reproduces the HBC filtering
checkpoint, and both marker tables contain all 15 prespecified PBMC genes, but
only 5 of those genes peak in the same optimally mapped cluster. Its current
integrated neighborhoods are not a numerical replacement for Seurat. The
[validation report](./validation.md) gives the measurements rather than hiding
that gap.

## Credit

The experimental teaching scenario and public checkpoints come from the
[HBC Introduction to single-cell RNA-seq](https://hbctraining.github.io/Intro-to-scRNAseq/)
course. The HBC materials are CC BY 4.0. This book is newly written and links
to the course for attribution; it does not reproduce the course text or code.
