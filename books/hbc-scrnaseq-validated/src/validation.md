# Validation report

This report is backed by fresh executions on the same six hashed HBC 10x input
files. The Seurat oracle and BioLang run both retained exactly 29,629 joined
barcodes and 14,065 genes. Cell comparisons join on sample plus original 10x
barcode; cluster integers are never assumed to correspond.

## Measured environments

- Run date: 9 August 2026, Windows 11 x64.
- External oracle: R 4.5.2, Seurat 5.5.1, SeuratObject 5.4.0,
  sctransform 0.4.3, Matrix 1.7-4, glmGamPoi 1.22.0.
- Seurat settings: SCT v2 per sample, 3,000 integration features, the HBC
  30-dimension anchor default, 40 downstream PCs, 20 neighbors, Louvain, and
  resolution 0.8.
- BioLang: release CLI, NVIDIA GeForce RTX 3080 through Vulkan,
  `countsketch_subspace_gpu`, 46,484 anchors.
- The CPU backend was not rerun after correcting BioLang to separate the 30
  anchor dimensions from its 50-PC downstream calculation. Older CPU figures
  are not mixed into this report.

The current Seurat stack produced 19 clusters at resolution 0.8. Its measured
trajectory for resolutions 0.4, 0.6, 0.8, 1.0, and 1.4 was
14, 16, 19, 22, and 27. The historical HBC rendered lesson reports 17 at 0.8;
that remains a historical course checkpoint, not the oracle for this package
stack.

## Cell, feature, and neighborhood results

| Metric | Measured result | Interpretation |
|---|---:|---|
| joined cells | 29,629 / 29,629 | exact cell identity match |
| filtered genes | 14,065 / 14,065 | exact QC match |
| integration features | 2,596 / 3,000 (86.53%) | overlap relative to either 3,000-gene set |
| feature-set Jaccard | 76.26% | 2,596 shared out of 3,404 unique genes |
| clusters: historical HBC / Seurat / BioLang | 17 / 19 / 15 | cluster counts do not match |
| adjusted Rand index | 0.5276 | partial partition agreement |
| adjusted mutual information | 0.6924 | partial information agreement |
| optimal one-to-one mapped-cell accuracy | 65.66% | penalizes splits, merges, and four unmatched Seurat clusters |
| integrated-PC 15-NN Jaccard | 0.0648 | approximate NNDescent comparison, seed 123456 |
| UMAP 15-NN Jaccard | 0.0159 | exact two-dimensional k-d-tree comparison |

The very low PC-neighborhood overlap locates the primary parity gap upstream of
UMAP, in the independently implemented sketched CCA/anchor correction and its
downstream representation. Matching the cluster count by tuning resolution
would not repair that difference.

## Marker results

Both tools used log-normalized, uncorrected RNA values, `min.pct = 0.1`, a
log2-fold-change threshold of 0.25, positive markers only, and Wilcoxon tests.
Seurat returned 12,634 positive cluster-gene rows; BioLang returned 9,359.

| Marker metric | Measured result |
|---|---:|
| positive rows: Seurat / BioLang | 12,634 / 9,359 |
| mapped cluster-gene intersection | 3,291 |
| overlap relative to smaller marker table | 35.16% |
| mapped marker-pair Jaccard | 17.60% |
| mapped top-50 overlap | 199 / 750 (26.53%) |
| prespecified PBMC genes present in both tables | 15 / 15 |
| prespecified genes peaking in the same mapped cluster | 5 / 15 |

The 15/15 presence result shows that canonical PBMC signals survive. The 5/15
peak-cluster result and low mapped marker overlap show that the partitions and
their interpretation are not interchangeable. The earlier draft's "8/8"
marker statement was not backed by a Seurat marker run and has been removed.

## Timing and memory observations

- Seurat completed QC, SCT, CCA integration, PCA, UMAP, clustering, and export
  in 14 minutes 48 seconds. Ctrl and stim SCT took 74 and 70 seconds.
- BioLang completed the full notebook, including marker tests and exports, in
  12 minutes 12 seconds. UMAP was written 9 minutes 11 seconds after launch.
- The Seurat marker-only audit took 10 minutes 30 seconds, of which 10 minutes
  13 seconds was `FindAllMarkers` without the optional `presto` accelerator.
- BioLang's observed peak resident working set was about 22.4 GB. Seurat's GC
  log recorded about 10.8 GB maximum vector use, while sampled resident memory
  peaked near 7.2 GB. These are different measurement mechanisms, so they are
  evidence of a BioLang memory gap, not a precise ratio.

## Decision

BioLang covers the complete HBC teaching workflow and exactly reproduces its
filtering result. On this machine it completed the measured GPU notebook faster
than the Seurat clustering oracle, but used substantially more peak memory.
Feature, partition, neighborhood, and marker comparisons are only partial.

This version is **not a drop-in numerical replacement for Seurat** and must not
be described as producing the same integrated clustering, UMAP, or marker
assignment. Future parity work should improve the integrated representation
and rerun this identical joined-cell gate. The scripts, compact outputs, logs,
hashes, and comparison JSON are retained in the
[evidence snapshot](./evidence/2026-08-09/README.md).
