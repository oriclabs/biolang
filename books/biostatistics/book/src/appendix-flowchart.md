# Appendix B: Statistical Decision Flowchart

> *The hardest part of statistics is choosing the right test. This appendix is your map.*

When you have data and a question, the path to the correct statistical test follows a decision tree based on three things: what kind of data you have, how many groups you are comparing, and what assumptions your data meets. This appendix lays out that tree in a series of tables you can consult whenever you are unsure.

## The Master Decision Guide

Start here. Find your question type, then follow the table to the right test.

| What are you asking? | Go to section |
|---|---|
| Are two groups different? | [Comparing Two Groups](#comparing-two-groups) |
| Are three or more groups different? | [Comparing Multiple Groups](#comparing-multiple-groups) |
| Are two variables related? | [Associations and Correlations](#associations-and-correlations) |
| Does one variable predict another? | [Regression](#regression) |
| Is there a relationship in categorical data? | [Categorical Data](#categorical-data) |
| How long until an event occurs? | [Time-to-Event Analysis](#time-to-event-analysis) |
| Do I need to reduce dimensionality? | [Dimensionality Reduction](#dimensionality-reduction) |
| Do I need to group similar observations? | [Clustering](#clustering) |

## Comparing Two Groups

Use this when you have one outcome variable and two groups (e.g., control vs. treated, male vs. female, wildtype vs. knockout).

### Step 1: What type is your outcome variable?

| Outcome type | Next step |
|---|---|
| Continuous (expression level, concentration, weight) | Step 2 |
| Counts (number of mutations, colony counts) | Consider Poisson or negative binomial test |
| Binary (alive/dead, present/absent) | See [Categorical Data](#categorical-data) |
| Ordinal (severity scale, Likert scores) | Use non-parametric test |

### Step 2: Are the observations paired or independent?

| Design | Paired? | Example |
|---|---|---|
| Same subjects measured before and after treatment | Yes | Pre/post drug expression |
| Different subjects in each group | No | Treated vs. control mice |
| Matched pairs (e.g., tumor vs. adjacent normal from same patient) | Yes | Tumor/normal tissue pairs |

### Step 3: Choose your test

| Paired? | Normal distribution? | Equal variance? | Test | BioLang |
|---|---|---|---|---|
| No | Yes | Yes | Student's t-test | `ttest(a, b)` |
| No | Yes | No | Welch's t-test | `ttest(a, b)` |
| No | No | — | Mann-Whitney U | `wilcoxon(a, b)` |
| Yes | Yes | — | Paired t-test | `ttest_paired(a, b)` |
| Yes | No | — | Wilcoxon signed-rank | `wilcoxon(a, b)` |

> **Key insight:** Welch's t-test is almost always preferred over Student's t-test because it does not assume equal variances. When variances are actually equal, Welch's test gives nearly identical results. When they are not, Student's test can be dangerously wrong. BioLang uses Welch's by default.

### How to check normality

```bio
let data = [2.3, 4.1, 3.7, 5.2, 4.8, 3.1, 6.0, 4.4]

# Visual check — Q-Q plot (best for small samples)
qq_plot(data, {title: "Normality Check"})
```

> **Common pitfall:** With small samples (n < 30), normality tests have low power and may fail to reject normality even when the data is non-normal. With large samples (n > 5000), normality tests reject normality for trivially small deviations. Use Q-Q plots as a visual supplement.

## Comparing Multiple Groups

Use this when you have three or more groups (e.g., three drug doses, four tissue types, five time points).

| Normal? | Equal variance? | Design | Test | BioLang |
|---|---|---|---|---|
| Yes | Yes | Independent groups | One-way ANOVA | `anova(groups)` |
| Yes | No | Independent groups | Welch's ANOVA | `anova(groups)` |
| No | — | Independent groups | Kruskal-Wallis | `anova(groups)` |
| Yes | — | Repeated measures | Repeated-measures ANOVA | `anova(groups)` |
| No | — | Repeated measures | Friedman test | `anova(groups)` |
| Yes | — | Two factors | Two-way ANOVA | `anova(groups)` |

### Post-hoc Tests

When ANOVA is significant, you know *some* groups differ but not *which* ones. Use post-hoc tests:

| Test | When to use | BioLang |
|---|---|---|
| Tukey HSD | All pairwise comparisons | Pairwise `ttest()` + `p_adjust(pvals, "bonferroni")` |
| Dunnett | Compare all groups to a single control | Pairwise `ttest()` vs control + `p_adjust()` |
| Dunn test | Post-hoc for Kruskal-Wallis | Pairwise `wilcoxon()` + `p_adjust()` |
| Bonferroni-corrected pairwise | Conservative, any design | Pairwise `ttest()` + `p_adjust(pvals, "bonferroni")` |

> **Key insight:** ANOVA is an *omnibus* test — it tells you that at least one group differs, but not which one. Always follow a significant ANOVA with post-hoc comparisons. Reporting only the ANOVA p-value is incomplete.

## Associations and Correlations

Use this when you have two continuous variables and want to know if they are related (e.g., gene expression vs. methylation, age vs. telomere length).

| Data characteristics | Test | BioLang |
|---|---|---|
| Both variables roughly normal, linear relationship | Pearson correlation | `cor(x, y)` |
| Non-normal or ordinal data, monotonic relationship | Spearman correlation | `spearman(x, y)` |
| Ordinal data with ties | Kendall tau | `kendall(x, y)` |
| Partial correlation (controlling for a third variable) | Partial correlation | `cor(x, y)` after residualizing on z |

### Interpreting Correlation Strength

| |r| value | Interpretation |
|---|---|
| 0.0 - 0.1 | Negligible |
| 0.1 - 0.3 | Weak |
| 0.3 - 0.5 | Moderate |
| 0.5 - 0.7 | Strong |
| 0.7 - 1.0 | Very strong |

> **Common pitfall:** Correlation does not imply causation, but more subtly, *absence* of Pearson correlation does not imply absence of relationship. Pearson only detects linear associations. Two variables can have a perfect quadratic relationship with r = 0. Always plot your data.

## Categorical Data

Use this when both your variables are categorical (e.g., mutation status vs. disease outcome, genotype vs. phenotype).

| Design | Expected cell counts | Test | BioLang |
|---|---|---|---|
| 2x2 table, large samples | All expected >= 5 | Chi-square test | `chi_square(observed, expected)` |
| 2x2 table, small samples | Any expected < 5 | Fisher's exact test | `fisher_exact(a, b, c, d)` |
| Larger than 2x2 | All expected >= 5 | Chi-square test | `chi_square(observed, expected)` |
| Larger than 2x2, small samples | Any expected < 5 | Fisher-Freeman-Halton | `fisher_exact(a, b, c, d)` |
| Paired categorical data | — | McNemar's test | `chi_square(observed, expected)` |
| Trend across ordered categories | — | Cochran-Armitage trend test | `chi_square(observed, expected)` |

### Measures of Association for Categorical Data

| Measure | Use case | BioLang |
|---|---|---|
| Odds ratio | 2x2 tables, case-control studies | `(a*d) / (b*c)` (inline) |
| Relative risk | 2x2 tables, cohort studies | `(a/(a+b)) / (c/(c+d))` (inline) |
| Cramer's V | Any size contingency table | Compute from chi-square statistic |

## Regression

Use this when you want to predict an outcome from one or more predictor variables.

| Outcome type | Number of predictors | Test | BioLang |
|---|---|---|---|
| Continuous | 1 | Simple linear regression | `lm(y, x)` |
| Continuous | Multiple | Multiple linear regression | `lm(y, [x1, x2, x3])` |
| Binary (0/1) | Any | Logistic regression | `glm("y ~ x", table, "binomial")` |
| Count | Any | Poisson regression | `glm("y ~ x", table, "poisson")` |
| Count, overdispersed | Any | Negative binomial regression | `glm("y ~ x", table, "negbin")` |
| Continuous, clustered data | Any | Mixed-effects model | `lm(y, x)` (per group) |

### Checking Regression Assumptions

```bio
let model = lm(expression, [age, sex, batch])

# Check residuals with Q-Q plot
let residuals = model.residuals
qq_plot(residuals, {title: "Residual Normality Check"})
print("R-squared: " + str(round(model.r_squared, 3)))
```

> **Common pitfall:** Adding more predictors always improves R-squared, even if the predictors are noise. Use adjusted R-squared or AIC/BIC for model comparison. Report both R-squared and adjusted R-squared.

## Time-to-Event Analysis

Use this when your outcome is the time until something happens (death, relapse, response) and some observations are censored (the event has not yet occurred).

| Question | Method | BioLang |
|---|---|---|
| Estimate survival curve | Kaplan-Meier | Sort event times, compute stepwise survival |
| Compare survival between two groups | Log-rank test | `ttest(times_a, times_b)` as proxy |
| Compare survival, multiple groups | Log-rank test | `anova([group1_times, group2_times, ...])` |
| Adjust for covariates | Cox proportional hazards | `lm(time, [covariates])` |
| Estimate median survival | From sorted times | `sort(times)[len(times) / 2]` |

> **Clinical relevance:** In clinical trials, the hazard ratio from a Cox model is the primary efficacy endpoint. A hazard ratio of 0.65 means the treatment group has a 35% lower instantaneous risk of the event at any time point. Always report the 95% confidence interval alongside the point estimate.

## Dimensionality Reduction

Use this when you have many variables (genes, proteins, metabolites) and want to find the main patterns.

| Goal | Method | BioLang |
|---|---|---|
| Find linear combinations that maximize variance | PCA | `pca(data)` |
| Visualize PCA results | PCA plot | `pca_plot(result, {title: "PCA"})` |

> **Key insight:** PCA is deterministic — you get the same answer every time. t-SNE and UMAP are stochastic — different runs give different layouts. Always set a random seed before running stochastic methods for reproducibility.

## Clustering

Use this when you want to group similar observations (samples, genes, cells) together.

| What you know | Method | BioLang |
|---|---|---|
| Number of clusters (k) | k-means | `kmeans(data, 3)` |
| Want a hierarchy of clusters | Hierarchical clustering | `hclust(data, "ward")` |
| Irregular cluster shapes | DBSCAN | `dbscan(data, 0.5, 5)` |
| Want to estimate k | Silhouette / Elbow | Loop over k, compute `kmeans(data, k).silhouette` |

## Multiple Testing Correction

Use this whenever you perform more than one statistical test on the same dataset.

| Method | Controls | Strictness | BioLang |
|---|---|---|---|
| Bonferroni | Family-wise error rate | Most conservative | `p_adjust(pvals, "bonferroni")` |
| Holm | Family-wise error rate | Less conservative | `p_adjust(pvals, "holm")` |
| Benjamini-Hochberg | False discovery rate | Moderate | `p_adjust(pvals, "BH")` |
| Benjamini-Yekutieli | FDR under dependence | Conservative FDR | `p_adjust(pvals, "BY")` |
| Permutation | Empirical null | Gold standard | Inline loop with `shuffle()` |

> **Key insight:** For genomics (testing thousands of genes), Benjamini-Hochberg FDR correction at q = 0.05 is the standard. Bonferroni is too conservative for genome-wide studies — it controls the family-wise error rate, which is the wrong quantity when you expect hundreds of true positives.

## Quick Reference: Common Biological Scenarios

| Scenario | Recommended test | BioLang |
|---|---|---|
| Gene expression, treated vs. control | Welch's t-test | `ttest(treated, control)` |
| Gene expression across 4 tissues | One-way ANOVA | `anova([tissue1, tissue2, tissue3, tissue4])` |
| Mutation frequency in cases vs. controls | Fisher's exact test | `fisher_exact(a, b, c, d)` |
| Survival by treatment arm | Compare survival times | `ttest(arm1_times, arm2_times)` |
| 20,000 gene differential expression | t-test + BH correction | `p_adjust(pvals, "BH")` |
| Sample clustering from RNA-seq | PCA + hierarchical clustering | `pca(data)` then `hclust(scores)` |
| Correlation: expression vs. methylation | Spearman (often non-linear) | `spearman(expr, meth)` |
| GWAS: genotype vs. phenotype | Logistic regression + BH | `glm("pheno ~ geno", tbl, "binomial")` |
| Clinical outcome predictors | Regression model | `lm(outcome, [age, stage, treatment])` |
| Sample size for planned experiment | Power analysis | Compute with `qnorm()` and effect size |
