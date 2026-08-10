# Normalization and integration

Library-size normalization is useful for visualization and marker testing, but
clustering needs a representation whose variance is less dominated by depth.
BioLang fits each sample independently with its paper-derived SCT v2-style
offset negative-binomial model. A deterministic sample of 5,000 cells and
2,000 expressed genes fits the regularized parameter trends. Residual variance
is streamed across every nonzero gene, but only the top 5,000 residual columns
per sample are materialized. Mitochondrial percentage is removed in a second,
non-regularized cell-level regression. These settings mirror the
[HBC SCT normalization lesson](https://hbctraining.github.io/Intro-to-scRNAseq-Quarto/lessons/07_SCT_normalization.html).

```biolang
let ctrl_sct = sc.sctransform(ctrl_filtered, 5000, ctrl_mito)
let stim_sct = sc.sctransform(stim_filtered, 5000, stim_mito)
let features = sc.select_integration_features(ctrl_sct, stim_sct, 3000)
```

Within the bounded-memory workflow, cross-sample candidates must exist in both
5,000-column residual matrices and are ordered by median within-sample rank.
This is deliberately stricter than Seurat's `PrepSCTIntegration`, which can
recompute residuals missing from one sample. The capped shared-axis selection
contained 2,596 of the same 3,000 genes as the HBC-prescribed Seurat oracle
(86.53 percent).

## Anchor integration

This stage follows the HBC lesson's 3,000-feature, CCA-anchor
[integration path](https://hbctraining.github.io/Intro-to-scRNAseq/lessons/09_integration_code_harmony.html).

The scaled residual matrices enter a matrix-free CCA. CountSketch bounds the
feature width and block subspace iteration avoids the quadratic cell-by-cell
cross-product. A compatible GPU performs those block products; CPU f64 remains
the fallback. Mutual cross-sample neighbors become anchors, shared-neighbor
scores are rescaled by their 1st and 90th percentiles, and local Gaussian
weights correct the query sample.

```biolang
let anchors = sc.find_integration_anchors(
    ctrl_sct, stim_sct, "cca", 3000, 30, 5, 30)
let corrected = sc.integrate_data(anchors, 100, 1.0)
```

The HBC call leaves anchor dimensions at Seurat's 30-dimension default, then
computes enough integrated PCs to use dimensions 1 through 40 downstream.
BioLang previously tied those two choices together; the validation run follows
the corrected 30-anchor/50-PCA implementation. It found 46,484 anchors and
recorded `countsketch_subspace_gpu`. The CPU backend was not rerun after this
correction, so no current CPU comparison is claimed. Corrected coordinates are
used only for neighbors, clusters, and visualization; raw counts and
log-normalized RNA remain available for biological measurement.

This follows the same published method family and analysis choices as HBC, but
the sketch, independent optimizer, and anchor correction do not reproduce
Seurat's numerical state. The approximate integrated-PC 15-neighbor Jaccard
against the oracle is 0.0648 on the measured GPU run, so the remaining
difference is material and explicitly reported rather than hidden behind
matching API names.
