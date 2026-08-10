# SCTransform v2 clean-room algorithm audit

Status: 2026-08-10

BioLang's implementation is MIT-licensed and independently implemented in
Rust. No GPL `sctransform` or `glmGamPoi` implementation source was copied,
translated, linked, or made a BioLang dependency. Separately installed R
packages are used only as black-box numeric oracles by validation scripts
outside BioLang. This is an engineering record, not legal advice.

## Result

The paper- and public-documentation-defined SCTransform v2 numerical contract
is implemented, and the controlled synthetic fixture passes both correlation
and scale-sensitive conformance gates. The realistic HBC control does **not**
yet pass the calibrated contract. In particular, `log10(theta)` correlates at
0.999216 while raw theta has regression slope 0.9389 and median relative error
7.26%. Correlation describes the shape of that curve; it does not establish
scale agreement.

No general "99% parity" claim follows from these results. End-to-end
integration, PCA, neighbours, clustering, UMAP, and marker testing are separate
algorithms and have their own measured gaps.

## Implemented contract

| Stage | BioLang implementation | Status |
|---|---|---|
| Count model | Offset negative-binomial model, `mu_cg = total_c * exp(intercept_g)` | Matched |
| Gene abundance | Sparse equivalent of `exp(mean(log1p(count))) - 1` | Matched |
| Feature filter | Remove genes detected in fewer than five cells | Matched |
| Fit cells | 5,000-cell Seurat profile, fixed seed 1448145; independently implemented R-compatible MT19937/rejection sampling | Exact HBC subset |
| Fit genes | 2,000 genes, inverse-density weighted sequential sample without replacement | Same documented law; exact subset under-specified |
| Initial fit | Offset NB with Cox-Reid adjusted profile likelihood | Matched statistical objective |
| Poisson exclusion | Exclude mean `< 0.001` or sample variance `<= mean`; theta is infinite | Matched |
| Parameter outliers | Failed/non-finite non-Poisson fits are excluded before smoothing | Matches characterized fixtures; finite-score edge cases remain under-specified |
| Theta target | Smooth `log10(1 + geometric_mean/theta)` and map back | Matched current `od_factor` default |
| Bandwidth | Independent Sheather-Jones solve-the-equation implementation, multiplied by 3 | Matched public `bw.SJ` observations |
| Smoother | Gaussian Nadaraya-Watson regression using R `ksmooth`'s quartile-defined bandwidth scale | Matched public `ksmooth` observations |
| Pearson residual | `(count - mu) / sqrt(mu + mu^2/theta)` | Matched |
| v2 variance floor | At least `(median(non-zero UMI) / 5)^2` | Matched |
| Seurat clip | `+/-sqrt(n_cells / 30)` before centring | Matched |
| Residual variance | Clipped sample variance with denominator `n_cells - 1` | Matched |
| Seurat scale-data | Centre residuals, do not scale to unit variance | Matched |
| Variable features | Rank by residual variance; wrapper retains 3,000 | Matched structure |
| Covariates | Optional second-stage residual regression | Matches the supported Seurat `vars.to.regress` profile, not general `vst` latent/batch models |
| Corrected UMI | Not returned by the core Rust result | Not implemented; not needed by the current HBC residual workflow |

## Measured standalone conformance

The oracle was `sctransform 0.4.3` with `glmGamPoi 1.22.0`, v2 offset mode,
Seurat's 5,000-cell cap and residual clip, 2,000 fit genes, `od_factor`,
`umi_median`, and bandwidth adjustment 3.

| Metric | Synthetic 480 x 120 | HBC control 14,847 x 14,065 |
|---|---:|---:|
| Modelled gene-set agreement | 116/116 (100%) | 13,799/13,799 (100%) |
| Fit-cell subset agreement | all cells | 5,000/5,000 (100%) |
| `log10(theta)` Pearson | 99.9980% | 99.9216% |
| Raw theta regression slope | 1.0173 | **0.9389** |
| Raw theta relative error, median / p90 / max | 3.36% / 3.94% / 4.06% | **7.26% / 12.52% / 53.99%** |
| Intercept Pearson | 99.9945% | 99.9620% |
| Intercept slope / offset / RMSE | 0.9972 / -0.0105 / 0.0194 | 0.9891 / -0.0921 / 0.0650 |
| Residual-variance Pearson | 99.9992% | 99.8771% |
| Residual-variance Spearman | 99.9946% | 99.9816% |
| Residual-variance slope | 1.0032 | **1.0269** |
| Residual probe | all 116 genes x 64 cells | top 3,000 features x 64 cells; 2,946 genes shared |
| Residual-variance range covered by oracle probe | 0.102-1.477 (full range) | 0.886-69.719 (includes full maximum) |
| Joined residual observations | 7,424 | 188,544 |
| Residual Pearson / slope | 99.9998% / 1.0004 | 99.9824% / 0.9953 |
| Residual RMSE / oracle residual SD | 0.20% | 1.92% |
| Median relative error where `abs(residual) > 1` | 0.07% | 1.07% |
| Median per-gene residual correlation | 100.0000% | 99.9997% |
| Top-feature overlap | 50/50 (100%) | 2,946/3,000 (98.20%) |
| Fit-gene subset agreement | 96 oracle genes all shared; BioLang fit 116 | 510/2,000 (25.50%) |

The HBC manifests now record the timed transform on both sides: 3.475 seconds
for BioLang release CPU and 39.890 seconds for the R oracle, a ratio of 11.48.
The BioLang CLI took 26.5 seconds including construction and serialization of
the 188,544-row validation CSV. These are same-host observations from separate
processes, not a statistically controlled benchmark, and are not used as an
accuracy claim.

The comparator retains correlation gates and adds scale-sensitive gates:
regression slopes must be in `[0.98, 1.02]`; raw-theta median and p90 relative
errors must be at most 5% and 10%; intercept RMSE must be at most 0.10; residual
RMSE must be at most 2% of the oracle residual SD; and the median relative error
for oracle residuals with absolute value greater than one must be at most 2%.
It also requires the residual probe to cover at least 95% of the requested
top-feature set. HBC currently fails the three raw-theta gates and the
residual-variance slope gate. The failure is intentional evidence, not hidden
behind the high correlations.

Generated evidence is ignored under `validation-results/`. The relevant final
records are:

- `sctransform-comparison-synthetic-v8.json`;
- `hbc-sctransform-comparison-ctrl-v6.json`;
- `hbc-sctransform-oracle-ctrl-v3/manifest.csv`;
- `hbc-sctransform-biolang-ctrl-v6/manifest.csv`.

## Under-specified boundary

The papers specify inverse density sampling, but not the density grid,
interpolation, tie handling, or all random draws surrounding gene selection.
The current R package exposes the chosen genes, not those mechanics. BioLang
therefore uses an independent Gaussian density estimate and the public R
sequential weighted-sampling contract. It does not embed oracle-selected gene
names or consult R at runtime.

This boundary explains why the realistic parameter curves are highly
correlated while their calibration and the exact 2,000-gene subset differ.
Claiming an exact subset would require either a complete public specification
or copying implementation details, which is outside this clean-room boundary.

Other incomplete surfaces are corrected UMI reconstruction and general
latent/batch/non-regularized model matrices. They should be implemented only
when a BioLang API needs them and validated as separate profiles.

## Reproduction

The validation-only files are:

- `sctransform_oracle.R`: R black-box oracle and deterministic fixtures;
- `prepare_hbc_sctransform_fixture.R`: public-Matrix-only HBC QC fixture;
- `sctransform_biolang.bl`: standalone BioLang exporter;
- `compare_sctransform_results.py`: dependency-light numeric comparator.

Run the R oracle and BioLang exporter in separate processes against the same
fresh MEX directory, then pass their output directories to the comparator. The
GPL packages are not required to build, test, distribute, or use BioLang.

## Public references

- Hafemeister C, Satija R (2019), [Normalization and variance stabilization of
  single-cell RNA-seq data using regularized negative binomial regression](https://genomebiology.biomedcentral.com/counter/pdf/10.1186/s13059-019-1874-1.pdf).
- Choudhary S, Satija R (2022), [Comparison and evaluation of statistical error
  models for scRNA-seq](https://genomebiology.biomedcentral.com/track/pdf/10.1186/s13059-021-02584-9).
- [Public `sctransform` reference manual](https://cran.r-universe.dev/sctransform/doc/manual.html).
- [Public Seurat `SCTransform` reference](https://satijalab.org/seurat/reference/sctransform).
- [Public glmGamPoi reference manual](https://bioconductor.org/packages/devel/bioc/manuals/glmGamPoi/man/glmGamPoi.pdf).
- [R `ksmooth` bandwidth contract](https://stat.ethz.ch/R-manual/R-devel/RHOME/library/stats/html/ksmooth.html).
- [R weighted sampling contract](https://www.stat.ethz.ch/R-manual/R-devel/library/base/html/sample.html).
- Sheather SJ, Jones MC (1991), [A reliable data-based bandwidth selection method](https://academic.oup.com/jrsssb/article/53/3/683/7028194).
