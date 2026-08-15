# Appendix E: BioLang Statistics Quick Reference

> *Every statistical function in BioLang, organized for quick lookup.*

This appendix lists BioLang's statistical builtins by category. For each function, you will find the signature, a brief description, and a one-liner example. Functions are listed within each category in roughly the order you would learn them.

## Descriptive Statistics

| Function | Description | Example |
|---|---|---|
| `mean(x)` | Arithmetic mean | `mean([2, 4, 6])` → `4.0` |
| `median(x)` | Middle value | `median([1, 3, 7])` → `3.0` |
| `stdev(x)` | Sample standard deviation | `stdev([2, 4, 6])` → `2.0` |
| `variance(x)` | Sample variance | `variance([2, 4, 6])` → `4.0` |
| `min(x)` | Minimum value | `min([3, 1, 4])` → `1` |
| `max(x)` | Maximum value | `max([3, 1, 4])` → `4` |
| `sum(x)` | Sum of all values | `sum([1, 2, 3])` → `6` |
| `len(x)` | Number of elements | `len([1, 2, 3])` → `3` |
| `quantile(x, p)` | p-th quantile | `quantile([1,2,3,4,5], 0.75)` → `4.0` |
| `summary(x)` | Summary statistics | `summary(data)` → record with min, Q1, median, Q3, max, mean |
| `round(x, digits)` | Round to n decimal places | `round(3.14159, 2)` → `3.14` |
| `abs(x)` | Absolute value | `abs(-3.5)` → `3.5` |
| `sqrt(x)` | Square root | `sqrt(16)` → `4.0` |
| `log2(x)` | Base-2 logarithm | `log2(8)` → `3.0` |
| `log10(x)` | Base-10 logarithm | `log10(1000)` → `3.0` |

## Guided Exploration

These functions are provided by the `statistics` package:

| Function | Description |
|---|---|
| `stat.explore(values, options?)` | Full-data descriptive facts, clues, alternatives, and limitations |
| `stat.preprocessing(values, options?)` | Observable issues and non-applied preprocessing/normalization options |
| `stat.compare(values, groups, options?)` | Per-group exploration and analysis alternatives |
| `stat.relationship(x, y, options?)` | Complete-pair Pearson/Spearman and relationship alternatives |
| `stat.categorical(values, options?)` | Counts, proportions, missingness, modes, and rare-level clues |
| `stat.guide(report, context?)` | Attach the question and experimental unit without automatic test selection |
| `stat.explain(report, detail?)` | Render `quick`, `learning`, or `audit` explanations |
| `stat.distribution_plot(values, options?)` | Annotated distribution showing observations, centre, spread, and review flags |
| `stat.distribution_ascii(values, options?)` | Terminal histogram with centre, IQR, exclusions, and review flags |
| `stat.profile(table, options?)` | Dataset-wide type, integrity, missingness, range, and design audit |
| `stat.missingness(table, options?)` | Missingness by row, column, pattern, co-occurrence, optional group, and observed/missing comparisons |
| `stat.design_check(table, options?)` | Repeated, paired, longitudinal, nested, clustered, imbalance, and confounding clues |
| `stat.preview_transform(values, method, options?)` | Non-mutating before/after transformation comparison |
| `stat.uncertainty(values, options?)` | Seeded bootstrap interval for centres, spread, differences, or correlations |
| `stat.shape(values, options?)` | Shape and multiple-peak evidence without a distribution diagnosis |
| `stat.normal_qq_plot(values, options?)` | Normal-distribution Q-Q diagnostic in SVG or ASCII |
| `stat.group_plot(values, groups, options?)` | Grouped observations and robust summaries in SVG or ASCII |
| `stat.relationship_plot(x, y, options?)` | Relationship diagnostic in SVG or ASCII |
| `stat.categorical_plot(values, options?)` | Frequency bars in SVG or ASCII |
| `stat.missingness_plot(table, options?)` | Missingness map in SVG or ASCII |
| `stat.normalization_guide(matrix, options?)` | Dense/sparse matrix audit and domain-aware normalization choices |
| `stat.scan(table, options?)` | Recommended non-mutating first pass with evidence-linked next steps |
| `stat.overview_ascii(table, options?)` | Compact terminal-safe whole-table summary |
| `stat.associations(table, options?)` | Bounded type-appropriate pairwise effect-size screen |
| `stat.linear_diagnostics(x, y, options?)` | Residual form, spread, tails, order, and influence clues for a simple line |
| `stat.linear_diagnostic_plot(x, y, options?)` | Residual-versus-fitted or residual Q-Q display in SVG or ASCII |
| `stat.report(table, options?)` | Self-contained HTML or Markdown health report with provenance and copyable next steps |
| `stat.distribution_clues(values, options?)` | Likelihood/AIC and shape clues for common continuous and count families without model selection |
| `stat.multiple_linear_diagnostics(predictors, outcome, options?)` | Encodings, interactions, VIF, influence, intervals, residuals, and held-out error |
| `stat.omics_profile(matrix, options?)` | Sparse-safe modality-aware profile for common biological matrices |
| `stat.robust_linear_diagnostics(predictors, outcome, options?)` | Compare Huber and OLS coefficients as an outlier-sensitivity check |
| `stat.weighted_summary(values, weights, options?)` | Weighted summaries plus effective sample size and weight concentration |
| `stat.time_series_diagnostics(values, options?)` | Trend, autocorrelation, Ljung-Box, and first-difference clues |
| `stat.cluster_diagnostics(values, clusters, options?)` | ICC, cluster-size imbalance, and approximate information loss |
| `stat.means(values, options?)` | Definitions of average paired with compatible spread and use conditions |

See [Guided Exploration in BioLang](appendix-guided-exploration.md) for runnable
examples and the non-mutating recommendation contract.

## Probability Distributions

BioLang follows the familiar `d/p/q/r` convention where a form is currently implemented: `d` is density/mass, `p` is cumulative probability, `q` is a quantile, and `r` draws samples. The tables below list only forms currently registered by the runtime.

### Continuous

| Distribution | Available functions |
|---|---|
| Normal | `dnorm(x, mean?, sd?)`, `pnorm(x, mean?, sd?)`, `qnorm(p, mean?, sd?)`, `rnorm(n, mean?, sd?)` |
| Uniform | `dunif(x, min?, max?)`, `punif(x, min?, max?)` |
| Exponential | `dexp(x, rate?)`, `pexp(x, rate?)` |

```bio
dnorm(0, 0, 1)
pnorm(1.96, 0, 1)
qnorm(0.975, 0, 1)
rnorm(100, 0, 1)

dunif(0.5, 0, 1)
punif(0.7, 0, 1)

dexp(1.0, 0.5)
pexp(2.0, 0.5)
```

### Discrete

| Distribution | Available functions |
|---|---|
| Binomial | `dbinom(k, size, probability)`, `pbinom(k, size, probability)`, `rbinom(n, size, probability)` |
| Poisson | `dpois(k, lambda)`, `ppois(k, lambda)`, `rpois(n, lambda)` |

```bio
dbinom(10, 20, 0.5)
pbinom(10, 20, 0.5)
rbinom(100, 20, 0.5)

dpois(5, 5.0)
ppois(7, 5.0)
rpois(100, 5.0)
```

## Hypothesis Tests

### Comparing Two Groups

| Function | Description | Example |
|---|---|---|
| `ttest(a, b)` | Welch's two-sample t-test | `ttest(ctrl, treat)` |
| `ttest_paired(a, b)` | Paired t-test | `ttest_paired(before, after)` |
| `ttest_one(x, mu)` | One-sample t-test | `ttest_one(diffs, 0)` |
| `wilcoxon(a, b)` | Mann-Whitney/Wilcoxon rank-sum for independent groups | `wilcoxon(ctrl, treat)` |

### Comparing Multiple Groups

| Function | Description | Example |
|---|---|---|
| `anova(groups)` | Classical one-way ANOVA | `anova([g1, g2, g3])` |

> **Post-hoc comparisons:** Follow a significant ANOVA with pairwise tests and p-value correction:
>
> ```text
# Conceptual or diagnostic example; not directly executable.
> # Pairwise t-tests with Bonferroni correction
> let pvals = []
> for i in 0..len(groups) {
>   for j in (i+1)..len(groups) {
>     let result = ttest(groups[i], groups[j])
>     pvals = pvals + [result.p_value]
>   }
> }
> let adjusted = p_adjust(pvals, "bonferroni")
> ```

### Categorical Data

| Function | Description | Example |
|---|---|---|
| `chi_square(observed, expected)` | Chi-square goodness-of-fit for flat observed/expected lists | `chi_square(observed, expected)` |
| `fisher_exact(a, b, c, d)` | Fisher's exact test (2x2) | `fisher_exact(10, 5, 3, 12)` |

> **Effect sizes for categorical data** are computed inline:
>
> ```text
# Conceptual or diagnostic example; not directly executable.
> # Odds ratio
> let or = (a * d) / (b * c)
>
> # Relative risk
> let rr = (a / (a + b)) / (c / (c + d))
> ```

## Correlation

| Function | Description | Example |
|---|---|---|
| `cor(x, y)` | Pearson correlation | `cor(expr, meth)` |
| `spearman(x, y)` | Spearman rank correlation | `spearman(expr, meth)` |
| `kendall(x, y)` | Kendall tau correlation | `kendall(expr, meth)` |

## Regression

| Function | Description | Example |
|---|---|---|
| `lm(x, y)` | Simple linear regression | `lm(age, expression)` |
| `lm(formula, table)` | Multiple linear regression | `lm(~expression ~ age + sex + batch, data)` |
| `glm(formula, table, family?)` | Generalized linear model | `glm(~outcome ~ age + marker, data, "binomial")` |

Supported GLM families: `"binomial"` (or `"logistic"`), `"gaussian"` (or `"linear"`), and `"poisson"`.

Access model results:

```bio
let age = [34, 41, 52, 59, 63, 70]
let expression = [8.1, 8.8, 9.4, 10.2, 10.6, 11.1]
let model = lm(age, expression)
print("R-squared: " + str(round(model.r_squared, 3)))
let residuals = range(0, len(age))
  |> map(|i| expression[i] - (model.intercept + model.slope * age[i]))
histogram(residuals, {title: "Residual Distribution"})
```

## Multiple Testing Correction

| Function | Description | Example |
|---|---|---|
| `p_adjust(pvals, method)` | Adjust p-values | `p_adjust(pvals, "BH")` |

Supported methods: `"bonferroni"`, `"holm"`, and `"BH"` (Benjamini-Hochberg).

```bio
# Typical genomics workflow: test all genes, then correct
let pvals = genes |> map(|g| ttest(g.ctrl, g.treat).p_value)
let padj = p_adjust(pvals, "BH")
let sig_count = padj |> filter(|p| p < 0.05) |> len()
print("Significant genes (FDR < 0.05): " + str(sig_count))
```

## Dimensionality Reduction and Clustering

| Function | Description | Example |
|---|---|---|
| `pca(data)` | Principal Component Analysis | `pca(expr_matrix)` |
| `kmeans(data, k)` | k-means clustering | `kmeans(data, 3)` |
| `hclust(data, method)` | Hierarchical clustering | `hclust(data, "ward")` |
| `dbscan(data, eps, min_pts)` | DBSCAN clustering | `dbscan(data, 0.5, 5)` |

```bio
# PCA then clustering
let expr_matrix = [
  [8.2, 7.9, 3.1, 2.8],
  [8.6, 8.1, 3.4, 3.0],
  [3.0, 3.3, 8.4, 8.0],
  [2.7, 3.0, 8.8, 8.2]
]
let result = pca(expr_matrix)
pca_plot(matrix(expr_matrix), {title: "Sample PCA"})

# Cluster the same samples into two groups
let clusters = kmeans(expr_matrix, 2)
```

## Statistical Visualization

### SVG Plots (file output)

| Function | Description | Example |
|---|---|---|
| `histogram(data, options)` | Histogram | `histogram(data, {bins: 30, title: "Distribution"})` |
| `density(data, options)` | Kernel density estimate | `density(data, {title: "Density"})` |
| `violin(data, options)` | Violin plot | `violin([g1, g2], {labels: ["A", "B"], title: "Groups"})` |
| `heatmap(table, options)` | Heatmap with optional clustering | `heatmap(matrix, {cluster_rows: true, title: "Expression"})` |
| `volcano(table, options)` | Volcano plot for DE results | `volcano(de_results, {fc_threshold: 1.0, title: "DE"})` |
| `manhattan(table, options)` | Manhattan plot for GWAS | `manhattan(gwas_results, {significance_line: 5e-8, title: "GWAS"})` |
| `qq_plot(p_values, options)` | Observed-versus-expected genomic p-value Q-Q plot | `qq_plot(p_values, {title: "GWAS Q-Q"})` |
| `normal_qq_plot(values, options)` | Observed-versus-theoretical normal Q-Q diagnostic | `normal_qq_plot(values, {title: "Normal Q-Q"})` |
| `forest_plot(table, options)` | Forest plot for meta-analysis | `forest_plot(meta_tbl, {null_value: 0, title: "Meta-analysis"})` |
| `roc_curve(table, options)` | ROC curve | `roc_curve(roc_tbl, {title: "Classifier ROC"})` |
| `pca_plot(result, options)` | PCA scatter plot | `pca_plot(result, {title: "PCA"})` |
| `plot(table, options)` | General line/scatter plot | `plot(tbl, {type: "line", title: "Trend"})` |

### ASCII Plots (terminal output)

| Function | Description | Example |
|---|---|---|
| `scatter(x, y)` | ASCII scatter plot | `scatter(age, expression)` |
| `boxplot(data, options)` | ASCII box plot | `boxplot(table({"Ctrl": g1, "Treat": g2}), {title: "Comparison"})` |
| `bar_chart(data, options?)` | ASCII bar chart from a record or table | `bar_chart(count_table, {label: "group", value: "count"})` |
| `sparkline(data)` | Inline sparkline | `sparkline(timeseries)` |
| `hist(data, options)` | ASCII histogram | `hist(data, {bins: 20})` |

> **Note:** All visualization options are passed as a record (second argument): `fn(data, {key: value, ...})`. Options are always optional — you can call any plot function with just the data argument.

## Resampling and Simulation

BioLang provides building blocks for resampling methods rather than dedicated functions:

```bio
# Bootstrap confidence interval for the median
let data = [2.3, 4.1, 3.7, 5.2, 4.8, 3.1, 6.0, 4.4]
let n_boot = 10000
let boot_medians = range(0, n_boot) |> map(|i| {
  let resample = range(0, len(data)) |> map(|j| data[random_int(0, len(data) - 1)])
  median(resample)
})
let sorted = sort(boot_medians)
let lo = sorted[round(n_boot * 0.025, 0)]
let hi = sorted[round(n_boot * 0.975, 0)]
print("95% CI: [" + str(lo) + ", " + str(hi) + "]")

# Permutation test
let observed_diff = abs(mean(treated) - mean(control))
let combined = treated + control
let n_perm = 10000
let null_diffs = range(0, n_perm) |> map(|i| {
  let shuffled = shuffle(combined)
  let perm_a = shuffled[0..len(treated)]
  let perm_b = shuffled[len(treated)..len(combined)]
  abs(mean(perm_a) - mean(perm_b))
})
let p_value = len(null_diffs |> filter(|d| d >= observed_diff)) / n_perm
```

## Utility Functions

| Function | Description | Example |
|---|---|---|
| `shuffle(x)` | Random permutation | `shuffle(labels)` |
| `random_int(a, b)` | Random integer in [a, b] | `random_int(0, 99)` |
| `sort(x)` | Sort values ascending | `sort([3, 1, 2])` → `[1, 2, 3]` |
| `range(a, b)` | Integer sequence [a, b) | `range(0, 10)` → `[0, 1, ..., 9]` |
| `len(x)` | Number of elements | `len([1, 2, 3])` → `3` |
| `str(x)` | Convert to string | `str(42)` → `"42"` |
| `table(rows, cols, fill)` | Create a table | `table(10, 3, 0)` |

## Power Analysis

BioLang provides `power_analysis(effect_size, alpha, power?, n?)` for an approximate two-sample t-test calculation. Omitting `n` returns the required per-group sample size:

```bio
# Sample size for two-sample t-test
# H0: mu1 = mu2, H1: mu1 != mu2
let effect_size = 0.5     # Cohen's d
let alpha = 0.05
let power = 0.80
let planned = power_analysis(effect_size, alpha, power)
print("Required n per group: " + str(planned.n))

# Cohen's d (inline)
let d = abs(mean(a) - mean(b)) / sqrt(((len(a) - 1) * variance(a) + (len(b) - 1) * variance(b)) / (len(a) + len(b) - 2))
```

## Bayesian Methods

BioLang supports Bayesian analysis through conjugate update formulas computed inline:

```bio
# Beta-Binomial conjugate update
# Prior: Beta(alpha, beta); Data: k successes in n trials
# Posterior: Beta(alpha + k, beta + n - k)
let prior_a = 1.0
let prior_b = 1.0
let k = 15
let n = 20
let post_a = prior_a + k
let post_b = prior_b + (n - k)
let post_mean = post_a / (post_a + post_b)
print("Posterior mean: " + str(round(post_mean, 3)))

# Beta quantiles are not currently a BioLang builtin. Use a validated external
# implementation when a Beta posterior credible interval is required.

# Normal-Normal conjugate update
# Prior: N(mu0, sigma0^2); Data: n observations with mean x_bar and known sigma
let prior_mu = 0.0
let prior_prec = 1.0 / (10.0 ** 2)   # prior precision = 1/sigma0^2
let data_prec = len(data) / (stdev(data) ** 2)
let post_prec = prior_prec + data_prec
let post_mu = (prior_prec * prior_mu + data_prec * mean(data)) / post_prec
let post_sd = sqrt(1.0 / post_prec)
```

## Survival Analysis

BioLang provides censoring-aware log-rank and Cox functions. Do not replace a survival comparison with an ordinary t-test on observed times.

```bio
# events are Bool or 0/1 lists, where true/1 means the event was observed.
let result = log_rank_test(arm1_times, arm1_events, arm2_times, arm2_events)
print("p-value: " + str(round(result.p_value, 4)))
```

Use `cox_ph(time, event, covariates)` when covariate-adjusted proportional-hazards modelling is appropriate. A ratio of raw medians is not a hazard ratio.
