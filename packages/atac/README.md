# scATAC-seq in BioLang

The `atac` package now covers the practical core of a consensus-peak scATAC
integration workflow without constructing a dense genome-wide peak matrix.

```biolang
import "atac" as atac
import "singlecell" as sc

let peaks = atac.consensus(sample_peak_tables, 1)
    |> atac.filter_peaks(20, 10000, hg38_blacklist)

let result = atac.peaks_sparse("fragments.tsv.gz", peaks, qc_barcodes)
    |> atac.tfidf(10000)
    |> atac.top_features(20)
    |> atac.lsi(50)
    |> atac.select_dimensions(range(1, 30))  # R dimensions 2:30
    |> atac.harmony(batch_ids)
    |> sc.neighbors(20)
    |> sc.cluster_leiden(20, 0.4)
    |> sc.run_umap(29)
```

For multiple libraries, quantify each against the same `peaks` table and use
`atac.merge_samples(objects, sample_ids)` before TF-IDF. It preserves all sample
labels across a four-or-more-object merge, which is required for Harmony and
the article's mixing score.

## NGS101 Parts 1 and 2

Part 1 is an upstream data-production workflow, not an algorithm BioLang should
silently replace. It downloads four SRA runs and uses the proprietary Cell
Ranger ATAC 2.2.0 pipeline with the 10x GRCh38-2024-A reference. The article
loads `filtered_peak_bc_matrix.h5`; BioLang reads the equivalent official MEX
directory (`filtered_peak_bc_matrix/` containing `peaks.bed`, `barcodes.tsv`,
and `matrix.mtx`) with `read_10x_sparse`, plus `fragments.tsv.gz` and
`singlecell.csv`. Both matrix representations are emitted by the same Cell
Ranger run and contain the same feature-by-barcode counts.

The four article samples are severe1/SRR31499777/GSM8650274,
severe2/SRR31499804/GSM8650278, healthy1/SRR31499787/GSM8650264, and
healthy2/SRR31499788/GSM8650263. Preserve those sample names because Part 3
uses them as the Harmony batch labels.

The Part 1 severe1 checkpoint is 110,909 Cell Ranger peaks. Loading the MEX
directory must therefore produce `obj.n_genes == 110909` and `len(obj.peaks) ==
110909` before Part 2 removes non-standard chromosomes and rare features.

Part 2 QC can be computed without making a dense peak matrix:

```biolang
import "atac" as atac

# Official Cell Ranger ATAC MEX; returns a sparse cell x peak object and obj.peaks.
let obj = read_10x_sparse("filtered_peak_bc_matrix")

# Cell Ranger's called barcodes, in the matrix order.
let nucleosome = atac.signac_nucleosome("fragments.tsv.gz", barcodes)
let tss_qc = atac.tss_metrics("fragments.tsv.gz", tss_positions, barcodes)
let matrix_qc = atac.peak_metrics(obj.matrix, peaks_before_blacklist, hg38_blacklist)

# These two columns come from Cell Ranger's singlecell.csv.
let pct_reads_in_peaks = atac.frip(peak_region_fragments, passed_filters)

let cell_filter = atac.ngs101_part2_filter(
    passed_filters, tss_enrichment, pct_reads_in_peaks,
    nucleosome_signal, blacklist_ratio
)

# Run after applying cell_filter and external AMULET singlet barcodes.
let min_cells = max(10, ceil(0.005 * filtered_obj.n_cells))
let peak_indices = atac.detected_features(filtered_obj.matrix, min_cells)
```

`signac_nucleosome` deliberately reproduces the article's deprecated Signac
`NucleosomeSignal` default: it reads the first `number_of_cells * 5000` valid
fragment rows and reports fragments 147--294 bp divided by fragments below
147 bp. `tss_metrics` reproduces the Signac 1.17 `fast=FALSE` score used by the
article, including its cut-matrix endpoint convention and minus-strand reversal.
It returns the per-cell score but does not retain Signac's full per-base matrix.
`peak_metrics` computes blacklist fraction before blacklist peaks are removed.

For severe1, the Part 2 checkpoints are:

| Checkpoint | Article value |
|---|---:|
| Standard peaks before `min.cells` | 110,819 |
| Peaks and cells after `min.cells=10`, `min.features=200` | 110,667 x 4,614 |
| Median TSS enrichment / nucleosome signal / FRiP | 5.7426 / 0.5164 / 62.50% |
| Cells after the six QC thresholds | 3,924 (85.0%) |
| AMULET doublets removed at q < 0.05 | 405 |
| Final cells / peaks after the 0.5% peak floor | 3,519 / 89,129 |

The six cell filters are `passed_filters > 3000`, `passed_filters < 100000`,
`TSS.enrichment > 2`, `pct_reads_in_peaks > 15`, `nucleosome_signal < 4`, and
`blacklist_ratio < 0.05`. After doublet removal, retain peaks detected in at
least `max(10, ceil(0.005 * number_of_cells))` cells.

AMULET remains an external validation boundary. The article uses
`scDblFinder::amulet`, a GPL-3 package; it is not linked into or copied into the
MIT BioLang runtime. Its barcode/q-value output can be read as a table and
applied before the rare-peak filter.

## What matches the reference workflow

- Consensus peak union (`min_samples = 1`) and multi-sample support filtering.
- Strict peak widths `> min_width` and `< max_width`, human standard
  chromosomes, and whole-peak removal on blacklist overlap.
- Sparse fragment-by-peak quantification in cell x peak orientation.
- TF-IDF method 1:
  `log1p((count / cell_total) * (n_cells / peak_total) * scale_factor)`.
- Numeric top-feature cutoffs use total peak counts and retain `count > cutoff`.
- Uncentered partial SVD, then component-wise mean centering and sample-SD
  scaling without clipping.
- Sequencing-depth correlations, exclusion/selection of LSI dimensions, native
  Harmony correction, other-sample nearest-neighbour mixing diagnostics, SNN
  graphs, UMAP, and resolution sweeps through the `singlecell` package.

Peak counts and TF-IDF remain CSR sparse. LSI scores, loadings, and Harmony
embeddings use compact native numeric matrices. A 30,571 x 185,581 dense f64
matrix would require about 45.4 GB before temporary copies; the sparse path's
numeric storage instead scales with non-zero fragment/peak overlaps.

## Important differences

The mathematical preprocessing is compatible, but an identical final cluster
labeling is not promised. Current BioLang uses its restarted Lanczos partial SVD
rather than the current RSpectra implementation, native Harmony rather than the
R package, and Leiden or Louvain rather than Seurat's SLM `algorithm = 3`.
These can produce equivalent biological structure without identical component
signs, neighbour ties, or community boundaries.

Reciprocal-LSI anchor integration, SLM, clustree diagrams, and genomic coverage
tracks are not yet direct equivalents. Use Harmony plus Leiden for the native
workflow; use a black-box R comparison when exact published labels matter.

## NGS101 Part 3 validation targets

The article's numbers are data-and-version-specific targets, not defaults to
tune toward:

| Checkpoint | Article value | BioLang status |
|---|---:|---|
| Consensus peaks | 185,581 | Pipeline implemented; real article inputs not run |
| QC cells | 30,571 (3,485 / 4,773 / 6,861 / 15,452) | Requires the Part 2 QC barcode lists |
| Features with total count > 20 | 184,302 | Exact Signac cutoff semantics implemented |
| LSI-1 depth correlation | 0.939 | Exact Pearson diagnostic implemented |
| Uncorrected / Harmony mixing | 0.162 / 0.485 | Exact article metric implemented (`k=30`, self removed) |
| Imbalance-aware random ceiling | 0.657 | Exact `1 - sum(p_sample^2)` calculation implemented |
| SLM cluster sweep | 14 / 17 / 21 / 22 / 24 / 26 | Not exact: BioLang currently offers Leiden/Louvain, not SLM |

The article regenerated these samples from raw SRA reads with Cell Ranger ATAC
2.2.0. GEO's smaller processed matrices for GSE282769 were generated with Cell
Ranger ATAC 1.2.0, so they are useful for a separate real-data comparison but
are not a valid oracle for the article's exact peak, component, or cluster
counts. A defensible exact comparison needs the article's four Part 1 output
directories and four Part 2 QC barcode lists.

There is also a published-page inconsistency that validation must retain rather
than hide: Part 2 ends severe1 with 3,519 cells, while Part 3 loads severe1 with
3,485 cells. Until the tutorial's saved RDS objects or exact barcode lists are
available, neither number can be derived from the other page and both should be
recorded as page-specific targets.

The formulas and workflow behavior follow the published LSI/Signac and Harmony
methods and their public documentation:

- [Signac TF-IDF documentation](https://stuartlab.org/signac/reference/runtfidf)
- [Signac partial SVD documentation](https://stuartlab.org/signac/reference/runsvd)
- [Signac depth-correlation documentation](https://stuartlab.org/signac/reference/depthcor)
- [Signac paper](https://www.nature.com/articles/s41592-021-01282-5)
- [Harmony paper](https://www.nature.com/articles/s41592-019-0619-0)
- [NGS101 Part 1](https://ngs101.com/how-to-analyze-single-cell-atac-seq-data-a-complete-beginners-guide-part-1-from-fastq-to-peaks/)
- [NGS101 Part 2](https://ngs101.com/how-to-analyze-single-cell-atac-seq-data-a-complete-beginners-guide-part-2-thorough-quality-control-with-signac/)
- [GEO GSE282769](https://www.ncbi.nlm.nih.gov/geo/query/acc.cgi?acc=GSE282769)
