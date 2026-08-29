# BioLang statistics

The `statistics` package combines statistical tests with explainable exploratory
analysis. Exploration functions return ordinary BioLang records: every number,
clue, alternative, limitation, and recommendation can be inspected or reused.

Release archives already include this package. If `bl` was installed by Cargo
or copied without the companion `packages` directory, install it once with
`bl install statistics`; it is then available from any working directory
without setting `BIOLANG_PATH`.

```biolang
import "statistics" as stat

let values = [12.1, 12.4, 12.8, 13.0, 13.2, 13.5, 29.0]
let report = stat.explore(values, {name: "protein concentration"})

# One command chooses a notebook/browser visual or terminal-safe rendering.
stat.show(report, {detail: "learning", format: "auto"})
```

## Start with the question, not a test name

The task-first helpers are the shortest safe route for routine analyses. They
name the method, explain why it can answer the question, list assumptions and
alternatives, and expose the equivalent low-level BioLang call. They do not
guess whether observations are paired or independent and they never transform
or remove data.

```biolang
import "statistics" as stat

# Welch is explicit and is the default only for this two-independent-group task.
let result = stat.compare_groups(control, treated, {method: "welch"})
stat.show(result, {detail: "learning", format: "auto"})

# Every decision remains inspectable.
result.method
result.assumptions
result.alternatives
result.reproducible_call
```

| Scientific question | Task-first helper | Available methods |
|---|---|---|
| Do two independent groups differ? | `stat.compare_groups(a, b, options?)` | Welch, pooled t, Mann-Whitney, permutation |
| Did matched measurements change? | `stat.paired_change(before, after, options?)` | paired t, paired Wilcoxon |
| Do several groups differ? | `stat.compare_many(groups, options?)` | Welch/classical ANOVA, Kruskal-Wallis |
| Are categories associated? | `stat.count_association(counts, options?)` | chi-square, Fisher exact |
| Is a common odds ratio plausible across strata? | `stat.stratified_association(strata)` | Tarone-adjusted Breslow-Day |
| How are two numeric measurements related? | `stat.numeric_relationship(x, y, options?)` | linear/Pearson, Spearman, Kendall |
| Is there a dose-response trend? | `stat.dose_response(dose, outcome, options?)` | linear, Spearman |
| What is survival over time? | `stat.survival_summary(time, event, options?)` | Kaplan-Meier; optional Cox model |
| What pooled effect is supported by studies? | `stat.meta_summary(effects, variances, options?)` | fixed or random effects |

From a terminal, `bl stats` lists the same questions. It can generate a
readable notebook whose input columns and method are stated in the code:

```text
bl stats compare measurements.csv --columns control,treated --output comparison.bln
bl notebook comparison.bln
```

Analysts who already know R function names can separately install the optional
`rstats` compatibility package. It is a small MIT-licensed naming façade over
BioLang primitives, not an R interpreter; new work should prefer the
question-oriented helpers above.

In the CLI, choose how SVG results are handled with `:plot ascii`,
`:plot unicode`, `:plot file`, or `:plot open`. Use
`stat.show(report, {format: "ascii"})` when the returned value itself must be
plain text. `format: "raw"` returns the unchanged report record for downstream
code. The supported formats are `auto`, `ascii`, `svg`, and `raw`.

## Guided exploration API

| Function | Purpose |
|---|---|
| `stat.explore(values, options?)` | Centre, spread, shape, missingness, transformation candidates, and review flags |
| `stat.compare(values, groups, options?)` | Per-group evidence and appropriate analysis alternatives |
| `stat.relationship(x, y, options?)` | Complete-pair counts, Pearson/Spearman association, regression line, and alternatives |
| `stat.categorical(values, options?)` | Counts, proportions, modes, missingness, and rare-level clues |
| `stat.guide(report, context?)` | Add an explicit scientific question and experimental unit |
| `stat.explain(report, detail?)` | Render `quick`, `learning`, or `audit` text |
| `stat.show(report, options?)` | Present an explanation plus an automatic, ASCII, or SVG visual; `raw` preserves the report |
| `stat.distribution_plot(values, options?)` | Annotated histogram, observations, mean, median, IQR, SD bands, and outlier flags |
| `stat.distribution_ascii(values, options?)` | CLI-safe histogram with mean, median, IQR, SD, exclusions, and review flags |
| `stat.normal_diagram(values?, options?)` | Teaching bell curve with 1/2/3-SD areas, optional observed coverage and z-tail highlighting |
| `stat.visualize(report, options?)` | Render `stat.explore()` guidance as SVG or terminal-safe ASCII |
| `stat.preprocessing(values, options?)` | Observable data-quality issues and non-applied normalization/transformation alternatives |
| `stat.profile(table, options?)` | Whole-table types, summaries, missingness, duplicates, range checks, and design clues |
| `stat.missingness(table, options?)` | Missingness by row, column, pair, and optional group |
| `stat.design_check(table, options?)` | Repeated units, imbalance, subject/time duplicates, and batch/group confounding |
| `stat.preview_transform(values, method, options?)` | Before/after evidence for log, log1p, square-root, z-score, robust, or min-max scaling |
| `stat.uncertainty(values, options?)` | Seeded bootstrap intervals for centres, spread, group differences, and correlations |
| `stat.shape(values, options?)` | Skewness, kurtosis, histogram-peak, and normal-Q-Q evidence without a diagnosis |
| `stat.normal_qq_plot(values, options?)` | Normal-distribution Q-Q diagnostic, distinct from genomic `qq_plot()` |
| `stat.group_plot(values, groups, options?)` | Group observations and robust summaries in SVG or ASCII |
| `stat.relationship_plot(x, y, options?)` | Scatterplot and fitted line in SVG or ASCII; optional grouping and confidence or prediction bands |
| `stat.categorical_plot(values, options?)` | Frequency bars in SVG or ASCII |
| `stat.missingness_plot(table, options?)` | Missingness map in SVG or ASCII |
| `stat.normalization_guide(matrix, options?)` | Dense/sparse matrix audit and domain-aware normalization alternatives |
| `stat.scan(table, options?)` | One-command profile, association screen, column evidence, and prioritized next steps |
| `stat.overview_ascii(table, options?)` | Compact, terminal-safe whole-table summary similar to a statistical skim |
| `stat.associations(table, options?)` | Bounded Pearson/Spearman, Cramer's V, and categorical-numeric effect-size screen |
| `stat.linear_diagnostics(x, y, options?)` | Residual form, spread, Q-Q, order, and influence clues for a simple linear model |
| `stat.linear_diagnostic_plot(x, y, options?)` | Residual-versus-fitted or residual Q-Q display in SVG or ASCII |
| `stat.report(table, options?)` | Self-contained HTML or Markdown data-health report with provenance and copyable next steps |
| `stat.distribution_clues(values, options?)` | Scale-sensitive normal, log-normal, Poisson, and negative-binomial fit clues without model selection |
| `stat.multiple_linear_diagnostics(predictors, outcome, options?)` | Categorical encoding, interactions, VIF, influence, intervals, and deterministic held-out error |
| `stat.omics_profile(matrix, options?)` | Sparse-safe guidance for bulk RNA-seq, single-cell, proteomics, metabolomics, or microbiome matrices |
| `stat.robust_linear_diagnostics(predictors, outcome, options?)` | Huber-versus-OLS coefficient sensitivity without automatic deletion or unsupported inference |
| `stat.weighted_summary(values, weights, options?)` | Weighted centre/spread, effective sample size, and weight-concentration diagnostics |
| `stat.time_series_diagnostics(values, options?)` | Trend, ACF, Ljung-Box, and first-difference clues for an ordered regular series |
| `stat.cluster_diagnostics(values, clusters, options?)` | One-way ICC, cluster sizes, and approximate loss of independent information |
| `stat.means(values, options?)` | Arithmetic, geometric, harmonic, trimmed, RMS, median, and mode with compatible spread guidance |
| `stat.compare_groups(a, b, options?)` | Explicit two-group mean, rank, paired, or permutation analysis with assumptions and alternatives |
| `stat.compare_many(groups, options?)` | Explicit several-group ANOVA or rank analysis |
| `stat.count_association(counts, options?)` | Explicit chi-square or Fisher analysis for count tables |
| `stat.stratified_association(strata, options?)` | Tarone-adjusted Breslow-Day homogeneity analysis for stratified 2x2 tables |
| `stat.numeric_relationship(x, y, options?)` | Explicit linear, Pearson, Spearman, or Kendall analysis |
| `stat.paired_change(before, after, options?)` | Paired t or signed-rank analysis without losing the pairing |
| `stat.dose_response(dose, outcome, options?)` | Explicit linear or rank dose-response summary |
| `stat.survival_summary(time, event, options?)` | Kaplan-Meier summary and optional Cox model |
| `stat.meta_summary(effects, variances, options?)` | Fixed- or random-effects meta-analysis |
| `stat.task_catalog()` | Machine-readable task, input, and method schema for CLIs and Studio |

Suggestions are deterministic heuristics, not automatic scientific decisions.
The package never removes observations or applies a transformation. It also does
not infer pairing, independence, experimental units, batches, or confounding.

For a fitted relationship, choose the band that answers the question:

```biolang
# Uncertainty around the estimated average response.
stat.relationship_plot(x, y, {interval: "confidence", confidence: 0.95})

# Wider uncertainty for one future observation.
stat.relationship_plot(x, y, {interval: "prediction", confidence: 0.95})

# One colour, fitted line, and band per category. String groups are ordered
# alphabetically, so their colours remain stable when rows are reordered.
stat.relationship_plot(x, y, {
    group: treatment,
    interval: "confidence",
    confidence: 0.95,
    legend_title: "Treatment",
    theme: "ggplot"
})

# Inspect or export the exact coordinates without reading pixels.
let fit = linear_fit_data(x, y, {confidence: 0.95})
fit.data
```

Frequency and missingness plots are inspectable in the same way:

```biolang
# Stable first-observed labels, counts, proportions, and missing total.
let frequencies = categorical_data(stages)
frequencies.data

# Full-table missing counts plus a bounded grid for any renderer.
let missing = missingness_data(data, {max_rows: 100, max_columns: 40})
missing.column_summary
missing.cells
```

The `max_rows` and `max_columns` limits change only the display grid. They do
not change `missing_cells` or `column_summary`, which always inspect the full
table.

## Multi-group inference

The inference builtins use explicit method labels and return effect sizes:

```biolang
let groups = [control, low_dose, high_dose]

# Unequal-variance comparison of group means.
let global = anova(groups, {variance: "welch"})

# Rank/distribution comparison with tie correction.
let ranked = kruskal_wallis(groups)

# Use only with a defensible classical shared-variance ANOVA model.
let all_pairs = tukey_hsd(groups, {confidence: 0.95})

# Pair-specific Welch tests with explicit multiplicity correction.
let welch_pairs = pairwise_ttest(groups, {
    variance: "welch",
    adjust: "holm"
})
```

`tukey_hsd()` is a genuine studentized-range Tukey-Kramer procedure;
`pairwise_ttest()` is deliberately named differently. Kruskal-Wallis is not a
test of medians, and separate pairwise rank-sum tests are not Dunn's test.

For repeated measurements on the same subjects, use the paired signed-rank
API rather than the independent-group `wilcoxon()` function:

```biolang
let change = wilcoxon_paired(before, after, {
    method: "normal",
    continuity: true
})
```

The result defines differences as `before - after`, reports the positive-rank
V statistic and a paired rank-biserial effect size. Exact mode is available
only when the non-zero absolute differences are untied, so it never silently
substitutes an approximation.

`stat.explore()` also returns `visual_guide`. Its `ascii` and `svg` fields explain
centre, spread, and scale; `sd_bands` reports the observed fraction within one,
two, and three SD of the mean. The familiar 68/95/99.7 percentages are included
only as normal-model references. They are not used as an automatic outlier rule.
`reading_order` gives a short, plain-language sequence for interpreting the
distribution without changing the data.

## Normal curve and guided rendering

The theoretical diagram needs no data:

```biolang
stat.normal_diagram()
```

Supplying observations adds their measured 1/2/3-SD coverage beneath the
normal reference. It does not declare the observations normal:

```biolang
let values = [9.8, 10.1, 10.3, 10.7, 11.0, 11.4, 12.2]

# Notebook/web SVG; exported HTML also offers canvas and PNG fallback.
stat.normal_diagram(values)

# Terminal-safe bell curve with the right tail beyond z = 1.96 highlighted.
println(stat.normal_diagram(values, {
    format: "ascii",
    z: 1.96,
    tail: "right"
}))

let report = stat.explore(values)
stat.visualize(report)
println(stat.visualize(report, {format: "ascii"}))
```

`tail` may be `left`, `right`, or `two`. Highlight probabilities use the
standard normal CDF. The diagram labels the empirical rule as a model reference,
not an outlier cutoff.

## Choose a centre together with its spread

```biolang
let choices = stat.means([1, 2, 4, 8], {trim_fraction: 0.25})
choices.centre_spread_pairs
```

Arithmetic mean pairs with SD for additive, reasonably symmetric variation;
median pairs with IQR or MAD for skewed or heavy-tailed data; geometric mean
pairs with geometric SD or a fold interval for positive multiplicative data.
Weighted and harmonic means answer more specialized design/rate questions, and
trimmed means require a declared trim. Mode is described using counts or
proportions. No centre is automatically selected.

For preprocessing advice, provide the measurement type when known:

```biolang
let prep = stat.preprocessing(counts, {data_type: "counts"})
```

The result distinguishes many observed zeros from a formal zero-inflation
diagnosis, checks that count data are non-negative integers, and explains why
library-size normalization requires sample-level totals rather than one vector.

## Whole-dataset first pass

```biolang
let audit = stat.profile(data, {
    subject_column: "patient_id",
    group_column: "treatment",
    batch_column: "sequencing_batch",
    ranges: {age: {min: 0, max: 120}}
})

println(stat.explain(audit))
println(stat.missingness_plot(data, {format: "ascii"}))
```

Role and range declarations are supplied by the analyst; BioLang does not infer
them from convenient column names. `profile()` uses all rows, and visualization
sampling is disclosed separately from full-data calculations.

Design context can additionally declare `time_column`, `cluster_column`,
`replicate_column`, `assignment_unit_column`, `weights_column`, `control_level`,
`randomized`, `blinded`, and `sampling_method`. BioLang reports paired,
longitudinal, nested, clustered, assignment-unit, and sampling-weight clues, but
never claims independence or randomization from a table alone.

For the recommended first pass, use the composed scan and inspect the ordinary
records it returns:

```biolang
let scan = stat.scan(data, {
    subject_column: "patient_id",
    group_column: "treatment",
    batch_column: "sequencing_batch"
})

println(stat.overview_ascii(data))
println(stat.explain(scan))
scan.recommendations
scan.associations.pairs
```

`scan.column_details` is deliberately compact: numeric columns keep summaries
and guidance but not every flagged row, while categorical columns keep only the
five most frequent levels. Each record provides a focused-call example for full
detail. This prevents the first pass from duplicating a large table in memory.

Association screening is bounded to 50 eligible columns and returns at most 100
pairs by default; both limits are configurable and any truncation is reported.
Numeric pairs use Pearson and Spearman, categorical pairs use Cramer's V, and
categorical-numeric pairs use eta-squared with its square root as the common
screening magnitude. These are exploratory effect-size clues, not hypothesis
tests or evidence of causation. A declared `subject_column` is excluded from
pairwise screening; use `exclude_columns: ["sample_id"]` for other identifiers.
Integer-coded groups remain numeric unless explicitly listed with
`categorical_columns: ["stage_code"]`.

After fitting a simple straight-line relationship, inspect its residuals:

```biolang
let diagnostics = stat.linear_diagnostics(dose, response)
println(stat.explain(diagnostics))
stat.linear_diagnostic_plot(dose, response)
stat.linear_diagnostic_plot(dose, response, {view: "qq"})
```

The diagnostics report residual spread and Q-Q alignment, changing-spread and
curvature clues, Cook's distances, standardized residual flags, and a
Durbin-Watson value in the current observation order. They do not certify that a
model is valid, and no observation is removed automatically.

For a multivariable model, pass a predictor table and outcome list. Numeric
columns remain numeric; string and Boolean columns use first-observed treatment
contrasts. The exact encoding is returned in `model.encodings`:

```biolang
let predictors = table({age: age, arm: arm, baseline: baseline})
let model = stat.multiple_linear_diagnostics(predictors, response, {
    interactions: ["baseline:arm"],
    validation_group_column: "patient_id",
    validation_folds: 5,
    seed: 42
})
```

The result includes coefficients, large-sample intervals, VIF, Cook and leverage
flags, residual evidence, and deterministic held-out RMSE/MAE. When
`validation_group_column` is supplied, the identifier is excluded from the
model and every row from a subject, site, family, or batch remains in the same
fold. Without it, validation is row-wise.

For a sensitivity check when large residuals may dominate an ordinary fit:

```biolang
let sensitivity = stat.robust_linear_diagnostics(predictors, response)
sensitivity.coefficients
```

This uses Huber iteratively reweighted least squares and reports both OLS and
Huber estimates. It does not delete rows and intentionally supplies no p-values:
robust uncertainty still depends on the sampling and dependence design.

For supplied sampling, frequency, or analytic weights:

```biolang
let weighted = stat.weighted_summary(response, sampling_weight, {
    weight_kind: "probability"
})
```

Inspect the weighted/unweighted shift, effective sample size, unequal-weight
design effect, and maximum weight share. These are diagnostics, not a
replacement for strata-, cluster-, calibration-, or replicate-weight survey
estimators.

For a regularly spaced ordered series:

```biolang
let temporal = stat.time_series_diagnostics(signal, {max_lag: 12})
```

The result reports ACF values, a Ljung-Box check, linear trend, and the scale of
first differences. Missing time points are rejected rather than silently
compressed. No ARIMA or intervention model is automatically selected.

For repeated subjects, sites, families, plates, or other declared clusters:

```biolang
let dependence = stat.cluster_diagnostics(response, patient_id)
```

This reports a one-way random-intercept ICC, cluster-size imbalance, and an
approximate design effect/effective sample size. It does not fit fixed effects,
random slopes, nested/crossed effects, GEE, or a mixed model; use the result to
recognize when independent-row uncertainty is implausible.

## Reproducible report

```biolang
let report = stat.report(data, {
    format: "html",
    title: "Baseline data review",
    subject_column: "patient_id",
    group_column: "treatment",
    batch_column: "sequencing_batch",
    seed: 42,
    generated_at: "2026-08-15T10:30:00+10:00"
})

# Leave the report record as the last expression in a notebook cell.
report
```

The report contains the overview, evidence-linked recommendations, strongest
association clues, BioLang version, compute backend, options, seed, and
caller-supplied timestamp. It contains no scripts or run buttons, so it can be
rendered in a notebook or saved as ordinary HTML. Markdown is available with
`{format: "markdown"}`.

Missingness reports include bounded co-missing patterns and comparisons of
numeric measurements between rows where another field is observed versus
missing. These are prompts to investigate collection and design; they do not
diagnose MCAR, MAR, or MNAR.

`stat.distribution_clues()` uses likelihood and AIC, rather than correlation
alone, to compare only candidate families that fit the data domain. Delta AIC,
variance/mean, expected zeros, and a moment mixture clue are reported, but
`model_selected` remains false.

## Omics matrix first pass

```biolang
let matrix_report = stat.omics_profile(counts, {
    modality: "single_cell",
    sample_axis: "columns"
})
```

Supported modalities are `bulk_rnaseq`, `single_cell`, `proteomics`,
`metabolomics`, `microbiome`, and `generic`. Sparse matrices are never
densified; axis moments require O(samples + features) additional memory.
Variance/mean rankings are descriptive clues and do not silently select
features.

For transformation work, preview before choosing:

```biolang
let preview = stat.preview_transform(values, "log1p")
println(stat.explain(preview))
```

The transformed vector is returned as `preview.values`, but the input is never
mutated. The report compares skewness, SD, IQR, zero handling, range compression,
rank preservation, interpretation, and cautions.

The combined workflow is available in
[`examples/complete_guided_report.bl`](examples/complete_guided_report.bl).
Grouped validation, robust sensitivity, weights, and ordered data are combined
in [`examples/robust_weighted_timeseries.bl`](examples/robust_weighted_timeseries.bl).

## Model diagnostics

The guided layer fits and diagnoses three common model families:

```biolang
let binary = stat.glm_diagnostics(predictors, outcome, {family: "binomial"})
let repeated = stat.random_intercept_model(predictors, outcome, patient_ids, {method: "reml"})
let survival = stat.cox_diagnostics(follow_up_time, event, predictors)
```

`glm_diagnostics()` supports binomial and Poisson outcomes and reports residual
deviance, dispersion, influence, and calibration or zero-count clues. It also
reports `converged` and `iterations`. Iteratively reweighted least squares
returns its final iterate whether or not the coefficients settled, so check
`converged` before interpreting anything else: a separated logistic fit still
produces coefficients, and they are an artefact of where iteration stopped
rather than an estimate. A fit that did not converge is disclosed as a
`fit_not_converged` issue and is marked in the rendered summary.
`random_intercept_model()` reports fixed effects, variance components, ICC,
partially pooled intercepts, and marginal and conditional fitted values.
`cox_diagnostics()` uses a full multivariable Newton fit with Breslow ties and
reports hazard-ratio intervals, partial likelihood, baseline hazard,
concordance, martingale/deviance residuals, and a clearly labelled descriptive
Schoenfeld screen. That screen is not a formal `cox.zph` replacement.

All three preserve their inputs and return review clues rather than automatic
deletion decisions. Model diagnostics cannot create a causal study design.

External R validation is documented in
[`validation/README.md`](validation/README.md). An unavailable R installation is
reported as “not run,” never as a pass.
