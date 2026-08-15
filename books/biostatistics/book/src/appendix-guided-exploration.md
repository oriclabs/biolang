# Appendix F: Guided Exploration in BioLang

This appendix is a practical first stop when you receive unfamiliar data. The
goal is not to make BioLang choose a test for you. It shows the calculated facts,
explains the clues, lists reasonable alternatives, and identifies information
that the values cannot provide.

## Start with the observations

```bio
import "statistics" as stat

let protein = [12.1, 12.4, 12.8, 13.0, 13.2, 13.5, 29.0]
let report = stat.explore(protein, {name: "protein concentration"})

println(stat.explain(report))
```

The report keeps calculated facts separate from suggestions:

- `report.data` says exactly how many values were received, used, missing, or
  non-finite.
- `report.summary` contains the mean, median, variance, standard deviation,
  quartiles, IQR, MAD, skewness, and repeated-value mode when one exists.
- `report.clues` gives the calculated evidence behind each statement.
- `report.suggestion` proposes a centre and spread, marked as heuristic.
- `report.alternatives` explains other defensible summaries and their limits.
- `report.outliers` identifies original zero-based indices. These are review
  flags, never automatic deletions.
- `report.transformations` contains candidates, never silently transformed data.

## See centre and spread on the observations

```bio
stat.distribution_plot(protein, {
    title: "Protein concentration: centre and spread"
})
```

The visual combines a histogram, observations, mean, median, quartiles, IQR,
mean plus or minus one to three standard deviations, and Tukey-fence flags.
Calculations and the histogram always use all finite values. With very large
vectors the dot layer is thinned for browser safety, and the figure states the
exact display stride.

For a terminal, use the same evidence as an ASCII chart:

```bio
println(stat.distribution_ascii(protein, {width: 56, height: 10}))
```

It prints a full-data histogram, mean and median positions, the IQR, SD, Tukey
review-flag count, and any missing/non-finite exclusions. `width` controls the
number of character bins and `height` controls the tallest bar.

| Mark | Meaning |
|---|---|
| Mean | Equal-share centre; sensitive to distant observations |
| Median | Half the observations lie on either side |
| SD | Typical distance from the mean, in the original units |
| Variance | SD squared; the same spread expressed in squared units |
| IQR | Width of the middle 50%, from Q1 to Q3 |
| MAD | Typical absolute distance from the median |
| Tukey flag | Observation outside Q1 - 1.5 IQR or Q3 + 1.5 IQR; inspect it, do not automatically remove it |

The familiar 68-95-99.7 percentages for one, two, and three SD apply only to an
approximately normal distribution. The bands are still distances on other
distributions, but not guaranteed coverage percentages.

## Add the design separately

Numbers do not reveal whether rows are patients, technical replicates, cells
from the same donor, or repeated measurements. Supply that information:

```bio
let guided = stat.guide(report, {
    question: "describe baseline protein concentration",
    experimental_unit: "patient"
})

println(guided.guidance.design_note)
```

BioLang records this context but deliberately leaves
`automatic_test_selection` false. Independence, pairing, batches, censoring, and
confounding require scientific knowledge.

## Inspect preprocessing and distortion clues

```bio
let counts = [0, 0, 0, 1, 2, 40]
let prep = stat.preprocessing(counts, {data_type: "counts"})
```

`prep.issues` reports only observable clues: missing or non-finite values,
constant data, many zeros, strong asymmetry, low resolution or heaping, wide
positive ranges, and Tukey-fence review flags. “Many zeros” is not labelled as a
zero-inflated model diagnosis.

`prep.suggestions` describes keeping the original scale, log or log1p
transforms, z-score or robust standardisation, min-max scaling, and—when the
measurement type supports it—exposure/library-size or logit approaches. Every
option states what it changes, when it is useful, and why it may be wrong.
`prep.automatic_changes` is always false.

Count normalisation cannot be inferred from one vector: BioLang asks for the
sample-by-feature axes and valid sample-level library size or exposure. Likewise,
batch correction requires explicit batch and experimental-unit information.

## Compare groups

```bio
let expression = [4.1, 4.4, 4.7, 6.0, 6.4, 7.1]
let treatment = ["control", "control", "control", "drug", "drug", "drug"]

let groups = stat.compare(expression, treatment, {paired: false})
println(stat.explain(groups))
```

The result contains a complete exploration report for every group. Its
alternatives distinguish the questions answered by Welch's t-test,
Mann-Whitney rank-sum, resampling, and regression. Mann-Whitney is described as
a rank/distribution comparison; it is not automatically called a test of
medians.

For paired observations, explicitly set `{paired: true}` and verify that the two
lists are aligned by experimental unit.

## Explore a relationship

```bio
let dose = [1, 2, 3, 4, 5]
let response = [2.1, 3.9, 6.2, 8.0, 9.8]

let association = stat.relationship(dose, response)
println(stat.explain(association))
```

BioLang reports the number of complete pairs, Pearson correlation, Spearman
correlation, and the simple linear slope. A high correlation does not prove
agreement or causation. A low Pearson correlation does not rule out a nonlinear
relationship. Plot the pairs before interpreting the coefficients.

For a bounded whole-table screen, `stat.associations(study)` uses Pearson and
Spearman for numeric pairs, Cramer's V for categorical pairs, and eta-squared
for categorical/numeric pairs. It reports complete-pair counts, skipped
high-cardinality variables, and every work limit. A strong pair can mean
expected biology, redundancy, confounding, data leakage, or a shared measuring
process; it is not causation or a corrected hypothesis test.

An explicitly declared `subject_column` is excluded from this screen. Add other
identifiers through `exclude_columns: ["sample_id"]`; group and batch columns
remain visible because their association may expose confounding.
Integer-coded groups remain numeric unless declared with
`categorical_columns: ["stage_code"]`.

After fitting a simple straight-line relationship, inspect residual evidence:

```bio
let diagnostics = stat.linear_diagnostics(dose, response)
println(stat.explain(diagnostics))
stat.linear_diagnostic_plot(dose, response)
stat.linear_diagnostic_plot(dose, response, {view: "qq"})
```

The record covers residual spread, Q-Q alignment, changing-width and curvature
clues, Cook's distance, large standardized residuals, and Durbin-Watson in the
current observation order. These are prompts to inspect the plots and study
design. They do not prove assumptions or justify deleting a point.

## Explore categories

```bio
let response = ["complete", "partial", "complete", "none", nil]
let categories = stat.categorical(response)

println(stat.explain(categories))
```

This reports counts, proportions, missing values, tied modes, and rare-level
clues in first-observed order. Rare categories are not automatically combined;
that decision needs scientific justification.

## Choose the explanation depth

```bio
println(stat.explain(report, "quick"))
println(stat.explain(report, "learning"))
println(stat.explain(report, "audit"))
```

- `quick` gives the result and main clue.
- `learning` explains centre, spread, shape, and limitations.
- `audit` adds the schema, full-data calculation policy, and non-mutating
  recommendation policy.

## Begin with the whole table

When you do not yet know where to start, use the composed scan. It combines the
table profile, missingness and design checks, bounded pairwise association
screening, per-column evidence, and prioritized next steps:

```bio
let first_pass = stat.scan(study, {
    subject_column: "patient",
    group_column: "group",
    batch_column: "batch"
})

println(stat.overview_ascii(study))
println(stat.explain(first_pass))
first_pass.recommendations
```

Every recommendation contains the observation that triggered it, why it may
matter, a next step, and example BioLang code. It is still advice: `scan()` does
not remove a row, fill a missing value, transform a measurement, or select a
test.

The scan keeps compact column details rather than copying every flagged row or
every category. Numeric details retain summaries and guidance; categorical
details retain the five most frequent levels. Each includes the focused
`stat.explore()` or `stat.categorical()` call for full evidence.

A single vector cannot reveal duplicated rows, mixed column types, missingness
patterns, repeated patients, or batch/group confounding. Profile the dataset and
declare the roles that BioLang cannot know:

```bio
let study = table({
    patient: ["p1", "p1", "p2", "p3"],
    group: ["control", "control", "drug", "drug"],
    batch: ["run1", "run1", "run2", "run2"],
    age: [34, 34, nil, 150],
    response: [1.2, 1.7, 8.0, 30.0]
})

let audit = stat.profile(study, {
    subject_column: "patient",
    group_column: "group",
    batch_column: "batch",
    ranges: {age: {min: 0, max: 120}}
})

println(stat.explain(audit))
```

`audit.columns` contains type, observed/missing/unique counts, ID-like and
constant clues, declared-range violations, and numeric summaries. The report
also contains `audit.missingness` and `audit.design`. Duplicate and impossible
value clues ask for review; they do not delete rows.

Study roles are explicit options, not guesses from names. For example, a column
called `mouse` is not assumed to be the experimental unit unless you supply
`{subject_column: "mouse"}`.

When available, also declare `time_column`, `cluster_column`,
`replicate_column`, `assignment_unit_column`, `weights_column`, `control_level`,
`randomized`, `blinded`, and `sampling_method`. The design report can then
surface paired, longitudinal, nested, clustered, assignment-unit, and weighting
clues. It still cannot establish independence or reconstruct an assignment or
sampling process from recorded values.

## Examine missingness, not only its total

```bio
let missing = stat.missingness(study, {group_column: "group"})
println(stat.explain(missing))
println(stat.missingness_plot(study, {format: "ascii"}))
```

This maps missingness by column and row, values missing together, and optional
group-specific rates. It also reports frequent co-missing patterns and bounded
comparisons of numeric measurements between rows where another field is
observed versus missing. A difference is evidence to investigate. It does not by
itself establish whether data are missing completely at random, at random, or
not at random, and BioLang performs no automatic imputation.

## Compare plausible distribution families without choosing one

```bio
let families = stat.distribution_clues([0, 0, 1, 1, 2, 4, 9, 15])
families.candidates
families.issues
```

Normal, log-normal, Poisson, and negative-binomial candidates are fitted only
when the values satisfy their domains. The report uses log-likelihood and AIC,
so it can see scale differences that correlation cannot. It also reports the
count variance/mean ratio, observed versus fitted Poisson zeros, and a moment
mixture clue. Small delta AIC means “better among these candidates,” not
“correct.” Covariates, exposure, mixtures, censoring, and dependence can make
every candidate inadequate, so `model_selected` remains false.

## Preview a transformation before choosing it

```bio
let response = [1.2, 1.7, 2.0, 8.0, 9.2, 30.0]
let preview = stat.preview_transform(response, "log")

println(stat.explain(preview))
```

Available previews are `log`, `log1p`, `sqrt`, `zscore`, `robust`, and
`minmax`. The report compares the before/after centre, SD, IQR, skewness, zero
handling, range compression, rank order, interpretation, and cautions.
`preview.values` contains the proposed result, while `preview.input_modified`
remains false.

The preview can show that a graph looks more symmetric. It cannot prove that a
transformation answers the scientific question or makes a test valid.

## Put uncertainty around the estimate

```bio
let interval = stat.uncertainty(response, {
    statistic: "median",
    repetitions: 2000,
    confidence: 0.95,
    seed: 42
})
```

`stat.uncertainty()` supports `mean`, `median`, and `sd`; independent
`difference_mean` or `difference_median` with an `other` vector; and `pearson`
or `spearman` with a `y` vector. It uses a deterministic percentile bootstrap
and records the seed and successful replicate count.

The row must be the resampling unit. Resampling cells as independent units when
the experimental unit is a donor would create falsely narrow intervals.

## Inspect shape without assigning a diagnosis

```bio
let shape = stat.shape(response, {bins: 8})
let qq = stat.normal_qq_plot(response)
```

The shape report includes histogram peak locations, skewness, excess kurtosis,
and normal-Q-Q alignment. Peak counts depend on bin width, so
`multiple_peak_clue` is never labelled proof of multimodality or separate
biological populations. `stat.normal_qq_plot()` is the normal-distribution
diagnostic; BioLang's older global `qq_plot()` remains a genomic p-value plot.

All focused diagnostics have browser and terminal forms:

```bio
stat.group_plot(response, ["A", "A", "A", "B", "B", "B"])
println(stat.group_plot(response, ["A", "A", "A", "B", "B", "B"], {
    format: "ascii"
}))
```

The same `{format: "ascii"}` choice is available for relationship,
categorical, normal-Q-Q, and missingness plots.

## Choose normalization from the measurement process

```bio
let counts = [[0, 10, 20], [2, 20, 40], [1, 4, 5]]
let norm = stat.normalization_guide(counts, {
    data_type: "counts",
    sample_axis: "rows"
})

println(stat.explain(norm))
```

The complete dense or sparse matrix is audited for zeros, invalid counts,
non-finite values, zero-total samples, and unequal totals. Guidance differs for
counts, compositional data, proportions, and continuous measurements. It lists
required inputs and cautions for offsets, robust size factors, total-count
scaling, variance-stabilising transforms, log-ratios, binomial models, feature
scaling, and batch-aware modelling. No normalization is automatically applied.

## Diagnose a multivariable line transparently

```bio
let predictors = table({
    baseline: baseline,
    treatment: treatment,
    age: age
})

let model = stat.multiple_linear_diagnostics(predictors, response, {
    interactions: ["baseline:treatment"],
    validation_group_column: "patient",
    validation_folds: 5,
    seed: 42
})
```

Numeric predictors remain numeric. String and Boolean predictors use
first-observed treatment contrasts; `model.encodings` records the reference and
expanded feature names. The report includes approximate confidence intervals,
VIF, residual Q-Q and spread clues, Durbin-Watson in row order, leverage, Cook's
distance, and deterministic held-out RMSE/MAE.

The intervals use a large-sample normal critical value and the returned record
says so. `validation_group_column` excludes the identifier from the predictors
and keeps each donor, family, site, or batch wholly inside one fold. If it is
omitted, validation is row-wise. Ordered observations may need forward-chaining
rather than either form.

### Ask whether a few large residuals drive the conclusion

```bio
let sensitivity = stat.robust_linear_diagnostics(predictors, response)
sensitivity.coefficients
```

Each returned coefficient shows the ordinary least-squares estimate beside a
Huber estimate. A large change means the conclusion is sensitive to rows with
large residuals. It does **not** mean those rows are wrong, and BioLang neither
deletes them nor reports unjustified robust p-values.

### Understand supplied weights before using them

```bio
let weighted = stat.weighted_summary(response, sampling_weight, {
    weight_kind: "probability"
})
```

Look at the shift from the unweighted mean, effective sample size, design
effect, and largest weight share. A sample of 100 rows can carry much less than
100 independent observations' worth of information when a few weights
dominate. This summary does not account for survey strata or clusters.

### Check an ordered series without pretending order means independence

```bio
let temporal = stat.time_series_diagnostics(signal, {max_lag: 12})
temporal.autocorrelations
temporal.issues
```

The function describes trend, lag dependence, Ljung-Box evidence, and first
differences. It requires a complete regularly spaced series because removing a
missing value would incorrectly make two non-adjacent measurements neighbours.
It does not select an ARIMA, seasonal, or intervention model.

### See why repeated rows are not independent samples

```bio
let dependence = stat.cluster_diagnostics(response, patient_id)
dependence.intraclass_correlation
dependence.approximate_effective_sample_size
```

An ICC near zero means little similarity was measured within the declared
clusters. A positive ICC means observations from the same patient, site, plate,
or family tend to resemble one another. The approximate effective sample size
makes the loss of independent information visible. This is a one-way clue, not
a fitted mixed model: fixed effects, random slopes, nesting, and crossed effects
still need an analysis chosen from the study design.

## Create one notebook-friendly report

```bio
let health = stat.report(study, {
    format: "html",
    title: "Study data review",
    subject_column: "patient",
    group_column: "group",
    batch_column: "batch",
    seed: 42,
    generated_at: "2026-08-15T10:30:00+10:00"
})

# Leave the report record as the last expression in a notebook cell.
health
```

The report includes an overview, prioritized evidence and copyable BioLang
commands, association clues, version, backend, seed, options, and caller-supplied
timestamp. It contains no scripts or run buttons. Markdown is available with
`{format: "markdown"}`, and the structured scan remains in `health.scan`.

## Use modality-aware matrix guidance

```bio
let matrix_health = stat.omics_profile(counts, {
    modality: "single_cell",
    sample_axis: "columns"
})
```

Guidance differs for bulk RNA-seq, single-cell counts, proteomics,
metabolomics, microbiome, and generic matrices. The report covers sparsity,
invalid-count clues, sample-total variation, detection, feature mean-variance
dependence, and a bounded variance/mean ranking. Sparse inputs remain sparse
with O(samples + features) additional memory.

## Validate numerical conventions externally

The package contains an external base-R validation runner under
`packages/statistics/validation`. It checks scale-sensitive differences for
descriptive statistics, type-7 quantiles, raw MAD, adjusted skewness,
transformation summaries, correlations, regression coefficients, robust-fit
sensitivity, weighted moments, time-series diagnostics, residual diagnostics,
influence flags, and matrix totals. Its manifest records both
versions, metric-specific tolerances, and timings. If R is absent, the
runner reports “not run”; that state is never treated as a validation pass.

## Reproducibility contract

The guided exploration API uses the schema
`biolang.stats.exploration/v1`. It is deterministic and available in both native
and WASM runtimes. It never removes an observation, applies a transformation,
selects an analysis by its p-value, or infers the experimental unit.

The complete runnable examples are
`packages/statistics/examples/dataset_diagnostics.bl` and
`packages/statistics/examples/guided_dataset_scan.bl`, and the package tests
execute the numeric, grouped, relationship, categorical, preprocessing, and
dataset-level workflows.
