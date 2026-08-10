# Single-cell method provenance and parity contract

BioLang's current single-cell implementation is MIT code written from
mathematical and algorithmic descriptions in published papers. Seurat and
SeuratObject are also MIT-licensed; exact-parity work may inspect or port their
covered R/C++ files when the original copyright and licence notice, source
version, and modifications are recorded. Copyleft dependency implementations
are not copied, translated, linked, or required at runtime. External tools may
be used as disposable result oracles; their packages, lock files, generated
code, and outputs are not inputs to the BioLang implementation.

## Implemented correspondence

| BioLang operation | Paper-derived method | Compatibility target | Comparison invariant |
|---|---|---|---|
| `normalize` | library-size normalization followed by `log1p` | scale factor 10,000 | elementwise values |
| `variable_genes` | VST mean-variance trend and standardized variance | 2,000 features | ranking overlap and selected set |
| `scale` | feature centering, sample SD scaling, clipping | clip at 10 | elementwise tolerance |
| `run_pca` | centered block subspace PCA; deterministic 5,000-cell fit and all-cell projection above 5,000 cells | 50 PCs | variance and pairwise geometry; signs may flip |
| `neighbors` | exact k-nearest neighbours for <=4,096 cells; bounded GPU distance/top-k batches when available; deterministic random-projection forest fallback; Jaccard SNN | k=20, first 10 PCs, prune 1/15 | exact edge/weight agreement on small data; recall and partition agreement on large data |
| `cluster_louvain` | multilevel modularity optimization | resolution 0.8 | partition ARI/NMI; labels may permute |
| `run_umap` | fuzzy simplicial graph and stochastic layout over the scalable neighbour index | cosine, 30 neighbours, min.dist 0.3, seed 42 | neighbour preservation after alignment |
| `find_markers` | Wilcoxon rank-sum with per-contrast BH correction | min.pct .01, logFC .1 | statistic direction, p-values, adjusted p-values |
| `sctransform` | SCT v2-style offset negative-binomial Pearson residuals, deterministic 5,000-cell/2,000-gene fit, parameter smoothing, and optional non-regularized cell-covariate regression | residual clipping and per-sample fit | residual correlation, variable-feature overlap |
| `find_integration_anchors` | CountSketch/matrix-free CCA or RPCA, mutual neighbours, quantile-rescaled neighbourhood anchor scores | k.anchor=5, k.score=30 | anchor population/type agreement |
| `integrate_data` | distance- and score-weighted anchor correction | k.weight=100, sd.weight=1 | batch mixing while retaining biological separation |

## Primary method sources

- Stuart et al. (2019), [Comprehensive Integration of Single-Cell Data](https://doi.org/10.1016/j.cell.2019.05.031).
- Hafemeister and Satija (2019), [Normalization and variance stabilization of single-cell RNA-seq data using regularized negative binomial regression](https://doi.org/10.1186/s13059-019-1874-1).
- McInnes, Healy, and Melville (2018), [UMAP: Uniform Manifold Approximation and Projection for Dimension Reduction](https://arxiv.org/abs/1802.03426).
- Dasgupta and Freund (2008), [Random projection trees and low dimensional manifolds](https://doi.org/10.1145/1390156.1390193).
- Blondel et al. (2008), [Fast unfolding of communities in large networks](https://doi.org/10.1088/1742-5468/2008/10/P10008).

## Acceleration and licensing

Native GPU acceleration is implemented with `wgpu` and BioLang-owned WGSL
kernels. The dependency chain is MIT/Apache-2.0; it does not introduce CUDA,
Seurat, R, or GPL code. `bl doctor` reports the selected adapter. Use the global
`--no-gpu` flag or `BIOLANG_GPU=off` for the f64 CPU fallback.

## What “matching” means

Scalar preprocessing results should match within floating-point tolerance.
Eigenvector signs, cluster integer IDs, and two-dimensional layouts are not
identifiable, so equality is assessed using invariant quantities: explained
variance and pairwise geometry for PCA, ARI/NMI for clustering, and aligned
neighbourhood preservation for UMAP. Passing a small synthetic fixture verifies
plumbing, not universal biological equivalence; validation should include
multiple tissues, depths, batch structures, and deliberately unshared cell
types before publication use.
