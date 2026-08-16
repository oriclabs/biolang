# Day 2: Your Data at a Glance — Descriptive Statistics

<div class="day-meta">
<span class="badge">Day 2 of 30</span>
<span class="badge">Prerequisites: Day 1</span>
<span class="badge">~50 min</span>
<span class="badge">Hands-on</span>
</div>

## The Problem

Dr. Sarah Chen stares at her screen. The Illumina NovaSeq 6000 finished its run overnight, and now she has 10,247 quality scores — one for each tile on the flow cell. Her PI needs a decision by the morning meeting: is this data usable, or do they need to re-run the library, burning another $4,000 and two days?

She cannot read 10,247 numbers. She cannot scroll through them and develop an intuition. She has five minutes before the meeting starts. What she needs is a way to compress 10,247 numbers into a handful of meaningful summaries that answer three questions: What is the typical quality? How much does it vary? Are there any red flags?

This is the job of descriptive statistics. They are a useful early step before modelling, alongside checks of units, missingness, study design, and the experimental unit. A single summary cannot certify that a dataset is usable.

## What Are Descriptive Statistics?

Descriptive statistics are summaries. Think of them as a movie trailer for your data. The full movie (the raw dataset) might be two hours long, but the trailer gives you the genre, the tone, and the key plot points in two minutes. A good trailer does not lie about the movie. A good set of descriptive statistics does not lie about the data.

There are three things you need to know about any dataset:
1. **Center** — Where is the "middle" of the data?
2. **Spread** — How far do values range from that center?
3. **Shape** — Is the data symmetric? Skewed? Are there outliers?

### Why Do These Three Matter?

**Center tells you what is "normal."** If the mean quality score of your sequencing run is Q35, you know tiles are performing well. If it is Q15, something went wrong. Without center, you have no reference point — you cannot say whether a single observation is typical or alarming. In clinical genomics, center defines the baseline: what is the typical variant allele frequency in this cohort? What is the average coverage across your target regions?

**Spread tells you how much you can trust the center.** Two experiments might both report a mean IC50 of 12 nM, but one has values ranging from 11 to 13 (tight, reproducible) while the other ranges from 2 to 45 (noisy, unreliable). The center is the same — the spread tells you completely different stories. High spread means your next measurement could land anywhere; low spread means you can make confident predictions. In RNA-seq, high within-group variance buries real differential expression in noise.

**Shape gives modelling clues.** Many models make assumptions about errors, residuals, conditional outcomes, or sampling processes—not simply about the raw measurements. A right-skewed outcome can suggest a log-scale preview, a count model, or a robust summary, but the scientific estimand and design decide which is appropriate. Bimodality can be a clue to mixtures, batch effects, censoring, or subgroups; it is not by itself proof of two biological populations.

> **Key insight:** Report centre with a compatible spread, retain a view of the full distribution, and state the measurement scale. Standard deviation is defined for skewed data, but it can be a poor description of a *typical* distance when the mean and tails dominate; median with IQR or MAD is often a useful companion.

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="280" viewBox="0 0 680 280" xmlns="http://www.w3.org/2000/svg" style="background: #fafbfc; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="28" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">The Three Pillars of Descriptive Statistics</text>

  <!-- Center -->
  <rect x="30" y="50" width="190" height="200" rx="8" fill="#dbeafe" stroke="#2563eb" stroke-width="1.5"/>
  <text x="125" y="75" text-anchor="middle" font-size="14" font-weight="bold" fill="#1e3a8a">CENTER</text>
  <text x="125" y="95" text-anchor="middle" font-size="11" fill="#1e3a8a">"What is typical?"</text>
  <line x1="70" y1="140" x2="180" y2="140" stroke="#6b7280" stroke-width="1"/>
  <path d="M 70,138 Q 100,120 125,105 Q 150,120 180,138" fill="none" stroke="#2563eb" stroke-width="2"/>
  <line x1="125" y1="105" x2="125" y2="140" stroke="#dc2626" stroke-width="2" stroke-dasharray="4,3"/>
  <circle cx="125" cy="105" r="3" fill="#dc2626"/>
  <text x="125" y="160" text-anchor="middle" font-size="10" fill="#dc2626">mean / median</text>
  <text x="125" y="180" text-anchor="middle" font-size="10" fill="#475569">Without center, you can't</text>
  <text x="125" y="195" text-anchor="middle" font-size="10" fill="#475569">judge if a value is normal</text>
  <text x="125" y="210" text-anchor="middle" font-size="10" fill="#475569">or alarming</text>
  <text x="125" y="235" text-anchor="middle" font-size="10" font-style="italic" fill="#2563eb">Q35 = good run</text>
  <text x="125" y="248" text-anchor="middle" font-size="10" font-style="italic" fill="#dc2626">Q15 = re-sequence</text>

  <!-- Spread -->
  <rect x="245" y="50" width="190" height="200" rx="8" fill="#dcfce7" stroke="#16a34a" stroke-width="1.5"/>
  <text x="340" y="75" text-anchor="middle" font-size="14" font-weight="bold" fill="#14532d">SPREAD</text>
  <text x="340" y="95" text-anchor="middle" font-size="11" fill="#14532d">"How much can I trust it?"</text>
  <line x1="285" y1="140" x2="395" y2="140" stroke="#6b7280" stroke-width="1"/>
  <!-- Tight distribution -->
  <path d="M 295,138 Q 310,125 320,108 Q 330,98 340,95 Q 350,98 360,108 Q 370,125 385,138" fill="none" stroke="#16a34a" stroke-width="2"/>
  <text x="340" y="155" text-anchor="middle" font-size="9" fill="#16a34a">Tight = reliable</text>
  <!-- Wide distribution hint -->
  <path d="M 285,138 Q 300,130 315,125 Q 340,118 365,125 Q 380,130 395,138" fill="none" stroke="#dc2626" stroke-width="1.5" stroke-dasharray="3,3"/>
  <text x="340" y="170" text-anchor="middle" font-size="9" fill="#dc2626">Wide = noisy</text>
  <text x="340" y="195" text-anchor="middle" font-size="10" fill="#475569">Same mean, but different</text>
  <text x="340" y="210" text-anchor="middle" font-size="10" fill="#475569">confidence in the result</text>
  <text x="340" y="235" text-anchor="middle" font-size="10" font-style="italic" fill="#16a34a">IC50: 11-13 nM (trust it)</text>
  <text x="340" y="248" text-anchor="middle" font-size="10" font-style="italic" fill="#dc2626">IC50: 2-45 nM (don't)</text>

  <!-- Shape -->
  <rect x="460" y="50" width="190" height="200" rx="8" fill="#f3e8ff" stroke="#7c3aed" stroke-width="1.5"/>
  <text x="555" y="75" text-anchor="middle" font-size="14" font-weight="bold" fill="#4c1d95">SHAPE</text>
  <text x="555" y="95" text-anchor="middle" font-size="11" fill="#4c1d95">"Which tools are safe?"</text>
  <line x1="500" y1="140" x2="610" y2="140" stroke="#6b7280" stroke-width="1"/>
  <!-- Skewed distribution -->
  <path d="M 500,138 Q 515,130 530,110 Q 540,100 545,98 Q 555,100 570,115 Q 590,130 610,138" fill="none" stroke="#7c3aed" stroke-width="2"/>
  <text x="555" y="155" text-anchor="middle" font-size="9" fill="#7c3aed">Skewed → wrong test = wrong answer</text>
  <text x="555" y="195" text-anchor="middle" font-size="10" fill="#475569">Bell-shaped → t-test OK</text>
  <text x="555" y="210" text-anchor="middle" font-size="10" fill="#475569">Skewed → inspect scale and model</text>
  <text x="555" y="225" text-anchor="middle" font-size="10" fill="#475569">Bimodal → two populations?</text>
  <text x="555" y="248" text-anchor="middle" font-size="10" font-style="italic" fill="#7c3aed">#1 reason results fail to replicate</text>
</svg>
</div>

Every statistical analysis starts here. If you skip descriptive statistics and jump straight to hypothesis testing, you are performing surgery without an examination.

## Measures of Center

### Mean (Arithmetic Average)

The mean is the balance point. If your data values were weights placed along a ruler, the mean is the spot where the ruler would balance perfectly.

**Formula:** x&#x0304; = (1/n) &sum; x&#x1d62;

The mean uses every data point, which is both its strength and its weakness. It is the most efficient estimator of center when data is symmetric with no outliers. But it is exquisitely sensitive to extreme values.

**Example:** Five gene expression values (FPKM): 12, 15, 14, 13, 16. Mean = 14.0. Reasonable.

Now add one highly expressed gene: 12, 15, 14, 13, 16, 5000. Mean = 845.0. The mean has been dragged from 14 to 845 by a single outlier. It no longer represents "typical" expression.

> **Common pitfall:** Expression summaries answer different questions. The arithmetic mean describes equal-share abundance and is relevant to totals; the median describes a typical sample and resists a long tail. Inspect the distribution and measurement scale instead of declaring one universally better.

### Median (Middle Value)

The median is the value that splits the data in half: 50% of observations fall below it, 50% above. Sort the data and pick the middle number (or average the two middle numbers if n is even).

For our outlier-contaminated expression data: sorted = 12, 13, 14, 15, 16, 5000. Median = (14 + 15) / 2 = 14.5. The outlier barely matters.

The median is **robust** — it resists the pull of extreme values. This makes it the preferred measure of center for skewed distributions, which are the norm in biology.

### Mode (Most Frequent Value)

The mode is the most common value. It is most useful for categorical or discrete data: the most common blood type, the most frequent variant allele, the peak of a histogram.

For continuous data, the mode is the peak of the density curve. Bimodal distributions (two peaks) arise in biology more often than you might expect — for instance, CpG methylation levels often cluster near 0% and 100%, reflecting fully unmethylated and fully methylated states.

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="320" viewBox="0 0 680 320" xmlns="http://www.w3.org/2000/svg" style="background: #fafbfc; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="28" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">Mean, Median, and Mode in a Right-Skewed Distribution</text>
  <!-- Axes -->
  <line x1="60" y1="250" x2="640" y2="250" stroke="#9ca3af" stroke-width="1.5"/>
  <line x1="60" y1="250" x2="60" y2="50" stroke="#9ca3af" stroke-width="1.5"/>
  <text x="350" y="280" text-anchor="middle" font-size="12" fill="#6b7280">Value (e.g., gene expression, TPM)</text>
  <text x="30" y="155" text-anchor="middle" font-size="12" fill="#6b7280" transform="rotate(-90,30,155)">Frequency</text>
  <!-- Right-skewed bell curve (approximated with polygon) -->
  <path d="M 80,248 Q 120,245 150,220 Q 180,160 200,100 Q 215,65 230,58 Q 250,55 270,65 Q 300,90 340,140 Q 380,180 430,210 Q 490,235 560,245 Q 600,248 630,248" fill="#93c5fd" fill-opacity="0.3" stroke="#2563eb" stroke-width="2.5"/>
  <!-- Mode line (peak) -->
  <line x1="230" y1="55" x2="230" y2="250" stroke="#7c3aed" stroke-width="2" stroke-dasharray="6,4"/>
  <circle cx="230" cy="55" r="5" fill="#7c3aed"/>
  <text x="230" y="44" text-anchor="middle" font-size="12" font-weight="bold" fill="#7c3aed">Mode</text>
  <text x="230" y="268" text-anchor="middle" font-size="10" fill="#7c3aed">Peak of distribution</text>
  <!-- Median line -->
  <line x1="295" y1="98" x2="295" y2="250" stroke="#16a34a" stroke-width="2" stroke-dasharray="6,4"/>
  <circle cx="295" cy="98" r="5" fill="#16a34a"/>
  <text x="295" y="88" text-anchor="middle" font-size="12" font-weight="bold" fill="#16a34a">Median</text>
  <text x="295" y="268" text-anchor="middle" font-size="10" fill="#16a34a">50% of data on each side</text>
  <!-- Mean line (pulled right by tail) -->
  <line x1="370" y1="155" x2="370" y2="250" stroke="#dc2626" stroke-width="2" stroke-dasharray="6,4"/>
  <circle cx="370" cy="155" r="5" fill="#dc2626"/>
  <text x="370" y="145" text-anchor="middle" font-size="12" font-weight="bold" fill="#dc2626">Mean</text>
  <text x="370" y="268" text-anchor="middle" font-size="10" fill="#dc2626">Pulled right by outliers</text>
  <!-- Annotation arrow showing pull -->
  <path d="M 310,300 C 330,290 350,290 365,295" fill="none" stroke="#dc2626" stroke-width="1.5" marker-end="url(#arrowDay2)"/>
  <defs><marker id="arrowDay2" markerWidth="8" markerHeight="6" refX="8" refY="3" orient="auto"><path d="M0,0 L8,3 L0,6 Z" fill="#dc2626"/></marker></defs>
  <text x="340" y="313" text-anchor="middle" font-size="10" fill="#dc2626" font-style="italic">Outliers drag the mean toward the tail</text>
  <!-- Legend -->
  <rect x="490" y="60" width="150" height="80" rx="4" fill="white" stroke="#e5e7eb"/>
  <text x="500" y="78" font-size="11" font-weight="600" fill="#1e293b">Right-skewed data:</text>
  <line x1="500" y1="92" x2="520" y2="92" stroke="#7c3aed" stroke-width="2"/>
  <text x="525" y="96" font-size="10" fill="#7c3aed">Mode (smallest)</text>
  <line x1="500" y1="110" x2="520" y2="110" stroke="#16a34a" stroke-width="2"/>
  <text x="525" y="114" font-size="10" fill="#16a34a">Median (middle)</text>
  <line x1="500" y1="128" x2="520" y2="128" stroke="#dc2626" stroke-width="2"/>
  <text x="525" y="132" font-size="10" fill="#dc2626">Mean (largest)</text>
</svg>
</div>

### When to Use Each

Do not begin with “Which formula is most popular?” Begin with “What does average
mean in this scientific question?”

| Question being answered | Centre | Matching description of spread | Typical use |
|---|---|---|---|
| If the total were shared equally, what would each unit receive? | Arithmetic mean | SD for roughly symmetric additive variation | Technical measurements, additive outcomes |
| What value separates the lower half from the upper half? | Median | IQR or MAD | Skewed concentrations, durations, valid extremes |
| What single multiplication factor represents compound change? | Geometric mean | Geometric SD or multiplicative interval | Fold changes, growth factors, positive titres |
| What is the average after justified unequal representation? | Weighted mean | Weighted SD **and** design-aware uncertainty | Sampling weights, exposures, precision weights |
| What common rate applies over equal amounts of work? | Harmonic mean | Raw rate quantiles and uncertainty for the target rate | Equal-distance speeds, throughput per fixed work |
| What centre remains after a predeclared symmetric tail trim? | Trimmed mean | Winsorized SD or bootstrap interval | Robust sensitivity analysis |
| Which category or peak occurs most often? | Mode | Counts or proportions, possibly more than one mode | Blood group, genotype class, distribution peaks |

<div style="text-align: center; margin: 2em 0;">
<svg width="700" height="350" viewBox="0 0 700 350" xmlns="http://www.w3.org/2000/svg" role="img" aria-label="Three matching centre and spread pairs" style="max-width: 100%; background: #fafbfc; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="350" y="28" text-anchor="middle" font-size="16" font-weight="bold" fill="#1e293b">Choose centre and spread as a pair</text>
  <rect x="20" y="45" width="660" height="82" rx="7" fill="#eff6ff" stroke="#93c5fd"/>
  <text x="38" y="67" font-size="13" font-weight="bold" fill="#1e40af">Additive, roughly symmetric</text>
  <line x1="55" y1="96" x2="350" y2="96" stroke="#64748b" stroke-width="1.5"/>
  <circle cx="115" cy="96" r="5" fill="#2563eb"/><circle cx="155" cy="96" r="5" fill="#2563eb"/><circle cx="195" cy="96" r="5" fill="#2563eb"/><circle cx="235" cy="96" r="5" fill="#2563eb"/><circle cx="275" cy="96" r="5" fill="#2563eb"/>
  <line x1="195" y1="75" x2="195" y2="113" stroke="#dc2626" stroke-width="2"/><line x1="155" y1="116" x2="235" y2="116" stroke="#16a34a" stroke-width="3"/>
  <text x="420" y="82" font-size="13" fill="#334155">Arithmetic mean = balance point</text><text x="420" y="105" font-size="13" fill="#334155">SD = additive distance from it</text>
  <rect x="20" y="137" width="660" height="82" rx="7" fill="#f0fdf4" stroke="#86efac"/>
  <text x="38" y="159" font-size="13" font-weight="bold" fill="#166534">Skewed or heavy-tailed</text>
  <line x1="55" y1="188" x2="350" y2="188" stroke="#64748b" stroke-width="1.5"/>
  <circle cx="90" cy="188" r="5" fill="#16a34a"/><circle cx="112" cy="188" r="5" fill="#16a34a"/><circle cx="130" cy="188" r="5" fill="#16a34a"/><circle cx="150" cy="188" r="5" fill="#16a34a"/><circle cx="172" cy="188" r="5" fill="#16a34a"/><circle cx="310" cy="188" r="5" fill="#dc2626"/>
  <line x1="140" y1="167" x2="140" y2="205" stroke="#7c3aed" stroke-width="2"/><rect x="112" y="207" width="60" height="6" fill="#7c3aed" rx="3"/>
  <text x="420" y="174" font-size="13" fill="#334155">Median = halfway observation</text><text x="420" y="197" font-size="13" fill="#334155">IQR/MAD = robust spread</text>
  <rect x="20" y="229" width="660" height="100" rx="7" fill="#faf5ff" stroke="#d8b4fe"/>
  <text x="38" y="251" font-size="13" font-weight="bold" fill="#6b21a8">Positive, multiplicative</text>
  <line x1="70" y1="283" x2="350" y2="283" stroke="#64748b" stroke-width="1.5"/>
  <circle cx="90" cy="283" r="5" fill="#7c3aed"/><circle cx="165" cy="283" r="5" fill="#7c3aed"/><circle cx="240" cy="283" r="5" fill="#7c3aed"/><circle cx="315" cy="283" r="5" fill="#7c3aed"/>
  <text x="90" y="303" text-anchor="middle" font-size="11" fill="#475569">1</text><text x="165" y="303" text-anchor="middle" font-size="11" fill="#475569">2</text><text x="240" y="303" text-anchor="middle" font-size="11" fill="#475569">4</text><text x="315" y="303" text-anchor="middle" font-size="11" fill="#475569">8</text>
  <line x1="202" y1="263" x2="202" y2="300" stroke="#dc2626" stroke-width="2"/>
  <text x="420" y="268" font-size="13" fill="#334155">Geometric mean = ratio balance</text><text x="420" y="291" font-size="13" fill="#334155">Geometric SD = multiply/divide factor</text><text x="420" y="314" font-size="11" fill="#6b7280">1, 2, 4, 8 are equally spaced on a log scale.</text>
</svg>
</div>

The mean and median can both be worth reporting when they answer useful
questions. A difference between them is a clue about asymmetry or mixtures—not
proof that either summary is dishonest.

### “Mean” Has Several Meanings

#### Arithmetic mean: additive balance

For values 2, 4, and 9, the arithmetic mean is `(2 + 4 + 9) / 3 = 5`.
It is the correct balance point when differences add. Its natural companion is
the SD, because both use distance from the arithmetic mean.

#### Geometric mean: multiplicative balance

For positive values, take the mean on the log scale and transform back. A
half-fold change followed by a two-fold change has geometric mean
`sqrt(0.5 × 2) = 1`: no average multiplicative change. The arithmetic mean is
1.25 and answers a different, additive question.

A geometric SD is a **factor**, not an additive distance. Geometric mean 20 with
geometric SD 1.5 gives a one-geometric-SD range of `20 / 1.5` to `20 × 1.5`,
not `20 ± 1.5`.

#### Harmonic mean: a rate over fixed work

Suppose the same distance is travelled at 60 km/h and 30 km/h. The arithmetic
mean is 45, but the overall equal-distance speed is the harmonic mean, 40 km/h,
because more time is spent at the slower rate. Do not use it merely because a
variable is called a rate: the exposure or numerator must justify it.

#### Weighted mean: observations do not contribute equally

Weights may represent frequency, sampling probability, exposure, or precision;
these meanings are not interchangeable. A weighted SD describes spread under
the weights, but a valid standard error may also need survey strata, clusters,
replicate weights, or a model of measurement precision.

#### Trimmed mean: a declared compromise

A 10% trimmed mean removes the lowest and highest 10% symmetrically before
averaging. It can be a useful sensitivity summary, but choosing the trim after
seeing an inconvenient result can hide biology. Show the untrimmed distribution
and use a winsorized spread or bootstrap interval.

#### Root mean square: magnitude, not “typical” centre

RMS squares values, averages, and takes the square root. It is useful for signal
magnitude and error size, but it is not normally a replacement for the mean or
median of observed measurements.

BioLang calculates these side by side and explains their compatible spreads:

```bio
import "statistics" as stat

let choices = stat.means([1, 2, 4, 8], {trim_fraction: 0.25})
println("Arithmetic mean: " + str(choices.arithmetic_mean))
println("Geometric mean: " + str(choices.geometric_mean))
println("Harmonic mean: " + str(choices.harmonic_mean))
choices.centre_spread_pairs
```

`stat.means()` never selects an answer automatically. If weights are part of
the scientific design, use `stat.weighted_summary(values, weights)` as well.

## Measures of Spread

Knowing the center is not enough. Two datasets can have identical means and wildly different behaviors. Consider drug response in two patient cohorts — both might have a mean survival of 12 months, but in one cohort everyone lives 11-13 months, while in the other, half die in 2 months and half live 22 months. The clinical implications are completely different.

### Range

The simplest measure: maximum minus minimum. It tells you the total extent of the data but is completely determined by the two most extreme points.

### Variance and Standard Deviation

Variance measures the average squared distance from the mean:

**Variance:** s&sup2; = (1 / (n-1)) &sum; (x&#x1d62; - x&#x0304;)&sup2;

**Standard Deviation:** s = &radic;(s&sup2;)

We divide by (n-1) rather than n (Bessel's correction) because a sample underestimates the true population variance. The standard deviation is in the same units as the data, making it more interpretable than variance.

Variance is **not** the whole width of a distribution, and SD does not mean “the
left side plus the right side.” They encode the same spread in different units:

- SD is a typical distance from the arithmetic mean, expressed in the original unit.
- Variance is SD squared, expressed in squared units and useful inside statistical formulas.
- Adding 10 to every value moves the mean by 10 but leaves SD and variance unchanged.
- Multiplying every value by 3 multiplies SD by 3 and variance by 9.

`mean ± SD` is most interpretable for a roughly symmetric, single-peaked
distribution. For skewed data, pair the median with IQR or MAD. For positive
multiplicative data, pair the geometric mean with a geometric SD or fold range.
The 68–95–99.7 rule below is a property of an approximately normal model, not a
universal definition of SD and not an automatic outlier-removal rule.

**Rule of thumb:** For normally distributed data, about 68% of values fall within 1 SD of the mean, 95% within 2 SDs, and 99.7% within 3 SDs. If a quality score is more than 3 SDs below the mean, something is wrong with that tile.

### Interquartile Range (IQR)

The IQR is the range of the middle 50% of the data: Q3 - Q1, where Q1 is the 25th percentile and Q3 is the 75th percentile.

Like the median, the IQR is robust to outliers. It is the foundation of the box plot and a standard measure of spread for skewed data.

### Coefficient of Variation (CV)

CV = (SD / Mean) &times; 100%. The CV expresses variability relative to the mean, making it useful for comparing spread across measurements with different scales.

**Example:** If gene A has mean expression 1000 TPM with SD 100, and gene B has mean 10 TPM with SD 5, which is more variable? Gene A has higher SD, but gene B has a higher CV (50% vs 10%). Gene B's expression is relatively more variable.

| Measure | Formula | Robust to Outliers? | Use Case |
|---|---|---|---|
| Range | max - min | No | Quick overview |
| Variance | s&sup2; | No | Mathematical convenience |
| Standard Deviation | s | No | Symmetric data |
| IQR | Q3 - Q1 | Yes | Skewed data, box plots |
| CV | (SD / Mean) &times; 100% | No | Comparing variability across scales |

## Shape: Skewness and Kurtosis

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="250" viewBox="0 0 680 250" xmlns="http://www.w3.org/2000/svg" style="background: #fafbfc; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="26" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">Three Distribution Shapes</text>
  <!-- Left-skewed (Negative Skew) -->
  <rect x="15" y="38" width="210" height="195" rx="6" fill="#f8fafc" stroke="#e2e8f0"/>
  <text x="120" y="58" text-anchor="middle" font-size="12" font-weight="600" fill="#7c3aed">Left-Skewed (Negative)</text>
  <line x1="35" y1="200" x2="210" y2="200" stroke="#9ca3af" stroke-width="1"/>
  <path d="M 45,198 Q 65,195 85,185 Q 105,150 120,100 Q 130,75 145,65 Q 165,60 175,68 Q 185,80 190,110 Q 195,150 200,198" fill="#c4b5fd" fill-opacity="0.4" stroke="#7c3aed" stroke-width="2"/>
  <text x="120" y="218" text-anchor="middle" font-size="10" fill="#7c3aed">Long tail on the left</text>
  <text x="120" y="230" text-anchor="middle" font-size="10" fill="#6b7280">Mean &lt; Median &lt; Mode</text>
  <!-- Symmetric (Normal) -->
  <rect x="235" y="38" width="210" height="195" rx="6" fill="#f8fafc" stroke="#e2e8f0"/>
  <text x="340" y="58" text-anchor="middle" font-size="12" font-weight="600" fill="#2563eb">Symmetric (Normal)</text>
  <line x1="255" y1="200" x2="430" y2="200" stroke="#9ca3af" stroke-width="1"/>
  <path d="M 265,198 Q 280,195 295,180 Q 310,145 320,100 Q 330,72 340,62 Q 350,72 360,100 Q 370,145 385,180 Q 400,195 415,198" fill="#93c5fd" fill-opacity="0.4" stroke="#2563eb" stroke-width="2"/>
  <text x="340" y="218" text-anchor="middle" font-size="10" fill="#2563eb">Mirror-image symmetry</text>
  <text x="340" y="230" text-anchor="middle" font-size="10" fill="#6b7280">Mean = Median = Mode</text>
  <!-- Right-skewed (Positive Skew) -->
  <rect x="455" y="38" width="210" height="195" rx="6" fill="#f8fafc" stroke="#e2e8f0"/>
  <text x="560" y="58" text-anchor="middle" font-size="12" font-weight="600" fill="#dc2626">Right-Skewed (Positive)</text>
  <line x1="475" y1="200" x2="650" y2="200" stroke="#9ca3af" stroke-width="1"/>
  <path d="M 485,198 Q 490,150 495,110 Q 500,80 505,68 Q 515,60 535,65 Q 550,75 560,100 Q 575,150 595,185 Q 615,195 635,198" fill="#fecaca" fill-opacity="0.4" stroke="#dc2626" stroke-width="2"/>
  <text x="560" y="218" text-anchor="middle" font-size="10" fill="#dc2626">Long tail on the right</text>
  <text x="560" y="230" text-anchor="middle" font-size="10" fill="#6b7280">Mode &lt; Median &lt; Mean</text>
</svg>
</div>

### Skewness

Skewness measures asymmetry. A skewness of 0 means perfectly symmetric (like the normal distribution). Positive skewness means a right tail (most values cluster low, with a few very high values). Negative skewness means a left tail.

**Biological reality:** Many—not all—biological measurements are positively skewed. Expression abundance, protein abundance, cell counts, and read depths can have many low values and a few high ones because of biological heterogeneity, multiplicative mechanisms, sampling, and technical limits. Mixtures and batch effects can create similar shapes, so skewness alone does not reveal the cause.

### Kurtosis

Kurtosis measures the "tailedness" of the distribution — how likely extreme values are compared to a normal distribution. High kurtosis means heavy tails (more outliers than expected). Low kurtosis means light tails.

In genomics, variant allele frequency distributions often have high kurtosis, reflecting the mixture of common variants (clustered near 50%) and rare variants (near 0%).

> **Key insight:** Skewness and kurtosis are clues, not pass/fail gates. Model assumptions usually concern residuals or a sampling distribution, not necessarily the raw outcome. Plot the data, preserve the study design, and inspect the fitted model rather than choosing a test from one shape number.

## Descriptive Statistics in BioLang

Let us return to Dr. Chen's problem. She has 10,247 quality scores and five minutes.

### Loading and Summarizing Data

```bio
import "statistics" as stat

set_seed(42)
# Simulate sequencing quality scores (Phred scale, typically 0-40)
let quality_scores = rnorm(10247, 32.5, 3.8)

# Values are calculated when this block runs; none are pasted in advance.
let report = stat.explore(quality_scores, {
    name: "tile quality",
    data_type: "Phred scores"
})
println(report.explanation)
println(report.visual_guide.ascii)
```

That call calculates the observed centre, spread, shape, missing-value counts, and review flags. It does **not** decide whether the run is usable: Dr. Chen must compare the observed distribution with the instrument's QC specification and inspect per-cycle, per-tile, and library-level evidence.

### Computing Individual Statistics

```bio
# Individual measures of centre
let avg = mean(quality_scores)
let med = median(quality_scores)

# Individual measures of spread
let sd = stdev(quality_scores)
let v = variance(quality_scores)
let q1 = report.summary.q1
let q3 = report.summary.q3
let i = report.summary.iqr

print(f"Mean: {avg}, Median: {med}")
print(f"SD: {sd}, IQR: {i}")
print(f"Q1: {q1}, Q3: {q3}, Skewness: {report.summary.skewness}")
```

An exact numeric mode is usually unhelpful for continuous measurements because
nearby values need not repeat. If a peak matters scientifically, show the
distribution and state the binning or density method used to identify it.

### The `summary()` Function

For a more detailed report:

```bio
let choices = stat.means(quality_scores)
println(choices.quick_explanation)
println(choices.centre_spread_pairs)
```

### Working with Real Sequencing Data

```bio
set_seed(42)
# In practice, you'd load from a file:
# let scores = read_csv("data/expression.csv") |> column("phred_score")

# Simulate a problematic run with bimodal quality
let good_tiles = rnorm(8000, 33.0, 2.5)
let bad_tiles = rnorm(2247, 18.0, 4.0)
let mixed_scores = good_tiles + bad_tiles

let mixed_stats = summary(mixed_scores)
print(mixed_stats)
# Mean: 29.7 — looks okay at first glance
# Median: 32.1 — the median reveals the truth: most tiles are fine
# Skewness: -1.4 — strongly left-skewed, warning sign!

# The mean alone would have hidden the problem.
# Always look at the full distribution.
```

> **Common pitfall:** Relying on the mean alone can hide bimodal distributions. The "bad tile" problem above is common in sequencing — a mean of 29.7 looks passable, but 22% of tiles are failing. Always visualize.

## Visualization: Always Plot Before You Test

Numbers tell part of the story. Plots tell the rest. Anscombe's Quartet — four datasets with identical means, variances, and correlations but wildly different structures — demonstrates why you must always look at your data.

### Histograms

A histogram bins your data and counts frequencies. It reveals the distribution's shape at a glance.

```bio
# Basic histogram (quality_scores is defined earlier in this chapter)
histogram(quality_scores, {bins: 50, title: "Sequencing Quality Distribution"})
```

```bio
# Histogram of the problematic run (uses mixed_scores from earlier block)
histogram(mixed_scores, {bins: 50, title: "Bimodal Quality — Problem Run"})
# This immediately reveals the two peaks that the mean masked.
```

The good run produces a single symmetric peak around 32-33. The problem run shows two distinct peaks — one centered at 18 and one at 33. No summary statistic captures this as effectively as the histogram.

### Box Plots

A box plot displays the median (center line), IQR (box), and whiskers (typically 1.5 &times; IQR). Points beyond the whiskers are marked as individual outliers.

<div style="text-align: center; margin: 2em 0;">
<svg width="660" height="380" viewBox="0 0 660 380" xmlns="http://www.w3.org/2000/svg" style="background: #fafbfc; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="330" y="28" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">Anatomy of a Box Plot</text>
  <!-- The box plot itself (vertical orientation, centered) -->
  <!-- Outer whisker lines -->
  <line x1="260" y1="320" x2="260" y2="280" stroke="#2563eb" stroke-width="2"/>
  <line x1="240" y1="320" x2="280" y2="320" stroke="#2563eb" stroke-width="2.5"/>
  <line x1="260" y1="130" x2="260" y2="100" stroke="#2563eb" stroke-width="2"/>
  <line x1="240" y1="100" x2="280" y2="100" stroke="#2563eb" stroke-width="2.5"/>
  <!-- Box -->
  <rect x="220" y="130" width="80" height="150" fill="#93c5fd" fill-opacity="0.3" stroke="#2563eb" stroke-width="2.5" rx="2"/>
  <!-- Median line -->
  <line x1="220" y1="195" x2="300" y2="195" stroke="#dc2626" stroke-width="3"/>
  <!-- Outliers -->
  <circle cx="260" cy="62" r="6" fill="none" stroke="#ef4444" stroke-width="2"/>
  <circle cx="260" cy="348" r="6" fill="none" stroke="#ef4444" stroke-width="2"/>
  <!-- Annotation lines and labels -->
  <!-- Q3 label -->
  <line x1="302" y1="130" x2="360" y2="130" stroke="#9ca3af" stroke-width="1" stroke-dasharray="3,3"/>
  <text x="365" y="134" font-size="12" fill="#2563eb" font-weight="600">Q3 (75th percentile)</text>
  <!-- Median label -->
  <line x1="302" y1="195" x2="360" y2="195" stroke="#9ca3af" stroke-width="1" stroke-dasharray="3,3"/>
  <text x="365" y="199" font-size="12" fill="#dc2626" font-weight="600">Median (Q2, 50th)</text>
  <!-- Q1 label -->
  <line x1="302" y1="280" x2="360" y2="280" stroke="#9ca3af" stroke-width="1" stroke-dasharray="3,3"/>
  <text x="365" y="284" font-size="12" fill="#2563eb" font-weight="600">Q1 (25th percentile)</text>
  <!-- IQR brace -->
  <path d="M 195,130 L 185,130 L 185,280 L 195,280" fill="none" stroke="#16a34a" stroke-width="2"/>
  <text x="175" y="210" text-anchor="end" font-size="12" fill="#16a34a" font-weight="600">IQR</text>
  <text x="175" y="224" text-anchor="end" font-size="10" fill="#16a34a">Q3 - Q1</text>
  <!-- Upper whisker label -->
  <line x1="282" y1="100" x2="360" y2="80" stroke="#9ca3af" stroke-width="1" stroke-dasharray="3,3"/>
  <text x="365" y="76" font-size="11" fill="#6b7280">Upper whisker</text>
  <text x="365" y="90" font-size="10" fill="#6b7280">max(data) or Q3 + 1.5*IQR</text>
  <!-- Lower whisker label -->
  <line x1="282" y1="320" x2="360" y2="335" stroke="#9ca3af" stroke-width="1" stroke-dasharray="3,3"/>
  <text x="365" y="332" font-size="11" fill="#6b7280">Lower whisker</text>
  <text x="365" y="346" font-size="10" fill="#6b7280">min(data) or Q1 - 1.5*IQR</text>
  <!-- Outlier labels -->
  <line x1="268" y1="62" x2="360" y2="50" stroke="#9ca3af" stroke-width="1" stroke-dasharray="3,3"/>
  <text x="365" y="48" font-size="11" fill="#ef4444" font-weight="600">Outlier</text>
  <text x="365" y="62" font-size="10" fill="#ef4444">Beyond 1.5*IQR from box</text>
  <line x1="268" y1="348" x2="360" y2="362" stroke="#9ca3af" stroke-width="1" stroke-dasharray="3,3"/>
  <text x="365" y="366" font-size="11" fill="#ef4444" font-weight="600">Outlier</text>
  <!-- Middle 50% annotation -->
  <rect x="50" y="142" width="130" height="44" rx="4" fill="#f0fdf4" stroke="#16a34a" stroke-width="1"/>
  <text x="115" y="160" text-anchor="middle" font-size="10" fill="#16a34a" font-weight="600">The box contains the</text>
  <text x="115" y="175" text-anchor="middle" font-size="10" fill="#16a34a" font-weight="600">middle 50% of the data</text>
</svg>
</div>

```bio
set_seed(42)
# Compare quality across lanes
let lane1 = rnorm(2000, 33.0, 2.0)
let lane2 = rnorm(2000, 31.5, 3.5)
let lane3 = rnorm(2000, 28.0, 5.0)

let bp_table = table({"Lane 1": lane1, "Lane 2": lane2, "Lane 3": lane3})
boxplot(bp_table, {title: "Quality Score Distribution by Lane"})
# Lane 3 immediately stands out: lower median, wider spread, more outliers.
```

Box plots shine when comparing groups. You can see at a glance which lane is problematic, without computing a single number.

### Combining Plots and Stats

```bio
set_seed(42)
# Full QC report in a few lines
let scores = rnorm(10000, 32.5, 3.8)

# Side-by-side: histogram + box plot
histogram(scores, {bins: 40, title: "Quality Score Distribution"})
boxplot(scores, {title: "Quality Score Box Plot"})

# Summary table
let stats = summary(scores)
print(f"QC Decision: {if stats.mean > 30 then "PASS" else "FAIL"}")
print(f"Mean Q-score: {stats.mean:.1}")
print(f"Tiles below Q20: {scores |> filter(|s| s < 20) |> len()}")
```

## Python and R Equivalents

For those coming from Python or R, here are the equivalent operations.

**Python:**
```python
import numpy as np
from scipy import stats

scores = np.random.normal(32.5, 3.8, 10247)

# Measures of center
np.mean(scores)          # 32.48
np.median(scores)        # 32.51
stats.mode(scores)       # most frequent

# Measures of spread
np.std(scores, ddof=1)   # 3.81 (ddof=1 for sample SD)
np.var(scores, ddof=1)   # 14.52
np.percentile(scores, [25, 50, 75])
stats.iqr(scores)        # 5.14

# Shape
stats.skew(scores)       # -0.01
stats.kurtosis(scores)   # -0.01 (scipy uses excess kurtosis)

# Comprehensive summary
import pandas as pd
pd.Series(scores).describe()
```

**R:**
```r
scores <- rnorm(10247, mean = 32.5, sd = 3.8)

# Measures of center
mean(scores)       # 32.48
median(scores)     # 32.51

# Measures of spread
sd(scores)         # 3.81
var(scores)        # 14.52
quantile(scores, c(0.25, 0.5, 0.75))
IQR(scores)        # 5.14

# Shape (requires moments package)
library(moments)
skewness(scores)   # -0.01
kurtosis(scores)   # 2.99

# Comprehensive summary
summary(scores)

# Visualization
hist(scores, breaks = 50, main = "Quality Distribution")
boxplot(scores, main = "Quality Box Plot")
```

## Worked Example: The QC Decision

Let us walk through Dr. Chen's complete analysis.

```bio
set_seed(42)
# Step 1: Load the data
let scores = rnorm(10247, 32.5, 3.8)

# Step 2: Quick summary
let stats = summary(scores)

# Step 3: QC criteria
let pass_threshold = 30.0     # Minimum acceptable mean quality
let fail_fraction_limit = 0.10 # Max 10% tiles below Q20

# Step 4: Compute QC metrics
let tiles_below_q20 = scores |> filter(|s| s < 20) |> len()
let fraction_below_q20 = tiles_below_q20 / len(scores)

# Step 5: Decision
let qc_pass = stats.mean >= pass_threshold and fraction_below_q20 <= fail_fraction_limit

print("=== Sequencing Run QC Report ===")
print(f"Total tiles:          {len(scores)}")
print(f"Mean quality:         {stats.mean:.2}")
print(f"Median quality:       {stats.median:.2}")
print(f"Std deviation:        {stats.sd:.2}")
print(f"Tiles below Q20:      {tiles_below_q20} ({fraction_below_q20 * 100:.1}%)")
print(f"QC Result:            {if qc_pass then "PASS" else "FAIL"}")
print("================================")

# Step 6: Visualize
histogram(scores, {bins: 50, title: "Run QC: Quality Score Distribution"})
```

This takes about 10 seconds to run. Dr. Chen has her answer well before the meeting.

## Exercises

### Exercise 1: Gene Expression Summary

Compute descriptive statistics for a set of gene expression values and identify the best measure of center.

```bio
# Gene expression values (TPM) for 20 genes
let expression = [0.5, 1.2, 3.4, 5.1, 8.7, 12.3, 15.0, 22.4, 45.6, 78.9,
                  120.5, 250.3, 0.1, 2.8, 6.5, 0.3, 1100.0, 33.2, 0.8, 18.5]

# TODO: Compute mean, median, stdev, skewness
# TODO: Which measure of center best represents "typical" expression? Why?
# TODO: Create a histogram. What shape do you see?
```

### Exercise 2: Comparing Sequencing Runs

Two sequencing runs produced quality scores. Determine which run is better.

```bio
set_seed(42)
let run_a = rnorm(5000, 31.0, 2.5)
let run_b = rnorm(5000, 33.0, 6.0)

# TODO: Compute summary() for both runs
# TODO: Which has higher mean? Which has lower variability?
# TODO: Compute the CV for each. Which is more consistent?
# TODO: Create side-by-side box plots
# TODO: If you could only pick one run, which would you choose and why?
```

### Exercise 3: Outlier Detection

Identify outliers using the IQR method (values below Q1 - 1.5*IQR or above Q3 + 1.5*IQR).

```bio
# Protein abundance measurements with some suspicious values
let protein = [45.2, 48.1, 50.3, 47.8, 49.5, 46.7, 51.2, 48.9,
               150.0, 47.3, 49.8, 46.1, 50.5, 48.4, 3.0, 49.1]

# TODO: Compute Q1, Q3, IQR
# TODO: Calculate lower and upper fences
# TODO: Identify which values are outliers
# TODO: Compute mean with and without outliers — how much does it change?
```

### Exercise 4: Multi-Sample QC Dashboard

Build a QC summary for multiple samples.

```bio
set_seed(42)
let samples = {
    "Sample_A": rnorm(1000, 34.0, 2.0),
    "Sample_B": rnorm(1000, 29.0, 4.0),
    "Sample_C": rnorm(1000, 32.0, 3.0),
    "Sample_D": rnorm(1000, 20.0, 5.0)
}

# TODO: Loop through samples, compute summary() for each
# TODO: Flag any sample with mean < 25 or CV > 20%
# TODO: Create a box plot comparing all four samples
```

## Key Takeaways

- Descriptive statistics compress large datasets into interpretable summaries of center, spread, and shape.
- The **mean** is efficient for an additive expectation but sensitive to extremes; the **median** is robust and describes a ranked halfway point. For skewed data, report the pair that matches the estimand and often show both.
- **Standard deviation** measures absolute spread; **IQR** is robust to outliers; **CV** enables comparison across different scales.
- **Skewness** and **kurtosis** reveal whether your data's shape matches the assumptions of common statistical tests.
- **Always visualize** your data with histograms and box plots before computing any test. Summary statistics can hide multimodal distributions, outliers, and other structural features.
- `summary()` in BioLang provides comprehensive descriptive statistics in a single function call.
- Descriptive statistics are not optional preliminaries — they are the foundation of every analysis.

## What's Next

You now know how to summarize data, but you may have noticed something: we keep saying "normally distributed" and "skewed" without precisely defining what a distribution is. Tomorrow, on Day 3, we dive into the mathematical shapes that biological data follows — the normal distribution, the log-normal, the Poisson, and the binomial. You will learn why gene expression data refuses to be normal, why read counts follow a Poisson process (sort of), and how to test whether your data fits the distribution you think it does. Understanding distributions is the key to choosing the right statistical test — and avoiding the wrong one.
