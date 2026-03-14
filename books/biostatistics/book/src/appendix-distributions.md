# Appendix C: Distribution Reference Card

> *Every statistical test assumes a distribution. This appendix is your field guide to the ones that matter in biology.*

This reference covers the probability distributions you will encounter most frequently in biostatistics. For each distribution, you will find its parameters, key properties, a description of its shape, where it appears in biology, and the BioLang functions for working with it.

## How to Use This Reference

Each distribution entry includes:

- **Parameters** — the values that define the distribution's shape
- **Mean and Variance** — closed-form expressions
- **Shape** — what the distribution looks like
- **Biological use** — where this distribution appears in real data
- **BioLang functions** — `d` (density/mass), `p` (cumulative probability), `q` (quantile/inverse CDF), `r` (random samples)

## Continuous Distributions

### Normal (Gaussian)

| Property | Value |
|---|---|
| Parameters | mu (mean), sigma (standard deviation) |
| Mean | mu |
| Variance | sigma^2 |
| Shape | Symmetric bell curve centered at mu |
| Support | (-infinity, +infinity) |

**Biological use:** Gene expression levels (after log transformation), measurement errors, heights and weights in populations, many biological quantities after the central limit theorem applies.

**BioLang functions:**

```bio
dnorm(x, 0, 1)       # Probability density at x
pnorm(x, 0, 1)       # P(X <= x)
qnorm(p, 0, 1)       # Value at cumulative probability p
rnorm(100, 0, 1)     # Generate 100 random values
```

> **Key insight:** Raw gene expression counts are *not* normally distributed. They follow count distributions (Poisson, negative binomial). However, log-transformed expression values (log2 CPM, log2 FPKM) are approximately normal, which is why log transformation is so common in genomics.

### Log-Normal

| Property | Value |
|---|---|
| Parameters | mu (mean of log), sigma (SD of log) |
| Mean | exp(mu + sigma^2/2) |
| Variance | (exp(sigma^2) - 1) * exp(2*mu + sigma^2) |
| Shape | Right-skewed, always positive |
| Support | (0, +infinity) |

**Biological use:** Fold changes in gene expression, protein concentrations, cell sizes, bacterial colony counts, drug IC50 values. Any quantity that results from multiplicative processes.

```bio
dlnorm(x, 0, 1)     # Probability density at x
plnorm(x, 0, 1)     # P(X <= x)
qlnorm(p, 0, 1)     # Value at cumulative probability p
rlnorm(100, 0, 1)   # Generate 100 random values
```

> **Key insight:** When your data is right-skewed and always positive, try log-transforming it. If the log-transformed values look normal, your original data is log-normal, and you should perform statistics on the log scale.

### Student's t

| Property | Value |
|---|---|
| Parameters | df (degrees of freedom) |
| Mean | 0 (for df > 1) |
| Variance | df / (df - 2) (for df > 2) |
| Shape | Bell curve like normal, but heavier tails |
| Support | (-infinity, +infinity) |

**Biological use:** The test statistic in t-tests. Critical for small-sample inference. As df increases, the t-distribution approaches the normal distribution. With df = 30+, they are nearly identical.

```bio
dt(x, 10)            # Probability density at x
pt(x, 10)            # P(X <= x)
qt(p, 10)            # Value at cumulative probability p
rt(100, 10)          # Generate 100 random values
```

### F Distribution

| Property | Value |
|---|---|
| Parameters | df1 (numerator df), df2 (denominator df) |
| Mean | df2 / (df2 - 2) (for df2 > 2) |
| Variance | Complex expression involving df1 and df2 |
| Shape | Right-skewed, always positive |
| Support | (0, +infinity) |

**Biological use:** The test statistic in ANOVA and regression F-tests. Compares the ratio of two variances. An F-value much larger than 1 indicates that between-group variance exceeds within-group variance.

```bio
df(x, 5, 20)         # Probability density at x
pf(x, 5, 20)         # P(X <= x)
qf(p, 5, 20)         # Value at cumulative probability p
rf(100, 5, 20)       # Generate 100 random values
```

### Chi-Square

| Property | Value |
|---|---|
| Parameters | df (degrees of freedom) |
| Mean | df |
| Variance | 2 * df |
| Shape | Right-skewed (less skewed as df increases) |
| Support | (0, +infinity) |

**Biological use:** Goodness-of-fit tests (Hardy-Weinberg equilibrium), tests of independence in contingency tables (genotype vs. phenotype associations), variance tests.

```bio
dchisq(x, 5)         # Probability density at x
pchisq(x, 5)         # P(X <= x)
qchisq(p, 5)         # Value at cumulative probability p
rchisq(100, 5)       # Generate 100 random values
```

### Exponential

| Property | Value |
|---|---|
| Parameters | lambda (rate) |
| Mean | 1 / lambda |
| Variance | 1 / lambda^2 |
| Shape | Monotonically decreasing from lambda at x=0 |
| Support | (0, +infinity) |

**Biological use:** Time between events in a Poisson process — inter-arrival times of mutations along a chromosome, time between cell divisions, radioactive decay (used in dating). The "memoryless" distribution: the probability of the event occurring in the next minute is the same regardless of how long you have been waiting.

```bio
dexp(x, 0.5)         # Probability density at x
pexp(x, 0.5)         # P(X <= x)
qexp(p, 0.5)         # Value at cumulative probability p
rexp(100, 0.5)       # Generate 100 random values
```

### Gamma

| Property | Value |
|---|---|
| Parameters | alpha (shape), beta (rate) |
| Mean | alpha / beta |
| Variance | alpha / beta^2 |
| Shape | Right-skewed (alpha < 1: L-shaped; alpha = 1: exponential; alpha > 1: bell-shaped skewed right) |
| Support | (0, +infinity) |

**Biological use:** Waiting times for multiple events (time until k-th mutation), protein expression variance, Bayesian prior for rate parameters. Generalizes the exponential distribution (exponential is Gamma with alpha = 1).

```bio
dgamma(x, 2.0, 1.0)     # Probability density at x
pgamma(x, 2.0, 1.0)     # P(X <= x)
qgamma(p, 2.0, 1.0)     # Value at cumulative probability p
rgamma(100, 2.0, 1.0)   # Generate 100 random values
```

### Beta

| Property | Value |
|---|---|
| Parameters | alpha, beta (shape parameters) |
| Mean | alpha / (alpha + beta) |
| Variance | alpha * beta / ((alpha + beta)^2 * (alpha + beta + 1)) |
| Shape | Flexible: uniform (1,1), U-shaped (<1,<1), bell-shaped (>1,>1), skewed |
| Support | (0, 1) |

**Biological use:** Proportions and probabilities — allele frequencies, methylation beta-values (fraction of methylated CpGs), GC content fractions, Bayesian prior for probabilities. The natural distribution for data bounded between 0 and 1.

```bio
dbeta(x, 2.0, 5.0)      # Probability density at x
pbeta(x, 2.0, 5.0)      # P(X <= x)
qbeta(p, 2.0, 5.0)      # Value at cumulative probability p
rbeta(100, 2.0, 5.0)    # Generate 100 random values
```

> **Key insight:** DNA methylation data from bisulfite sequencing produces beta-values bounded between 0 and 1. Using a beta distribution (or a beta regression) is statistically appropriate. Using a normal distribution on raw beta-values can produce nonsensical predictions outside [0, 1].

### Uniform

| Property | Value |
|---|---|
| Parameters | a (minimum), b (maximum) |
| Mean | (a + b) / 2 |
| Variance | (b - a)^2 / 12 |
| Shape | Flat (constant density between a and b) |
| Support | [a, b] |

**Biological use:** Null distribution for p-values (under the null hypothesis, p-values are uniformly distributed on [0, 1] — this is what Q-Q plots check), random positions along a chromosome, non-informative Bayesian priors.

```bio
dunif(x, 0, 1)       # Probability density at x
punif(x, 0, 1)       # P(X <= x)
qunif(p, 0, 1)       # Value at cumulative probability p
runif(100, 0, 1)     # Generate 100 random values
```

## Discrete Distributions

### Binomial

| Property | Value |
|---|---|
| Parameters | n (trials), p (success probability) |
| Mean | n * p |
| Variance | n * p * (1 - p) |
| Shape | Symmetric when p = 0.5, skewed otherwise |
| Support | {0, 1, 2, ..., n} |

**Biological use:** Number of successes in n independent trials — number of reads mapping to a variant allele, number of patients responding to treatment, number of CpG sites methylated out of n examined.

```bio
dbinom(k, 20, 0.5)      # P(X = k)
pbinom(k, 20, 0.5)      # P(X <= k)
qbinom(q, 20, 0.5)      # Smallest k with P(X <= k) >= q
rbinom(100, 20, 0.5)    # Generate 100 random values
```

### Poisson

| Property | Value |
|---|---|
| Parameters | lambda (rate) |
| Mean | lambda |
| Variance | lambda |
| Shape | Right-skewed for small lambda, approximately normal for large lambda |
| Support | {0, 1, 2, ...} |

**Biological use:** Count data when events are rare and independent — number of mutations in a genomic region, number of reads at a locus (low coverage), number of rare variants per gene, colony counts on a plate.

```bio
dpois(k, 5.0)        # P(X = k)
ppois(k, 5.0)        # P(X <= k)
qpois(q, 5.0)        # Smallest k with P(X <= k) >= q
rpois(100, 5.0)      # Generate 100 random values
```

> **Key insight:** The Poisson distribution assumes that the mean equals the variance. In real RNA-seq data, the variance almost always exceeds the mean (overdispersion). This is why DESeq2 and edgeR use the negative binomial distribution instead.

### Negative Binomial

| Property | Value |
|---|---|
| Parameters | r (number of successes), p (success probability); or mu (mean), size (dispersion) |
| Mean | mu |
| Variance | mu + mu^2 / size |
| Shape | Right-skewed, always more dispersed than Poisson |
| Support | {0, 1, 2, ...} |

**Biological use:** The workhorse of RNA-seq differential expression. Models count data with overdispersion (variance > mean). Used by DESeq2, edgeR, and most modern DE tools. Also models read counts in ChIP-seq, ATAC-seq, and scRNA-seq.

```bio
dnbinom(k, 10, 5)    # P(X = k), params: k, mu, size
pnbinom(k, 10, 5)    # P(X <= k)
qnbinom(q, 10, 5)    # Smallest k with P(X <= k) >= q
rnbinom(100, 10, 5)  # Generate 100 random values
```

### Hypergeometric

| Property | Value |
|---|---|
| Parameters | N (population), K (successes in population), n (draws) |
| Mean | n * K / N |
| Variance | n * K/N * (1 - K/N) * (N - n) / (N - 1) |
| Shape | Similar to binomial but for sampling without replacement |
| Support | {max(0, n+K-N), ..., min(n, K)} |

**Biological use:** Enrichment analysis — gene ontology enrichment (Fisher's exact test is a hypergeometric test), pathway overrepresentation, overlap between gene lists. "If I draw n genes from a genome of N, and K belong to this pathway, what is the probability of seeing k or more pathway genes by chance?"

```bio
dhyper(k, 200, 19800, 500)    # P(X = k), params: k, K, N-K, n
phyper(k, 200, 19800, 500)    # P(X <= k)
qhyper(q, 200, 19800, 500)    # Smallest k with P(X <= k) >= q
rhyper(100, 200, 19800, 500)  # Generate 100 random values
```

> **Key insight:** Fisher's exact test for 2x2 tables is equivalent to a hypergeometric test. When you run gene ontology enrichment and see a "Fisher's exact p-value," the software is computing hypergeometric probabilities.

## Quick Reference Table

| Distribution | Type | Parameters | Mean | Variance | Primary biological use |
|---|---|---|---|---|---|
| Normal | Continuous | mu, sigma | mu | sigma^2 | Log-transformed expression, measurements |
| Log-Normal | Continuous | mu, sigma | exp(mu + sigma^2/2) | Complex | Fold changes, concentrations |
| Student's t | Continuous | df | 0 | df/(df-2) | t-test statistic |
| F | Continuous | df1, df2 | df2/(df2-2) | Complex | ANOVA F-statistic |
| Chi-Square | Continuous | df | df | 2*df | Contingency tables, GOF tests |
| Exponential | Continuous | lambda | 1/lambda | 1/lambda^2 | Inter-event times |
| Gamma | Continuous | alpha, beta | alpha/beta | alpha/beta^2 | Waiting times, Bayesian priors |
| Beta | Continuous | alpha, beta | alpha/(alpha+beta) | Complex | Proportions, methylation |
| Uniform | Continuous | a, b | (a+b)/2 | (b-a)^2/12 | Null p-values, random positions |
| Binomial | Discrete | n, p | n*p | n*p*(1-p) | Variant allele counts |
| Poisson | Discrete | lambda | lambda | lambda | Rare event counts |
| Negative Binomial | Discrete | mu, size | mu | mu + mu^2/size | RNA-seq counts (overdispersed) |
| Hypergeometric | Discrete | N, K, n | n*K/N | Complex | Enrichment analysis |

## Relationships Between Distributions

Understanding how distributions relate to each other helps with intuition:

- **Poisson** is the limit of **Binomial** as n goes to infinity and p goes to 0 with n*p = lambda
- **Exponential** is **Gamma** with alpha = 1
- **Chi-Square** with df = k is **Gamma** with alpha = k/2 and beta = 1/2
- **Normal** is the limit of **Student's t** as df goes to infinity
- **Normal** approximates **Binomial** when n*p > 5 and n*(1-p) > 5
- **Normal** approximates **Poisson** when lambda > 20
- **Negative Binomial** reduces to **Poisson** when size goes to infinity (no overdispersion)
- **Hypergeometric** approaches **Binomial** when N is much larger than n

```bio
# Visualize the Poisson-Normal convergence
[1, 5, 10, 30] |> each(|lam| {
  let data = rpois(10000, lam)
  histogram(data, {title: "Poisson(lambda=" + str(lam) + ")", bins: 30})
})
```
