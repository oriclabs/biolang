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
`survival::coxph`. The current suite contains 147 scale-sensitive checks. It
records versions, per-backend elapsed time,
scale-sensitive differences, tolerances, and the final result in
`validation/results/manifest.json`.

Arithmetic, geometric, harmonic, trimmed, and root-mean-square calculations
used by `stat.means()` are also compared with independent R expressions.

The same run generates and checks four real-data fixtures from R-distributed
datasets: non-missing `airquality` ozone observations (including equal-month
analysis weights and a log-scale preview), the ordered `Nile` flow series,
repeated measurements from the Diet 1 arm of `ChickWeight`, and censored
follow-up from `survival::lung`. The CSV files and both implementations'
outputs remain under the ignored `validation/results/` directory. They are
oracle inputs, not bundled runtime dependencies or project datasets.

Scalar arithmetic uses a `1e-12` relative gate. Normal-Q-Q correlation uses a
`1e-9` gate because R and BioLang deliberately use independent inverse-normal
quantile approximations; integer flag counts must match exactly.

Exit code `2` means R was unavailable and validation was not run. It must never
be presented as a pass. `-RequireR` converts that condition into a terminating
error for CI.
