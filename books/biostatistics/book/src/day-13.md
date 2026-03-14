# Day 13: Correlation — Finding Relationships

## The Problem

Dr. Sarah Kim is studying breast cancer transcriptomics across 200 tumor samples. She notices that **BRCA1** and **BARD1** expression seem to rise and fall together — when one is high, the other tends to be high too. Exciting! These genes encode proteins that form a heterodimer critical for DNA repair.

But her collaborator raises a concern: "Both genes are upregulated in rapidly dividing cells. Couldn't **cell proliferation** be driving both signals independently? You might be seeing a spurious association."

Sarah needs to:
1. **Quantify** how strongly BRCA1 and BARD1 co-vary
2. **Determine** whether the relationship is statistically significant
3. **Control** for the confounding effect of proliferation
4. **Visualize** relationships across an entire panel of DNA repair genes

This is the domain of **correlation analysis** — one of the most used (and most misused) tools in all of biology.

## What Is Correlation?

Correlation measures the **strength and direction** of the relationship between two variables. It answers: "When one variable goes up, does the other tend to go up, go down, or do nothing?"

The correlation coefficient ranges from **-1 to +1**:

| Value | Interpretation | Example |
|-------|---------------|---------|
| +1.0 | Perfect positive | Identical twins' heights |
| +0.7 to +0.9 | Strong positive | BRCA1 and BARD1 expression |
| +0.3 to +0.7 | Moderate positive | BMI and blood pressure |
| -0.3 to +0.3 | Weak / none | Shoe size and IQ |
| -0.3 to -0.7 | Moderate negative | Exercise and resting heart rate |
| -0.7 to -1.0 | Strong negative | Tumor suppressor vs. proliferation rate |
| -1.0 | Perfect negative | Altitude and air pressure |

> **Key insight:** Correlation is symmetric — the correlation between X and Y is the same as between Y and X. It doesn't imply direction or causation.

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="200" viewBox="0 0 680 200" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="22" text-anchor="middle" font-size="14" font-weight="bold" fill="#1e293b">Correlation Spectrum: r = -0.9 to r = +0.9</text>
  <!-- Panel 1: r = -0.9 -->
  <rect x="18" y="35" width="120" height="110" rx="4" fill="#f8fafc" stroke="#e2e8f0"/>
  <text x="78" y="52" text-anchor="middle" font-size="11" font-weight="bold" fill="#2563eb">r = -0.9</text>
  <line x1="30" y1="130" x2="126" y2="130" stroke="#d1d5db" stroke-width="0.5"/>
  <line x1="30" y1="130" x2="30" y2="60" stroke="#d1d5db" stroke-width="0.5"/>
  <line x1="35" y1="68" x2="120" y2="125" stroke="#2563eb" stroke-width="1.5" stroke-opacity="0.5"/>
  <circle cx="40" cy="70" r="2.5" fill="#2563eb"/><circle cx="48" cy="72" r="2.5" fill="#2563eb"/>
  <circle cx="55" cy="80" r="2.5" fill="#2563eb"/><circle cx="62" cy="82" r="2.5" fill="#2563eb"/>
  <circle cx="70" cy="90" r="2.5" fill="#2563eb"/><circle cx="78" cy="95" r="2.5" fill="#2563eb"/>
  <circle cx="85" cy="100" r="2.5" fill="#2563eb"/><circle cx="92" cy="105" r="2.5" fill="#2563eb"/>
  <circle cx="100" cy="112" r="2.5" fill="#2563eb"/><circle cx="108" cy="118" r="2.5" fill="#2563eb"/>
  <circle cx="115" cy="122" r="2.5" fill="#2563eb"/>
  <text x="78" y="148" text-anchor="middle" font-size="10" fill="#6b7280">Strong negative</text>
  <!-- Panel 2: r = -0.5 -->
  <rect x="152" y="35" width="120" height="110" rx="4" fill="#f8fafc" stroke="#e2e8f0"/>
  <text x="212" y="52" text-anchor="middle" font-size="11" font-weight="bold" fill="#3b82f6">r = -0.5</text>
  <line x1="164" y1="130" x2="260" y2="130" stroke="#d1d5db" stroke-width="0.5"/>
  <line x1="164" y1="130" x2="164" y2="60" stroke="#d1d5db" stroke-width="0.5"/>
  <line x1="170" y1="78" x2="254" y2="115" stroke="#3b82f6" stroke-width="1.5" stroke-opacity="0.5"/>
  <circle cx="175" cy="70" r="2.5" fill="#3b82f6"/><circle cx="183" cy="90" r="2.5" fill="#3b82f6"/>
  <circle cx="190" cy="78" r="2.5" fill="#3b82f6"/><circle cx="198" cy="95" r="2.5" fill="#3b82f6"/>
  <circle cx="205" cy="85" r="2.5" fill="#3b82f6"/><circle cx="212" cy="100" r="2.5" fill="#3b82f6"/>
  <circle cx="220" cy="92" r="2.5" fill="#3b82f6"/><circle cx="228" cy="110" r="2.5" fill="#3b82f6"/>
  <circle cx="235" cy="105" r="2.5" fill="#3b82f6"/><circle cx="243" cy="118" r="2.5" fill="#3b82f6"/>
  <circle cx="250" cy="125" r="2.5" fill="#3b82f6"/>
  <text x="212" y="148" text-anchor="middle" font-size="10" fill="#6b7280">Moderate neg.</text>
  <!-- Panel 3: r = 0 -->
  <rect x="286" y="35" width="120" height="110" rx="4" fill="#f8fafc" stroke="#e2e8f0"/>
  <text x="346" y="52" text-anchor="middle" font-size="11" font-weight="bold" fill="#6b7280">r = 0</text>
  <line x1="298" y1="130" x2="394" y2="130" stroke="#d1d5db" stroke-width="0.5"/>
  <line x1="298" y1="130" x2="298" y2="60" stroke="#d1d5db" stroke-width="0.5"/>
  <circle cx="310" cy="75" r="2.5" fill="#6b7280"/><circle cx="320" cy="110" r="2.5" fill="#6b7280"/>
  <circle cx="325" cy="85" r="2.5" fill="#6b7280"/><circle cx="335" cy="120" r="2.5" fill="#6b7280"/>
  <circle cx="340" cy="70" r="2.5" fill="#6b7280"/><circle cx="350" cy="100" r="2.5" fill="#6b7280"/>
  <circle cx="355" cy="65" r="2.5" fill="#6b7280"/><circle cx="365" cy="115" r="2.5" fill="#6b7280"/>
  <circle cx="370" cy="80" r="2.5" fill="#6b7280"/><circle cx="380" cy="95" r="2.5" fill="#6b7280"/>
  <circle cx="385" cy="125" r="2.5" fill="#6b7280"/>
  <text x="346" y="148" text-anchor="middle" font-size="10" fill="#6b7280">No correlation</text>
  <!-- Panel 4: r = +0.5 -->
  <rect x="420" y="35" width="120" height="110" rx="4" fill="#f8fafc" stroke="#e2e8f0"/>
  <text x="480" y="52" text-anchor="middle" font-size="11" font-weight="bold" fill="#f97316">r = +0.5</text>
  <line x1="432" y1="130" x2="528" y2="130" stroke="#d1d5db" stroke-width="0.5"/>
  <line x1="432" y1="130" x2="432" y2="60" stroke="#d1d5db" stroke-width="0.5"/>
  <line x1="438" y1="118" x2="522" y2="75" stroke="#f97316" stroke-width="1.5" stroke-opacity="0.5"/>
  <circle cx="440" cy="120" r="2.5" fill="#f97316"/><circle cx="448" cy="105" r="2.5" fill="#f97316"/>
  <circle cx="455" cy="115" r="2.5" fill="#f97316"/><circle cx="463" cy="98" r="2.5" fill="#f97316"/>
  <circle cx="470" cy="108" r="2.5" fill="#f97316"/><circle cx="480" cy="92" r="2.5" fill="#f97316"/>
  <circle cx="488" cy="100" r="2.5" fill="#f97316"/><circle cx="495" cy="85" r="2.5" fill="#f97316"/>
  <circle cx="503" cy="90" r="2.5" fill="#f97316"/><circle cx="510" cy="78" r="2.5" fill="#f97316"/>
  <circle cx="518" cy="72" r="2.5" fill="#f97316"/>
  <text x="480" y="148" text-anchor="middle" font-size="10" fill="#6b7280">Moderate pos.</text>
  <!-- Panel 5: r = +0.9 -->
  <rect x="554" y="35" width="120" height="110" rx="4" fill="#f8fafc" stroke="#e2e8f0"/>
  <text x="614" y="52" text-anchor="middle" font-size="11" font-weight="bold" fill="#dc2626">r = +0.9</text>
  <line x1="566" y1="130" x2="662" y2="130" stroke="#d1d5db" stroke-width="0.5"/>
  <line x1="566" y1="130" x2="566" y2="60" stroke="#d1d5db" stroke-width="0.5"/>
  <line x1="572" y1="125" x2="656" y2="65" stroke="#dc2626" stroke-width="1.5" stroke-opacity="0.5"/>
  <circle cx="575" cy="122" r="2.5" fill="#dc2626"/><circle cx="583" cy="118" r="2.5" fill="#dc2626"/>
  <circle cx="590" cy="112" r="2.5" fill="#dc2626"/><circle cx="598" cy="105" r="2.5" fill="#dc2626"/>
  <circle cx="606" cy="100" r="2.5" fill="#dc2626"/><circle cx="614" cy="93" r="2.5" fill="#dc2626"/>
  <circle cx="622" cy="88" r="2.5" fill="#dc2626"/><circle cx="630" cy="80" r="2.5" fill="#dc2626"/>
  <circle cx="638" cy="75" r="2.5" fill="#dc2626"/><circle cx="646" cy="70" r="2.5" fill="#dc2626"/>
  <circle cx="653" cy="66" r="2.5" fill="#dc2626"/>
  <text x="614" y="148" text-anchor="middle" font-size="10" fill="#6b7280">Strong positive</text>
  <!-- Bottom scale bar -->
  <rect x="78" y="170" width="524" height="8" rx="4" fill="url(#corrGrad)"/>
  <defs>
    <linearGradient id="corrGrad" x1="0%" y1="0%" x2="100%" y2="0%">
      <stop offset="0%" stop-color="#2563eb"/>
      <stop offset="50%" stop-color="#d1d5db"/>
      <stop offset="100%" stop-color="#dc2626"/>
    </linearGradient>
  </defs>
  <text x="78" y="192" text-anchor="start" font-size="10" fill="#2563eb">-1.0</text>
  <text x="340" y="192" text-anchor="middle" font-size="10" fill="#6b7280">0</text>
  <text x="602" y="192" text-anchor="end" font-size="10" fill="#dc2626">+1.0</text>
</svg>
</div>

## Three Types of Correlation

### 1. Pearson's r: The Linear Standard

Pearson's correlation coefficient measures **linear** association:

$$r = \frac{\sum_{i=1}^{n}(x_i - \bar{x})(y_i - \bar{y})}{\sqrt{\sum_{i=1}^{n}(x_i - \bar{x})^2 \cdot \sum_{i=1}^{n}(y_i - \bar{y})^2}}$$

**Assumptions:**
- Both variables are continuous
- Relationship is approximately linear
- Data are roughly normally distributed (for inference)
- No extreme outliers (a single outlier can flip the sign)

**Strengths:** Most powerful when assumptions are met. Directly related to R² in linear regression.

**Weakness:** Sensitive to outliers. Misses non-linear relationships entirely.

### 2. Spearman's ρ (rho): The Rank-Based Alternative

Spearman's correlation operates on **ranks** rather than raw values. It measures **monotonic** association — whether Y tends to increase (or decrease) as X increases, even if not linearly.

**How it works:**
1. Rank each variable separately (1, 2, 3, ...)
2. Compute Pearson's r on the ranks

**Assumptions:**
- Ordinal or continuous data
- Monotonic relationship (doesn't need to be linear)

**Strengths:** Robust to outliers. Works with non-normal distributions. Handles log-scale expression data naturally.

**Weakness:** Less powerful than Pearson when linearity holds.

### 3. Kendall's τ (tau): The Most Robust

Kendall's tau counts **concordant** and **discordant** pairs:

$$\tau = \frac{(\text{concordant pairs}) - (\text{discordant pairs})}{\binom{n}{2}}$$

A pair (i, j) is concordant if both xᵢ > xⱼ and yᵢ > yⱼ (or both less). It's discordant if they disagree.

**Strengths:** Most robust to outliers. Better for small samples. Has clearer probabilistic interpretation.

**Weakness:** Computationally slower for large datasets. Values tend to be smaller than Pearson/Spearman for the same data.

### Decision Table: Which Correlation to Use?

| Situation | Best Choice | Why |
|-----------|-------------|-----|
| Both variables normal, linear relationship | Pearson | Most powerful |
| Skewed data (e.g., raw gene expression) | Spearman | Rank-based, outlier-robust |
| Small sample (n < 30) | Kendall | Most reliable for small n |
| Ordinal data (e.g., tumor grade 1-4) | Spearman or Kendall | Ranks are appropriate |
| Suspect outliers | Spearman or Kendall | Rank-based methods resist outliers |
| Large RNA-seq dataset, log-transformed | Pearson or Spearman | Both work well after log transform |
| Survival times with censoring | Kendall | Handles ties from censoring |

> **Common pitfall:** Using Pearson on raw RNA-seq counts. These are heavily right-skewed — a few highly expressed genes dominate the correlation. Always log-transform first or use Spearman.

## Partial Correlation: Controlling for Confounders

Standard correlation between X and Y can be **inflated** or **deflated** by a confounding variable Z that influences both. Partial correlation removes the effect of Z:

$$r_{XY \cdot Z} = \frac{r_{XY} - r_{XZ} \cdot r_{YZ}}{\sqrt{(1 - r_{XZ}^2)(1 - r_{YZ}^2)}}$$

**Example:** BRCA1 and BARD1 might correlate at r = 0.85. But if cell proliferation (MKI67 expression) drives both, the partial correlation controlling for MKI67 might drop to r = 0.45 — still real, but weaker.

> **Clinical relevance:** In pharmacogenomics, two drug targets may appear correlated simply because both are overexpressed in a particular cancer subtype. Partial correlation controlling for subtype reveals whether the targets have an independent relationship.

## Anscombe's Quartet: Why You Must Visualize

In 1973, Francis Anscombe created four datasets with **identical** Pearson correlations (r = 0.816), identical means, and identical regression lines — but wildly different patterns:

| Dataset | Pattern | Lesson |
|---------|---------|--------|
| I | Normal linear | Correlation works correctly |
| II | Perfect curve | r misses non-linearity |
| III | Tight line + one outlier | Single point inflates r |
| IV | Vertical cluster + outlier | r is meaningless |

**The lesson:** Never trust a correlation coefficient without a scatter plot.

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="230" viewBox="0 0 680 230" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="22" text-anchor="middle" font-size="14" font-weight="bold" fill="#1e293b">Anscombe's Quartet: All Have r = 0.82 — But Look Different!</text>
  <!-- Dataset I: Normal linear -->
  <rect x="20" y="40" width="150" height="130" rx="4" fill="#f8fafc" stroke="#e2e8f0"/>
  <text x="95" y="56" text-anchor="middle" font-size="11" font-weight="bold" fill="#2563eb">I: Linear</text>
  <line x1="35" y1="155" x2="155" y2="155" stroke="#d1d5db" stroke-width="0.5"/>
  <line x1="35" y1="155" x2="35" y2="65" stroke="#d1d5db" stroke-width="0.5"/>
  <line x1="40" y1="145" x2="150" y2="72" stroke="#2563eb" stroke-width="1" stroke-opacity="0.4"/>
  <circle cx="45" cy="140" r="2.5" fill="#2563eb"/><circle cx="55" cy="132" r="2.5" fill="#2563eb"/>
  <circle cx="65" cy="125" r="2.5" fill="#2563eb"/><circle cx="78" cy="118" r="2.5" fill="#2563eb"/>
  <circle cx="88" cy="108" r="2.5" fill="#2563eb"/><circle cx="98" cy="100" r="2.5" fill="#2563eb"/>
  <circle cx="108" cy="95" r="2.5" fill="#2563eb"/><circle cx="118" cy="87" r="2.5" fill="#2563eb"/>
  <circle cx="128" cy="80" r="2.5" fill="#2563eb"/><circle cx="140" cy="75" r="2.5" fill="#2563eb"/>
  <circle cx="148" cy="70" r="2.5" fill="#2563eb"/>
  <text x="95" y="178" text-anchor="middle" font-size="10" fill="#16a34a">Correlation works</text>
  <!-- Dataset II: Perfect curve -->
  <rect x="185" y="40" width="150" height="130" rx="4" fill="#f8fafc" stroke="#e2e8f0"/>
  <text x="260" y="56" text-anchor="middle" font-size="11" font-weight="bold" fill="#7c3aed">II: Curved</text>
  <line x1="200" y1="155" x2="320" y2="155" stroke="#d1d5db" stroke-width="0.5"/>
  <line x1="200" y1="155" x2="200" y2="65" stroke="#d1d5db" stroke-width="0.5"/>
  <path d="M 205 148 Q 260 60 315 148" fill="none" stroke="#7c3aed" stroke-width="1" stroke-opacity="0.4"/>
  <circle cx="210" cy="145" r="2.5" fill="#7c3aed"/><circle cx="222" cy="125" r="2.5" fill="#7c3aed"/>
  <circle cx="234" cy="100" r="2.5" fill="#7c3aed"/><circle cx="246" cy="82" r="2.5" fill="#7c3aed"/>
  <circle cx="258" cy="72" r="2.5" fill="#7c3aed"/><circle cx="268" cy="70" r="2.5" fill="#7c3aed"/>
  <circle cx="278" cy="78" r="2.5" fill="#7c3aed"/><circle cx="288" cy="92" r="2.5" fill="#7c3aed"/>
  <circle cx="298" cy="110" r="2.5" fill="#7c3aed"/><circle cx="308" cy="130" r="2.5" fill="#7c3aed"/>
  <circle cx="315" cy="145" r="2.5" fill="#7c3aed"/>
  <text x="260" y="178" text-anchor="middle" font-size="10" fill="#dc2626">r misses non-linearity</text>
  <!-- Dataset III: Outlier inflates -->
  <rect x="350" y="40" width="150" height="130" rx="4" fill="#f8fafc" stroke="#e2e8f0"/>
  <text x="425" y="56" text-anchor="middle" font-size="11" font-weight="bold" fill="#f97316">III: Outlier</text>
  <line x1="365" y1="155" x2="485" y2="155" stroke="#d1d5db" stroke-width="0.5"/>
  <line x1="365" y1="155" x2="365" y2="65" stroke="#d1d5db" stroke-width="0.5"/>
  <line x1="370" y1="110" x2="480" y2="90" stroke="#f97316" stroke-width="1" stroke-opacity="0.4"/>
  <circle cx="375" cy="110" r="2.5" fill="#f97316"/><circle cx="385" cy="108" r="2.5" fill="#f97316"/>
  <circle cx="395" cy="106" r="2.5" fill="#f97316"/><circle cx="405" cy="105" r="2.5" fill="#f97316"/>
  <circle cx="415" cy="103" r="2.5" fill="#f97316"/><circle cx="425" cy="102" r="2.5" fill="#f97316"/>
  <circle cx="435" cy="100" r="2.5" fill="#f97316"/><circle cx="445" cy="98" r="2.5" fill="#f97316"/>
  <circle cx="455" cy="97" r="2.5" fill="#f97316"/><circle cx="465" cy="95" r="2.5" fill="#f97316"/>
  <circle cx="475" cy="68" r="4" fill="#dc2626" stroke="#dc2626" stroke-width="1"/>
  <text x="425" y="178" text-anchor="middle" font-size="10" fill="#dc2626">One point inflates r</text>
  <!-- Dataset IV: Vertical + outlier -->
  <rect x="515" y="40" width="150" height="130" rx="4" fill="#f8fafc" stroke="#e2e8f0"/>
  <text x="590" y="56" text-anchor="middle" font-size="11" font-weight="bold" fill="#dc2626">IV: Clustered</text>
  <line x1="530" y1="155" x2="650" y2="155" stroke="#d1d5db" stroke-width="0.5"/>
  <line x1="530" y1="155" x2="530" y2="65" stroke="#d1d5db" stroke-width="0.5"/>
  <circle cx="568" cy="145" r="2.5" fill="#dc2626"/><circle cx="568" cy="130" r="2.5" fill="#dc2626"/>
  <circle cx="568" cy="115" r="2.5" fill="#dc2626"/><circle cx="568" cy="100" r="2.5" fill="#dc2626"/>
  <circle cx="568" cy="85" r="2.5" fill="#dc2626"/><circle cx="568" cy="135" r="2.5" fill="#dc2626"/>
  <circle cx="568" cy="120" r="2.5" fill="#dc2626"/><circle cx="568" cy="105" r="2.5" fill="#dc2626"/>
  <circle cx="568" cy="90" r="2.5" fill="#dc2626"/><circle cx="568" cy="75" r="2.5" fill="#dc2626"/>
  <circle cx="638" cy="68" r="4" fill="#dc2626" stroke="#dc2626" stroke-width="1"/>
  <text x="590" y="178" text-anchor="middle" font-size="10" fill="#dc2626">r is meaningless</text>
  <!-- Common annotation -->
  <text x="340" y="205" text-anchor="middle" font-size="12" font-weight="bold" fill="#7c3aed">All four datasets: r = 0.82, same mean, same variance, same regression line</text>
  <text x="340" y="222" text-anchor="middle" font-size="12" fill="#1e293b">Lesson: ALWAYS visualize before trusting a correlation coefficient</text>
</svg>
</div>

## Correlation Matrix and Heatmaps

When studying many variables simultaneously, a **correlation matrix** shows all pairwise correlations. For p variables, this is a p × p symmetric matrix with 1s on the diagonal.

For genomics, this is invaluable for:
- Identifying co-expression modules
- Detecting batch effects (technical variables cluster together)
- Revealing pathway relationships

A **heatmap** visualization uses color intensity to represent correlation strength, making patterns immediately visible across dozens or hundreds of variables.

## Testing Significance: Is This Correlation Real?

A correlation of r = 0.3 might be noise in 10 samples but highly significant in 1000. The **correlation test** evaluates:

- **H₀:** ρ = 0 (no correlation in the population)
- **H₁:** ρ ≠ 0

The test statistic follows a t-distribution with n-2 degrees of freedom:

$$t = r\sqrt{\frac{n-2}{1-r^2}}$$

> **Key insight:** With large n (common in genomics), even tiny correlations become "significant." A correlation of r = 0.05 is significant at p < 0.05 when n > 1500. Always report both the coefficient AND the p-value, and judge practical significance by the magnitude of r.

## Correlation ≠ Causation

This cannot be overstated. Correlation tells you variables **co-vary** — nothing more.

Classic examples in biology:
- Ice cream sales correlate with drowning deaths (confounder: hot weather)
- Shoe size correlates with reading ability in children (confounder: age)
- Stork population correlates with birth rate across European countries (confounder: rural vs. urban)

To establish causation, you need:
1. **Temporal precedence** — cause precedes effect
2. **Experimental manipulation** — perturb X, observe Y change
3. **Elimination of confounders** — no third variable explains both
4. **Biological mechanism** — plausible pathway

<div style="text-align: center; margin: 2em 0;">
<svg width="660" height="240" viewBox="0 0 660 240" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="330" y="24" text-anchor="middle" font-size="14" font-weight="bold" fill="#1e293b">Correlation Does NOT Imply Causation</text>
  <text x="330" y="44" text-anchor="middle" font-size="12" fill="#6b7280">A confounding variable creates a spurious association</text>
  <!-- Confounding variable (top) -->
  <rect x="235" y="62" width="190" height="40" rx="20" fill="#f97316" fill-opacity="0.15" stroke="#f97316" stroke-width="2"/>
  <text x="330" y="87" text-anchor="middle" font-size="14" font-weight="bold" fill="#f97316">Hot Weather</text>
  <text x="330" y="56" text-anchor="middle" font-size="11" fill="#9ca3af">(Confounder)</text>
  <!-- Left variable -->
  <rect x="60" y="160" width="180" height="40" rx="20" fill="#2563eb" fill-opacity="0.12" stroke="#2563eb" stroke-width="2"/>
  <text x="150" y="185" text-anchor="middle" font-size="14" font-weight="bold" fill="#2563eb">Ice Cream Sales</text>
  <!-- Right variable -->
  <rect x="420" y="160" width="180" height="40" rx="20" fill="#dc2626" fill-opacity="0.12" stroke="#dc2626" stroke-width="2"/>
  <text x="510" y="185" text-anchor="middle" font-size="14" font-weight="bold" fill="#dc2626">Drowning Deaths</text>
  <!-- Causal arrows from confounder -->
  <line x1="275" y1="102" x2="185" y2="155" stroke="#f97316" stroke-width="2.5" marker-end="url(#arrowOrange)"/>
  <line x1="385" y1="102" x2="475" y2="155" stroke="#f97316" stroke-width="2.5" marker-end="url(#arrowOrange)"/>
  <!-- Spurious correlation (dashed) -->
  <line x1="240" y1="180" x2="420" y2="180" stroke="#9ca3af" stroke-width="2" stroke-dasharray="8,4"/>
  <text x="330" y="175" text-anchor="middle" font-size="12" fill="#9ca3af">r = 0.87 (spurious!)</text>
  <!-- Cross mark on spurious line -->
  <line x1="322" y1="192" x2="338" y2="208" stroke="#dc2626" stroke-width="3"/>
  <line x1="338" y1="192" x2="322" y2="208" stroke="#dc2626" stroke-width="3"/>
  <text x="330" y="228" text-anchor="middle" font-size="12" fill="#dc2626" font-weight="bold">No causal link exists between ice cream and drowning</text>
  <!-- Arrow markers -->
  <defs>
    <marker id="arrowOrange" markerWidth="10" markerHeight="7" refX="9" refY="3.5" orient="auto">
      <polygon points="0 0, 10 3.5, 0 7" fill="#f97316"/>
    </marker>
  </defs>
</svg>
</div>

## Correlation in BioLang

### Basic Correlations

```bio
set_seed(42)
# Generate tumor expression data for 200 samples
let n = 200

# Simulate BRCA1 and BARD1 with true correlation + noise
let proliferation = rnorm(n, 10, 3)
let brca1 = proliferation * 0.8 + rnorm(n, 5, 2)
let bard1 = proliferation * 0.7 + rnorm(n, 4, 2)

# Pearson correlation — assumes linearity
let r_pearson = cor(brca1, bard1)
print("Pearson r: {r_pearson}")  # ~0.78

# Spearman rank correlation — monotonic, robust
let r_spearman = spearman(brca1, bard1)
print("Spearman ρ: {r_spearman}")  # ~0.76

# Kendall tau — concordant/discordant pairs, most robust
let r_kendall = kendall(brca1, bard1)
print("Kendall τ: {r_kendall}")  # ~0.57 (typically smaller)
```

### Statistical Testing

```bio
# Test whether correlation is significantly different from zero
# cor() returns the coefficient; use a t-test transformation for p-value
let r = cor(brca1, bard1)
let t_stat = r * sqrt((n - 2) / (1 - r * r))
print("r = {r}, t = {t_stat}")

# Spearman returns {coefficient, pvalue}
let spearman_result = spearman(brca1, bard1)
print("Spearman ρ = {spearman_result}")
```

### Partial Correlation: Removing Confounders

```bio
set_seed(42)
# BRCA1-BARD1 correlation controlling for proliferation (MKI67)
let mki67 = proliferation + rnorm(n, 0, 1)

# Raw correlation
let r_raw = cor(brca1, bard1)
print("Raw Pearson r: {r_raw}")  # ~0.78

# Partial correlation controlling for MKI67
# Compute manually: regress out confounder from both variables
let r_xz = cor(brca1, mki67)
let r_yz = cor(bard1, mki67)
let r_partial = (r_raw - r_xz * r_yz) / sqrt((1 - r_xz * r_xz) * (1 - r_yz * r_yz))
print("Partial r (controlling MKI67): {r_partial}")  # lower — confounder removed

# The difference reveals how much of the BRCA1-BARD1
# association was driven by shared proliferation signal
print("Reduction: {((r_raw - r_partial) / r_raw * 100) |> round(1)}%")
```

### Correlation Matrix and Heatmap

```bio
set_seed(42)
# Build expression matrix for 8 DNA repair genes
let base = rnorm(200, 0, 1)

let genes = {
    "BRCA1":  base * 0.8 + rnorm(200, 10, 2),
    "BARD1":  base * 0.7 + rnorm(200, 8, 2),
    "RAD51":  base * 0.6 + rnorm(200, 12, 3),
    "PALB2":  base * 0.5 + rnorm(200, 9, 2),
    "ATM":    rnorm(200, 11, 3),
    "TP53":   base * -0.4 + rnorm(200, 15, 4),
    "MDM2":   base * -0.3 + rnorm(200, 7, 2),
    "GAPDH":  rnorm(200, 20, 1)
}

# Compute pairwise correlations
let gene_names = ["BRCA1", "BARD1", "RAD51", "PALB2", "ATM", "TP53", "MDM2", "GAPDH"]
for i in 0..8 {
    for j in (i+1)..8 {
        let r = cor(genes[gene_names[i]], genes[gene_names[j]])
        print("{gene_names[i]} vs {gene_names[j]}: r = {r |> round(3)}")
    }
}

# Visualize as heatmap — instantly reveals co-expression modules
heatmap(genes, {title: "DNA Repair Gene Co-Expression", color_scale: "RdBu"})
```

### Visualizing with Scatter Plots

```bio
# Scatter plot with correlation annotation
let scatter_data = table({"BRCA1": brca1, "BARD1": bard1})
plot(scatter_data, {type: "scatter", x: "BRCA1", y: "BARD1",
    title: "BRCA1 vs BARD1 Co-Expression",
    x_label: "BRCA1 Expression (log2 FPKM)",
    y_label: "BARD1 Expression (log2 FPKM)"})
```

### Demonstrating Anscombe's Quartet Effect

```bio
set_seed(42)
# Two datasets with identical Pearson r but different patterns
let x_linear = rnorm(100, 10, 3)
let y_linear = x_linear * 0.5 + rnorm(100, 0, 2)

let x_curve = rnorm(100, 10, 5)
let y_curve = (x_curve - 10) ** 2 / 10 + rnorm(100, 0, 1)

print("Linear: Pearson r = {cor(x_linear, y_linear)}")
print("Curved: Pearson r = {cor(x_curve, y_curve)}")
print("Linear: Spearman ρ = {spearman(x_linear, y_linear)}")
print("Curved: Spearman ρ = {spearman(x_curve, y_curve)}")

# Spearman catches the monotonic but non-linear pattern
# Always plot your data!
```

**Python:**

```python
import numpy as np
from scipy import stats
import seaborn as sns
import matplotlib.pyplot as plt

# Pearson
r, p = stats.pearsonr(brca1, bard1)

# Spearman
rho, p = stats.spearmanr(brca1, bard1)

# Kendall
tau, p = stats.kendalltau(brca1, bard1)

# Partial correlation (using pingouin)
import pingouin as pg
partial = pg.partial_corr(data=df, x='BRCA1', y='BARD1', covar='MKI67')

# Correlation matrix heatmap
corr = df.corr(method='spearman')
sns.heatmap(corr, annot=True, cmap='RdBu_r', center=0)
```

**R:**

```r
# Pearson
cor.test(brca1, bard1, method = "pearson")

# Spearman
cor.test(brca1, bard1, method = "spearman")

# Kendall
cor.test(brca1, bard1, method = "kendall")

# Partial correlation (using ppcor)
library(ppcor)
pcor.test(brca1, bard1, mki67)

# Correlation matrix heatmap
library(corrplot)
corrplot(cor(gene_matrix, method = "spearman"),
         method = "color", type = "upper")
```

## Exercises

### Exercise 1: Compare Three Methods

Compute Pearson, Spearman, and Kendall correlations for the following gene pairs. Which method gives the most different result, and why?

```bio
set_seed(42)
let n = 150

# Gene pair 1: linear relationship with outliers
let gene_a = rnorm(n, 8, 2)
let gene_b = gene_a * 0.6 + rnorm(n, 3, 1)

# Add 5 extreme outliers
# (imagine contaminated samples)

# Compute all three correlations for gene_a vs gene_b
# Which is most affected by outliers?
```

### Exercise 2: Partial Correlation in Drug Response

Three variables are measured across 100 cancer cell lines: drug sensitivity (IC50), target gene expression, and cell doubling time. The target gene and IC50 appear correlated. Is the relationship real, or driven by growth rate?

```bio
set_seed(42)
let n = 100
let growth_rate = rnorm(n, 24, 6)

let target_expr = growth_rate * 0.5 + rnorm(n, 10, 3)
let ic50 = growth_rate * -0.3 + rnorm(n, 50, 10)

# 1. Compute raw correlation between target_expr and ic50
# 2. Compute partial correlation controlling for growth_rate
# 3. What fraction of the apparent association was confounded?
```

### Exercise 3: Build and Interpret a Heatmap

Create a correlation heatmap for the following gene expression panel. Identify which genes cluster into co-expression modules.

```bio
set_seed(42)
let n = 300

# Simulate 3 biological modules:
# Module 1 (immune): CD8A, GZMB, PRF1, IFNG
# Module 2 (proliferation): MKI67, TOP2A, PCNA
# Module 3 (housekeeping): GAPDH, ACTB

# Create correlated expression within modules
# and weak/no correlation between modules
# Compute pairwise cor() and visualize with heatmap
# Which modules emerge from the clustering?
```

### Exercise 4: Significance vs. Magnitude

Generate datasets with n = 20 and n = 2000. Show that a weak correlation (r ≈ 0.1) is non-significant with small n but highly significant with large n. Argue why the p-value alone is misleading.

```bio
set_seed(42)
# Small sample: n = 20, weak correlation
let x_small = rnorm(20, 0, 1)
let y_small = x_small * 0.1 + rnorm(20, 0, 1)

# Large sample: n = 2000, same weak correlation
let x_large = rnorm(2000, 0, 1)
let y_large = x_large * 0.1 + rnorm(2000, 0, 1)

# Compute cor() on both and test significance
# Compare p-values and correlation magnitudes
# What should you report?
```

### Exercise 5: Anscombe Challenge

Create two synthetic gene-pair datasets where Pearson r is nearly identical (~0.7) but scatter plots reveal completely different biology — one linear, one with a threshold effect (flat then rising). Show that Spearman catches the difference.

## Key Takeaways

- **Pearson** measures linear association; **Spearman** measures monotonic (rank-based); **Kendall** counts concordant pairs and is most robust
- Correlation ranges from -1 to +1; the sign indicates direction, the magnitude indicates strength
- **Always visualize** — identical correlations can hide wildly different patterns (Anscombe's quartet)
- **Partial correlation** removes the effect of confounders, revealing true associations
- With large genomics datasets, tiny correlations become "significant" — always report the **magnitude** alongside the p-value
- **Correlation is not causation** — co-expression does not imply co-regulation or functional relationship
- Spearman is generally the safest default for gene expression data

## What's Next

Now that we can quantify relationships between two variables, Day 14 takes the next step: **using one variable to predict another**. We'll build our first linear regression models, learning to fit lines, interpret slopes, and critically evaluate whether our predictions are trustworthy.
