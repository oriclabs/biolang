# Day 5: Sampling, Bias, and Why n Matters

<div class="day-meta">
<span class="badge">Day 5 of 30</span>
<span class="badge">Prerequisites: Days 1-4</span>
<span class="badge">~55 min</span>
<span class="badge">Hands-on</span>
</div>

## The Problem

Dr. Elena Vasquez is the lead biostatistician for a pharmaceutical company. A new immunotherapy drug has shown promising results in cell lines and mouse models. Now the clinical team is designing the Phase II trial and they want her sign-off on the sample size.

The clinical lead proposes 20 patients per arm — treatment and placebo. "It's faster, cheaper, and we can get to Phase III sooner," he argues. Elena runs the numbers and shakes her head. With 20 patients per arm and the expected effect size, the trial has only a 23% chance of detecting the drug's benefit even if it truly works. That means a 77% chance of concluding the drug is ineffective when it actually saves lives.

She recommends 200 patients per arm. The clinical lead winces at the cost — $12 million more and 18 extra months of enrollment. But Elena is firm: "Would you rather spend $12 million now and know the answer, or spend $50 million on a Phase III that was doomed from the start because Phase II was too small to see the signal?"

This tension — between the cost of collecting more data and the cost of drawing wrong conclusions from too little — is the central drama of experimental design. Today you will understand why sample size is not a bureaucratic detail but the most consequential decision in any study.

## What Are Populations and Samples?

### The Population

The **population** is the complete set of items you want to understand. It is usually too large, too expensive, or physically impossible to measure in its entirety.

| Research Question | Population |
|---|---|
| Does this drug lower blood pressure? | All humans with hypertension |
| Is gene X differentially expressed in tumors? | All tumors of this type, past and future |
| What is the allele frequency of rs1234? | All humans alive today |
| Does this sequencing protocol introduce bias? | All possible runs of this protocol |

### The Sample

The **sample** is the subset you actually observe. Everything you learn comes from the sample, but everything you want to know is about the population. Statistics is the bridge between the two.

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="300" viewBox="0 0 680 300" xmlns="http://www.w3.org/2000/svg" style="background: #fafbfc; border: 1px solid #e5e7eb; border-radius: 8px;">
  <defs>
    <marker id="arrowSamp" markerWidth="10" markerHeight="7" refX="10" refY="3.5" orient="auto"><path d="M0,0 L10,3.5 L0,7 Z" fill="#2563eb"/></marker>
    <marker id="arrowSampR" markerWidth="10" markerHeight="7" refX="10" refY="3.5" orient="auto"><path d="M0,0 L10,3.5 L0,7 Z" fill="#16a34a"/></marker>
  </defs>
  <text x="340" y="26" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">Population vs. Sample</text>
  <!-- Population circle -->
  <circle cx="180" cy="165" r="120" fill="#dbeafe" fill-opacity="0.3" stroke="#2563eb" stroke-width="2.5"/>
  <text x="180" y="58" text-anchor="middle" font-size="13" font-weight="bold" fill="#2563eb">POPULATION</text>
  <!-- Many dots in population -->
  <circle cx="120" cy="110" r="3" fill="#3b82f6" opacity="0.5"/><circle cx="145" cy="95" r="3" fill="#3b82f6" opacity="0.5"/>
  <circle cx="170" cy="100" r="3" fill="#3b82f6" opacity="0.5"/><circle cx="200" cy="90" r="3" fill="#3b82f6" opacity="0.5"/>
  <circle cx="230" cy="105" r="3" fill="#3b82f6" opacity="0.5"/><circle cx="250" cy="120" r="3" fill="#3b82f6" opacity="0.5"/>
  <circle cx="105" cy="140" r="3" fill="#3b82f6" opacity="0.5"/><circle cx="130" cy="135" r="3" fill="#3b82f6" opacity="0.5"/>
  <circle cx="155" cy="125" r="3" fill="#3b82f6" opacity="0.5"/><circle cx="185" cy="130" r="3" fill="#3b82f6" opacity="0.5"/>
  <circle cx="210" cy="118" r="3" fill="#3b82f6" opacity="0.5"/><circle cx="240" cy="140" r="3" fill="#3b82f6" opacity="0.5"/>
  <circle cx="265" cy="155" r="3" fill="#3b82f6" opacity="0.5"/><circle cx="90" cy="170" r="3" fill="#3b82f6" opacity="0.5"/>
  <circle cx="115" cy="165" r="3" fill="#3b82f6" opacity="0.5"/><circle cx="140" cy="155" r="3" fill="#3b82f6" opacity="0.5"/>
  <circle cx="165" cy="148" r="3" fill="#3b82f6" opacity="0.5"/><circle cx="195" cy="155" r="3" fill="#3b82f6" opacity="0.5"/>
  <circle cx="220" cy="145" r="3" fill="#3b82f6" opacity="0.5"/><circle cx="250" cy="165" r="3" fill="#3b82f6" opacity="0.5"/>
  <circle cx="80" cy="195" r="3" fill="#3b82f6" opacity="0.5"/><circle cx="105" cy="190" r="3" fill="#3b82f6" opacity="0.5"/>
  <circle cx="130" cy="180" r="3" fill="#3b82f6" opacity="0.5"/><circle cx="155" cy="175" r="3" fill="#3b82f6" opacity="0.5"/>
  <circle cx="180" cy="170" r="3" fill="#3b82f6" opacity="0.5"/><circle cx="205" cy="178" r="3" fill="#3b82f6" opacity="0.5"/>
  <circle cx="235" cy="185" r="3" fill="#3b82f6" opacity="0.5"/><circle cx="260" cy="195" r="3" fill="#3b82f6" opacity="0.5"/>
  <circle cx="95" cy="215" r="3" fill="#3b82f6" opacity="0.5"/><circle cx="120" cy="210" r="3" fill="#3b82f6" opacity="0.5"/>
  <circle cx="145" cy="200" r="3" fill="#3b82f6" opacity="0.5"/><circle cx="170" cy="195" r="3" fill="#3b82f6" opacity="0.5"/>
  <circle cx="200" cy="200" r="3" fill="#3b82f6" opacity="0.5"/><circle cx="225" cy="210" r="3" fill="#3b82f6" opacity="0.5"/>
  <circle cx="255" cy="220" r="3" fill="#3b82f6" opacity="0.5"/><circle cx="110" cy="235" r="3" fill="#3b82f6" opacity="0.5"/>
  <circle cx="140" cy="228" r="3" fill="#3b82f6" opacity="0.5"/><circle cx="165" cy="220" r="3" fill="#3b82f6" opacity="0.5"/>
  <circle cx="195" cy="225" r="3" fill="#3b82f6" opacity="0.5"/><circle cx="220" cy="235" r="3" fill="#3b82f6" opacity="0.5"/>
  <circle cx="150" cy="248" r="3" fill="#3b82f6" opacity="0.5"/><circle cx="180" cy="245" r="3" fill="#3b82f6" opacity="0.5"/>
  <circle cx="210" cy="250" r="3" fill="#3b82f6" opacity="0.5"/>
  <text x="180" y="280" text-anchor="middle" font-size="11" fill="#2563eb">N = all subjects of interest</text>
  <text x="180" y="294" text-anchor="middle" font-size="10" fill="#6b7280">Usually too large to measure</text>
  <!-- Arrow from population to sample -->
  <line x1="305" y1="140" x2="395" y2="140" stroke="#2563eb" stroke-width="2" marker-end="url(#arrowSamp)"/>
  <text x="350" y="132" text-anchor="middle" font-size="11" font-weight="600" fill="#2563eb">Random</text>
  <text x="350" y="148" text-anchor="middle" font-size="11" font-weight="600" fill="#2563eb">sampling</text>
  <!-- Sample circle -->
  <circle cx="480" cy="145" r="70" fill="#f0fdf4" fill-opacity="0.5" stroke="#16a34a" stroke-width="2.5"/>
  <text x="480" y="68" text-anchor="middle" font-size="13" font-weight="bold" fill="#16a34a">SAMPLE</text>
  <!-- Fewer dots in sample -->
  <circle cx="450" cy="115" r="4" fill="#22c55e" opacity="0.7"/><circle cx="475" cy="108" r="4" fill="#22c55e" opacity="0.7"/>
  <circle cx="500" cy="115" r="4" fill="#22c55e" opacity="0.7"/><circle cx="460" cy="135" r="4" fill="#22c55e" opacity="0.7"/>
  <circle cx="485" cy="130" r="4" fill="#22c55e" opacity="0.7"/><circle cx="510" cy="138" r="4" fill="#22c55e" opacity="0.7"/>
  <circle cx="445" cy="155" r="4" fill="#22c55e" opacity="0.7"/><circle cx="470" cy="152" r="4" fill="#22c55e" opacity="0.7"/>
  <circle cx="495" cy="155" r="4" fill="#22c55e" opacity="0.7"/><circle cx="520" cy="158" r="4" fill="#22c55e" opacity="0.7"/>
  <circle cx="460" cy="175" r="4" fill="#22c55e" opacity="0.7"/><circle cx="490" cy="172" r="4" fill="#22c55e" opacity="0.7"/>
  <circle cx="508" cy="178" r="4" fill="#22c55e" opacity="0.7"/>
  <text x="480" y="228" text-anchor="middle" font-size="11" fill="#16a34a">n = subset observed</text>
  <text x="480" y="242" text-anchor="middle" font-size="10" fill="#6b7280">What we actually measure</text>
  <!-- Arrow back: inference -->
  <line x1="395" y1="195" x2="305" y2="195" stroke="#16a34a" stroke-width="2" marker-end="url(#arrowSampR)"/>
  <text x="350" y="190" text-anchor="middle" font-size="11" font-weight="600" fill="#16a34a">Statistical</text>
  <text x="350" y="205" text-anchor="middle" font-size="11" font-weight="600" fill="#16a34a">inference</text>
  <!-- Quality factors -->
  <rect x="410" y="260" width="240" height="32" rx="4" fill="#fffbeb" stroke="#f59e0b" stroke-width="1"/>
  <text x="530" y="272" text-anchor="middle" font-size="10" fill="#92400e" font-weight="600">Quality depends on: (1) how selected</text>
  <text x="530" y="285" text-anchor="middle" font-size="10" fill="#92400e" font-weight="600">(bias) and (2) how large (precision)</text>
</svg>
</div>

The quality of the bridge depends entirely on two factors:
1. **How the sample was selected** (bias)
2. **How large the sample is** (precision)

> **Key insight:** A large biased sample is worse than a small unbiased one. The 1936 Literary Digest poll surveyed 2.4 million people and predicted Alf Landon would win the presidential election in a landslide. George Gallup surveyed 50,000 and correctly predicted Roosevelt. The Literary Digest sample was drawn from telephone directories and automobile registrations — overrepresenting wealthy voters. Size could not compensate for bias.

## The Sampling Distribution

This is one of the most important concepts in all of statistics, and it is the one that most students find counterintuitive at first.

Imagine you draw a sample of 30 patients, measure their blood pressure, and compute the mean. You get 125 mmHg. Now imagine you draw a different sample of 30 patients and compute the mean. You might get 121 mmHg. A third sample: 128 mmHg.

If you repeated this process thousands of times — each time drawing 30 patients and computing the mean — you would get thousands of sample means. These sample means form a distribution called the **sampling distribution of the mean**.

The sampling distribution is NOT the distribution of individual data points. It is the distribution of a statistic (like the mean) computed from repeated samples.

### Seeing It in Action

```bio
set_seed(42)
# Simulate the sampling distribution

# The "population": blood pressure values (slightly right-skewed)
let population = rnorm(100000, 125, 18)

# Draw 1000 samples of size 30, compute mean of each
let sample_means_30 = []
for i in 0..1000 {
    let s = sample(population, 30)
    sample_means_30 = sample_means_30 + [mean(s)]
}

# Draw 1000 samples of size 200, compute mean of each
let sample_means_200 = []
for i in 0..1000 {
    let s = sample(population, 200)
    sample_means_200 = sample_means_200 + [mean(s)]
}

# Compare the distributions
print("Population:         mean = {mean(population):.1}, SD = {stdev(population):.1}")
print("Sample means (n=30):  mean = {mean(sample_means_30):.1}, SD = {stdev(sample_means_30):.1}")
print("Sample means (n=200): mean = {mean(sample_means_200):.1}, SD = {stdev(sample_means_200):.1}")

histogram(sample_means_30, {bins: 40, title: "Sampling Distribution (n=30)"})
histogram(sample_means_200, {bins: 40, title: "Sampling Distribution (n=200)"})
```

Two crucial observations:
1. Both sampling distributions are centered at the true population mean (~125). Samples are unbiased estimators.
2. The n=200 distribution is much **narrower** than n=30. Larger samples give more precise estimates.

## The Central Limit Theorem

The Central Limit Theorem (CLT) is perhaps the single most important result in statistics. It says:

**Regardless of the shape of the population distribution, the sampling distribution of the mean approaches a normal distribution as sample size increases.**

This is remarkable. The underlying data can be skewed, bimodal, uniform, or any shape at all. As long as you take large enough samples and compute means, those means will be approximately normally distributed.

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="420" viewBox="0 0 680 420" xmlns="http://www.w3.org/2000/svg" style="background: #fafbfc; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="26" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">The Central Limit Theorem in Action</text>
  <text x="340" y="44" text-anchor="middle" font-size="11" fill="#6b7280">Original distribution is heavily right-skewed (exponential)</text>
  <!-- Row 1: Original skewed population -->
  <rect x="30" y="55" width="620" height="78" rx="6" fill="#f8fafc" stroke="#e2e8f0"/>
  <text x="85" y="75" text-anchor="middle" font-size="11" font-weight="600" fill="#dc2626">Population</text>
  <text x="85" y="90" text-anchor="middle" font-size="10" fill="#6b7280">(exponential)</text>
  <line x1="150" y1="120" x2="630" y2="120" stroke="#9ca3af" stroke-width="1"/>
  <path d="M 155,118 Q 160,60 180,68 Q 200,85 240,100 Q 300,110 380,115 Q 480,118 630,118" fill="#fecaca" fill-opacity="0.4" stroke="#dc2626" stroke-width="2"/>
  <text x="580" y="110" font-size="10" fill="#dc2626">Skewness ~ 2.0</text>
  <!-- Arrow down -->
  <text x="340" y="148" text-anchor="middle" font-size="20" fill="#6b7280">&#x25BC;</text>
  <text x="430" y="148" text-anchor="middle" font-size="10" fill="#6b7280">Take repeated samples, compute means</text>
  <!-- Row 2: n=5 -->
  <rect x="30" y="158" width="200" height="78" rx="6" fill="#f8fafc" stroke="#e2e8f0"/>
  <text x="55" y="178" text-anchor="middle" font-size="11" font-weight="600" fill="#7c3aed">n = 5</text>
  <line x1="80" y1="225" x2="220" y2="225" stroke="#9ca3af" stroke-width="1"/>
  <path d="M 85,223 Q 95,215 105,195 Q 115,170 125,165 Q 135,168 145,180 Q 160,200 180,215 Q 200,222 215,223" fill="#c4b5fd" fill-opacity="0.3" stroke="#7c3aed" stroke-width="2"/>
  <text x="155" y="222" font-size="9" fill="#7c3aed">Still skewed</text>
  <!-- Row 2: n=30 -->
  <rect x="240" y="158" width="200" height="78" rx="6" fill="#f8fafc" stroke="#e2e8f0"/>
  <text x="265" y="178" text-anchor="middle" font-size="11" font-weight="600" fill="#2563eb">n = 30</text>
  <line x1="290" y1="225" x2="430" y2="225" stroke="#9ca3af" stroke-width="1"/>
  <path d="M 295,223 Q 305,218 315,200 Q 330,175 340,165 Q 350,162 360,165 Q 370,175 385,200 Q 400,218 420,223" fill="#93c5fd" fill-opacity="0.3" stroke="#2563eb" stroke-width="2"/>
  <text x="360" y="222" font-size="9" fill="#2563eb">Nearly normal</text>
  <!-- Row 2: n=100 -->
  <rect x="450" y="158" width="200" height="78" rx="6" fill="#f8fafc" stroke="#e2e8f0"/>
  <text x="478" y="178" text-anchor="middle" font-size="11" font-weight="600" fill="#16a34a">n = 100</text>
  <line x1="500" y1="225" x2="640" y2="225" stroke="#9ca3af" stroke-width="1"/>
  <path d="M 510,223 Q 520,220 530,210 Q 545,185 555,170 Q 562,163 570,160 Q 578,163 585,170 Q 595,185 610,210 Q 620,220 630,223" fill="#bbf7d0" fill-opacity="0.3" stroke="#16a34a" stroke-width="2"/>
  <text x="570" y="222" font-size="9" fill="#16a34a">Normal!</text>
  <!-- Summary row -->
  <rect x="60" y="252" width="560" height="40" rx="6" fill="#eff6ff" stroke="#2563eb" stroke-width="1"/>
  <text x="340" y="268" text-anchor="middle" font-size="11" font-weight="600" fill="#1e40af">As n increases: sampling distribution of the mean becomes normal AND narrower</text>
  <text x="340" y="284" text-anchor="middle" font-size="10" fill="#3b82f6">Width (SE) = SD / sqrt(n) -- quadruple n to halve the spread</text>
  <!-- Width comparison bars -->
  <text x="340" y="310" text-anchor="middle" font-size="12" font-weight="bold" fill="#1e293b">Spread of Sample Means</text>
  <!-- n=5 bar -->
  <text x="100" y="335" text-anchor="end" font-size="11" fill="#7c3aed" font-weight="600">n = 5</text>
  <rect x="110" y="325" width="280" height="14" rx="3" fill="#c4b5fd" fill-opacity="0.5" stroke="#7c3aed" stroke-width="1"/>
  <text x="400" y="337" font-size="10" fill="#7c3aed">SE = SD/2.2</text>
  <!-- n=30 bar -->
  <text x="100" y="360" text-anchor="end" font-size="11" fill="#2563eb" font-weight="600">n = 30</text>
  <rect x="110" y="350" width="115" height="14" rx="3" fill="#93c5fd" fill-opacity="0.5" stroke="#2563eb" stroke-width="1"/>
  <text x="235" y="362" font-size="10" fill="#2563eb">SE = SD/5.5</text>
  <!-- n=100 bar -->
  <text x="100" y="385" text-anchor="end" font-size="11" fill="#16a34a" font-weight="600">n = 100</text>
  <rect x="110" y="375" width="63" height="14" rx="3" fill="#bbf7d0" fill-opacity="0.5" stroke="#16a34a" stroke-width="1"/>
  <text x="183" y="387" font-size="10" fill="#16a34a">SE = SD/10</text>
  <!-- n=1000 bar -->
  <text x="100" y="410" text-anchor="end" font-size="11" fill="#1e293b" font-weight="600">n = 1000</text>
  <rect x="110" y="400" width="20" height="14" rx="3" fill="#e2e8f0" fill-opacity="0.5" stroke="#6b7280" stroke-width="1"/>
  <text x="140" y="412" font-size="10" fill="#6b7280">SE = SD/31.6</text>
</svg>
</div>

### Demonstrating the CLT

```bio
set_seed(42)
# Create a wildly non-normal population: exponential (very right-skewed)
let skewed_pop = rnorm(100000, 2, 1) |> map(|x| exp(x))

# The population is extremely skewed
histogram(skewed_pop, {bins: 50, title: "Population: Exponential (Very Skewed)"})
let pop_stats = summary(skewed_pop)
print("Population skewness: {pop_stats.skewness:.2}")

# Sample means with n=5 (still somewhat skewed)
let means_n5 = []
for i in 0..2000 {
    let s = sample(skewed_pop, 5)
    means_n5 = means_n5 + [mean(s)]
}
histogram(means_n5, {bins: 50, title: "Sample Means, n=5"})
print("n=5 skewness: {skewness(means_n5):.2}")

# Sample means with n=30 (approaching normal)
let means_n30 = []
for i in 0..2000 {
    let s = sample(skewed_pop, 30)
    means_n30 = means_n30 + [mean(s)]
}
histogram(means_n30, {bins: 50, title: "Sample Means, n=30"})
print("n=30 skewness: {skewness(means_n30):.2}")

# Sample means with n=100 (very close to normal)
let means_n100 = []
for i in 0..2000 {
    let s = sample(skewed_pop, 100)
    means_n100 = means_n100 + [mean(s)]
}
histogram(means_n100, {bins: 50, title: "Sample Means, n=100"})
print("n=100 skewness: {skewness(means_n100):.2}")

# Verify normality visually with Q-Q plot
qq_plot(means_n100, {title: "Q-Q Plot: Sample Means n=100"})
```

Watch the skewness drop toward zero as n increases. By n=100, the sampling distribution is indistinguishable from a normal curve, even though the underlying data is wildly skewed.

> **Key insight:** The CLT is why the normal distribution dominates statistics. Even when individual observations are non-normal, means of samples are approximately normal. Since most statistical tests are fundamentally about comparing means, the normal distribution is the right reference distribution for the test statistic — even when the raw data is not normal.

### When Does the CLT "Kick In"?

The speed of convergence to normality depends on how non-normal the population is:

| Population Shape | n Needed for CLT |
|---|---|
| Already normal | Any n (even n=1) |
| Slightly skewed | n &ge; 15 |
| Moderately skewed | n &ge; 30 |
| Heavily skewed | n &ge; 50-100 |
| Extremely skewed or heavy-tailed | n &ge; 100+ |

The "n &ge; 30" rule of thumb is a rough guideline, not a universal truth.

## Standard Error: The Precision of Your Estimate

The standard deviation of the sampling distribution has a special name: the **standard error** (SE).

**SE = SD / &radic;n**

This formula encodes the fundamental relationship between sample size and precision:
- Double your sample size -> SE decreases by a factor of sqrt(2) ~ 1.41
- Quadruple your sample size -> SE halves
- To cut SE in half, you need 4 times as many observations

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="280" viewBox="0 0 680 280" xmlns="http://www.w3.org/2000/svg" style="background: #fafbfc; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="26" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">Standard Error Shrinks with Sample Size</text>
  <text x="340" y="44" text-anchor="middle" font-size="11" fill="#6b7280">Error bars show 95% CI for the mean (mean +/- 1.96*SE), SD = 18 mmHg</text>
  <!-- Axes -->
  <line x1="80" y1="220" x2="640" y2="220" stroke="#9ca3af" stroke-width="1.5"/>
  <line x1="80" y1="220" x2="80" y2="60" stroke="#9ca3af" stroke-width="1.5"/>
  <text x="360" y="255" text-anchor="middle" font-size="12" fill="#6b7280">Sample Size (n)</text>
  <text x="30" y="145" text-anchor="middle" font-size="12" fill="#6b7280" transform="rotate(-90,30,145)">Estimate of Mean</text>
  <!-- True mean reference line -->
  <line x1="80" y1="140" x2="640" y2="140" stroke="#16a34a" stroke-width="1.5" stroke-dasharray="6,4"/>
  <text x="645" y="138" font-size="10" fill="#16a34a" font-weight="600">True mean</text>
  <!-- n=10: large error bars -->
  <line x1="140" y1="70" x2="140" y2="210" stroke="#dc2626" stroke-width="2.5"/>
  <line x1="128" y1="70" x2="152" y2="70" stroke="#dc2626" stroke-width="2"/>
  <line x1="128" y1="210" x2="152" y2="210" stroke="#dc2626" stroke-width="2"/>
  <circle cx="140" cy="140" r="5" fill="#dc2626"/>
  <text x="140" y="230" text-anchor="middle" font-size="11" fill="#1e293b" font-weight="600">n=10</text>
  <text x="140" y="244" text-anchor="middle" font-size="9" fill="#dc2626">SE=5.7</text>
  <!-- n=25 -->
  <line x1="240" y1="89" x2="240" y2="191" stroke="#ef4444" stroke-width="2.5"/>
  <line x1="228" y1="89" x2="252" y2="89" stroke="#ef4444" stroke-width="2"/>
  <line x1="228" y1="191" x2="252" y2="191" stroke="#ef4444" stroke-width="2"/>
  <circle cx="240" cy="140" r="5" fill="#ef4444"/>
  <text x="240" y="230" text-anchor="middle" font-size="11" fill="#1e293b" font-weight="600">n=25</text>
  <text x="240" y="244" text-anchor="middle" font-size="9" fill="#ef4444">SE=3.6</text>
  <!-- n=50 -->
  <line x1="340" y1="104" x2="340" y2="176" stroke="#f59e0b" stroke-width="2.5"/>
  <line x1="328" y1="104" x2="352" y2="104" stroke="#f59e0b" stroke-width="2"/>
  <line x1="328" y1="176" x2="352" y2="176" stroke="#f59e0b" stroke-width="2"/>
  <circle cx="340" cy="140" r="5" fill="#f59e0b"/>
  <text x="340" y="230" text-anchor="middle" font-size="11" fill="#1e293b" font-weight="600">n=50</text>
  <text x="340" y="244" text-anchor="middle" font-size="9" fill="#f59e0b">SE=2.5</text>
  <!-- n=100 -->
  <line x1="440" y1="115" x2="440" y2="165" stroke="#2563eb" stroke-width="2.5"/>
  <line x1="428" y1="115" x2="452" y2="115" stroke="#2563eb" stroke-width="2"/>
  <line x1="428" y1="165" x2="452" y2="165" stroke="#2563eb" stroke-width="2"/>
  <circle cx="440" cy="140" r="5" fill="#2563eb"/>
  <text x="440" y="230" text-anchor="middle" font-size="11" fill="#1e293b" font-weight="600">n=100</text>
  <text x="440" y="244" text-anchor="middle" font-size="9" fill="#2563eb">SE=1.8</text>
  <!-- n=500 -->
  <line x1="540" y1="128" x2="540" y2="152" stroke="#16a34a" stroke-width="2.5"/>
  <line x1="528" y1="128" x2="552" y2="128" stroke="#16a34a" stroke-width="2"/>
  <line x1="528" y1="152" x2="552" y2="152" stroke="#16a34a" stroke-width="2"/>
  <circle cx="540" cy="140" r="5" fill="#16a34a"/>
  <text x="540" y="230" text-anchor="middle" font-size="11" fill="#1e293b" font-weight="600">n=500</text>
  <text x="540" y="244" text-anchor="middle" font-size="9" fill="#16a34a">SE=0.8</text>
  <!-- n=1000 -->
  <line x1="620" y1="131" x2="620" y2="149" stroke="#14532d" stroke-width="2.5"/>
  <line x1="608" y1="131" x2="632" y2="131" stroke="#14532d" stroke-width="2"/>
  <line x1="608" y1="149" x2="632" y2="149" stroke="#14532d" stroke-width="2"/>
  <circle cx="620" cy="140" r="5" fill="#14532d"/>
  <text x="620" y="230" text-anchor="middle" font-size="11" fill="#1e293b" font-weight="600">n=1000</text>
  <text x="620" y="244" text-anchor="middle" font-size="9" fill="#14532d">SE=0.6</text>
  <!-- Annotation -->
  <rect x="380" y="60" width="260" height="30" rx="4" fill="#fffbeb" stroke="#f59e0b" stroke-width="1"/>
  <text x="510" y="80" text-anchor="middle" font-size="10" fill="#92400e" font-weight="600">Diminishing returns: 10x more data only ~3x precision</text>
  <!-- Curve connecting CI widths -->
  <path d="M 140,70 Q 180,88 240,89 Q 290,100 340,104 Q 390,112 440,115 Q 490,125 540,128 Q 580,130 620,131" fill="none" stroke="#9ca3af" stroke-width="1" stroke-dasharray="3,3"/>
  <path d="M 140,210 Q 180,195 240,191 Q 290,180 340,176 Q 390,168 440,165 Q 490,155 540,152 Q 580,150 620,149" fill="none" stroke="#9ca3af" stroke-width="1" stroke-dasharray="3,3"/>
</svg>
</div>

```bio
set_seed(42)
# Demonstrate how SE shrinks with sample size
let population_sd = 18.0  # Blood pressure SD

let sample_sizes = [5, 10, 20, 30, 50, 100, 200, 500, 1000]

print("Sample Size | Theoretical SE | Observed SE")
print("------------|----------------|------------")

for n in sample_sizes {
    let theoretical_se = population_sd / sqrt(n)

    # Simulate to verify
    let means = []
    for i in 0..1000 {
        let s = rnorm(n, 125, population_sd)
        means = means + [mean(s)]
    }
    let observed_se = stdev(means)

    print("  {n:>6}    |     {theoretical_se:>6.2}     |    {observed_se:.2}")
}
```

| Sample Size | SE (mmHg) | 95% CI Width |
|---|---|---|
| 20 | 4.02 | &plusmn; 7.9 |
| 50 | 2.55 | &plusmn; 5.0 |
| 100 | 1.80 | &plusmn; 3.5 |
| 200 | 1.27 | &plusmn; 2.5 |
| 1000 | 0.57 | &plusmn; 1.1 |

With 20 patients, your estimate of mean blood pressure could easily be off by 8 mmHg — enough to misclassify a treatment as effective or ineffective. With 200 patients, you are unlikely to be off by more than 2.5 mmHg.

> **Common pitfall:** Researchers often confuse SD and SE. The SD describes variability among individual observations. The SE describes precision of the sample mean. They answer different questions. Report the right one. SD for describing data; SE for describing the precision of an estimate.

## Types of Bias

Sample size controls precision, but even infinite precision cannot fix a biased sample. Bias is a systematic error that pushes your estimate in a consistent direction.

### Selection Bias

Your sample is not representative of the population you want to study.

**Example:** A study of gene expression in breast cancer recruits patients only from a single academic medical center. These patients tend to have more advanced disease (referral bias), are more likely to be white (geographic bias), and have better follow-up (compliance bias). The results may not generalize to community hospitals or diverse populations.

**Genomics example:** If you study "healthy controls" by recruiting university employees, your sample overrepresents educated, relatively affluent people — not the general population.

### Survivorship Bias

You only observe subjects who "survived" some selection process, missing those who did not.

**Classic example:** During WWII, the military examined bullet holes in returning planes and planned to add armor where holes were most common. Statistician Abraham Wald pointed out the error: they were only seeing planes that survived. The areas with no holes were where planes had been hit and crashed. Armor should go where holes were absent.

**Biological example:** If you study long-term cancer survivors to find prognostic biomarkers, you miss the patients who died quickly. Your biomarkers will predict survival among survivors, not among all patients.

### Ascertainment Bias

The way you identify subjects systematically skews who gets included.

**Example:** A study finds that children with autism have more genetic variants than controls. But the autistic children were ascertained through clinical evaluation (which involves deep phenotyping and genetic testing), while controls were population-based. The ascertainment process itself led to more thorough variant discovery in cases.

### Measurement Bias

The way you measure introduces systematic error.

**Example:** A technician consistently reads gel bands as slightly brighter than they are. All expression measurements are systematically inflated. If this bias is constant across all samples, relative comparisons are still valid. If it varies between conditions (e.g., the technician knows which samples are treatment), it corrupts everything.

**Genomics example:** GC content bias in sequencing — regions with extreme GC content are systematically under-represented in coverage, biasing any analysis that depends on read depth.

### Publication Bias

Studies with significant results are more likely to be published than studies with null results. The published literature systematically overestimates effect sizes.

**Example:** 20 groups test whether gene X is associated with disease Y. One group (by chance) finds p < 0.05 and publishes. The other 19 find nothing and file the results away. The published literature now says gene X is associated with disease Y, but the full evidence says otherwise.

| Bias Type | What Goes Wrong | Genomics Example |
|---|---|---|
| Selection | Non-representative sample | Single-center cohort |
| Survivorship | Missing failures | Studying only long-term survivors |
| Ascertainment | Systematic identification skew | More testing in cases vs controls |
| Measurement | Systematic instrument/observer error | GC bias, batch effects |
| Publication | Only positive results published | "Significant" GWAS hits that don't replicate |

> **Key insight:** Bias cannot be fixed by increasing sample size. A biased study with 10,000 subjects gives you a very precise wrong answer. Always evaluate bias before interpreting results.

## Why n Matters: The Power Preview

Statistical **power** is the probability of detecting a real effect when it exists. It is 1 minus the Type II error rate (1 - &beta;). Convention targets 80% power, meaning a 20% chance of missing a real effect.

Power depends on four factors:
1. **Effect size** — How large is the true difference? Bigger effects are easier to detect.
2. **Sample size (n)** — More data = more power.
3. **Variability (&sigma;)** — Less noise = more power.
4. **Significance threshold (&alpha;)** — More stringent threshold = less power.

Think of detecting a treatment effect as hearing a whisper in a crowd. The whisper is the signal (effect size). The crowd noise is variability. Adding more listeners (larger n) helps. Making the crowd quieter (reducing variability) helps. Demanding absolute certainty before you will believe you heard something (lower &alpha;) makes it harder.

### Simulating Power

```bio
set_seed(42)
# Simulate a clinical trial to understand power

# True effect: treatment mean = 125 (control = 130, lower is better)
let control_mean = 130.0
let treatment_mean = 125.0  # 5 mmHg real difference
let sd = 18.0

# Function to run one trial and check if we detect the difference
# Returns 1 if p < 0.05, 0 otherwise
let run_trial = fn(n_per_arm) {
    let control = rnorm(n_per_arm, control_mean, sd)
    let treatment = rnorm(n_per_arm, treatment_mean, sd)
    let result = ttest(treatment, control)
    if result.p_value < 0.05 { 1 } else { 0 }
}

# Run 1000 simulated trials for different sample sizes
let sizes = [20, 50, 100, 200, 500]
print("n per arm | Estimated Power")
print("----------|----------------")

for n in sizes {
    let detections = 0
    for i in 0..1000 {
        detections = detections + run_trial(n)
    }
    let power = detections / 1000.0
    print("  {n:>5}   |     {power * 100:.1}%")
}
```

Typical results:

| n per arm | Power | Interpretation |
|---|---|---|
| 20 | ~23% | Miss the effect 77% of the time — nearly useless |
| 50 | ~47% | Coin flip — unacceptable for a clinical trial |
| 100 | ~71% | Getting close but still below the 80% standard |
| 200 | ~94% | Excellent — high confidence in detecting the effect |
| 500 | ~99.9% | Virtually certain to detect even subtle effects |

This is Dr. Vasquez's argument in numbers. With 20 patients per arm, the trial has a 77% chance of producing a false negative — concluding the drug does not work when it does. That is not an experiment; it is a waste of money.

## The Bootstrap: Estimation Without Formulas

The **bootstrap** is a resampling method that estimates the sampling distribution empirically. Instead of relying on mathematical formulas, you resample your data with replacement thousands of times and compute your statistic each time.

The bootstrap is invaluable when:
- The formula for the standard error is unknown or complicated
- The CLT may not apply (small n, skewed data)
- You want confidence intervals for any statistic (median, correlation, ratio)

```bio
set_seed(42)
# Bootstrap estimation of the standard error of the median

# Original sample: 50 gene expression values
let expression = rnorm(50, 3.0, 1.5) |> map(|x| exp(x))

let observed_median = median(expression)
print("Observed median: {observed_median:.2}")

# Bootstrap: resample with replacement 5000 times
let boot_medians = []
for i in 0..5000 {
    let resample = sample(expression, len(expression))
    boot_medians = boot_medians + [median(resample)]
}

# Bootstrap SE
let boot_se = stdev(boot_medians)
print("Bootstrap SE of median: {boot_se:.2}")

# Bootstrap 95% confidence interval (percentile method)
let ci_lower = quantile(boot_medians, 0.025)
let ci_upper = quantile(boot_medians, 0.975)
print("95% Bootstrap CI: [{ci_lower:.2}, {ci_upper:.2}]")

histogram(boot_medians, {bins: 50, title: "Bootstrap Distribution of the Median"})
```

> **Key insight:** The bootstrap treats your sample as a stand-in for the population. By resampling from your sample, you simulate what would happen if you could repeatedly sample from the population. It is remarkably effective even for small samples.

## Hands-On: CLT with Allele Frequencies

Let us experience the Central Limit Theorem using realistic genetic data.

```bio
set_seed(42)
# Simulate allele frequency estimation from 1000 Genomes-style data

# True allele frequency of a common variant
let true_af = 0.23

# Simulate genotyping different numbers of individuals
let sample_sizes = [10, 30, 100, 500]

for n in sample_sizes {
    # Simulate 2000 studies, each genotyping n individuals
    let estimated_afs = []
    for study in 0..2000 {
        # Each individual contributes 2 alleles (diploid)
        let n_alleles = 2 * n
        let alt_count = rbinom(1, n_alleles, true_af) |> sum()
        let af_estimate = alt_count / n_alleles
        estimated_afs = estimated_afs + [af_estimate]
    }

    let se = stdev(estimated_afs)
    let theoretical_se = sqrt(true_af * (1.0 - true_af) / (2.0 * n))

    print("n={n}: SE={se:.4} (theoretical: {theoretical_se:.4})")
    histogram(estimated_afs, {bins: 40, title: "Allele Frequency Estimates (n={n})"})
}

# With n=10: estimates range wildly (0.05 to 0.50)
# With n=500: estimates tightly clustered around 0.23
```

This simulation shows exactly why GWAS studies need thousands of individuals. With 10 people, you cannot reliably estimate an allele frequency to better than &plusmn;10 percentage points. With 500, you can nail it to within &plusmn;1-2 percentage points.

## Python and R Equivalents

**Python:**
```python
import numpy as np
from scipy import stats

# Sampling distribution simulation
population = np.random.normal(125, 18, 100000)
sample_means = [np.mean(np.random.choice(population, 30)) for _ in range(1000)]
print(f"SE: {np.std(sample_means):.2f}")  # Should be close to 18/sqrt(30)

# Bootstrap
from scipy.stats import bootstrap
data = np.random.exponential(1, 50)
res = bootstrap((data,), np.median, n_resamples=5000)
print(f"95% CI: [{res.confidence_interval.low:.2f}, {res.confidence_interval.high:.2f}]")

# Standard error
se = np.std(data, ddof=1) / np.sqrt(len(data))
```

**R:**
```r
# Sampling distribution
population <- rnorm(100000, mean = 125, sd = 18)
sample_means <- replicate(1000, mean(sample(population, 30)))
sd(sample_means)  # Empirical SE

# Bootstrap
library(boot)
boot_fn <- function(data, indices) median(data[indices])
results <- boot(data, boot_fn, R = 5000)
boot.ci(results, type = "perc")

# Standard error
se <- sd(data) / sqrt(length(data))

# Central Limit Theorem demo
par(mfrow = c(2, 2))
for (n in c(5, 10, 30, 100)) {
  means <- replicate(2000, mean(rexp(n, rate = 1)))
  hist(means, breaks = 40, main = paste("n =", n))
}
```

## Exercises

### Exercise 1: Experience the CLT

Take a uniform distribution (flat, definitely not normal) and show the CLT in action.

```bio
set_seed(42)
# Uniform population: values equally likely between 0 and 100
let uniform_pop = rnorm(100000, 50, 28.87)
# (Approximation — true uniform has SD = range/sqrt(12))

# TODO: Draw histograms of the population (should be flat-ish)
# TODO: Take 1000 samples of size n=5, compute means, draw histogram
# TODO: Repeat for n=10, n=30, n=100
# TODO: At what n does the sampling distribution look convincingly normal?
# TODO: Compute skewness at each n to quantify the convergence
```

### Exercise 2: SE and Confidence

You measure tumor volumes in 25 mice (mean = 450 mm&sup3;, SD = 120 mm&sup3;).

```bio
let n = 25
let sample_mean = 450.0
let sample_sd = 120.0

# TODO: Compute the standard error
# TODO: Compute an approximate 95% CI using mean +/- 2*SE
# TODO: How large would n need to be for the 95% CI to have a width of +/-10 mm3?
# TODO: How large for +/-5 mm3?
```

### Exercise 3: Bias Identification

For each scenario, identify the type of bias and explain how it could affect results.

1. A study of BRCA1 mutation frequency recruits subjects from a cancer genetics clinic.
2. A survival analysis of pancreatic cancer uses patients diagnosed 5+ years ago (all long-term survivors by definition).
3. RNA-seq libraries are prepared on two different days — all treatment samples on Day 1, all controls on Day 2.
4. A GWAS consortium publishes results for the 10 strongest associations and files the rest.

### Exercise 4: Bootstrap a Correlation

Estimate the uncertainty in a correlation coefficient using the bootstrap.

```bio
set_seed(42)
# Gene expression vs. protein abundance (moderate correlation)
let n = 40
let gene_expr = rnorm(n, 5.0, 2.0)
let noise = rnorm(n, 0, 1.5)
let protein = gene_expr |> map(|x| 0.7 * x) |> zip(noise) |> map(|pair| pair.0 + pair.1)

let observed_r = cor(gene_expr, protein)
print("Observed correlation: {observed_r:.3}")

# TODO: Bootstrap the correlation 5000 times
# TODO: Compute the 95% bootstrap CI
# TODO: Is the correlation significantly different from zero?
# TODO: Plot the bootstrap distribution of r
```

### Exercise 5: Power Simulation

Explore how effect size and variability interact with sample size.

```bio
set_seed(42)
# TODO: Run the trial simulation from the chapter, but now vary the effect size
# Test with differences of 2, 5, 10, and 20 mmHg (SD=18 throughout)
# At n=50 per arm, which effect sizes can you reliably detect?
# TODO: Now fix the difference at 5 mmHg and vary SD (10, 18, 30)
# How does variability affect the required sample size?
```

## Key Takeaways

- **Population vs. sample:** You study a sample to learn about a population. The quality of inference depends on sample size and sampling method.
- The **sampling distribution** is the distribution of a statistic computed from repeated samples. It is narrower than the data distribution and centered at the true value.
- The **Central Limit Theorem** guarantees that sample means are approximately normal regardless of the population distribution, given sufficient sample size. This is why normal-based tests work so broadly.
- **Standard error (SE = SD/&radic;n)** quantifies the precision of your estimate. Quadrupling n halves the SE.
- **Bias** (selection, survivorship, ascertainment, measurement, publication) is a systematic distortion that cannot be fixed by increasing n. Identify and prevent bias at the design stage.
- **Statistical power** is the probability of detecting a real effect. Underpowered studies waste resources and miss real effects. The four determinants of power are effect size, sample size, variability, and significance threshold.
- The **bootstrap** provides empirical estimates of standard errors and confidence intervals for any statistic, without relying on distributional assumptions.

## What's Next

You have now completed the foundations. You know how to summarize data (Day 2), understand its distributional shape (Day 3), reason about probabilities (Day 4), and appreciate the central role of sample size and sampling variability (Day 5). Starting next week, we put these foundations to work. Day 6 introduces **confidence intervals** — the formal framework for saying "I'm 95% sure the true value lies between here and here." You will see how the standard error you learned today transforms into a rigorous statement about uncertainty, and why confidence intervals are more informative than p-values alone. The testing begins.
