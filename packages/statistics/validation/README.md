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
Welch tests with Holm correction. The current suite compares 268 numeric metrics: 264 numerical
equivalences and four deliberately documented differences from R defaults.
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
Normal-Q-Q correlation uses a `1e-9` gate because R and BioLang deliberately
use independent inverse-normal quantile approximations; integer flag counts
must match exactly. Model-fit and distribution-tail tolerances are declared per
metric in `run.ps1` and recorded as absolute limits in the manifest.

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
