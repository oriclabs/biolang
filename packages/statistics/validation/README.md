# External R validation

This directory validates scale-sensitive BioLang statistics against an external
base-R process. R is a development oracle only; it is not linked, bundled, or
required by the BioLang runtime or `statistics` package.

From the repository root on Windows:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File packages/statistics/validation/run.ps1 -RequireR
```

If R is installed but is not on `PATH`, provide its executable directly:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File packages/statistics/validation/run.ps1 `
  -RscriptPath "C:\Program Files\R\R-4.5.2\bin\Rscript.exe" -RequireR
```

The runner checks means, quantiles, sample variance/SD, raw MAD, adjusted sample
skewness, log1p summaries, Pearson/Spearman association, regression coefficients,
simple-linear residual MSE, Q-Q alignment, spread/curvature clues, Durbin-Watson,
Cook's distance flags, standardized residual flags, and count-matrix totals. It
also checks Pearson/Spearman screening, Cramer's V, categorical-numeric
eta-squared, and the common screening magnitude. The extended oracle checks
scale-sensitive distribution log-likelihood/AIC values, negative-binomial
dispersion, multivariable coefficients and diagnostics, and omics matrix axis
summaries. It additionally compares Huber regression with R `MASS::rlm`,
weighted moments with explicit R formulas, and ACF/Ljung-Box/trend values with
R's time-series functions. One-way ICC components and the cluster design-effect
approximation are checked with explicit R formulas. Binomial and Poisson GLMs
are checked against `stats::glm`, a random-intercept REML fit against
`nlme::lme`, and a multivariable Breslow Cox model against
`survival::coxph`. Classic inference is also run independently in both
backends: pooled, one-sample, and paired t-tests; independent Mann-Whitney and
paired Wilcoxon signed-rank tests; ANOVA;
Fisher exact; chi-square; Pearson correlation; and BH, Bonferroni, and Holm
adjustments. It now validates explicit Welch inference, mean-difference
confidence intervals, standardized t-test effects, exact and continuity-corrected
rank tests, rank-biserial effects, and labelled Fisher odds-ratio intervals. The
multi-group extension independently checks classical and Welch ANOVA, raw sums
of squares, eta/omega squared, tie-corrected Kruskal-Wallis with epsilon
squared, every Tukey-Kramer contrast and simultaneous interval, and pairwise
Welch tests with Holm correction. Plot geometry adds 2,536 scale-sensitive checks
against base R: explicit histogram edges, counts, densities, closure and
endpoint handling; type-7 and Tukey-hinge box summaries; tied ECDF jumps;
normal Q-Q plotting positions and reference line; and the complete 256-point
Gaussian violin/KDE grid. Box, ECDF, Q-Q and KDE geometry are all repeated on
the real, skewed `airquality` ozone fixture. Ordinary least-squares fitted
values, confidence bands and wider prediction bands are checked on ozone by
month against both `predict.lm` and statsmodels. Clinical geometry is checked
independently too: Kaplan-Meier time, risk, event and censor sets, product-limit
survival and Greenwood standard errors against `survival::survfit`; tied-score
ROC AUC against an explicit Mann-Whitney calculation; and frozen forest-plot
estimates, confidence limits and weights against base-R fixtures. This gives
independent genomic gates too: cumulative Manhattan offsets and p-value
transforms; genetic Q-Q plotting positions, exact beta order-statistic bounds
and λGC; and stable within-chromosome rainfall distances with explicit
duplicate-position floors. Cytoband order and chromosome lengths, CNV genomic
segment bounds and threshold states, half-open coverage overlap/clipping,
regional feature lanes, lollipop domains and count-scaled splice arcs are also
checked independently. Circular-genome validation independently checks
chromosome-length-weighted angles, line/CNV track radii, interval endpoints,
ribbon angles and count-scaled link widths. This gives 2,804 R/BioLang metrics: 2,800 intended
numerical equivalences and four documented convention differences. When Python,
NumPy and statsmodels are available, 2,160 further geometry checks run
independently, for 4,964 metrics in total.
Finite edge cases include a three-observation t-test, tied ranks, zero/one and
tied p-values, a zero-heavy count distribution, and an extreme regression
outlier.

The default run performs three interleaved repetitions. It reports median
elapsed time and maximum sampled working set rather than treating one noisy run
as a benchmark. Override this when a quick correctness-only run is useful:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File packages/statistics/validation/run.ps1 `
  -BenchmarkRepeats 1 -RequireR
```

The runner writes three ignored artifacts under `validation/results/`:

- `manifest.json`: versions, raw repetition measurements, every tolerance and
  classification, and group-level slope/intercept/RMSE/error percentiles;
- `checks.csv`: one row per numeric comparison for filtering or plotting;
- `report.md`: a compact human-readable result, timing, memory, and accuracy
  summary.

Correlation is not used as a parity gate. Scale-sensitive absolute and relative
errors remain attached to each metric, and group summaries expose proportional
or offset bias. Relative error is omitted for references whose magnitude is at
or below `1e-12`, where an absolute error is the meaningful quantity.

Arithmetic, geometric, harmonic, trimmed, and root-mean-square calculations
used by `stat.means()` are also compared with independent R expressions.

## Frozen conformance constants

The oracle above needs R and is run by hand, so it cannot fail a refactor.
`model_conformance.R` closes that gap for the four model fitters. It prints R's
answers for fixtures small enough to inline in a Rust test, which are frozen in
`crates/bl-runtime/tests/stats_model_conformance.rs` and therefore run under an
ordinary `cargo test`, on any machine, without R and without redistributing an R
dataset.

```powershell
Rscript packages/statistics/validation/model_conformance.R
```

Two definitional differences from R are documented in that test rather than
absorbed into loose tolerances. `hatvalues()` reads the QR that `glm.fit` built
during the final IRLS iteration, so its weights belong to the previous
coefficient vector; BioLang recomputes the hat matrix at the converged
coefficients, and the frozen leverage constants are R's hat recomputed the same
way. Separately, `glm.fit` stops when the deviance stops changing while this
IRLS stops when the coefficients stop changing, so the two fits settle at
slightly different points.

The same run generates and checks four real-data fixtures from R-distributed
datasets: non-missing `airquality` ozone observations (including equal-month
analysis weights and a log-scale preview), the ordered `Nile` flow series,
repeated measurements from the Diet 1 arm of `ChickWeight`, and censored
follow-up from `survival::lung`. The CSV files and both implementations'
outputs remain under the ignored `validation/results/` directory. They are
oracle inputs, not bundled runtime dependencies or project datasets.

Scalar arithmetic generally uses a `1e-12 * max(1, abs(reference))` gate.
Normal-Q-Q model diagnostics use a `1e-9` gate. The renderer-neutral Q-Q
geometry applies two Newton refinements to its independent inverse-normal
approximation and is checked against both R and Python at the same strict gate;
integer membership and count fields must match exactly. Model-fit and
distribution-tail tolerances are declared per metric in `run.ps1` and recorded
as absolute limits in the manifest.

## Renderer-neutral plot data

`plot_spec(table, options)` returns a `biolang.plot.spec/v1` Record. Scatter,
line, error-bar and confidence-band calls to `plot()` render this same object.
`render_plot(spec, {format: ...})` supports SVG, portable ASCII, Unicode
Braille, and standalone HTML containing the original SVG plus an optional
Canvas/PNG fallback. The default remains an SVG `Str`, preserving redirected
CLI output and existing notebook compatibility.

`boxplot_data`, `ecdf_data`, `normal_qq_data`, `violin_data`, `linear_fit_data`,
`histogram_data`, `categorical_data`, and `missingness_data` return
`biolang.plot.geometry/v1` Records/Tables with stable rows, method labels and
exclusions. These values can be written to JSON/CSV for validation; renderers
do not need to infer the statistics from pixels. Categorical geometry retains
first-observed order. Missingness geometry reports counts from the full table
while exposing a separately bounded, deterministic display grid.

`linear_fit_data(x, y, {confidence: 0.95, at: [...]})` keeps uncertainty
explicit: `confidence_lower/upper` describe uncertainty in the fitted mean,
while `prediction_lower/upper` describe a new observation and are therefore
wider. `stat.relationship_plot()` consumes the same geometry when its
`interval` option is `"confidence"` or `"prediction"`.

The four `expected_convention_difference` rows are not loose passes. BioLang's
pooled t-test is separately compared with `t.test(var.equal = TRUE)`, its
normal-approximation rank test with `wilcox.test(exact = FALSE, correct =
FALSE)`, and its sample odds ratio with the explicit cross-product ratio. The
R-default Welch degrees of freedom/p-value, exact small-sample Wilcoxon p-value,
and Fisher conditional-MLE odds ratio must remain different; if they become
equal, the run fails with `convention_changed_review_required` so the API and
documentation are reviewed together.

Exit code `2` means R was unavailable and validation was not run. It must never
be presented as a pass. `-RequireR` converts that condition into a terminating
error for CI.

## Dense plot benchmark

`plot_benchmark.ps1` builds the dedicated release-mode probe and compares the
vector and embedded-raster point layers at 1,000, 5,000, 20,000 and 100,000
points. It records render time, SVG bytes, element count and sampled peak
working set in `plot-benchmark.json`:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File `
  packages/statistics/validation/plot_benchmark.ps1
```

The checked result is machine-specific evidence, not a universal speed claim.
On the recorded Windows x64 run, rasterising 5,000 points was slower and made a
larger file. At 20,000 it reduced approximately 20,082 SVG elements to 83 and
1.38 MB to 0.45 MB, so `umap_plot` now switches at 20,000 points. Rasterisation
is intended to bound browser/DOM and encoded-output cost; it is not claimed to
make figure generation faster. Users can set `raster: "auto"`, `"on"`, or
`"off"`, adjust `raster_threshold`, and choose `raster_scale` from 1 through 4.

### Point thinning

`manhattan` accepts `thin: true`, which keeps at most one variant per device
pixel and drops the rest, choosing the survivor by significance so the strongest
signal in each pixel is the one that is drawn. It is off by default. When it
removes anything, the figure says so in both a visible note and the SVG
`<desc>`, because a reader has no other way to know a figure is not showing
every variant.

Measured with `plot_dense_benchmark` on the same Windows x64 machine, using a
worst case in which many variants share a position:

| n | raster | thin | SVG bytes | median ms |
|---|---|---|---|---|
| 60,000 | off | off | 4,109,860 | 51 |
| 60,000 | off | on | 1,554,368 | 42 |
| 60,000 | on | off | 1,329,727 | 295 |
| 60,000 | on | on | 1,349,326 | 213 |
| 500,000 | off | off | 34,229,736 | 424 |
| 500,000 | off | on | 7,273,573 | 316 |
| 500,000 | on | off | 1,426,891 | 2,886 |
| 500,000 | on | on | 1,488,970 | 948 |

Read that as two separate effects rather than one. On the vector path thinning
is a size measure: 4.7x smaller at 500,000 variants. On the raster path it is a
time measure and **not** a size measure -- 3x faster at 500,000 variants, while
the PNG grows about 4%. Removing overdraw leaves isolated anti-aliased discs
where there had been saturated blobs, and the blobs compressed better.

The cost is what thinning removes. Comparing the rendered raster layers at
60,000 variants with thinning off and on, 54 of 687,514 painted pixels were
lost (0.008%, all anti-aliased disc edges) and none were gained; where both
were painted, the median pixel did not change opacity at all, the 95th
percentile moved by 3 of 255, and the densest pileups moved by 138. So the
outline of the figure survives and its shading does not: density stops being
readable as shade. That is why it is opt-in and why the figure discloses it.

For comparison, `raster_scale` is the blunter instrument for the same problem
and needs no such disclaimer, since it changes only resolution:

| plot | scale 1 | scale 2 (default) | scale 3 |
|---|---|---|---|
| manhattan | 419,935 | 1,329,727 | 2,423,171 |
| pca_plot | 345,013 | 910,173 | 1,514,861 |
| volcano_plot | 164,730 | 489,082 | 902,686 |

Indexed-colour PNG was measured and rejected. It needs 256 or fewer distinct
pixel values, and these layers carry 1,452 (`pca_plot`) to 29,914
(`manhattan`), because anti-aliased points at alpha 0.7 overlapping each other
produce a continuum of blends. Only `qq_plot` (12 values) and `rainfall` (33)
would qualify, and both already encode to about 13 KB. Lossless recompression
at zlib level 9 was also measured: 1% to 10% on the large layers, which does
not justify replacing `tiny_skia::Pixmap::encode_png`.
