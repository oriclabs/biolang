# Day 3: Distributions — The Shape of Biological Variation

<div class="day-meta">
<span class="badge">Day 3 of 30</span>
<span class="badge">Prerequisites: Days 1-2</span>
<span class="badge">~55 min</span>
<span class="badge">Hands-on</span>
</div>

## The Problem

Dr. James Park has RNA-seq data from 12 tumor samples. He needs to identify genes that differ between treatment and control groups. He considers a t-test for a difference in means, then notices that raw sequencing abundance has a mean-variance relationship, zeros, and library-size structure that a simple gene-wise normal model does not represent.

Gene FPKM values range from 0 to 50,000. Most genes sit near zero, with a handful expressed at astronomical levels. The histogram looks nothing like a bell curve — it is a massive spike at zero with a long right tail stretching to infinity. He runs the t-test anyway. Out of 20,000 genes, 4,700 come back "significant" at p < 0.05. That is 23.5% of all genes, far more than seems biologically plausible for this modest treatment.

This is an illustrative warning, not a measured analysis: those 4,700 calls could reflect treatment effects, confounding, multiplicity, mean-variance dependence, or a mismatched model. A t-test concerns a mean and the sampling behaviour of its statistic; it does not require every raw observation to look perfectly normal. For RNA-seq counts, methods that model counts, library size, and dispersion are usually more defensible than gene-wise tests on raw FPKM values.

## What Is a Distribution?

A distribution is a recipe that tells you how likely each possible value is. Think of it as a map of a city, where the height at each point represents how many people live there. Downtown is a tall spike (many people). The suburbs are a gentle slope. The surrounding farmland is nearly flat.

Every dataset has an underlying distribution — the theoretical shape that generated the data you observe. When you draw a histogram of your data, you are estimating this shape from a finite sample. With 10 data points, the histogram is choppy and unreliable. With 10,000, it smooths out and begins to reveal the true underlying curve.

Why does this matter? Statistical procedures rely on assumptions about sampling, dependence, model form, and sometimes a distribution. A t-test's normal model concerns group outcomes or paired differences under its derivation; regression diagnostics focus on conditional residual behaviour; a chi-square approximation needs adequate expected counts; Poisson regression assumes a conditional count mean/variance structure. Departures matter by degree and purpose, so inspect their effect rather than using one pass/fail shape test.

> **Key insight:** Treat a model as an explicit approximation. State its estimand and assumptions, inspect diagnostics and sensitivity analyses, and interpret uncertainty conditionally on those choices.

## The Normal Distribution

The normal distribution — the bell curve — is the most famous distribution in statistics, and for good reason. It arises naturally whenever many small, independent effects add together.

### Properties

The normal distribution is defined by two parameters:
- **&mu; (mu):** the mean, which determines the center
- **&sigma; (sigma):** the standard deviation, which determines the width

The curve is perfectly symmetric around &mu;. It extends infinitely in both directions, though values far from the mean are exceedingly unlikely.

### The 68-95-99.7 Rule

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="330" viewBox="0 0 680 330" xmlns="http://www.w3.org/2000/svg" style="background: #fafbfc; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="26" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">The 68-95-99.7 Rule (Empirical Rule)</text>
  <defs>
    <clipPath id="bellClip"><rect x="60" y="40" width="560" height="220"/></clipPath>
  </defs>
  <!-- Axis -->
  <line x1="60" y1="250" x2="620" y2="250" stroke="#9ca3af" stroke-width="1.5"/>
  <!-- 99.7% region (3 sigma) -->
  <path d="M 100,248 Q 130,245 160,235 Q 190,215 220,185 Q 260,130 300,82 Q 320,62 340,55 Q 360,62 380,82 Q 420,130 460,185 Q 490,215 520,235 Q 550,245 580,248 Z" fill="#dbeafe" fill-opacity="0.5" stroke="none"/>
  <!-- 95% region (2 sigma) -->
  <path d="M 140,248 Q 170,242 200,225 Q 230,195 265,145 Q 290,100 310,72 Q 325,60 340,55 Q 355,60 370,72 Q 390,100 415,145 Q 450,195 480,225 Q 510,242 540,248 Z" fill="#93c5fd" fill-opacity="0.5" stroke="none"/>
  <!-- 68% region (1 sigma) -->
  <path d="M 220,248 Q 240,240 260,218 Q 285,170 305,120 Q 320,80 330,62 Q 340,55 350,62 Q 360,80 375,120 Q 395,170 420,218 Q 440,240 460,248 Z" fill="#3b82f6" fill-opacity="0.4" stroke="none"/>
  <!-- Bell curve outline -->
  <path d="M 80,248 Q 110,246 140,240 Q 170,228 200,205 Q 230,170 260,130 Q 290,90 310,68 Q 325,56 340,52 Q 355,56 370,68 Q 390,90 420,130 Q 450,170 480,205 Q 510,228 540,240 Q 570,246 600,248" fill="none" stroke="#1e40af" stroke-width="2.5"/>
  <!-- Sigma markers on axis -->
  <line x1="340" y1="245" x2="340" y2="255" stroke="#1e293b" stroke-width="2"/>
  <text x="340" y="270" text-anchor="middle" font-size="12" font-weight="bold" fill="#1e293b">mu</text>
  <line x1="220" y1="245" x2="220" y2="255" stroke="#1e293b" stroke-width="1.5"/>
  <text x="220" y="270" text-anchor="middle" font-size="11" fill="#1e293b">-1sigma</text>
  <line x1="460" y1="245" x2="460" y2="255" stroke="#1e293b" stroke-width="1.5"/>
  <text x="460" y="270" text-anchor="middle" font-size="11" fill="#1e293b">+1sigma</text>
  <line x1="140" y1="245" x2="140" y2="255" stroke="#1e293b" stroke-width="1.5"/>
  <text x="140" y="270" text-anchor="middle" font-size="11" fill="#1e293b">-2sigma</text>
  <line x1="540" y1="245" x2="540" y2="255" stroke="#1e293b" stroke-width="1.5"/>
  <text x="540" y="270" text-anchor="middle" font-size="11" fill="#1e293b">+2sigma</text>
  <line x1="100" y1="245" x2="100" y2="255" stroke="#1e293b" stroke-width="1.5"/>
  <text x="100" y="270" text-anchor="middle" font-size="11" fill="#1e293b">-3sigma</text>
  <line x1="580" y1="245" x2="580" y2="255" stroke="#1e293b" stroke-width="1.5"/>
  <text x="580" y="270" text-anchor="middle" font-size="11" fill="#1e293b">+3sigma</text>
  <!-- Percentage labels with brackets -->
  <!-- 68% -->
  <line x1="220" y1="225" x2="220" y2="215" stroke="#1e40af" stroke-width="1.2"/>
  <line x1="220" y1="215" x2="460" y2="215" stroke="#1e40af" stroke-width="1.2"/>
  <line x1="460" y1="225" x2="460" y2="215" stroke="#1e40af" stroke-width="1.2"/>
  <text x="340" y="212" text-anchor="middle" font-size="13" font-weight="bold" fill="#1e40af">68.3%</text>
  <!-- 95% -->
  <line x1="140" y1="240" x2="140" y2="290" stroke="#3b82f6" stroke-width="1.2"/>
  <line x1="140" y1="290" x2="540" y2="290" stroke="#3b82f6" stroke-width="1.2"/>
  <line x1="540" y1="240" x2="540" y2="290" stroke="#3b82f6" stroke-width="1.2"/>
  <text x="340" y="304" text-anchor="middle" font-size="13" font-weight="bold" fill="#3b82f6">95.4%</text>
  <!-- 99.7% -->
  <line x1="100" y1="248" x2="100" y2="315" stroke="#93c5fd" stroke-width="1.2"/>
  <line x1="100" y1="315" x2="580" y2="315" stroke="#93c5fd" stroke-width="1.2"/>
  <line x1="580" y1="248" x2="580" y2="315" stroke="#93c5fd" stroke-width="1.2"/>
  <text x="340" y="328" text-anchor="middle" font-size="13" font-weight="bold" fill="#6b9bd2">99.7%</text>
</svg>
</div>

| Range | Probability | Meaning |
|---|---|---|
| &mu; &plusmn; 1&sigma; | 68.3% | About two-thirds of data |
| &mu; &plusmn; 2&sigma; | 95.4% | Nearly all data |
| &mu; &plusmn; 3&sigma; | 99.7% | Almost everything |
| Beyond 3&sigma; | 0.3% | Extreme outliers |

For an approximately normal reference distribution, about 0.27% of observations lie beyond ±3 SD. Crossing that line is a review flag, not a diagnosis: it may be a valid tail observation, a different subgroup, a recording problem, or evidence that mean/SD is the wrong descriptive frame.

### Biological Examples of Normality

The normal distribution is a good model for:
- **Measurement error:** Technical replicates of the same sample tend to be normally distributed.
- **Height and weight** in a homogeneous population (though mixtures of populations are not normal).
- **Blood pressure** readings in healthy adults.
- **Quantitative traits** influenced by many genes of small effect (additive genetic model).
- **Log-transformed gene expression** (more on this below).

### When Data Is NOT Normal

The normal distribution is a terrible model for:
- Raw gene expression (FPKM, TPM, counts) — heavily right-skewed
- Read counts — discrete and non-negative; their variance and zero frequency often exceed a simple Poisson model
- Allele frequencies — bounded between 0 and 1
- Survival times — always positive, typically right-skewed
- Any data with a hard boundary (concentrations cannot be negative)

```bio
set_seed(42)
# Generate and visualize normal data
let heights = rnorm(5000, 170, 8)

histogram(heights, {bins: 50, title: "Adult Heights (cm) — Normal Distribution"})
let stats = summary(heights)
print(f"Mean: {stats.mean:.1}, Median: {stats.median:.1}, Skewness: {stats.skewness:.3}")
# Mean and median nearly identical; skewness near zero — hallmarks of normality
```

## The Log-Normal Distribution

Some positive abundance measurements are reasonably described by a **log-normal** model: the data are skewed while their logarithms are closer to symmetric. Raw sequencing counts are discrete and commonly require count-specific sampling models; zeros, mixtures, and detection limits also prevent a universal log-normal claim.

### Why Gene Expression Is Log-Normal

Gene regulation is a cascade of multiplicative processes. A transcription factor binds (or doesn't), an enhancer activates (fold-change), mRNA is stabilized (half-life multiplied), ribosomes translate at varying rates (multiplied efficiency). When effects multiply rather than add, the result is log-normal, not normal.

This is a mathematical fact: if X = Y&#x2081; &times; Y&#x2082; &times; ... &times; Y&#x2099; and the Y values are independent, then log(X) = log(Y&#x2081;) + log(Y&#x2082;) + ... + log(Y&#x2099;). Sums of independent variables tend toward normal (by the Central Limit Theorem), so log(X) is approximately normal, meaning X is log-normal.

### Logarithms: Change the Question Deliberately

Logarithms turn ratios into differences and compress large positive values. That
can be useful, but it changes the estimand: the arithmetic mean of logged values
describes a geometric centre after back-transformation, not the arithmetic mean
on the original scale.

Bioinformaticians often use a log-like transform for visualization and
distance-based exploration of positive abundance data:

```bio
set_seed(42)
# Simulate gene expression (log-normal)
let log_expr = rnorm(5000, 3.0, 2.0)
let expression = log_expr |> map(|x| 2.0 ** x)  # 2^x to simulate FPKM

# Raw expression: heavily skewed
histogram(expression, {bins: 50, title: "Raw FPKM — Right Skewed"})
let raw_stats = summary(expression)
print(f"Raw — Mean: {raw_stats.mean:.1}, Median: {raw_stats.median:.1}, Skew: {raw_stats.skewness:.2}")

# Log2-transformed: approximately normal
let log2_expr = expression |> map(|x| log2(x + 1))  # +1 to handle zeros
histogram(log2_expr, {bins: 50, title: "log2(FPKM+1) — Approximately Normal"})
let log_stats = summary(log2_expr)
print(f"Log2 — Mean: {log_stats.mean:.1}, Median: {log_stats.median:.1}, Skew: {log_stats.skewness:.2}")
```

In this simulated example, the mean and median move closer and skewness falls.
That makes patterns easier to display, but it does **not** prove that a t-test or
linear model is appropriate. Check residuals, independence, variance, sampling,
and the scientific meaning of effects. Count models such as negative-binomial
regression usually operate on raw counts with an exposure or library-size
offset; logging counts first would change that model.

<div style="text-align: center; margin: 2em 0;">
<svg width="700" height="270" viewBox="0 0 700 270" xmlns="http://www.w3.org/2000/svg" role="img" aria-label="Linear and log base two axes" style="max-width: 100%; background: #fafbfc; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="350" y="28" text-anchor="middle" font-size="16" font-weight="bold" fill="#1e293b">A log scale changes distance from differences to ratios</text>
  <text x="45" y="66" font-size="13" font-weight="bold" fill="#1e40af">Linear axis</text>
  <line x1="90" y1="94" x2="620" y2="94" stroke="#64748b" stroke-width="2"/>
  <circle cx="150" cy="94" r="6" fill="#2563eb"/><circle cx="215" cy="94" r="6" fill="#2563eb"/><circle cx="345" cy="94" r="6" fill="#2563eb"/><circle cx="605" cy="94" r="6" fill="#2563eb"/>
  <text x="150" y="118" text-anchor="middle" font-size="12" fill="#334155">1</text><text x="215" y="118" text-anchor="middle" font-size="12" fill="#334155">2</text><text x="345" y="118" text-anchor="middle" font-size="12" fill="#334155">4</text><text x="605" y="118" text-anchor="middle" font-size="12" fill="#334155">8</text>
  <text x="350" y="140" text-anchor="middle" font-size="11" fill="#6b7280">Equal physical gaps mean equal differences: +1, +2, +4 grow.</text>
  <text x="45" y="174" font-size="13" font-weight="bold" fill="#6b21a8">log2 axis</text>
  <line x1="90" y1="202" x2="620" y2="202" stroke="#64748b" stroke-width="2"/>
  <circle cx="150" cy="202" r="6" fill="#7c3aed"/><circle cx="300" cy="202" r="6" fill="#7c3aed"/><circle cx="450" cy="202" r="6" fill="#7c3aed"/><circle cx="600" cy="202" r="6" fill="#7c3aed"/>
  <text x="150" y="226" text-anchor="middle" font-size="12" fill="#334155">1</text><text x="300" y="226" text-anchor="middle" font-size="12" fill="#334155">2</text><text x="450" y="226" text-anchor="middle" font-size="12" fill="#334155">4</text><text x="600" y="226" text-anchor="middle" font-size="12" fill="#334155">8</text>
  <text x="375" y="252" text-anchor="middle" font-size="11" fill="#6b7280">Equal physical gaps mean equal ratios: every step is &times;2.</text>
</svg>
</div>

#### When a log scale is a sensible candidate

- Values are positive and ratios or fold changes are scientifically meaningful.
- A fixed percentage change matters more than a fixed unit change.
- Spread grows with the level, suggesting multiplicative rather than additive noise.
- A few large positive values obscure the rest of a visualization.
- Effects are naturally interpreted as doubling, halving, or percentage change.

#### When not to apply a log mechanically

- Raw counts are being analyzed with a Poisson or negative-binomial count model.
- Values are negative, unless a scientifically justified different transform is used.
- Zeros are meaningful: adding 1 is a modelling choice whose impact is largest near zero.
- The variable is already logarithmic, such as pH, decibels, Phred quality, or log fold change.
- Values are proportions bounded by 0 and 1; a binomial/beta model or logit may match the question better.
- The outcome is censored survival time; logging does not account for censoring.
- The apparent skew is caused by mixed cell types, batches, or subgroups that should be modelled rather than compressed away.

#### `log`, `log1p`, `log2`, or `log10`?

| Transform | Domain | Interpretation |
|---|---|---|
| `log(x)` | `x > 0` | Natural log; convenient for models and calculus |
| `log1p(x)` | `x > -1` | Computes `log(1+x)` accurately; keeps zero at zero but introduces a pseudocount scale |
| `log2(x)` | `x > 0` | One unit means a doubling; useful for fold changes |
| `log10(x)` | `x > 0` | One unit means ten-fold; useful for orders of magnitude |

Changing the log base rescales the axis but does not create new information.
The important decisions are whether a log scale matches the scientific process,
how zeros are handled, and which scale the final effect is reported on.

Preview rather than guessing:

```bio
import "statistics" as stat

let preview = stat.preview_transform(expression, "log1p")
println("Raw skewness: " + str(preview.before.skewness))
println("Log1p skewness: " + str(preview.after.skewness))
println("Zeros before/after: " + str(preview.zeros_before) + "/" + str(preview.zeros_after))
```

> **Clinical relevance:** Differential-expression count models use raw counts with a count likelihood and normalization/offset terms. Downstream PCA, clustering, and visualization often use a variance-stabilizing or log-like representation. Keep those two roles separate and record which data layer each step used.

## The Poisson Distribution

The Poisson distribution models the number of events that occur in a fixed interval, when events happen independently at a constant average rate.

### Properties

- **Parameter:** &lambda; (lambda) — the average rate
- **Support:** 0, 1, 2, 3, ... (non-negative integers)
- **Mean = Variance = &lambda;** (this is the key property)

### Biological Examples

| Application | What is the "event"? | What is the "interval"? |
|---|---|---|
| RNA-seq read counts | One read mapping to a gene | One gene in one sample |
| Mutations | One mutation | Per megabase of genome |
| Rare diseases | One case | Per 100,000 population per year |
| Sequencing errors | One error | Per 1000 bases |

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="280" viewBox="0 0 680 280" xmlns="http://www.w3.org/2000/svg" style="background: #fafbfc; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="26" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">Distribution Shape Comparison</text>
  <!-- Panel 1: Normal -->
  <rect x="15" y="38" width="210" height="220" rx="6" fill="#f8fafc" stroke="#e2e8f0"/>
  <text x="120" y="58" text-anchor="middle" font-size="12" font-weight="600" fill="#2563eb">Normal</text>
  <text x="120" y="72" text-anchor="middle" font-size="10" fill="#6b7280">mu=0, sigma=1</text>
  <line x1="30" y1="225" x2="210" y2="225" stroke="#9ca3af" stroke-width="1"/>
  <path d="M 35,223 Q 50,222 65,215 Q 80,195 90,165 Q 100,120 110,85 Q 115,65 120,58 Q 125,65 130,85 Q 140,120 150,165 Q 160,195 175,215 Q 190,222 205,223" fill="#93c5fd" fill-opacity="0.3" stroke="#2563eb" stroke-width="2"/>
  <text x="120" y="244" text-anchor="middle" font-size="10" fill="#2563eb">Symmetric bell curve</text>
  <text x="120" y="256" text-anchor="middle" font-size="9" fill="#6b7280">Heights, measurement error</text>
  <!-- Panel 2: Log-Normal -->
  <rect x="235" y="38" width="210" height="220" rx="6" fill="#f8fafc" stroke="#e2e8f0"/>
  <text x="340" y="58" text-anchor="middle" font-size="12" font-weight="600" fill="#16a34a">Log-Normal</text>
  <text x="340" y="72" text-anchor="middle" font-size="10" fill="#6b7280">mu=0, sigma=1 (of log)</text>
  <line x1="250" y1="225" x2="430" y2="225" stroke="#9ca3af" stroke-width="1"/>
  <path d="M 260,223 Q 270,218 280,190 Q 288,145 295,95 Q 300,68 308,58 Q 318,62 330,90 Q 345,135 365,175 Q 385,205 410,218 Q 420,222 425,223" fill="#bbf7d0" fill-opacity="0.3" stroke="#16a34a" stroke-width="2"/>
  <text x="340" y="244" text-anchor="middle" font-size="10" fill="#16a34a">Right-skewed, always positive</text>
  <text x="340" y="256" text-anchor="middle" font-size="9" fill="#6b7280">Gene expression (FPKM/TPM)</text>
  <!-- Panel 3: Poisson -->
  <rect x="455" y="38" width="210" height="220" rx="6" fill="#f8fafc" stroke="#e2e8f0"/>
  <text x="560" y="58" text-anchor="middle" font-size="12" font-weight="600" fill="#7c3aed">Poisson</text>
  <text x="560" y="72" text-anchor="middle" font-size="10" fill="#6b7280">lambda=3</text>
  <line x1="470" y1="225" x2="650" y2="225" stroke="#9ca3af" stroke-width="1"/>
  <!-- Poisson bars (discrete) -->
  <rect x="478" y="210" width="16" height="13" fill="#7c3aed" opacity="0.5" rx="1"/>
  <rect x="498" y="175" width="16" height="48" fill="#7c3aed" opacity="0.5" rx="1"/>
  <rect x="518" y="130" width="16" height="93" fill="#7c3aed" opacity="0.5" rx="1"/>
  <rect x="538" y="105" width="16" height="118" fill="#7c3aed" opacity="0.5" rx="1"/>
  <rect x="558" y="130" width="16" height="93" fill="#7c3aed" opacity="0.5" rx="1"/>
  <rect x="578" y="165" width="16" height="58" fill="#7c3aed" opacity="0.5" rx="1"/>
  <rect x="598" y="190" width="16" height="33" fill="#7c3aed" opacity="0.5" rx="1"/>
  <rect x="618" y="210" width="16" height="13" fill="#7c3aed" opacity="0.5" rx="1"/>
  <rect x="638" y="218" width="16" height="5" fill="#7c3aed" opacity="0.5" rx="1"/>
  <!-- x-axis labels -->
  <text x="486" y="240" text-anchor="middle" font-size="9" fill="#6b7280">0</text>
  <text x="506" y="240" text-anchor="middle" font-size="9" fill="#6b7280">1</text>
  <text x="526" y="240" text-anchor="middle" font-size="9" fill="#6b7280">2</text>
  <text x="546" y="240" text-anchor="middle" font-size="9" fill="#6b7280">3</text>
  <text x="566" y="240" text-anchor="middle" font-size="9" fill="#6b7280">4</text>
  <text x="586" y="240" text-anchor="middle" font-size="9" fill="#6b7280">5</text>
  <text x="606" y="240" text-anchor="middle" font-size="9" fill="#6b7280">6</text>
  <text x="626" y="240" text-anchor="middle" font-size="9" fill="#6b7280">7</text>
  <text x="646" y="240" text-anchor="middle" font-size="9" fill="#6b7280">8</text>
  <text x="560" y="256" text-anchor="middle" font-size="10" fill="#7c3aed">Discrete counts</text>
  <text x="560" y="268" text-anchor="middle" font-size="9" fill="#6b7280">Read counts, mutations</text>
</svg>
</div>

### The Overdispersion Problem

In theory, RNA-seq counts should be Poisson. In practice, biological replicates show more variability than Poisson predicts — the variance exceeds the mean. This is **overdispersion**, caused by biological variability between samples.

The solution is the **negative binomial** distribution, which adds a dispersion parameter to allow variance > mean. This is why DESeq2 uses negative binomial, not Poisson.

```bio
set_seed(42)
# Poisson distribution for mutation counts

# Average 3.5 mutations per megabase in a tumor
let mutation_counts = rpois(1000, 3.5)

histogram(mutation_counts, {bins: 15, title: "Mutations per Megabase (Poisson, lambda=3.5)"})

# Verify mean ~ variance (Poisson property)
print(f"Mean: {mean(mutation_counts):.2}")
print(f"Variance: {variance(mutation_counts):.2}")
# Both should be close to 3.5

# Probability of seeing 10+ mutations in a region (hypermutation?)
let p_hyper = 1.0 - ppois(9, 3.5)
print(f"P(10+ mutations): {p_hyper:.4}")
# Very low — a region with 10+ mutations is genuinely unusual
```

## The Binomial Distribution

The binomial distribution models the number of successes in a fixed number of independent trials, each with the same probability of success.

### Properties

- **Parameters:** n (number of trials), p (probability of success)
- **Support:** 0, 1, 2, ..., n
- **Mean = np, Variance = np(1-p)**

### Biological Examples

| Application | What is a "trial"? | What is "success"? | What is p? |
|---|---|---|---|
| Heterozygous genotype | One offspring | Inherits variant allele | 0.5 |
| SNP calling | One read at a position | Carries alt allele | VAF |
| Drug response | One patient | Responds | Response rate |
| Hardy-Weinberg | One individual | Has genotype AA | p&sup2; |

### Hardy-Weinberg Equilibrium

For a biallelic locus with allele frequencies p and q = 1-p, Hardy-Weinberg predicts genotype frequencies:
- AA: p&sup2;
- Aa: 2pq
- aa: q&sup2;

Deviations from HWE can indicate selection, population structure, or genotyping error.

```bio
set_seed(42)
# Genotype frequencies under Hardy-Weinberg
let p = 0.3  # frequency of allele A
let q = 1.0 - p

let freq_AA = p * p         # 0.09
let freq_Aa = 2.0 * p * q   # 0.42
let freq_aa = q * q         # 0.49

print("Expected genotype frequencies:")
print(f"  AA: {freq_AA:.3}")
print(f"  Aa: {freq_Aa:.3}")
print(f"  aa: {freq_aa:.3}")

# Simulate genotyping 500 individuals
let n_individuals = 500
let AA_count = rbinom(1, n_individuals, freq_AA) |> sum()

# Probability of observing exactly k heterozygotes
let k = 200
let p_exact = dbinom(k, n_individuals, freq_Aa)
print(f"P(exactly {k} heterozygotes in {n_individuals}): {p_exact:.6}")

# Probability of 230+ heterozygotes (possible HWE violation?)
let p_excess = 1.0 - pbinom(229, n_individuals, freq_Aa)
print(f"P(230+ heterozygotes): {p_excess:.4}")
```

## Checking Your Distribution: Diagnostic Tools

Before running any parametric test, verify that your data matches its assumptions. Here are the essential tools.

### Q-Q Plots

A Q-Q (quantile-quantile) plot compares your data's quantiles against the theoretical quantiles of a reference distribution (usually normal). If data follows the reference distribution, points fall on a straight diagonal line. Deviations reveal departures from the assumed shape.

```bio
set_seed(42)
# Q-Q plot for normal data (should be a straight line)
let normal_data = rnorm(500, 0, 1)
normal_qq_plot(normal_data, {title: "Q-Q Plot: Normal Data"})

# Q-Q plot for log-normal data (curved — not normal!)
let lognormal_data = normal_data |> map(|x| exp(x))
normal_qq_plot(lognormal_data, {title: "Q-Q Plot: Log-Normal Data (Raw)"})

# Q-Q plot after log-transform (straight again)
let transformed = lognormal_data |> map(|x| log(x))
normal_qq_plot(transformed, {title: "Q-Q Plot: Log-Normal Data (After Log Transform)"})
```

**Reading a Q-Q plot:**
- Points on the line: data matches the assumed distribution
- Upward curve at the right end: right skew (heavy right tail)
- Downward curve at the left end: left skew (heavy left tail)
- S-shape: heavy tails on both sides (high kurtosis)

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="270" viewBox="0 0 680 270" xmlns="http://www.w3.org/2000/svg" style="background: #fafbfc; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="26" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">Q-Q Plot Interpretation Guide</text>
  <!-- Panel 1: Good fit -->
  <rect x="15" y="40" width="210" height="210" rx="6" fill="#f8fafc" stroke="#e2e8f0"/>
  <text x="120" y="60" text-anchor="middle" font-size="12" font-weight="600" fill="#16a34a">Good Fit (Normal)</text>
  <line x1="40" y1="220" x2="200" y2="220" stroke="#9ca3af" stroke-width="1"/>
  <line x1="40" y1="220" x2="40" y2="70" stroke="#9ca3af" stroke-width="1"/>
  <text x="120" y="240" text-anchor="middle" font-size="9" fill="#6b7280">Theoretical Quantiles</text>
  <text x="22" y="148" text-anchor="middle" font-size="9" fill="#6b7280" transform="rotate(-90,22,148)">Sample</text>
  <!-- Reference line -->
  <line x1="50" y1="210" x2="195" y2="78" stroke="#9ca3af" stroke-width="1.5" stroke-dasharray="5,3"/>
  <!-- Points along the line -->
  <circle cx="58" cy="206" r="3" fill="#16a34a" opacity="0.7"/>
  <circle cx="70" cy="196" r="3" fill="#16a34a" opacity="0.7"/>
  <circle cx="82" cy="185" r="3" fill="#16a34a" opacity="0.7"/>
  <circle cx="92" cy="175" r="3" fill="#16a34a" opacity="0.7"/>
  <circle cx="102" cy="165" r="3" fill="#16a34a" opacity="0.7"/>
  <circle cx="112" cy="152" r="3" fill="#16a34a" opacity="0.7"/>
  <circle cx="122" cy="142" r="3" fill="#16a34a" opacity="0.7"/>
  <circle cx="132" cy="132" r="3" fill="#16a34a" opacity="0.7"/>
  <circle cx="143" cy="121" r="3" fill="#16a34a" opacity="0.7"/>
  <circle cx="155" cy="110" r="3" fill="#16a34a" opacity="0.7"/>
  <circle cx="166" cy="100" r="3" fill="#16a34a" opacity="0.7"/>
  <circle cx="178" cy="90" r="3" fill="#16a34a" opacity="0.7"/>
  <circle cx="190" cy="82" r="3" fill="#16a34a" opacity="0.7"/>
  <text x="120" y="256" text-anchor="middle" font-size="10" fill="#16a34a">Points follow the diagonal</text>
  <!-- Panel 2: Heavy tails (S-curve) -->
  <rect x="235" y="40" width="210" height="210" rx="6" fill="#f8fafc" stroke="#e2e8f0"/>
  <text x="340" y="60" text-anchor="middle" font-size="12" font-weight="600" fill="#7c3aed">Heavy Tails (S-shape)</text>
  <line x1="260" y1="220" x2="420" y2="220" stroke="#9ca3af" stroke-width="1"/>
  <line x1="260" y1="220" x2="260" y2="70" stroke="#9ca3af" stroke-width="1"/>
  <text x="340" y="240" text-anchor="middle" font-size="9" fill="#6b7280">Theoretical Quantiles</text>
  <!-- Reference line -->
  <line x1="270" y1="210" x2="415" y2="78" stroke="#9ca3af" stroke-width="1.5" stroke-dasharray="5,3"/>
  <!-- S-shaped points -->
  <circle cx="278" cy="215" r="3" fill="#7c3aed" opacity="0.7"/>
  <circle cx="287" cy="210" r="3" fill="#7c3aed" opacity="0.7"/>
  <circle cx="296" cy="200" r="3" fill="#7c3aed" opacity="0.7"/>
  <circle cx="305" cy="185" r="3" fill="#7c3aed" opacity="0.7"/>
  <circle cx="315" cy="165" r="3" fill="#7c3aed" opacity="0.7"/>
  <circle cx="325" cy="152" r="3" fill="#7c3aed" opacity="0.7"/>
  <circle cx="340" cy="142" r="3" fill="#7c3aed" opacity="0.7"/>
  <circle cx="355" cy="132" r="3" fill="#7c3aed" opacity="0.7"/>
  <circle cx="368" cy="120" r="3" fill="#7c3aed" opacity="0.7"/>
  <circle cx="380" cy="100" r="3" fill="#7c3aed" opacity="0.7"/>
  <circle cx="392" cy="82" r="3" fill="#7c3aed" opacity="0.7"/>
  <circle cx="402" cy="74" r="3" fill="#7c3aed" opacity="0.7"/>
  <circle cx="410" cy="68" r="3" fill="#7c3aed" opacity="0.7"/>
  <text x="340" y="256" text-anchor="middle" font-size="10" fill="#7c3aed">S-curve: more extreme values</text>
  <!-- Panel 3: Right-skewed (curve up) -->
  <rect x="455" y="40" width="210" height="210" rx="6" fill="#f8fafc" stroke="#e2e8f0"/>
  <text x="560" y="60" text-anchor="middle" font-size="12" font-weight="600" fill="#dc2626">Right Skew (Curves Up)</text>
  <line x1="480" y1="220" x2="640" y2="220" stroke="#9ca3af" stroke-width="1"/>
  <line x1="480" y1="220" x2="480" y2="70" stroke="#9ca3af" stroke-width="1"/>
  <text x="560" y="240" text-anchor="middle" font-size="9" fill="#6b7280">Theoretical Quantiles</text>
  <!-- Reference line -->
  <line x1="490" y1="210" x2="635" y2="78" stroke="#9ca3af" stroke-width="1.5" stroke-dasharray="5,3"/>
  <!-- Curved-up points (right skew) -->
  <circle cx="498" cy="208" r="3" fill="#dc2626" opacity="0.7"/>
  <circle cx="510" cy="198" r="3" fill="#dc2626" opacity="0.7"/>
  <circle cx="520" cy="188" r="3" fill="#dc2626" opacity="0.7"/>
  <circle cx="530" cy="175" r="3" fill="#dc2626" opacity="0.7"/>
  <circle cx="540" cy="162" r="3" fill="#dc2626" opacity="0.7"/>
  <circle cx="552" cy="148" r="3" fill="#dc2626" opacity="0.7"/>
  <circle cx="564" cy="132" r="3" fill="#dc2626" opacity="0.7"/>
  <circle cx="578" cy="112" r="3" fill="#dc2626" opacity="0.7"/>
  <circle cx="592" cy="95" r="3" fill="#dc2626" opacity="0.7"/>
  <circle cx="604" cy="82" r="3" fill="#dc2626" opacity="0.7"/>
  <circle cx="612" cy="74" r="3" fill="#dc2626" opacity="0.7"/>
  <circle cx="618" cy="68" r="3" fill="#dc2626" opacity="0.7"/>
  <circle cx="623" cy="64" r="3" fill="#dc2626" opacity="0.7"/>
  <text x="560" y="256" text-anchor="middle" font-size="10" fill="#dc2626">Upper tail heavier than normal</text>
</svg>
</div>

### Shapiro-Wilk Test

The Shapiro-Wilk test formally tests whether data is normally distributed. A small p-value (< 0.05) means the data is significantly non-normal.

```bio
set_seed(42)
# Test normality visually with Q-Q plots and histograms
let normal_data = rnorm(200, 50, 10)
let skewed_data = rnorm(200, 3, 1) |> map(|x| exp(x))

# For normal data: Q-Q plot should show points on the diagonal
normal_qq_plot(normal_data, {title: "Q-Q: Normal Data"})
let stats_normal = summary(normal_data)
print(f"Normal data — Skewness: {stats_normal.skewness:.4}")
# Skewness near 0: consistent with normality

# For skewed data: Q-Q plot will curve away from the diagonal
normal_qq_plot(skewed_data, {title: "Q-Q: Skewed Data"})
let stats_skewed = summary(skewed_data)
print(f"Skewed data — Skewness: {stats_skewed.skewness:.4}")
# High skewness: definitely not normal
```

> **Common pitfall:** The Shapiro-Wilk test is very sensitive with large samples. With n = 10,000, it will reject normality for data that is "close enough" to normal for practical purposes. For large samples, rely more on Q-Q plots and skewness/kurtosis values than on the Shapiro-Wilk p-value.

### Histogram with Density Overlay

Overlay the theoretical density curve on your histogram to visually assess fit:

```bio
set_seed(42)
let data = rnorm(2000, 100, 15)

# Histogram with normal density overlay
histogram(data, {bins: 40, title: "Data with Normal Fit Overlay"})
density(data, {title: "Kernel Density Estimate"})
```

## Distribution Summary Table

| Distribution | Shape | Parameters | Mean | Variance | Biology Use Case |
|---|---|---|---|---|---|
| Normal | Symmetric bell | &mu;, &sigma; | &mu; | &sigma;&sup2; | Measurement error, heights, log-expression |
| Log-Normal | Right-skewed | &mu;, &sigma; (of log) | exp(&mu; + &sigma;&sup2;/2) | Complex | Raw gene expression, protein abundance |
| Poisson | Right-skewed (low &lambda;), symmetric (high &lambda;) | &lambda; | &lambda; | &lambda; | Read counts, mutation rates |
| Binomial | Varies | n, p | np | np(1-p) | Genotype counts, allele sampling |
| Negative Binomial | Right-skewed | r, p | r(1-p)/p | r(1-p)/p&sup2; | Overdispersed counts (DESeq2) |
| Beta | Flexible (0,1) | &alpha;, &beta; | &alpha;/(&alpha;+&beta;) | Complex | Allele frequencies, methylation |

## Python and R Equivalents

**Python:**
```python
import numpy as np
from scipy import stats
import matplotlib.pyplot as plt

# Normal distribution
x = np.linspace(-4, 4, 1000)
plt.plot(x, stats.norm.pdf(x, 0, 1))

# Poisson
counts = np.random.poisson(lam=3.5, size=1000)

# Binomial
genotypes = np.random.binomial(n=500, p=0.42, size=1000)

# Q-Q plot
stats.probplot(data, dist="norm", plot=plt)

# Shapiro-Wilk test
stat, p = stats.shapiro(data)

# Distribution fitting
params = stats.norm.fit(data)  # MLE fit
```

**R:**
```r
# Normal distribution
x <- seq(-4, 4, length.out = 1000)
plot(x, dnorm(x, 0, 1), type = "l")

# Poisson
counts <- rpois(1000, lambda = 3.5)

# Binomial
genotypes <- rbinom(1000, size = 500, prob = 0.42)

# Q-Q plot
qqnorm(data)
qqline(data)

# Shapiro-Wilk test
shapiro.test(data)

# Density overlay
hist(data, freq = FALSE)
curve(dnorm(x, mean(data), sd(data)), add = TRUE, col = "red")
```

## Why This Matters for Testing

Here is the critical connection between today's material and the rest of the book:

| If your data is... | Then you can use... | But NOT... |
|---|---|---|
| Normal | t-test, ANOVA, Pearson correlation | — |
| Log-normal | t-test after log-transform | t-test on raw values |
| Poisson counts | Poisson regression, exact tests | t-test |
| Overdispersed counts | Negative binomial (DESeq2, edgeR) | Poisson, t-test |
| Non-normal, unknown | Mann-Whitney, Kruskal-Wallis | t-test, ANOVA |
| Bounded (0,1) | Beta regression, logit transform | Linear regression |

Choosing the wrong test because you assumed the wrong distribution is one of the most common errors in computational biology. Dr. Park's mistake from our opening scenario — running a t-test on raw FPKM values — is committed daily in bioinformatics labs around the world.

> **Key insight:** The distribution is not a detail. It is the foundation. Get it right, and your downstream analysis is trustworthy. Get it wrong, and no amount of sophisticated testing can rescue your conclusions.

## Exercises

### Exercise 1: Identify the Distribution

For each dataset, determine the most appropriate distribution and justify your choice.

```bio
set_seed(42)
# Dataset A: Sequencing read counts per gene
let dataset_a = rpois(1000, 25)

# Dataset B: Patient blood pressure
let dataset_b = rnorm(500, 120, 15)

# Dataset C: Gene expression (raw TPM)
let log_vals = rnorm(2000, 2.0, 1.5)
let dataset_c = log_vals |> map(|x| 10.0 ** x)

# TODO: For each dataset, create a histogram and Q-Q plot
# TODO: Check normality visually with normal_qq_plot() and histogram()
# TODO: For dataset C, try log-transforming and re-check
# TODO: State which distribution best describes each and why
```

### Exercise 2: The Poisson Check

Verify whether mutation count data follows a Poisson distribution by checking the mean-variance relationship.

```bio
set_seed(42)
# Scenario 1: Pure Poisson (technical replicates)
let technical = rpois(500, 8.0)

# Scenario 2: Overdispersed (biological replicates)
# Simulate by mixing Poisson with varying lambda
let lambdas = rnorm(500, 8.0, 3.0) |> map(|x| max(x, 0.1))

# TODO: Compute mean and variance for technical replicates
# TODO: Are they approximately equal? (Poisson property)
# TODO: For overdispersed data, compute the dispersion ratio (variance/mean)
# TODO: What does a ratio >> 1 tell you about the data?
```

### Exercise 3: Transform and Test

Take a skewed dataset, find the right transformation, and verify normality.

```bio
set_seed(42)
let protein_abundance = rnorm(300, 4, 1.2) |> map(|x| exp(x))

# TODO: Plot histogram of raw data
# TODO: Check normality with normal_qq_plot() — is it normal?
# TODO: Apply log transform
# TODO: Plot histogram of transformed data
# TODO: Check normality of transformed data with normal_qq_plot()
# TODO: Compare skewness before and after
```

### Exercise 4: Distribution Detective

A collaborator gives you mystery data. Identify its distribution using all tools from today.

```bio
set_seed(42)
# Mystery data — what distribution is this?
let mystery = rbinom(1000, 50, 0.15)

# TODO: Compute summary()
# TODO: Create histogram
# TODO: Try Q-Q plot against normal
# TODO: Note: the data is discrete. What distributions produce discrete data?
# TODO: Estimate the parameters and identify the distribution
```

## Key Takeaways

- A **distribution** is the theoretical shape describing how likely each value is. Every dataset has one, and every statistical test assumes one.
- The **normal distribution** arises from additive effects and is defined by mean and standard deviation. It is appropriate for measurement error and many physiological traits.
- **Expression has no single universal distribution** — raw counts, normalized abundance, transformed values, genes, and samples answer different questions. Choose a count model or transformation from the measurement process and inspect model residuals.
- The **Poisson distribution** models count data (reads, mutations) with the key property that mean equals variance. When variance exceeds the mean (overdispersion), use the negative binomial instead.
- The **binomial distribution** models fixed trials with a success probability — relevant for genotype frequencies and allele sampling.
- **Q-Q plots** are the most informative visual diagnostic for distribution checking. The **Shapiro-Wilk test** provides a formal hypothesis test for normality.
- Choosing the right distribution is not optional — it determines which statistical tests are valid and which will produce misleading results.

## What's Next

You now understand the shapes that biological data takes. But distributions describe what values are likely — which is just another way of saying they describe probabilities. Tomorrow, on Day 4, we formalize probability itself. You will learn to compute the chance that a child inherits a BRCA1 mutation, understand why a positive genetic test might mean less than you think (Bayes' theorem will surprise you), and discover why the prosecutor's fallacy has sent innocent people to prison. Probability is the language of uncertainty, and uncertainty is the native tongue of biology.
