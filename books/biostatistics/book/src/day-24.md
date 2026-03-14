# Day 24: Bayesian Thinking for Biologists

<div class="day-meta">
<span class="badge">Day 24 of 30</span>
<span class="badge">Prerequisites: Days 4, 6-7</span>
<span class="badge">~60 min reading</span>
<span class="badge">Bayesian Inference</span>
</div>

## The Problem

A clinical sequencing lab has found a missense variant in a patient's BRCA2 gene. The patient has a family history of breast cancer. ClinVar classifies the variant as "Uncertain Significance" — VUS. The clinician needs to make a decision: recommend risk-reducing surgery, or watchful waiting?

You have several pieces of evidence:

1. **Population frequency**: The variant appears in 0.02% of a population database (gnomAD). Pathogenic BRCA2 variants are typically rare, but many rare variants are benign.
2. **Computational prediction**: Three algorithms (SIFT, PolyPhen, CADD) predict the variant is "likely damaging."
3. **Functional assay**: A cell-based splicing assay shows mild disruption.
4. **Family data**: Two of three affected relatives carry the variant (one does not, but that could be a phenocopy).

No single piece of evidence is conclusive. Each is uncertain. But together, they should shift your belief about pathogenicity. How do you combine them?

The frequentist framework has no natural mechanism for combining prior knowledge with new data. The Bayesian framework does — it is literally designed for this. Today, you will learn how to think like a Bayesian, and you will see why this way of reasoning is becoming standard practice in clinical variant classification.

## What Is Bayesian Statistics?

Bayesian statistics treats probability as a measure of belief, not a long-run frequency. Instead of asking "what would happen if I repeated this experiment infinitely many times?" it asks "given what I know, how confident should I be?"

The fundamental equation is Bayes' theorem:

**Posterior = (Likelihood x Prior) / Evidence**

Or more precisely:

P(H | D) = P(D | H) x P(H) / P(D)

Where:
- **P(H)** = Prior: your belief about hypothesis H before seeing data
- **P(D | H)** = Likelihood: probability of observing data D if H is true
- **P(H | D)** = Posterior: your updated belief after seeing the data
- **P(D)** = Evidence: total probability of the data (a normalizing constant)

### The Recipe

1. **Start with a prior**: What do you believe before seeing any data?
2. **Observe data**: Collect evidence.
3. **Compute the likelihood**: How probable is this data under each hypothesis?
4. **Update to get the posterior**: Combine prior and likelihood.

The posterior from one analysis becomes the prior for the next — evidence accumulates naturally.

> **Key insight:** Bayesian inference is sequential updating. Each new piece of evidence shifts your belief. This is exactly how clinical variant classification works: you start with a prior (population frequency suggests most variants are benign), then update with each line of evidence (computational predictions, functional assays, segregation data).

## Frequentist vs Bayesian: The Practical Difference

Consider testing whether a new drug reduces blood pressure.

**Frequentist answer**: "If the drug had no effect, there is a 2% probability of observing data this extreme. Therefore, p = 0.02, and we reject the null."

**Bayesian answer**: "Given the prior evidence and this data, there is a 94% probability that the drug reduces blood pressure, and the most likely reduction is 5 mmHg with a 95% credible interval of [1.2, 8.8] mmHg."

| Aspect | Frequentist | Bayesian |
|---|---|---|
| Probability means | Long-run frequency | Degree of belief |
| Parameters are | Fixed but unknown | Random variables with distributions |
| Prior knowledge | Not formally incorporated | Explicitly included via priors |
| Result | p-value, confidence interval | Posterior distribution, credible interval |
| "95% interval" means | 95% of such intervals would contain the true value | 95% probability the parameter is in this interval |
| Multiple comparisons | Requires correction (Day 12) | Naturally skeptical with informative priors |
| Small samples | Unreliable (CLT breaks down) | Works with proper priors |

> **Common pitfall:** A frequentist 95% confidence interval does NOT mean "95% probability the parameter is in this interval." It means "if we repeated the experiment many times, 95% of intervals constructed this way would contain the true value." This distinction confuses almost everyone. The Bayesian credible interval actually does mean what most people think a confidence interval means.

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="340" viewBox="0 0 680 340" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="28" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">Bayesian Updating: Prior + Data = Posterior</text>
  <!-- Panel 1: Prior -->
  <rect x="15" y="48" width="195" height="195" rx="6" fill="white" stroke="#3b82f6" stroke-width="1.5"/>
  <text x="112" y="68" text-anchor="middle" font-size="12" font-weight="bold" fill="#3b82f6">Prior Belief</text>
  <!-- Wide, flat-ish bell curve -->
  <polyline points="35,215 50,210 65,198 80,178 95,155 110,140 125,132 140,138 155,152 170,175 185,198 195,212" fill="none" stroke="#3b82f6" stroke-width="2.5"/>
  <polygon points="35,215 50,210 65,198 80,178 95,155 110,140 125,132 140,138 155,152 170,175 185,198 195,212 195,220 35,220" fill="#3b82f6" opacity="0.1"/>
  <line x1="35" y1="220" x2="195" y2="220" stroke="#9ca3af" stroke-width="1"/>
  <text x="112" y="238" text-anchor="middle" font-size="11" fill="#6b7280">Parameter value</text>
  <text x="112" y="90" text-anchor="middle" font-size="10" fill="#3b82f6">Wide, uncertain</text>
  <text x="112" y="104" text-anchor="middle" font-size="10" fill="#3b82f6">"I think ~10% but unsure"</text>

  <!-- Multiplication sign -->
  <text x="225" y="155" text-anchor="middle" font-size="22" font-weight="bold" fill="#9ca3af">&times;</text>

  <!-- Panel 2: Likelihood -->
  <rect x="245" y="48" width="195" height="195" rx="6" fill="white" stroke="#16a34a" stroke-width="1.5"/>
  <text x="342" y="68" text-anchor="middle" font-size="12" font-weight="bold" fill="#16a34a">Likelihood (Data)</text>
  <!-- Narrower bell curve, slightly shifted -->
  <polyline points="270,218 285,215 300,205 315,178 330,142 345,118 355,110 365,118 375,142 390,180 405,208 420,218" fill="none" stroke="#16a34a" stroke-width="2.5"/>
  <polygon points="270,218 285,215 300,205 315,178 330,142 345,118 355,110 365,118 375,142 390,180 405,208 420,218 420,220 270,220" fill="#16a34a" opacity="0.1"/>
  <line x1="270" y1="220" x2="420" y2="220" stroke="#9ca3af" stroke-width="1"/>
  <text x="342" y="238" text-anchor="middle" font-size="11" fill="#6b7280">Parameter value</text>
  <text x="342" y="90" text-anchor="middle" font-size="10" fill="#16a34a">Data says ~15%</text>
  <text x="342" y="104" text-anchor="middle" font-size="10" fill="#16a34a">3 carriers in 200 tested</text>

  <!-- Equals sign -->
  <text x="455" y="155" text-anchor="middle" font-size="22" font-weight="bold" fill="#9ca3af">=</text>

  <!-- Panel 3: Posterior -->
  <rect x="475" y="48" width="195" height="195" rx="6" fill="white" stroke="#7c3aed" stroke-width="1.5"/>
  <text x="572" y="68" text-anchor="middle" font-size="12" font-weight="bold" fill="#7c3aed">Posterior Belief</text>
  <!-- Tall narrow bell curve (more certain) -->
  <polyline points="510,218 520,215 535,205 548,175 558,135 568,105 575,95 582,105 592,135 602,175 615,205 630,215 645,218" fill="none" stroke="#7c3aed" stroke-width="2.5"/>
  <polygon points="510,218 520,215 535,205 548,175 558,135 568,105 575,95 582,105 592,135 602,175 615,205 630,215 645,218 645,220 510,220" fill="#7c3aed" opacity="0.15"/>
  <line x1="510" y1="220" x2="645" y2="220" stroke="#9ca3af" stroke-width="1"/>
  <text x="572" y="238" text-anchor="middle" font-size="11" fill="#6b7280">Parameter value</text>
  <text x="572" y="90" text-anchor="middle" font-size="10" fill="#7c3aed">Narrower, more certain</text>
  <text x="572" y="104" text-anchor="middle" font-size="10" fill="#7c3aed">Compromise: ~13%</text>

  <!-- Bottom note -->
  <rect x="15" y="258" width="650" height="70" rx="6" fill="#f1f5f9"/>
  <text x="340" y="278" text-anchor="middle" font-size="12" fill="#1e293b"><tspan font-weight="bold">Key:</tspan> The posterior combines prior and data. With more data, the posterior narrows (more certainty)</text>
  <text x="340" y="298" text-anchor="middle" font-size="12" fill="#1e293b">and shifts toward the data. With a <tspan font-weight="bold">flat prior</tspan>, the posterior equals the likelihood.</text>
  <text x="340" y="318" text-anchor="middle" font-size="11" fill="#6b7280">Posterior = (Prior &times; Likelihood) / Evidence</text>
</svg>
</div>

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="280" viewBox="0 0 680 280" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="28" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">Beta Distribution Shapes</text>
  <!-- Panel 1: Beta(1,1) = flat -->
  <rect x="15" y="45" width="150" height="130" rx="5" fill="white" stroke="#e5e7eb"/>
  <text x="90" y="62" text-anchor="middle" font-size="11" font-weight="bold" fill="#2563eb">Beta(1,1) = Flat</text>
  <line x1="35" y1="120" x2="145" y2="120" stroke="#2563eb" stroke-width="2.5"/>
  <line x1="35" y1="150" x2="145" y2="150" stroke="#9ca3af" stroke-width="0.5"/>
  <text x="35" y="164" font-size="9" fill="#9ca3af">0</text>
  <text x="145" y="164" text-anchor="end" font-size="9" fill="#9ca3af">1</text>
  <text x="90" y="80" text-anchor="middle" font-size="10" fill="#6b7280">"No idea"</text>

  <!-- Panel 2: Beta(2,5) = left-skewed -->
  <rect x="180" y="45" width="150" height="130" rx="5" fill="white" stroke="#e5e7eb"/>
  <text x="255" y="62" text-anchor="middle" font-size="11" font-weight="bold" fill="#dc2626">Beta(2,5)</text>
  <polyline points="200,148 210,130 220,105 230,88 240,95 250,110 260,125 270,135 280,142 290,146 300,148 310,149" fill="none" stroke="#dc2626" stroke-width="2.5"/>
  <polygon points="200,148 210,130 220,105 230,88 240,95 250,110 260,125 270,135 280,142 290,146 300,148 310,149 310,150 200,150" fill="#dc2626" opacity="0.1"/>
  <line x1="200" y1="150" x2="310" y2="150" stroke="#9ca3af" stroke-width="0.5"/>
  <text x="200" y="164" font-size="9" fill="#9ca3af">0</text>
  <text x="310" y="164" text-anchor="end" font-size="9" fill="#9ca3af">1</text>
  <text x="255" y="80" text-anchor="middle" font-size="10" fill="#6b7280">"Probably low"</text>

  <!-- Panel 3: Beta(5,2) = right-skewed -->
  <rect x="345" y="45" width="150" height="130" rx="5" fill="white" stroke="#e5e7eb"/>
  <text x="420" y="62" text-anchor="middle" font-size="11" font-weight="bold" fill="#16a34a">Beta(5,2)</text>
  <polyline points="365,149 375,148 385,146 395,142 405,135 415,125 425,110 435,95 445,88 455,105 465,130 475,148" fill="none" stroke="#16a34a" stroke-width="2.5"/>
  <polygon points="365,149 375,148 385,146 395,142 405,135 415,125 425,110 435,95 445,88 455,105 465,130 475,148 475,150 365,150" fill="#16a34a" opacity="0.1"/>
  <line x1="365" y1="150" x2="475" y2="150" stroke="#9ca3af" stroke-width="0.5"/>
  <text x="365" y="164" font-size="9" fill="#9ca3af">0</text>
  <text x="475" y="164" text-anchor="end" font-size="9" fill="#9ca3af">1</text>
  <text x="420" y="80" text-anchor="middle" font-size="10" fill="#6b7280">"Probably high"</text>

  <!-- Panel 4: Beta(10,10) = peaked symmetric -->
  <rect x="510" y="45" width="150" height="130" rx="5" fill="white" stroke="#e5e7eb"/>
  <text x="585" y="62" text-anchor="middle" font-size="11" font-weight="bold" fill="#7c3aed">Beta(10,10)</text>
  <polyline points="530,149 540,148 550,142 560,130 570,108 580,90 585,85 590,90 600,108 610,130 620,142 630,148 640,149" fill="none" stroke="#7c3aed" stroke-width="2.5"/>
  <polygon points="530,149 540,148 550,142 560,130 570,108 580,90 585,85 590,90 600,108 610,130 620,142 630,148 640,149 640,150 530,150" fill="#7c3aed" opacity="0.1"/>
  <line x1="530" y1="150" x2="640" y2="150" stroke="#9ca3af" stroke-width="0.5"/>
  <text x="530" y="164" font-size="9" fill="#9ca3af">0</text>
  <text x="640" y="164" text-anchor="end" font-size="9" fill="#9ca3af">1</text>
  <text x="585" y="80" text-anchor="middle" font-size="10" fill="#6b7280">"Fairly sure ~50%"</text>

  <!-- Bottom explanation -->
  <rect x="15" y="190" width="650" height="80" rx="6" fill="#f1f5f9"/>
  <text x="340" y="212" text-anchor="middle" font-size="12" fill="#1e293b">The <tspan font-weight="bold">Beta(a, b)</tspan> distribution lives on [0, 1] and models beliefs about probabilities.</text>
  <text x="340" y="232" text-anchor="middle" font-size="12" fill="#1e293b">Mean = a / (a + b). Larger a + b = more concentrated (more confident).</text>
  <text x="340" y="255" text-anchor="middle" font-size="11" fill="#6b7280">These priors express different degrees and directions of prior knowledge before seeing data.</text>
</svg>
</div>

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="260" viewBox="0 0 680 260" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="28" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">Credible Interval vs Confidence Interval</text>
  <!-- Bayesian side -->
  <rect x="20" y="48" width="300" height="180" rx="6" fill="white" stroke="#7c3aed" stroke-width="1.5"/>
  <text x="170" y="68" text-anchor="middle" font-size="13" font-weight="bold" fill="#7c3aed">Bayesian: 95% Credible Interval</text>
  <!-- Posterior curve -->
  <polyline points="50,180 65,175 80,162 95,140 110,115 125,95 140,82 155,78 170,82 185,95 200,115 215,140 230,162 245,175 260,180 280,182" fill="none" stroke="#7c3aed" stroke-width="2"/>
  <!-- Shaded region -->
  <polygon points="80,162 95,140 110,115 125,95 140,82 155,78 170,82 185,95 200,115 215,140 230,162 230,185 80,185" fill="#7c3aed" opacity="0.15"/>
  <!-- CI lines -->
  <line x1="80" y1="162" x2="80" y2="195" stroke="#7c3aed" stroke-width="1.5" stroke-dasharray="4,3"/>
  <line x1="230" y1="162" x2="230" y2="195" stroke="#7c3aed" stroke-width="1.5" stroke-dasharray="4,3"/>
  <line x1="80" y1="192" x2="230" y2="192" stroke="#7c3aed" stroke-width="2"/>
  <text x="155" y="208" text-anchor="middle" font-size="10" font-weight="bold" fill="#7c3aed">95% of this area</text>
  <text x="170" y="222" text-anchor="middle" font-size="10" fill="#1e293b">"95% probability the</text>
  <text x="170" y="234" text-anchor="middle" font-size="10" fill="#1e293b">parameter is in this range"</text>

  <!-- Frequentist side -->
  <rect x="360" y="48" width="300" height="180" rx="6" fill="white" stroke="#2563eb" stroke-width="1.5"/>
  <text x="510" y="68" text-anchor="middle" font-size="13" font-weight="bold" fill="#2563eb">Frequentist: 95% Confidence Interval</text>
  <!-- Multiple interval lines representing repeated experiments -->
  <g font-size="10" fill="#6b7280">
    <text x="375" y="92">Exp 1:</text>
    <line x1="420" y1="88" x2="560" y2="88" stroke="#3b82f6" stroke-width="2"/>
    <circle cx="490" cy="88" r="3" fill="#3b82f6"/>
    <text x="375" y="110">Exp 2:</text>
    <line x1="430" y1="106" x2="570" y2="106" stroke="#3b82f6" stroke-width="2"/>
    <circle cx="500" cy="106" r="3" fill="#3b82f6"/>
    <text x="375" y="128">Exp 3:</text>
    <line x1="415" y1="124" x2="555" y2="124" stroke="#dc2626" stroke-width="2"/>
    <circle cx="485" cy="124" r="3" fill="#dc2626"/>
    <text x="565" y="128" fill="#dc2626">misses!</text>
    <text x="375" y="146">Exp 4:</text>
    <line x1="435" y1="142" x2="575" y2="142" stroke="#3b82f6" stroke-width="2"/>
    <circle cx="505" cy="142" r="3" fill="#3b82f6"/>
    <text x="375" y="164">Exp 5:</text>
    <line x1="425" y1="160" x2="565" y2="160" stroke="#3b82f6" stroke-width="2"/>
    <circle cx="495" cy="160" r="3" fill="#3b82f6"/>
  </g>
  <!-- True parameter line -->
  <line x1="500" y1="78" x2="500" y2="175" stroke="#1e293b" stroke-width="1.5" stroke-dasharray="6,3"/>
  <text x="500" y="185" text-anchor="middle" font-size="10" font-weight="bold" fill="#1e293b">True value</text>
  <text x="510" y="202" text-anchor="middle" font-size="10" fill="#1e293b">"95% of such intervals</text>
  <text x="510" y="214" text-anchor="middle" font-size="10" fill="#1e293b">contain the true value"</text>

  <!-- Bottom comparison -->
  <rect x="20" y="235" width="640" height="20" rx="4" fill="#f1f5f9"/>
  <text x="340" y="250" text-anchor="middle" font-size="11" fill="#6b7280">The Bayesian interval has the direct interpretation most people want; the frequentist interval is about the procedure, not any single interval.</text>
</svg>
</div>

## The Beta-Binomial Model

The beta-binomial model is the entry point to Bayesian statistics because it is elegant, intuitive, and directly applicable to biology. It applies whenever your data is a count of successes out of trials — exactly the situation with variant frequencies, mutation rates, response rates, and detection probabilities.

### The Setup

- **Data**: k successes in n trials (e.g., 3 pathogenic carriers out of 200 individuals tested)
- **Parameter**: theta (the true probability of success)
- **Prior**: Beta(alpha, beta) distribution
- **Posterior**: Beta(alpha + k, beta + n - k)

The Beta distribution is "conjugate" to the Binomial — meaning the posterior is the same type of distribution as the prior, just with updated parameters. This makes computation trivial.

### Understanding the Beta Distribution

The Beta(alpha, beta) distribution lives on [0, 1] and describes beliefs about probabilities:

| Prior | alpha | beta | Interpretation |
|---|---|---|---|
| Beta(1, 1) | 1 | 1 | Uniform — "I have no idea, any probability is equally likely" |
| Beta(0.5, 0.5) | 0.5 | 0.5 | Jeffreys' prior — slightly favors extremes |
| Beta(10, 90) | 10 | 90 | "I believe the probability is around 10%" |
| Beta(1, 99) | 1 | 99 | "I believe the probability is very low (~1%)" |
| Beta(50, 50) | 50 | 50 | "I'm fairly sure it's around 50%" |

The mean of Beta(alpha, beta) is alpha / (alpha + beta). The larger alpha + beta, the more concentrated (confident) the distribution.

```bio
# Visualize different Beta priors
# The Beta distribution is not a BioLang builtin, but we can
# compute and visualize it using the formula:
# Beta(x; a, b) ~ x^(a-1) * (1-x)^(b-1)
let x = seq(0.01, 0.99, 0.01)

# Plot Beta(2,8) — slightly low probability
let beta_28 = x |> map(|v| pow(v, 1) * pow(1 - v, 7))
let total = sum(beta_28) * 0.01
let beta_28_norm = beta_28 |> map(|v| v / total)
let tbl = zip(x, beta_28_norm) |> map(|p| {x: p[0], density: p[1]}) |> to_table()
plot(tbl, {type: "line", x: "x", y: "density", title: "Beta(2,8) Prior"})
```

### Conjugate Updating

The magic of the beta-binomial model: if your prior is Beta(alpha, beta) and you observe k successes in n trials, the posterior is simply:

**Posterior = Beta(alpha + k, beta + n - k)**

No integrals, no MCMC, no approximations. Just add.

```bio
# Variant frequency estimation
# Prior: Beta(1, 99) — we expect ~1% carrier frequency
# Data: 3 carriers out of 200 tested

let prior_alpha = 1
let prior_beta = 99
let k = 3      # successes (carriers)
let n = 200    # trials (individuals)

let post_alpha = prior_alpha + k
let post_beta = prior_beta + (n - k)

# Posterior mean = alpha / (alpha + beta)
let post_mean = post_alpha / (post_alpha + post_beta)

# Approximate 95% credible interval using normal approximation
let post_var = (post_alpha * post_beta) / (pow(post_alpha + post_beta, 2) * (post_alpha + post_beta + 1))
let post_sd = sqrt(post_var)
let ci_lower = post_mean - 1.96 * post_sd
let ci_upper = post_mean + 1.96 * post_sd

print("Prior: Beta(" + str(prior_alpha) + ", " + str(prior_beta) + ")")
print("Prior mean: " + str(round(prior_alpha / (prior_alpha + prior_beta), 4)))
print("Posterior: Beta(" + str(post_alpha) + ", " + str(post_beta) + ")")
print("Posterior mean: " + str(round(post_mean, 4)))
print("95% credible interval: [" +
  str(round(ci_lower, 4)) + ", " +
  str(round(ci_upper, 4)) + "]")
```

## Credible Intervals vs Confidence Intervals

A 95% **credible interval** (Bayesian) contains the parameter with 95% probability. Full stop. This is the direct, intuitive interpretation.

A 95% **confidence interval** (frequentist) is a procedure that, if repeated many times, would contain the true parameter 95% of the time. For any specific interval, you cannot say the parameter is "95% likely" to be inside.

In practice, for large samples and weak priors, the two intervals are nearly identical. The difference matters most with small samples or strong priors.

```bio
# Compare Bayesian credible interval to frequentist CI

# Data: 8 responders out of 20 patients
let k = 8
let n = 20

# Bayesian with flat prior: Beta(1,1) + data = Beta(1+8, 1+12) = Beta(9, 13)
let a = 1 + k
let b = 1 + (n - k)
let bayes_mean = a / (a + b)
let bayes_var = (a * b) / (pow(a + b, 2) * (a + b + 1))
let bayes_sd = sqrt(bayes_var)
let bayes_lower = bayes_mean - 1.96 * bayes_sd
let bayes_upper = bayes_mean + 1.96 * bayes_sd

# Frequentist (Wilson interval)
let p_hat = k / n
let z = 1.96
let wilson_lower = (p_hat + z*z/(2*n) - z*sqrt(p_hat*(1-p_hat)/n + z*z/(4*n*n))) / (1 + z*z/n)
let wilson_upper = (p_hat + z*z/(2*n) + z*sqrt(p_hat*(1-p_hat)/n + z*z/(4*n*n))) / (1 + z*z/n)

print("Bayesian 95% credible interval: [" +
  str(round(bayes_lower, 3)) + ", " +
  str(round(bayes_upper, 3)) + "]")
print("Frequentist 95% Wilson CI:      [" +
  str(round(wilson_lower, 3)) + ", " +
  str(round(wilson_upper, 3)) + "]")
```

## Bayesian Normal Estimation

For continuous data (not just counts), the Bayesian approach models the unknown mean with a normal prior. If the data is also normal, the posterior for the mean is a normal distribution with:

- **Posterior mean** = weighted average of prior mean and data mean
- **Posterior variance** = combination of prior variance and data variance

The weight given to the data versus the prior depends on sample size. With large n, the data dominates and the prior barely matters.

```bio
# Bayesian estimation of mean enzyme activity
let data = [4.2, 5.1, 3.8, 6.3, 4.9, 5.5, 4.7, 5.8]

# Prior: mean = 4.0, sd = 2.0 (vague prior based on literature)
let prior_mean = 4.0
let prior_sd = 2.0
let prior_var = pow(prior_sd, 2)

# Data summary
let n = len(data)
let data_mean = mean(data)
let data_var = variance(data)

# Normal-Normal conjugate update
# Posterior precision = prior precision + data precision
let prior_prec = 1.0 / prior_var
let data_prec = n / data_var
let post_prec = prior_prec + data_prec
let post_var = 1.0 / post_prec
let post_sd = sqrt(post_var)

# Posterior mean = weighted average
let post_mean = (prior_prec * prior_mean + data_prec * data_mean) / post_prec

# 95% credible interval
let ci_lower = post_mean - 1.96 * post_sd
let ci_upper = post_mean + 1.96 * post_sd

print("Prior mean: 4.0, Prior SD: 2.0")
print("Data mean: " + str(round(data_mean, 2)))
print("Posterior mean: " + str(round(post_mean, 3)))
print("Posterior SD: " + str(round(post_sd, 3)))
print("95% credible interval: [" +
  str(round(ci_lower, 3)) + ", " +
  str(round(ci_upper, 3)) + "]")
```

### Prior Sensitivity

A critical question: how much does the prior matter? The answer depends on sample size.

```bio
# Same data, three different priors
let data = [4.2, 5.1, 3.8, 6.3, 4.9, 5.5, 4.7, 5.8]
let n = len(data)
let data_mean = mean(data)
let data_var = variance(data)
let data_prec = n / data_var

# Helper: compute posterior mean for normal-normal conjugate
# post_mean = (prior_prec * prior_mean + data_prec * data_mean) / (prior_prec + data_prec)

# Flat (vague) prior: mean=0, sd=100
let flat_prec = 1.0 / pow(100, 2)
let flat_post = (flat_prec * 0 + data_prec * data_mean) / (flat_prec + data_prec)

# Informative prior centered on truth: mean=5.0, sd=1.0
let good_prec = 1.0 / pow(1.0, 2)
let good_post = (good_prec * 5.0 + data_prec * data_mean) / (good_prec + data_prec)

# Informative prior centered far from truth: mean=10.0, sd=1.0
let bad_prec = 1.0 / pow(1.0, 2)
let bad_post = (bad_prec * 10.0 + data_prec * data_mean) / (bad_prec + data_prec)

print("Flat prior:        posterior mean = " + str(round(flat_post, 3)))
print("Good prior (5.0):  posterior mean = " + str(round(good_post, 3)))
print("Bad prior (10.0):  posterior mean = " + str(round(bad_post, 3)))
print("Data mean:         " + str(round(data_mean, 3)))
```

With only 8 observations, the informative priors pull the posterior toward them. With 800 observations, even a badly wrong prior would be overwhelmed by the data.

> **Common pitfall:** Using a highly informative prior with little data is dangerous — the prior dominates. Use weakly informative priors (centered on a reasonable value but with wide spread) unless you have strong external evidence to justify a tight prior. A flat prior is always safe but may be less efficient.

## Posterior Predictive Distribution

The posterior predictive distribution answers: "Given what I have learned from the data, what do I expect to see in future observations?"

This is enormously practical. After estimating a variant's pathogenicity probability from a training dataset, you want to predict: if I sequence the next 100 patients, how many carriers will I find?

```bio
set_seed(42)
# Posterior predictive for variant carrier count
# Prior: Beta(1, 99), Data: 3 carriers out of 200
let post_a = 1 + 3       # = 4
let post_b = 99 + 197    # = 296

# Posterior mean carrier frequency
let post_mean = post_a / (post_a + post_b)
print("Posterior mean frequency: " + str(round(post_mean, 4)))

# Simulate posterior predictive for next 500 individuals
# Draw theta from Beta posterior, then draw count from Binomial(500, theta)
let n_future = 500
let n_sim = 10000
let predictions = range(0, n_sim) |> map(|i| {
  # Approximate Beta draw using normal approximation
  let theta = post_mean + rnorm(1)[0] * sqrt(post_mean * (1 - post_mean) / (post_a + post_b))
  let theta_clamp = max(0.001, min(0.999, theta))
  # Simulate binomial count
  let count = range(0, n_future) |> filter(|j| rnorm(1)[0] < qnorm(theta_clamp)) |> len()
  count
})

let pred_mean = mean(predictions)
let sorted_pred = sort(predictions)
let ci_lo = sorted_pred[round(n_sim * 0.025, 0)]
let ci_hi = sorted_pred[round(n_sim * 0.975, 0)]

print("Predicted carriers in 500 individuals:")
print("  Mean: " + str(round(pred_mean, 1)))
print("  95% prediction interval: [" + str(ci_lo) + ", " + str(ci_hi) + "]")

histogram(predictions, {bins: 30,
  title: "Posterior Predictive — Carriers in Next 500",
  xlabel: "Number of Carriers", ylabel: "Frequency"})
```

## Sequential Updating: Evidence Accumulates

The most natural aspect of Bayesian analysis is sequential updating. The posterior from one analysis becomes the prior for the next. This mirrors how evidence actually accumulates in science.

```bio
# Variant pathogenicity: sequential updating with multiple evidence

# Step 1: Start with population base rate
# ~10% of rare missense variants in BRCA2 are pathogenic
let prior_a = 10
let prior_b = 90

print("=== Initial Prior ===")
print("P(pathogenic) ~ " +
  str(round(prior_a / (prior_a + prior_b), 3)))

# Step 2: Update with computational predictions
# 3 of 3 algorithms predict "damaging"
# For pathogenic variants, ~80% get 3/3 damaging
# For benign variants, ~15% get 3/3 damaging
# Likelihood ratio = 0.80 / 0.15 = 5.33
let lr1 = 0.80 / 0.15
let post1_a = prior_a * lr1
let post1_b = prior_b

print("\n=== After Computational Predictions ===")
print("P(pathogenic) ~ " +
  str(round(post1_a / (post1_a + post1_b), 3)))

# Step 3: Update with functional assay
# Mild splicing disruption
# For pathogenic variants: 60% show mild disruption
# For benign variants: 10% show mild disruption
# LR = 6.0
let lr2 = 0.60 / 0.10
let post2_a = post1_a * lr2
let post2_b = post1_b

print("\n=== After Functional Assay ===")
print("P(pathogenic) ~ " +
  str(round(post2_a / (post2_a + post2_b), 3)))

# Step 4: Update with family segregation
# 2 of 3 affected carry the variant
# For pathogenic: ~90% chance of this pattern
# For benign: ~25% chance (random segregation)
# LR = 3.6
let lr3 = 0.90 / 0.25
let post3_a = post2_a * lr3
let post3_b = post2_b

print("\n=== After Family Segregation ===")
print("P(pathogenic) ~ " +
  str(round(post3_a / (post3_a + post3_b), 3)))

let final_prob = post3_a / (post3_a + post3_b)
print("Final P(pathogenic): " + str(round(final_prob, 4)))
print("Final classification: " +
  if post3_a / (post3_a + post3_b) > 0.99 { "Pathogenic" }
  else if post3_a / (post3_a + post3_b) > 0.90 { "Likely Pathogenic" }
  else if post3_a / (post3_a + post3_b) < 0.10 { "Likely Benign" }
  else if post3_a / (post3_a + post3_b) < 0.01 { "Benign" }
  else { "VUS" }
)
```

> **Clinical relevance:** The ACMG/AMP variant classification framework (Richards et al., 2015) is implicitly Bayesian. It combines evidence from population data, computational predictions, functional studies, segregation data, and de novo status to classify variants into five tiers (Pathogenic, Likely Pathogenic, VUS, Likely Benign, Benign). Tavtigian et al. (2018) formalized this as an explicit Bayesian framework using likelihood ratios — exactly the approach shown above.

## When Bayesian Is Better — and When It Is Overkill

### Bayesian excels when:

- **Prior information exists** and should be formally incorporated (variant classification, clinical trials with historical data)
- **Sequential updating** is needed (monitoring a clinical trial as data accumulates)
- **Small samples** make frequentist methods unreliable (the prior stabilizes estimates)
- **You want direct probability statements** ("94% probability the drug works" vs "p = 0.02")
- **Multiple comparisons**: informative priors act as natural regularization, reducing false positives

### Frequentist is fine when:

- **No meaningful prior** exists (purely exploratory analysis)
- **Sample is large** (prior is overwhelmed anyway, results converge)
- **Regulatory requirements** demand frequentist analysis (FDA still primarily uses p-values)
- **Simplicity matters** (t-test is faster to explain than posterior distributions)

## Bayesian Thinking in BioLang

```bio
set_seed(42)
# ============================================
# Complete Bayesian workflow: drug response rate
# ============================================

# Prior: previous Phase II trial found 30% response rate in 40 patients
let prior_alpha = 12   # 30% of 40 = 12 responders
let prior_beta = 28    # 70% of 40 = 28 non-responders

# New data: Phase III trial, 45 responders out of 120 patients
let k = 45
let n = 120

# 1. Bayesian update (Beta-Binomial conjugate)
let post_a = prior_alpha + k
let post_b = prior_beta + (n - k)
let post_mean = post_a / (post_a + post_b)
let post_var = (post_a * post_b) / (pow(post_a + post_b, 2) * (post_a + post_b + 1))
let post_sd = sqrt(post_var)
let ci_lower = post_mean - 1.96 * post_sd
let ci_upper = post_mean + 1.96 * post_sd

print("=== Drug Response Rate Estimation ===")
print("Prior (Phase II): " + str(round(prior_alpha / (prior_alpha + prior_beta), 3)))
print("Data (Phase III): " + str(round(k / n, 3)))
print("Posterior mean: " + str(round(post_mean, 3)))
print("95% credible interval: [" +
  str(round(ci_lower, 3)) + ", " +
  str(round(ci_upper, 3)) + "]")

# 2. Visualize prior and posterior as Beta density curves
let xs = seq(0.1, 0.6, 0.005)
let prior_curve = xs |> map(|x| {
  x: x,
  density: pow(x, prior_alpha - 1) * pow(1 - x, prior_beta - 1),
  series: "Prior"
})
let post_curve = xs |> map(|x| {
  x: x,
  density: pow(x, post_a - 1) * pow(1 - x, post_b - 1),
  series: "Posterior"
})
let curves = concat(prior_curve, post_curve) |> to_table()
plot(curves, {type: "line", x: "x", y: "density", color: "series",
  title: "Prior vs Posterior"})

# 3. Probability response rate exceeds 30%
# Use normal approximation to Beta CDF
let z_30 = (0.30 - post_mean) / post_sd
let p_above_30 = 1.0 - pnorm(z_30)
print("\nP(response rate > 30%): " + str(round(p_above_30, 3)))

# 4. Probability response rate exceeds standard-of-care (25%)
let z_25 = (0.25 - post_mean) / post_sd
let p_above_soc = 1.0 - pnorm(z_25)
print("P(response rate > 25% standard-of-care): " + str(round(p_above_soc, 3)))

# 5. Posterior predictive: next 200 patients
# Expected responders = n_future * posterior_mean
let n_future = 200
let pred_mean = n_future * post_mean
let pred_sd = sqrt(n_future * post_mean * (1 - post_mean))
print("\nPredicted responders in next 200 patients:")
print("  Mean: " + str(round(pred_mean, 1)))
print("  95% PI: [" + str(round(pred_mean - 1.96 * pred_sd, 0)) +
  ", " + str(round(pred_mean + 1.96 * pred_sd, 0)) + "]")

# 6. Compare flat vs informative prior
let flat_a = 1 + k
let flat_b = 1 + (n - k)
let flat_mean = flat_a / (flat_a + flat_b)

print("\n=== Prior Sensitivity ===")
print("Informative prior -> posterior mean: " + str(round(post_mean, 3)))
print("Flat prior -> posterior mean: " + str(round(flat_mean, 3)))
print("Difference: " + str(round(abs(post_mean - flat_mean), 4)))
```

**Python:**

```python
from scipy import stats
import numpy as np
import matplotlib.pyplot as plt

# Beta-binomial
prior_a, prior_b = 12, 28
k, n = 45, 120
post_a, post_b = prior_a + k, prior_b + n - k

x = np.linspace(0, 1, 1000)
plt.plot(x, stats.beta.pdf(x, prior_a, prior_b), label='Prior')
plt.plot(x, stats.beta.pdf(x, post_a, post_b), label='Posterior')
plt.legend()
plt.show()

# Credible interval
ci = stats.beta.interval(0.95, post_a, post_b)
print(f"95% credible interval: [{ci[0]:.3f}, {ci[1]:.3f}]")

# P(theta > 0.30)
p_above = 1 - stats.beta.cdf(0.30, post_a, post_b)
```

**R:**

```r
# Beta-binomial
prior_a <- 12; prior_b <- 28
k <- 45; n <- 120
post_a <- prior_a + k; post_b <- prior_b + n - k

x <- seq(0, 1, length.out = 1000)
plot(x, dbeta(x, prior_a, prior_b), type="l", col="blue", ylab="Density")
lines(x, dbeta(x, post_a, post_b), col="red")
legend("topright", c("Prior", "Posterior"), col=c("blue","red"), lty=1)

# Credible interval
qbeta(c(0.025, 0.975), post_a, post_b)

# P(theta > 0.30)
1 - pbeta(0.30, post_a, post_b)
```

## Exercises

1. **Flat vs informative prior.** A variant has been seen in 2 out of 500 individuals. Compute the posterior for the population frequency using (a) a flat prior Beta(1,1), (b) an informative prior Beta(1,999) reflecting the belief that pathogenic variants are very rare. How different are the posteriors? At what sample size would the difference become negligible?

```bio
# Your code: two priors, compare posteriors
# Increase n and see when they converge
```

2. **Drug response updating.** Start with a Beta(5, 15) prior (25% response rate from a pilot). You enroll patients one at a time: responder, non-responder, non-responder, responder, responder, non-responder, responder, non-responder, non-responder, responder. After each patient, compute and print the posterior mean and 95% credible interval. Watch the uncertainty shrink.

```bio
# Your code: sequential update, one patient at a time
```

3. **Variant classification.** A VUS has prior odds of pathogenicity = 0.10 (10%). Apply three independent evidence lines with likelihood ratios 4.0, 2.5, and 6.0. What is the final posterior probability of pathogenicity? Would this variant be classified as "Likely Pathogenic" (>0.90)?

```bio
# Your code: sequential likelihood ratio updating
```

4. **Posterior predictive check.** Estimate a mutation rate from data (15 mutations in 1,000,000 bases). Then simulate 100 future datasets of 1,000,000 bases from the posterior predictive distribution. What range of mutation counts do you expect?

```bio
# Your code: bayes_binomial + posterior_predictive
```

5. **Prior sensitivity analysis.** For the mutation rate problem (15/1,000,000), compute posterior means and 95% CIs using five different priors: Beta(1,1), Beta(0.5,0.5), Beta(1,99999), Beta(15,999985), and Beta(100,6666567). Plot all five posteriors on the same axes. Which priors lead to meaningfully different conclusions?

```bio
# Your code: five priors, compare posteriors
```

## Key Takeaways

- Bayesian inference updates prior beliefs with observed data to produce posterior distributions: Posterior = Prior x Likelihood / Evidence.
- The beta-binomial model is the workhorse for proportions: if the prior is Beta(a, b) and you observe k successes in n trials, the posterior is Beta(a+k, b+n-k).
- Credible intervals have the direct interpretation people usually want: "95% probability the parameter is in this range."
- Sequential updating is natural in Bayesian inference — the posterior from one study becomes the prior for the next, exactly mirroring how scientific evidence accumulates.
- Clinical variant classification (ACMG/AMP) is implicitly Bayesian: prior odds from population data are updated with likelihood ratios from computational, functional, and segregation evidence.
- With large samples, the prior barely matters (the data dominates). With small samples, the prior can strongly influence results — choose it carefully and report sensitivity analyses.
- Bayesian methods complement frequentist methods; neither is universally superior.

## What's Next

We have learned to test hypotheses, build models, reduce dimensions, cluster data, resample, and reason Bayesianly. But all of these methods ultimately produce results that must be communicated — and the primary language of scientific communication is visual. Tomorrow, we tackle statistical visualization: which plot for which data, how to make plots that tell the truth, and how to avoid the visualization sins that plague the literature.
