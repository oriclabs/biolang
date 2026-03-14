# Appendix D: Glossary

> *Statistics has its own language. This glossary translates it into plain English, with biological context.*

Terms are listed alphabetically. Cross-references appear in *italics*.

---

**Adjusted R-squared.** A modified version of *R-squared* that penalizes the addition of unnecessary predictors. Unlike R-squared, adjusted R-squared can decrease when a non-informative variable is added to a model. Preferred over R-squared for comparing models with different numbers of predictors.

**Alpha (significance level).** The threshold you set before testing for declaring a result statistically significant. Conventionally 0.05 (5%), meaning you accept a 5% chance of a *Type I error*. In genome-wide studies, often set much lower (5 x 10^-8 for GWAS).

**Alternative hypothesis (H1).** The hypothesis that there is an effect or a difference. The complement of the *null hypothesis*. Example: "Drug-treated tumors are smaller than untreated tumors."

**ANOVA (Analysis of Variance).** A test for differences in means across three or more groups. Extends the *t-test* to multiple groups by comparing between-group variance to within-group variance. Produces an *F-statistic*.

**AUC (Area Under the Curve).** In the context of ROC analysis, the probability that a randomly chosen positive case ranks higher than a randomly chosen negative case. An AUC of 0.5 is random guessing; 1.0 is perfect classification.

**Batch effect.** Systematic technical variation introduced by processing samples in different batches, on different days, or with different reagents. A major confounder in genomics. Must be addressed through experimental design (randomization) or statistical correction (ComBat, limma).

**Bayesian statistics.** An approach to inference that combines prior knowledge with observed data to produce *posterior* probability distributions. Contrasts with *frequentist* statistics, which relies on long-run frequencies. Allows statements like "there is a 95% probability the true effect lies in this interval."

**Benjamini-Hochberg (BH).** A *multiple testing correction* method that controls the *false discovery rate* (FDR) rather than the *family-wise error rate*. Less conservative than *Bonferroni*. The standard correction in genomics.

**Beta (Type II error rate).** The probability of failing to reject the *null hypothesis* when it is actually false. *Power* equals 1 - beta. A beta of 0.2 means a 20% chance of missing a real effect.

**Bias.** Systematic deviation of an estimate from the true value. Distinct from random error (*variance*). An estimator can be precise (low variance) but biased (consistently wrong in one direction).

**Bimodal distribution.** A distribution with two peaks. In gene expression, bimodality often indicates two distinct cell populations or states (e.g., expressed vs. silenced genes).

**Blinding.** Concealing group assignments from participants, clinicians, or analysts to prevent bias. Single-blind: participants do not know their group. Double-blind: neither participants nor clinicians know.

**Bonferroni correction.** The simplest *multiple testing correction*: multiply each p-value by the number of tests (or equivalently, divide alpha by the number of tests). Controls the *family-wise error rate* but is very conservative for large numbers of tests.

**Bootstrap.** A *resampling* method that estimates the sampling distribution of a statistic by repeatedly drawing samples with replacement from the observed data. Does not assume any particular parametric distribution.

**Box plot.** A visualization showing the median, quartiles, and outliers of a distribution. The box spans the interquartile range (IQR); whiskers extend to 1.5 * IQR; points beyond are outliers.

**Categorical variable.** A variable that takes a limited set of discrete values (e.g., genotype: AA, AB, BB; tissue type: liver, brain, kidney). Contrasts with *continuous variable*.

**CDF (Cumulative Distribution Function).** The probability that a random variable takes a value less than or equal to x. F(x) = P(X <= x). Ranges from 0 to 1.

**Central limit theorem.** The theorem stating that the sampling distribution of the mean approaches a *normal distribution* as sample size increases, regardless of the shape of the original population distribution. The foundation of most parametric tests.

**Chi-square test.** A test for association between two *categorical variables*. Compares observed frequencies in a contingency table to expected frequencies under independence. Requires expected cell counts >= 5; otherwise use *Fisher's exact test*.

**Clinical significance.** A difference large enough to matter in practice, regardless of *statistical significance*. A drug that lowers blood pressure by 0.5 mmHg might be statistically significant with a large enough sample but clinically irrelevant.

**Clustering.** Grouping observations (samples, genes, cells) by similarity. Common methods: *k-means* (requires specifying k), *hierarchical* (produces a dendrogram), *DBSCAN* (finds clusters of arbitrary shape).

**Coefficient of variation (CV).** The ratio of the *standard deviation* to the *mean*, expressed as a percentage. Useful for comparing variability across measurements with different scales. CV = (SD / mean) * 100%.

**Confidence interval (CI).** A range of values that, if the experiment were repeated many times, would contain the true parameter value in the stated percentage of cases. A 95% CI does *not* mean "95% probability the true value is in this range" (that is the Bayesian *credible interval*).

**Confounding variable.** A variable that influences both the independent and dependent variables, creating a spurious association. Age confounds many gene expression studies because both expression and disease risk change with age.

**Continuous variable.** A variable that can take any value within a range (e.g., expression level, concentration, temperature). Contrasts with *categorical variable*.

**Correlation.** A measure of linear association between two variables. *Pearson* correlation (r) measures linear relationships; *Spearman* correlation (rho) measures monotonic relationships. Ranges from -1 to +1.

**Cox proportional hazards.** A regression model for *survival analysis* that estimates the effect of covariates on the hazard (instantaneous risk) of an event. Does not assume a particular survival distribution. Reports *hazard ratios*.

**Credible interval.** The *Bayesian* analog of a *confidence interval*. A 95% credible interval means there is a 95% probability the true parameter lies within the interval, given the data and prior. Requires specifying a *prior distribution*.

**Degrees of freedom (df).** The number of independent values that can vary in a statistical calculation. For a t-test with n1 and n2 observations, df is approximately n1 + n2 - 2 (for Student's) or a more complex formula (for Welch's).

**Differential expression.** A gene is differentially expressed if its expression level differs significantly between conditions (e.g., treated vs. control). Typically assessed with a *negative binomial* model (DESeq2, edgeR) and *BH correction*.

**Effect size.** A measure of the magnitude of a difference or association, independent of sample size. Common measures: *Cohen's d* (standardized mean difference), *odds ratio*, *hazard ratio*, *R-squared*.

**Cohen's d.** A standardized *effect size* for the difference between two means: d = (mean1 - mean2) / pooled_SD. Conventions: 0.2 = small, 0.5 = medium, 0.8 = large.

**Eta-squared.** An *effect size* for *ANOVA* representing the proportion of total variance explained by the group factor. Analogous to *R-squared* in regression.

**Empirical distribution.** The distribution derived directly from observed data, without assuming a parametric form. Used in *permutation tests* and *bootstrap* methods.

**Enrichment analysis.** Testing whether a set of genes (e.g., differentially expressed genes) contains more members of a particular pathway or GO category than expected by chance. Uses the *hypergeometric distribution* or *GSEA*.

**False discovery rate (FDR).** The expected proportion of rejected null hypotheses that are false positives. Controlled by *Benjamini-Hochberg* correction. At FDR = 0.05, you expect 5% of your "significant" findings to be false.

**Family-wise error rate (FWER).** The probability of making at least one *Type I error* across all tests. Controlled by *Bonferroni* and *Holm* corrections. More conservative than *FDR* control.

**Fisher's exact test.** A test for association in 2x2 contingency tables that computes the exact probability under the *hypergeometric distribution*. Preferred over *chi-square* when sample sizes are small or expected cell counts are below 5.

**Fold change.** The ratio of a value in one condition to the value in another. A fold change of 2 means doubled; 0.5 means halved. Often reported on the log2 scale: log2(FC) = 1 means doubled.

**Forest plot.** A visualization for *meta-analysis* showing effect sizes and confidence intervals from multiple studies, plus a combined estimate. Each study is a horizontal line; the diamond shows the pooled effect.

**Frequentist statistics.** The dominant framework in biostatistics, based on long-run frequencies. P-values, confidence intervals, and hypothesis tests are frequentist concepts. Contrasts with *Bayesian statistics*.

**GSEA (Gene Set Enrichment Analysis).** A method that tests whether a predefined set of genes shows concordant differences between conditions. Unlike overrepresentation analysis, GSEA uses the full ranked gene list rather than an arbitrary significance cutoff.

**GWAS (Genome-Wide Association Study).** A study design that tests hundreds of thousands to millions of genetic variants for association with a phenotype. Requires stringent *multiple testing correction* (typically p < 5 x 10^-8).

**Hazard ratio (HR).** The ratio of hazard rates between two groups in *survival analysis*. HR = 0.7 means the treatment group has 30% lower instantaneous risk. HR = 1 means no difference. Estimated by *Cox proportional hazards* regression.

**Heteroscedasticity.** Unequal variance across groups or across the range of a predictor. Violates assumptions of standard *t-tests* and *linear regression*. Detected by residual plots. Addressed by Welch's tests or robust standard errors.

**Hierarchical clustering.** A *clustering* method that builds a tree (dendrogram) by iteratively merging (agglomerative) or splitting (divisive) clusters. Common linkage methods: ward, complete, average, single.

**Holm correction.** A step-down *multiple testing correction* that is uniformly more powerful than *Bonferroni* while still controlling the *FWER*. Rejects the smallest p-value at alpha/m, the next at alpha/(m-1), and so on.

**Homoscedasticity.** Equal variance across groups or across the range of a predictor. An assumption of Student's *t-test* and standard *linear regression*.

**Hypothesis testing.** A formal procedure for deciding between two competing hypotheses (*null* and *alternative*) based on observed data. Produces a *test statistic* and *p-value*.

**Interquartile range (IQR).** The range between the 25th and 75th percentiles. Contains the middle 50% of the data. A robust measure of spread, less sensitive to outliers than *standard deviation*.

**Kaplan-Meier estimator.** A non-parametric method for estimating survival probabilities over time, accounting for censored observations. Produces the familiar step-function survival curve.

**k-means clustering.** A *clustering* algorithm that partitions n observations into k clusters by minimizing within-cluster variance. Requires specifying k in advance. Sensitive to initialization; run multiple times.

**Kruskal-Wallis test.** The non-parametric alternative to one-way *ANOVA*. Tests whether multiple groups have the same distribution. Based on ranks rather than raw values.

**Linear regression.** A model that predicts a continuous outcome as a linear function of one or more predictors: y = beta_0 + beta_1 * x_1 + ... + epsilon. Assumes normally distributed residuals and constant variance.

**Log-rank test.** A test for comparing *survival curves* between two or more groups. Tests the *null hypothesis* that the groups have equal hazard functions. The standard test for comparing *Kaplan-Meier* curves.

**Logistic regression.** A *regression* model for binary outcomes (0/1, yes/no, case/control). Models the log-odds of the outcome as a linear function of predictors. Reports *odds ratios*.

**Manhattan plot.** A visualization for *GWAS* results showing -log10(p-value) vs. genomic position. Significant associations appear as tall peaks above the genome-wide significance line. Named for its skyline-like appearance.

**Mann-Whitney U test.** A non-parametric alternative to the two-sample *t-test*. Tests whether one group tends to have larger values than the other. Based on ranks. Also called the Wilcoxon rank-sum test.

**Mean.** The arithmetic average. Sum of all values divided by the number of values. Sensitive to outliers. For skewed data, the *median* is often more representative.

**Median.** The middle value when data is sorted. Robust to outliers. Preferred over *mean* for skewed distributions. The 50th percentile.

**Meta-analysis.** A statistical method for combining results from multiple independent studies to produce a single pooled estimate. Uses weighted averages based on study precision. Visualized with *forest plots*.

**Mixed-effects model.** A *regression* model that includes both fixed effects (variables of interest) and random effects (grouping variables like patient, batch, or site). Accounts for non-independence in hierarchical or repeated-measures data.

**Mode.** The most frequently occurring value (discrete data) or the peak of the density curve (continuous data). A distribution can be *bimodal* or multimodal.

**Multiple testing correction.** Adjusting p-values or significance thresholds when performing many simultaneous tests to control the overall error rate. Methods include *Bonferroni*, *Holm*, *Benjamini-Hochberg*, and *permutation* testing.

**Negative binomial distribution.** A *discrete distribution* for count data that allows the variance to exceed the mean (*overdispersion*). The standard model for RNA-seq differential expression (DESeq2, edgeR).

**Non-parametric test.** A statistical test that does not assume a specific parametric distribution (e.g., normality). Examples: *Mann-Whitney U*, *Kruskal-Wallis*, *Wilcoxon signed-rank*. Generally less powerful than parametric tests when assumptions hold.

**Normal distribution.** The symmetric, bell-shaped distribution described by a *mean* and *standard deviation*. Many biological measurements are approximately normal, especially after log transformation. The basis for most parametric tests via the *central limit theorem*.

**Null hypothesis (H0).** The hypothesis that there is no effect, no difference, or no association. Statistical tests assess evidence against the null. Example: "There is no difference in gene expression between treated and control groups."

**Odds ratio (OR).** The ratio of odds of an event in one group to odds in another. OR = 1 means no association. OR > 1 means increased odds. Commonly reported in *logistic regression* and case-control studies.

**Outlier.** An observation that is unusually far from the rest of the data. In a *box plot*, observations beyond 1.5 * IQR from the quartiles. Can indicate errors, biological extremes, or violations of assumptions.

**Overdispersion.** Variance exceeding the mean in count data. Poisson models assume variance = mean; when this is violated, *negative binomial* models are more appropriate. Nearly universal in RNA-seq data.

**Paired test.** A test that accounts for the natural pairing of observations (e.g., before/after measurements on the same subject). More powerful than unpaired tests because pairing removes between-subject variability.

**Parametric test.** A statistical test that assumes the data follows a specific probability distribution (usually *normal*). Examples: *t-test*, *ANOVA*, *Pearson correlation*. More powerful than *non-parametric tests* when assumptions hold.

**PCA (Principal Component Analysis).** A *dimensionality reduction* method that finds orthogonal linear combinations of variables (principal components) that capture maximum variance. PC1 captures the most variance, PC2 the next most, and so on.

**PDF (Probability Density Function).** For continuous distributions, the function whose integral over an interval gives the probability of falling in that interval. The height of the curve at a point is not a probability.

**Pearson correlation.** A measure of linear association between two continuous variables. Ranges from -1 (perfect negative) to +1 (perfect positive). Assumes both variables are approximately *normally distributed*.

**Permutation test.** A non-parametric test that estimates the null distribution by repeatedly shuffling group labels and recomputing the test statistic. The p-value is the proportion of permuted statistics as extreme as the observed. Makes minimal assumptions.

**PMF (Probability Mass Function).** For *discrete distributions*, the function that gives the probability of each possible value. P(X = k) = pmf(k).

**Posterior distribution.** In *Bayesian statistics*, the updated distribution of a parameter after observing data. Combines the *prior distribution* with the likelihood. Posterior is proportional to prior times likelihood.

**Power.** The probability of correctly rejecting the *null hypothesis* when it is false. Power = 1 - *beta*. Conventionally set at 0.8 (80%). Depends on sample size, *effect size*, *alpha*, and variability.

**Prior distribution.** In *Bayesian statistics*, the distribution representing beliefs about a parameter before observing data. Can be informative (based on previous studies) or non-informative (vague).

**p-value.** The probability of observing data as extreme as (or more extreme than) what was observed, assuming the *null hypothesis* is true. It is *not* the probability that the null hypothesis is true.

**Q-Q plot (Quantile-Quantile plot).** A diagnostic plot that compares the quantiles of observed data to the quantiles of a theoretical distribution (usually *normal*). Points on the diagonal indicate good fit. Used to check normality and to assess genomic inflation in *GWAS*.

**Quartile.** Values that divide the data into four equal parts. Q1 (25th percentile), Q2 (50th percentile = *median*), Q3 (75th percentile). The *IQR* is Q3 - Q1.

**Randomization.** Random assignment of subjects to treatment groups to ensure that confounding variables are equally distributed across groups. The gold standard for causal inference in clinical trials.

**Relative risk (RR).** The ratio of risk (probability) of an event in the exposed group to risk in the unexposed group. RR = 1 means no association. Reported in cohort studies and clinical trials. Distinct from *odds ratio*.

**Resampling.** Methods that generate new datasets from the observed data by sampling with or without replacement. Includes *bootstrap* and *permutation* methods. Useful when parametric assumptions are questionable.

**Residual.** The difference between an observed value and the value predicted by a model. Patterns in residuals indicate model misspecification. Residual plots are essential diagnostics for *regression*.

**ROC curve (Receiver Operating Characteristic).** A plot of sensitivity (true positive rate) vs. 1-specificity (false positive rate) at various classification thresholds. The *AUC* summarizes overall discrimination.

**R-squared (coefficient of determination).** The proportion of variance in the outcome explained by the model. Ranges from 0 to 1. R-squared = 0.7 means the model explains 70% of the variability. Always increases with more predictors; use *adjusted R-squared* for model comparison.

**Sample size.** The number of observations in a study. Larger samples give more precise estimates and greater *power*. Sample size calculations use *power analysis* to determine the minimum n needed for a given *effect size* and *alpha*.

**Sensitivity.** The proportion of true positives correctly identified. Sensitivity = TP / (TP + FN). Also called the true positive rate or recall. A test with high sensitivity rarely misses real positives.

**Spearman correlation.** A non-parametric measure of monotonic association. Computed by applying *Pearson* correlation to the ranks of the data. Does not assume linearity or normality.

**Specificity.** The proportion of true negatives correctly identified. Specificity = TN / (TN + FP). A test with high specificity rarely produces false positives.

**Standard deviation (SD).** The square root of *variance*. Measures the typical deviation of observations from the *mean*. Reported in the same units as the data. About 68% of normal data falls within one SD of the mean.

**Standard error (SE).** The *standard deviation* of a sampling distribution. SE of the mean = SD / sqrt(n). Decreases with larger sample sizes. Used to construct *confidence intervals*.

**Survival analysis.** Statistical methods for analyzing time-to-event data with censoring. Key tools: *Kaplan-Meier* estimator, *log-rank test*, *Cox proportional hazards* model.

**t-test.** A test for comparing means of two groups. Student's t-test assumes equal variances; Welch's t-test does not. For paired observations, use the paired t-test. Assumes approximately *normal* data or large samples.

**Tukey HSD.** A post-hoc test for all pairwise comparisons after a significant *ANOVA*. Controls the *family-wise error rate* across all comparisons. Preferred when comparing all group pairs.

**Type I error.** Rejecting the *null hypothesis* when it is actually true (false positive). The probability of a Type I error is *alpha*. In genomics, controlled by *multiple testing correction*.

**Type II error.** Failing to reject the *null hypothesis* when it is actually false (false negative). The probability of a Type II error is *beta*. Reduced by increasing sample size or *effect size*.

**Variance.** The average squared deviation from the *mean*. Var = sum((x_i - mean)^2) / (n - 1) for a sample. Measures the spread of a distribution. The square of the *standard deviation*.

**VIF (Variance Inflation Factor).** A measure of multicollinearity in *regression*. VIF = 1 means no correlation between predictors; VIF > 5-10 suggests problematic multicollinearity. Computed for each predictor.

**Volcano plot.** A visualization for *differential expression* showing -log10(p-value) vs. log2(*fold change*). Significant and biologically meaningful genes appear in the upper left (downregulated) and upper right (upregulated) corners.

**Wilcoxon signed-rank test.** A non-parametric alternative to the *paired t-test*. Tests whether the median of paired differences is zero. Based on the ranks of the absolute differences.
