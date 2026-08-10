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
