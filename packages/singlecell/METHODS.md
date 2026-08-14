# Single-cell method provenance and parity contract

BioLang's current single-cell implementation is MIT code written from
mathematical and algorithmic descriptions in published papers. Seurat and
SeuratObject are also MIT-licensed; exact-parity work may inspect or port their
covered R/C++ files when the original copyright and licence notice, source
version, and modifications are recorded. Copyleft dependency implementations
are not copied, translated, or linked. An optional separately installed GPL
executable may be invoked as a process; BioLang's native implementation remains
available without it. External oracle packages, lock files, generated code,
and outputs are not implementation inputs.

## Implemented correspondence

| BioLang operation | Paper-derived method | Compatibility target | Comparison invariant |
|---|---|---|---|
| `normalize` | library-size normalization followed by `log1p` | scale factor 10,000 | elementwise values |
| `variable_genes` | VST mean-variance trend and standardized variance | 2,000 features | ranking overlap and selected set |
| `scale` | feature centering, sample SD scaling, clipping | clip at 10 | elementwise tolerance |
| `run_pca` | centered block subspace PCA; deterministic 5,000-cell fit and all-cell projection above 5,000 cells | 50 PCs | variance and pairwise geometry; signs may flip |
| `neighbors` | exact search for <=4,096 cells; native Spotify Annoy 1.17.3 50-tree Euclidean search; GPU/projection fallback for browser or unsupported metrics; Jaccard SNN | k=20, first 10 PCs, prune 1/15 | exact Seurat edge, weight, and partition agreement on fixed HBC PCs |
| `cluster_louvain` | Seurat 5.5.1 MIT Modularity Optimizer 1.3.0 path adapted to Rust, including Java-compatible RNG, 10 starts, and 10 repeated multilevel iterations | algorithm=1, resolution 0.8 | exact partition on an identical SNN graph; labels ordered by cluster size |
| `run_umap` | fuzzy simplicial graph and stochastic layout over the scalable neighbour index | cosine, 30 neighbours, min.dist 0.3, seed 42 | neighbour preservation after alignment |
| `find_markers` | Wilcoxon rank-sum with per-contrast BH correction | min.pct .01, logFC .1 | statistic direction, p-values, adjusted p-values |
| `sctransform` | SCT v2-style offset negative-binomial Pearson residuals, deterministic 5,000-cell/2,000-gene fit, parameter smoothing, and optional non-regularized cell-covariate regression | residual clipping and per-sample fit | residual correlation, variable-feature overlap |
| `find_integration_anchors` | Seurat 5.5.1 MIT anchor path adapted to Rust: bounded matrix-free CCA (32 guard vectors and 12 block-power passes; all features through 3,000; CountSketch above that), 50-tree Spotify Annoy cross-search, mutual neighbours, high-dimensional filter, four-neighbour score, quantile rescaling | k.anchor=5, k.filter=200, k.score=30, max.features=200 | candidate/retained anchor identities, scores, and filter decisions; exact on fixed full-precision embeddings |
| `integrate_data` | Seurat 5.5.1 MIT integration-vector direction and `FindWeightsC` kernel adapted to Rust | k.weight=100, sd.weight=1 | batch mixing while retaining biological separation |

## Primary method sources

- Stuart et al. (2019), [Comprehensive Integration of Single-Cell Data](https://doi.org/10.1016/j.cell.2019.05.031).
- Hafemeister and Satija (2019), [Normalization and variance stabilization of single-cell RNA-seq data using regularized negative binomial regression](https://doi.org/10.1186/s13059-019-1874-1).
- McInnes, Healy, and Melville (2018), [UMAP: Uniform Manifold Approximation and Projection for Dimension Reduction](https://arxiv.org/abs/1802.03426).
- Dasgupta and Freund (2008), [Random projection trees and low dimensional manifolds](https://doi.org/10.1145/1390156.1390193).
- Blondel et al. (2008), [Fast unfolding of communities in large networks](https://doi.org/10.1088/1742-5468/2008/10/P10008).

## Strict numeric replay

The default integration path is fully native and MIT-compatible. Iterative CCA
and PCA decompositions can nevertheless stop at slightly different floating-
point boundaries across numerical implementations. Since a handful of changed
anchors can move an unstable community boundary, the package also accepts
plain numeric artifacts from an independent provider:

- `find_integration_anchors(..., compatibility=record)` accepts paired
  `left_embedding`/`right_embedding` matrices and optional `filter_features`;
- `integrate_data(..., compatibility=record)` accepts an exact
  `weight_reduction` and `pca` solver options;
- `sc_pca(..., {solver: "lanczos", initial: values, ...})` provides a direct-
  matrix restarted-Lanczos path without changing the faster default solver.

These fields are a file/process interchange contract, not a link to R or a
particular licensed package. The producer is installed, licensed, and executed
separately. BioLang validates dimensions and continues to perform anchor
search/filter/score, correction, SNN construction, and Louvain clustering.
Supplying an external artifact must be disclosed in reproducibility metadata;
it is not evidence that the native decomposition is bit-for-bit identical.

When the separately licensed `bl-seurat-provider` executable is installed, the
complete strict path is available without manually loading artifacts:

```biolang
let anchors = sc.find_integration_anchors(
    control, stimulated, compatibility: "external"
)
let integrated = sc.integrate_data(anchors)
```

Set `BIOLANG_SEURAT_PROVIDER` when the executable is not on `PATH`. The anchor
record stores the selected provider and weighting reduction; `integrate_data`
then invokes the same provider for PCA. The run's `compute_backend` and PCA
`compute_method` disclose the external process. The CLI also prints each
external CCA/PCA invocation, and `external_provider_manifest` records the
protocol and pinned provider/package versions. This mode is unavailable in
WebAssembly and produces a clear error directing the user to the native CLI.

## Acceleration and licensing

Native GPU acceleration is implemented with `wgpu` and BioLang-owned WGSL
kernels. The dependency chain is MIT/Apache-2.0; it does not introduce CUDA,
Seurat, R, or GPL code. `bl doctor` reports the selected adapter. Use the global
`--no-gpu` flag or `BIOLANG_GPU=off` for the f64 CPU fallback.

The exact Seurat source version, archive hash, consulted files, modifications,
and upstream MIT text are recorded in
[`SEURAT_MIT_NOTICE.md`](./SEURAT_MIT_NOTICE.md). Seurat's GPL-family
dependencies are not implementation sources. Native Euclidean kNN uses the
Apache-2.0 Spotify Annoy core directly through BioLang's MIT bridge; it does not
include the GPL RcppAnnoy wrapper. GPU/projection search remains available for
the browser and unsupported metrics.

## What “matching” means

Scalar preprocessing results should match within floating-point tolerance.
Eigenvector signs, cluster integer IDs, and two-dimensional layouts are not
identifiable, so equality is assessed using invariant quantities: explained
variance and pairwise geometry for PCA, ARI/NMI for clustering, and aligned
neighbourhood preservation for UMAP. Passing a small synthetic fixture verifies
plumbing, not universal biological equivalence; validation should include
multiple tissues, depths, batch structures, and deliberately unshared cell
types before publication use.
