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
and scale-sensitive conformance gates. Neither real HBC condition passes every
calibrated gate. On control cells, `log10(theta)` correlates at 0.999216 while
regularized theta on its original scale has regression slope 0.9389 and median
relative error 7.26%. The newly separated unregularized estimator is much
closer: its median BioLang/oracle theta ratio is 1.0066. Correlation describes
the shape of a curve; it does not establish scale agreement.

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
| Regularized theta, original-scale regression slope | 1.0173 | **0.9389** |
| Regularized theta relative error, median / p90 / max | 3.36% / 3.94% / 4.06% | **7.26% / 12.52% / 53.99%** |
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

### Real-data replication across biological conditions

The HBC control and interferon-stimulated PBMC matrices are independently
filtered real 10x datasets from the course workflow. The same oracle and
BioLang profiles were run separately on each condition.

| Metric | HBC control | HBC stimulated |
|---|---:|---:|
| Cells / input genes | 14,847 / 14,065 | 14,782 / 14,065 |
| Modelled gene-set agreement | 13,799/13,799 (100%) | 13,695/13,695 (100%) |
| Shared finite raw-fit genes | 473 | 476 |
| Unregularized theta median BioLang/oracle ratio | 1.0066 | 1.0101 |
| Unregularized theta relative error, median / p90 | 0.66% / 4.24% | 1.01% / 6.60% |
| Unregularized theta regression slope | 1.0164 | 1.0991 |
| Regularized theta slope | 0.9389 | 1.0249 |
| Regularized theta relative error, median / p90 | 7.26% / 12.52% | 3.91% / 11.09% |
| `od_factor` absolute difference, median / p90 | 0.00291 / 0.00940 | 0.00197 / 0.00444 |
| Residual-variance slope | 1.0269 | 1.0482 |
| Top-3,000 feature overlap | 98.20% | 97.77% |
| Full feature-rank Spearman | 99.9816% | 99.9769% |
| Shared top-feature rank Spearman | 98.8250% | 98.5370% |
| Top-feature residual Pearson / slope | 0.999824 / 0.9953 | 0.999630 / 1.0101 |
| Residual RMSE / oracle residual SD | 1.92% | **2.93%** |
| Median relative error where `abs(residual) > 1` | 1.07% | 1.08% |
| Theta-to-residual median-error attenuation | 7.05% | 14.48% |
| BioLang / R transform seconds | 2.827 / 39.890 | 2.683 / 80.980 |
| All calibrated gates pass | **No** | **No** |

The unregularized median agreement supports locating most of the control theta
shift after the initial per-gene fit. It does not make the discrepancy vanish:
stimulated cells still have an unregularized-theta slope of 1.0991, and their
top-feature residual RMSE misses the 2% gate. The `od_factor` comparison is a
useful view of the smoothed target, but it is algebraically derived from the
same regularized theta values and must not be presented as independent proof.

The manifests record the transform itself. Total BioLang CLI times, including
construction and serialization of the validation CSV, were 16.16 seconds for
control and 14.73 seconds for stimulated cells. These are same-host
observations from separate processes, not a statistically controlled benchmark,
and are not used as an accuracy claim.

The new paired runner measures the complete process tree, including the R
worker launched by `Rscript`, and adds resource targets to the same report:

| Condition | BioLang wall / peak | R oracle wall / peak | R/BioLang ratio, time / memory |
|---|---:|---:|---:|
| HBC control | 15.60 s / 2.29 GiB | 39.36 s / 5.53 GiB | 2.52x / 2.41x |
| HBC stimulated | 15.36 s / 2.28 GiB | 37.49 s / 5.45 GiB | 2.44x / 2.40x |

Both resource gates pass on both conditions. These measurements include input
loading and validation-output serialization, use CPU BioLang, and are paired
same-host observations rather than a multi-replicate benchmark. The generated
records are `sctransform-auto-hbc-ctrl-20260810/comparison.json` and
`sctransform-auto-hbc-stim-20260810/comparison.json`.

The comparator retains correlation gates and adds scale-sensitive gates:
regression slopes must be in `[0.98, 1.02]`; regularized-theta median and p90
relative errors must be at most 5% and 10%; intercept RMSE must be at most 0.10; residual
RMSE must be at most 2% of the oracle residual SD; and the median relative error
for oracle residuals with absolute value greater than one must be at most 2%.
It also requires the residual probe to cover at least 95% of the requested
top-feature set. The raw-fit diagnostic additionally requires a median
BioLang/oracle theta ratio in `[0.95, 1.05]` and median relative error at most
5%. Control currently fails three regularized-theta gates and the
residual-variance slope gate. Stimulated cells fail the regularized-theta slope
and p90 gates, the residual-variance slope gate, and normalized residual RMSE.
These failures are intentional evidence, not hidden behind high correlations.

Generated evidence is ignored under `validation-results/`. The relevant final
records are:

- `sctransform-comparison-synthetic-v9.json`;
- `hbc-sctransform-comparison-ctrl-v8.json`;
- `hbc-sctransform-comparison-stim-v1.json`;
- `hbc-sctransform-oracle-ctrl-v3/manifest.csv`;
- `hbc-sctransform-oracle-stim-v1/manifest.csv`;
- `hbc-sctransform-biolang-ctrl-v7/manifest.csv`;
- `hbc-sctransform-biolang-stim-v1/manifest.csv`.

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

## Follow-up literature and disposition

The papers below cite, refine, or directly test the 2019 method. They do not
justify silently changing the Seurat-compatible profile; each proposed change
would be an explicit alternative with its own real-data validation.

- Choudhary and Satija (2022) is the direct SCTransform v2 refinement. Its
  fixed-slope offset model, Poisson-gene exclusion, `od_factor`
  regularization, and lower variance bound define BioLang's current v2
  profile: [Genome Biology 23, 27](https://doi.org/10.1186/s13059-021-02584-9).
- Ahlmann-Eltze and Huber (2020) showed faster, accurate Gamma-Poisson fitting
  that exploits sparse small counts and can operate on disk. The R oracle
  records this backend, while BioLang may independently adopt the published
  sparse numerical ideas—not its GPL implementation:
  [Bioinformatics 36, 5701-5702](https://doi.org/10.1093/bioinformatics/btaa1009).
- Lause, Berens, and Kobak (2021) argued that a parsimonious offset model with
  a shared `theta` near 100 produces analytic Pearson residuals similar to
  smoothed SCTransform residuals. This is a strong candidate for a fast,
  low-memory `analytic` profile, but it is not an exact replacement for the v2
  oracle: [Genome Biology 22, 258](https://doi.org/10.1186/s13059-021-02451-7).
- Townes et al. (2019) derived multinomial deviance feature selection and
  GLM-PCA. Direct count-space dimension reduction could avoid materializing a
  residual matrix, so it is relevant to the memory target, but would produce a
  different representation and ranking:
  [Genome Biology 20, 295](https://doi.org/10.1186/s13059-019-1861-6).
- Ahlmann-Eltze and Huber (2023) benchmarked 22 transformations and found no
  consistent downstream advantage for a sophisticated transformation over a
  shifted logarithm followed by suitably dimensioned PCA. This means parity
  metrics must be paired with neighbour, clustering, and marker benchmarks
  before claiming a scientific improvement:
  [Nature Methods 20, 665-672](https://doi.org/10.1038/s41592-023-01814-1).
- Cho et al. (2024) found that feature-selection conclusions depend on the
  evaluation criterion and reported strong results for high-deviation and
  high-expression selections. These should be compared as optional rankings
  using ground-truth clustering—not folded into the Seurat ranking:
  [Briefings in Bioinformatics 25, bbae317](https://doi.org/10.1093/bib/bbae317).

The practical next experiment is therefore two-track: keep tightening the v2
black-box contract, and separately benchmark analytic Pearson residuals plus
deviance-based feature selection for speed, peak memory, neighbour recovery,
clustering, and marker stability. A faster alternative is useful only when its
different scientific contract is named and measured.

## Reproduction

The validation-only files are:

- `run_sctransform_validation.py`: one-command, separate-process runner with
  whole-process timing and peak-memory measurement;
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
