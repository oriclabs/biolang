# Appendix E: BioLang Statistics Quick Reference

> *Every statistical function in BioLang, organized for quick lookup.*

This appendix lists BioLang's statistical builtins by category. For each function, you will find the signature, a brief description, and a one-liner example. Functions are listed within each category in roughly the order you would learn them.

## Descriptive Statistics

| Function | Description | Example |
|---|---|---|
| `mean(x)` | Arithmetic mean | `mean([2, 4, 6])` → `4.0` |
| `median(x)` | Middle value | `median([1, 3, 7])` → `3.0` |
| `sd(x)` | Sample standard deviation | `sd([2, 4, 6])` → `2.0` |
| `var(x)` | Sample variance | `var([2, 4, 6])` → `4.0` |
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

## Probability Distributions

Each distribution has four functions following the `d/p/q/r` convention: `d` (density/mass), `p` (cumulative probability), `q` (quantile/inverse CDF), `r` (random samples). All parameters are positional.

### Continuous

```bio
# Normal: dnorm(x, mu, sigma), pnorm(x, mu, sigma), qnorm(p, mu, sigma), rnorm(n, mu, sigma)
dnorm(0, 0, 1)        # Density at x=0 for standard normal
pnorm(1.96, 0, 1)     # P(X <= 1.96) ≈ 0.975
qnorm(0.975, 0, 1)    # Quantile at p=0.975 ≈ 1.96
rnorm(100, 0, 1)      # Generate 100 standard normal values

# Student's t: dt(x, df), pt(x, df), qt(p, df), rt(n, df)
dt(0, 10)
pt(2.228, 10)
qt(0.975, 10)
rt(100, 10)

# F: df(x, df1, df2), pf(x, df1, df2), qf(p, df1, df2), rf(n, df1, df2)
df(1.5, 5, 20)
pf(3.0, 5, 20)
qf(0.95, 5, 20)
rf(100, 5, 20)

# Chi-square: dchisq(x, df), pchisq(x, df), qchisq(p, df), rchisq(n, df)
dchisq(5.0, 5)
pchisq(11.07, 5)
qchisq(0.95, 5)
rchisq(100, 5)

# Beta: dbeta(x, alpha, beta), pbeta(x, alpha, beta), qbeta(p, alpha, beta), rbeta(n, alpha, beta)
dbeta(0.3, 2, 5)
pbeta(0.5, 2, 5)
qbeta(0.95, 2, 5)
rbeta(100, 2, 5)

# Gamma: dgamma(x, shape, rate), pgamma(x, shape, rate), qgamma(p, shape, rate), rgamma(n, shape, rate)
dgamma(1.0, 2, 1)
pgamma(3.0, 2, 1)
qgamma(0.95, 2, 1)
rgamma(100, 2, 1)

# Exponential: dexp(x, rate), pexp(x, rate), qexp(p, rate), rexp(n, rate)
dexp(1.0, 0.5)
pexp(2.0, 0.5)
qexp(0.95, 0.5)
rexp(100, 0.5)

# Log-Normal: dlnorm(x, mu, sigma), plnorm(x, mu, sigma), qlnorm(p, mu, sigma), rlnorm(n, mu, sigma)
dlnorm(1.0, 0, 1)
plnorm(2.0, 0, 1)
qlnorm(0.95, 0, 1)
rlnorm(100, 0, 1)

# Uniform: dunif(x, min, max), punif(x, min, max), qunif(p, min, max), runif(n, min, max)
dunif(0.5, 0, 1)
punif(0.7, 0, 1)
qunif(0.5, 0, 1)
runif(100, 0, 1)
```

### Discrete

```bio
# Binomial: dbinom(k, n, p), pbinom(k, n, p), qbinom(q, n, p), rbinom(size, n, p)
dbinom(10, 20, 0.5)       # P(X = 10)
pbinom(10, 20, 0.5)       # P(X <= 10)
qbinom(0.5, 20, 0.5)      # Smallest k with P(X <= k) >= 0.5
rbinom(100, 20, 0.5)      # Generate 100 random values

# Poisson: dpois(k, lambda), ppois(k, lambda), qpois(q, lambda), rpois(n, lambda)
dpois(5, 5.0)
ppois(7, 5.0)
qpois(0.95, 5.0)
rpois(100, 5.0)

# Negative Binomial: dnbinom(k, mu, size), pnbinom(k, mu, size), qnbinom(q, mu, size), rnbinom(n, mu, size)
dnbinom(8, 10, 5)
pnbinom(15, 10, 5)
qnbinom(0.95, 10, 5)
rnbinom(100, 10, 5)

# Hypergeometric: dhyper(k, K, N_minus_K, n), phyper(k, K, N_minus_K, n), qhyper(q, K, N_minus_K, n), rhyper(size, K, N_minus_K, n)
dhyper(5, 200, 19800, 500)
phyper(5, 200, 19800, 500)
qhyper(0.95, 200, 19800, 500)
rhyper(100, 200, 19800, 500)
```

## Hypothesis Tests

### Comparing Two Groups

| Function | Description | Example |
|---|---|---|
| `ttest(a, b)` | Welch's two-sample t-test | `ttest(ctrl, treat)` |
| `ttest_paired(a, b)` | Paired t-test | `ttest_paired(before, after)` |
| `ttest_one(x, mu)` | One-sample t-test | `ttest_one(diffs, 0)` |
| `wilcoxon(a, b)` | Wilcoxon rank-sum / signed-rank test | `wilcoxon(ctrl, treat)` |

### Comparing Multiple Groups

| Function | Description | Example |
|---|---|---|
| `anova(groups)` | One-way ANOVA (auto-detects Welch/Kruskal-Wallis) | `anova([g1, g2, g3])` |

> **Post-hoc comparisons:** Follow a significant ANOVA with pairwise tests and p-value correction:
>
> ```bio
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
| `chi_square(observed, expected)` | Chi-square test | `chi_square(observed, expected)` |
| `fisher_exact(a, b, c, d)` | Fisher's exact test (2x2) | `fisher_exact(10, 5, 3, 12)` |

> **Effect sizes for categorical data** are computed inline:
>
> ```bio
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
| `lm(y, x)` | Simple linear regression | `lm(expression, age)` |
| `lm(y, [x1, x2, ...])` | Multiple linear regression | `lm(expression, [age, sex, batch])` |
| `glm(formula, table, family)` | Generalized linear model | `glm("y ~ x", data, "binomial")` |

Supported GLM families: `"binomial"` (logistic), `"poisson"`, `"negbin"` (negative binomial).

Access model results:

```bio
let model = lm(expression, [age, sex, batch])
print("R-squared: " + str(round(model.r_squared, 3)))
let residuals = model.residuals
qq_plot(residuals, {title: "Residual Normality Check"})
```

## Multiple Testing Correction

| Function | Description | Example |
|---|---|---|
| `p_adjust(pvals, method)` | Adjust p-values | `p_adjust(pvals, "BH")` |

Supported methods: `"bonferroni"`, `"holm"`, `"BH"` (Benjamini-Hochberg), `"BY"` (Benjamini-Yekutieli).

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
let result = pca(expr_matrix)
pca_plot(result, {title: "Sample PCA"})

# Estimate optimal k via silhouette
let scores = range(2, 10) |> map(|k| {
  let km = kmeans(data, k)
  km.silhouette
})
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
| `qq_plot(data, options)` | Q-Q plot (normality check) | `qq_plot(residuals, {title: "Normality Check"})` |
| `forest_plot(table, options)` | Forest plot for meta-analysis | `forest_plot(meta_tbl, {null_value: 0, title: "Meta-analysis"})` |
| `roc_curve(table, options)` | ROC curve | `roc_curve(roc_tbl, {title: "Classifier ROC"})` |
| `pca_plot(result, options)` | PCA scatter plot | `pca_plot(result, {title: "PCA"})` |
| `plot(table, options)` | General line/scatter plot | `plot(tbl, {type: "line", title: "Trend"})` |

### ASCII Plots (terminal output)

| Function | Description | Example |
|---|---|---|
| `scatter(x, y, options)` | ASCII scatter plot | `scatter(age, expr, {xlabel: "Age", ylabel: "Expr"})` |
| `boxplot(data, options)` | ASCII box plot | `boxplot(table({"Ctrl": g1, "Treat": g2}), {title: "Comparison"})` |
| `bar_chart(labels, values, options)` | ASCII bar chart | `bar_chart(names, counts, {title: "Counts"})` |
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

BioLang does not have dedicated power analysis functions. Compute sample sizes using distribution quantiles and effect size formulas:

```bio
# Sample size for two-sample t-test
# H0: mu1 = mu2, H1: mu1 != mu2
let effect_size = 0.5     # Cohen's d
let alpha = 0.05
let power = 0.80
let z_alpha = qnorm(1 - alpha / 2, 0, 1)    # 1.96
let z_beta = qnorm(power, 0, 1)              # 0.842
let n_per_group = round(2 * ((z_alpha + z_beta) / effect_size) ** 2, 0)
print("Required n per group: " + str(n_per_group))

# Cohen's d (inline)
let d = abs(mean(a) - mean(b)) / sqrt(((len(a) - 1) * var(a) + (len(b) - 1) * var(b)) / (len(a) + len(b) - 2))
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

# 95% credible interval via Beta quantiles
let ci_lo = qbeta(0.025, post_a, post_b)
let ci_hi = qbeta(0.975, post_a, post_b)
print("95% CI: [" + str(round(ci_lo, 3)) + ", " + str(round(ci_hi, 3)) + "]")

# Normal-Normal conjugate update
# Prior: N(mu0, sigma0^2); Data: n observations with mean x_bar and known sigma
let prior_mu = 0.0
let prior_prec = 1.0 / (10.0 ** 2)   # prior precision = 1/sigma0^2
let data_prec = len(data) / (sd(data) ** 2)
let post_prec = prior_prec + data_prec
let post_mu = (prior_prec * prior_mu + data_prec * mean(data)) / post_prec
let post_sd = sqrt(1.0 / post_prec)
```

## Survival Analysis

BioLang provides basic building blocks for survival analysis. For simple comparisons:

```bio
# Compare survival times between two groups
let result = ttest(arm1_times, arm2_times)
print("p-value: " + str(round(result.p_value, 4)))

# Median survival
let med_a = sort(arm1_times)[len(arm1_times) / 2]
let med_b = sort(arm2_times)[len(arm2_times) / 2]
print("Median survival — Arm A: " + str(med_a) + ", Arm B: " + str(med_b))

# Approximate hazard ratio from median ratio
let hr = med_b / med_a

# Regression on survival times
let model = lm(survival_time, [age, stage, treatment])
```
