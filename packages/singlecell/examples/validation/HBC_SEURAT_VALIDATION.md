# HBC Seurat validation boundary

`hbc_seurat_reference.R` is an independent, validation-only execution of the
public HBC control/stimulated PBMC course workflow. It is not an implementation
source for BioLang and is not called by BioLang, Cargo, package tests, notebook
execution, or book builds.

This separation is deliberate. Seurat and SeuratObject are MIT-licensed, while
parts of their dependency graph use GPL-family licences. The R packages may be
installed and executed in a separate environment to measure compatibility, but
they are not BioLang runtime dependencies. MIT-covered Seurat R/C++ files may
be inspected or ported with their copyright and licence notice retained;
copyleft dependency implementations must not be copied into BioLang.

From the BioLang repository root:

```powershell
$env:BIOLANG_VALIDATION_R_LIB = (Resolve-Path .validation-r-library)
& 'C:\Program Files\R\R-4.5.2\bin\Rscript.exe' `
  packages/singlecell/examples/validation/hbc_seurat_reference.R `
  ctrl_raw stim_raw validation-results/hbc-seurat
```

The script follows the HBC lesson calls and records:

- SHA-256 hashes of all six 10x input files;
- exact cell and gene QC checkpoints;
- the 3,000 selected integration features;
- per-cell cluster, UMAP, and 40-PC manifests;
- the full cluster trajectory for resolutions 0.4, 0.6, 0.8, 1.0, and 1.4;
- R/Seurat session information, logs, artifact sizes, and hashes.

The HBC course leaves `dims` unspecified for `FindIntegrationAnchors`, which
means Seurat's 1:30 default is used there. The course then uses PCs 1:40 for
UMAP and `FindNeighbors`. Keeping those two choices distinct is necessary for
an honest course reproduction.

Generated results belong under `validation-results/`, which is ignored because
the PC and cell manifests can be large. Small, reviewed evidence snapshots may
be copied into the validated book only after a successful run; claims in the
book must name their originating artifact and input hashes.

## Measured result (2026-08-10)

The independent Seurat 5.5.1 run and the final BioLang CPU run joined all
29,629 cells exactly. Seurat returned 19 clusters and BioLang returned 21. The
standalone SCTransform curves are highly correlated, but calibrated HBC
conformance currently fails on raw theta and residual-variance slope (see
`SCTRANSFORM_ALGORITHM_AUDIT.md`). The end-to-end partition also does not
match: ARI is 0.5750, adjusted mutual information is 0.7269, and optimal
one-to-one mapped-cell accuracy is 69.92%. Integration-feature overlap is
2,716/3,000 (90.53%) and integrated-PC 15-neighbour mean Jaccard is 0.2278.

Marker validation found 2,993 overlapping mapped cluster/gene pairs, 191
overlapping mapped top-50 pairs, and matching peak clusters for 7/15 canonical
genes. See `validation-results/hbc-comparison-sct99-final-cpu.json` in the
validation workspace for the complete generated record.

Using Windows process `PeakWorkingSet64`, the paired resource run measured:

| Runtime | Wall time | Peak host working set |
|---|---:|---:|
| BioLang release, CPU | 481.2 s | 16.06 GiB |
| R 4.5.2 / Seurat 5.5.1 | 1548.3 s | 12.50 GiB |

BioLang is 3.22 times faster in that run, but uses 1.28 times more peak host
memory. It therefore does not yet meet the combined target of end-to-end 95%
scientific agreement while being both faster and lower-memory than Seurat.
