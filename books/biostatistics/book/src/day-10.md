# Day 10: Comparing Many Groups — ANOVA and Beyond

## The Problem

Dr. James Park's oncology team is testing a new targeted therapy at four dose levels: 0 mg (placebo), 25 mg, 50 mg, and 100 mg. Each group has 8 mice, and after 4 weeks they measure tumor volume in cubic millimeters. The team lead suggests: "Just do t-tests between all pairs of doses — that's 6 comparisons, no big deal."

But Dr. Park knows this is a trap. With 6 independent tests at alpha = 0.05, the probability of at least one false positive is not 5% — it is 1 - (0.95)^6 = 26.5%. Run 10 comparisons and it climbs to 40%. With 20,000 genes, the problem becomes catastrophic (we will tackle that on Day 12). The solution for comparing several groups at once is **Analysis of Variance** — ANOVA — which tests all groups simultaneously in a single, principled framework.

ANOVA has been the workhorse of experimental biology for nearly a century. Every drug dose-response study, every multi-tissue gene expression comparison, and every agricultural field trial relies on it. Today you will learn why it works, when it fails, and what to do after you get a significant result.

## What Is ANOVA?

Imagine a classroom of students from four different schools. ANOVA asks: "Is the variation in test scores *between* schools larger than what we would expect given the variation *within* each school?"

If all four schools have similar average scores, the between-school variation will be small relative to within-school variation. If one school's students consistently outscore the others, the between-school variation will dominate.

**ANOVA decomposes total variation into two sources:**

| Source | What It Measures | Symbol |
|---|---|---|
| Between-groups (treatment) | How much group means differ from the grand mean | SS_between |
| Within-groups (error) | How much individuals vary within their own group | SS_within |
| Total | Total variation in the data | SS_total |

**SS_total = SS_between + SS_within**

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="380" viewBox="0 0 680 380" xmlns="http://www.w3.org/2000/svg" style="background: #fafbfc; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="28" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">Between-Group vs Within-Group Variability</text>
  <text x="340" y="46" text-anchor="middle" font-size="12" fill="#6b7280">ANOVA asks: are the group means further apart than individual spread explains?</text>
  <!-- Axes -->
  <line x1="60" y1="300" x2="640" y2="300" stroke="#374151" stroke-width="1.5"/>
  <line x1="60" y1="60" x2="60" y2="300" stroke="#374151" stroke-width="1"/>
  <text x="30" y="185" text-anchor="middle" font-size="12" fill="#374151" transform="rotate(-90, 30, 185)">Tumor Volume (mm^3)</text>
  <!-- Y gridlines and labels -->
  <line x1="60" y1="100" x2="640" y2="100" stroke="#e5e7eb" stroke-width="0.5"/> <text x="55" y="104" text-anchor="end" font-size="9" fill="#9ca3af">500</text>
  <line x1="60" y1="140" x2="640" y2="140" stroke="#e5e7eb" stroke-width="0.5"/> <text x="55" y="144" text-anchor="end" font-size="9" fill="#9ca3af">400</text>
  <line x1="60" y1="180" x2="640" y2="180" stroke="#e5e7eb" stroke-width="0.5"/> <text x="55" y="184" text-anchor="end" font-size="9" fill="#9ca3af">300</text>
  <line x1="60" y1="220" x2="640" y2="220" stroke="#e5e7eb" stroke-width="0.5"/> <text x="55" y="224" text-anchor="end" font-size="9" fill="#9ca3af">200</text>
  <line x1="60" y1="260" x2="640" y2="260" stroke="#e5e7eb" stroke-width="0.5"/> <text x="55" y="264" text-anchor="end" font-size="9" fill="#9ca3af">100</text>
  <!-- Grand mean line -->
  <line x1="60" y1="170" x2="640" y2="170" stroke="#7c3aed" stroke-width="1.5" stroke-dasharray="8,4"/>
  <text x="645" y="168" font-size="10" fill="#7c3aed">Grand mean</text>
  <!-- Group 1: Placebo (mean ~499, points: 485,512,468,530,495,478,521,503) -->
  <text x="150" y="320" text-anchor="middle" font-size="12" fill="#374151" font-weight="bold">Placebo</text>
  <!-- Individual points -->
  <circle cx="120" cy="106" r="4" fill="#93c5fd" opacity="0.7"/> <!-- 485 -->
  <circle cx="130" cy="100" r="4" fill="#93c5fd" opacity="0.7"/> <!-- 512 -->
  <circle cx="140" cy="110" r="4" fill="#93c5fd" opacity="0.7"/> <!-- 468 -->
  <circle cx="150" cy="94" r="4" fill="#93c5fd" opacity="0.7"/> <!-- 530 -->
  <circle cx="160" cy="102" r="4" fill="#93c5fd" opacity="0.7"/> <!-- 495 -->
  <circle cx="170" cy="106" r="4" fill="#93c5fd" opacity="0.7"/> <!-- 478 -->
  <circle cx="180" cy="96" r="4" fill="#93c5fd" opacity="0.7"/> <!-- 521 -->
  <!-- Group mean line -->
  <line x1="110" y1="102" x2="190" y2="102" stroke="#2563eb" stroke-width="2.5"/>
  <text x="150" y="335" text-anchor="middle" font-size="10" fill="#2563eb">x = 499</text>
  <!-- Within-group bracket -->
  <line x1="195" y1="94" x2="200" y2="94" stroke="#9ca3af" stroke-width="1"/>
  <line x1="200" y1="94" x2="200" y2="112" stroke="#9ca3af" stroke-width="1"/>
  <line x1="195" y1="112" x2="200" y2="112" stroke="#9ca3af" stroke-width="1"/>
  <text x="215" y="106" font-size="8" fill="#9ca3af">within</text>
  <!-- Group 2: 25mg (mean ~432) -->
  <text x="310" y="320" text-anchor="middle" font-size="12" fill="#374151" font-weight="bold">25 mg</text>
  <circle cx="280" cy="136" r="4" fill="#c4b5fd" opacity="0.7"/>
  <circle cx="290" cy="130" r="4" fill="#c4b5fd" opacity="0.7"/>
  <circle cx="300" cy="140" r="4" fill="#c4b5fd" opacity="0.7"/>
  <circle cx="310" cy="128" r="4" fill="#c4b5fd" opacity="0.7"/>
  <circle cx="320" cy="134" r="4" fill="#c4b5fd" opacity="0.7"/>
  <circle cx="330" cy="138" r="4" fill="#c4b5fd" opacity="0.7"/>
  <circle cx="340" cy="130" r="4" fill="#c4b5fd" opacity="0.7"/>
  <line x1="270" y1="134" x2="350" y2="134" stroke="#7c3aed" stroke-width="2.5"/>
  <text x="310" y="335" text-anchor="middle" font-size="10" fill="#7c3aed">x = 432</text>
  <!-- Group 3: 50mg (mean ~322) -->
  <text x="460" y="320" text-anchor="middle" font-size="12" fill="#374151" font-weight="bold">50 mg</text>
  <circle cx="430" cy="176" r="4" fill="#fde68a" opacity="0.7"/>
  <circle cx="440" cy="170" r="4" fill="#fde68a" opacity="0.7"/>
  <circle cx="450" cy="182" r="4" fill="#fde68a" opacity="0.7"/>
  <circle cx="460" cy="168" r="4" fill="#fde68a" opacity="0.7"/>
  <circle cx="470" cy="176" r="4" fill="#fde68a" opacity="0.7"/>
  <circle cx="480" cy="180" r="4" fill="#fde68a" opacity="0.7"/>
  <circle cx="490" cy="170" r="4" fill="#fde68a" opacity="0.7"/>
  <line x1="420" y1="175" x2="500" y2="175" stroke="#f59e0b" stroke-width="2.5"/>
  <text x="460" y="335" text-anchor="middle" font-size="10" fill="#b45309">x = 322</text>
  <!-- Group 4: 100mg (mean ~195) -->
  <text x="590" y="320" text-anchor="middle" font-size="12" fill="#374151" font-weight="bold">100 mg</text>
  <circle cx="560" cy="224" r="4" fill="#bbf7d0" opacity="0.7"/>
  <circle cx="570" cy="218" r="4" fill="#bbf7d0" opacity="0.7"/>
  <circle cx="580" cy="228" r="4" fill="#bbf7d0" opacity="0.7"/>
  <circle cx="590" cy="215" r="4" fill="#bbf7d0" opacity="0.7"/>
  <circle cx="600" cy="222" r="4" fill="#bbf7d0" opacity="0.7"/>
  <circle cx="610" cy="228" r="4" fill="#bbf7d0" opacity="0.7"/>
  <circle cx="620" cy="218" r="4" fill="#bbf7d0" opacity="0.7"/>
  <line x1="550" y1="222" x2="630" y2="222" stroke="#16a34a" stroke-width="2.5"/>
  <text x="590" y="335" text-anchor="middle" font-size="10" fill="#16a34a">x = 195</text>
  <!-- Between-group arrow -->
  <line x1="150" y1="75" x2="150" y2="70" stroke="#dc2626" stroke-width="1"/>
  <line x1="150" y1="70" x2="590" y2="70" stroke="#dc2626" stroke-width="1.5"/>
  <line x1="590" y1="70" x2="590" y2="75" stroke="#dc2626" stroke-width="1"/>
  <text x="370" y="66" text-anchor="middle" font-size="11" fill="#dc2626" font-weight="bold">Between-group variability (large = significant F)</text>
  <!-- Legend -->
  <text x="340" y="365" text-anchor="middle" font-size="11" fill="#6b7280">Horizontal lines = group means. Dots = individual observations (within-group spread).</text>
</svg>
</div>

## The F-Statistic

The F-statistic is the ratio of between-group variance to within-group variance:

**F = MS_between / MS_within**

Where MS (mean square) = SS / df.

| Source | df | MS | F |
|---|---|---|---|
| Between | k - 1 | SS_between / (k - 1) | MS_between / MS_within |
| Within | N - k | SS_within / (N - k) | — |
| Total | N - 1 | — | — |

k = number of groups, N = total sample size.

- **F near 1**: Between-group variation is similar to within-group noise. No evidence of differences.
- **F much greater than 1**: Between-group variation exceeds what noise alone would produce. At least one group differs.

<div style="text-align: center; margin: 2em 0;">
<svg width="650" height="280" viewBox="0 0 650 280" xmlns="http://www.w3.org/2000/svg" style="background: #fafbfc; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="325" y="28" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">The F-Ratio: Between vs Within Variance</text>
  <!-- F = between / within -->
  <text x="325" y="58" text-anchor="middle" font-size="16" fill="#374151" font-weight="bold">F = MS_between / MS_within</text>
  <!-- Left bar: between-group variance -->
  <text x="175" y="90" text-anchor="middle" font-size="12" fill="#dc2626" font-weight="bold">MS_between</text>
  <text x="175" y="105" text-anchor="middle" font-size="10" fill="#6b7280">(treatment effect + noise)</text>
  <rect x="130" y="115" width="90" height="130" rx="4" fill="#fecaca" stroke="#dc2626" stroke-width="2"/>
  <text x="175" y="185" text-anchor="middle" font-size="28" fill="#dc2626" font-weight="bold">Big</text>
  <!-- Division symbol -->
  <text x="310" y="185" text-anchor="middle" font-size="36" fill="#374151" font-weight="bold">/</text>
  <!-- Right bar: within-group variance -->
  <text x="430" y="90" text-anchor="middle" font-size="12" fill="#3b82f6" font-weight="bold">MS_within</text>
  <text x="430" y="105" text-anchor="middle" font-size="10" fill="#6b7280">(noise only)</text>
  <rect x="385" y="185" width="90" height="60" rx="4" fill="#dbeafe" stroke="#2563eb" stroke-width="2"/>
  <text x="430" y="220" text-anchor="middle" font-size="20" fill="#2563eb" font-weight="bold">Small</text>
  <!-- Equals sign and result -->
  <text x="540" y="185" text-anchor="middle" font-size="28" fill="#374151" font-weight="bold">=</text>
  <rect x="570" y="155" width="65" height="55" rx="8" fill="#dcfce7" stroke="#16a34a" stroke-width="2"/>
  <text x="602" y="175" text-anchor="middle" font-size="14" fill="#16a34a" font-weight="bold">Large</text>
  <text x="602" y="195" text-anchor="middle" font-size="22" fill="#16a34a" font-weight="bold">F</text>
  <!-- Interpretation -->
  <text x="175" y="260" text-anchor="middle" font-size="11" fill="#374151">F near 1: groups similar (noise = noise)</text>
  <text x="475" y="260" text-anchor="middle" font-size="11" fill="#16a34a" font-weight="bold">F >> 1: groups differ (signal > noise)</text>
</svg>
</div>

> **Key insight:** ANOVA's null hypothesis is that ALL group means are equal. A significant F-statistic tells you "at least one group differs" but does NOT tell you which one(s). You need post-hoc tests for that.

## The Family-Wise Error Rate Problem

Why not just do multiple t-tests?

| Number of Groups | Pairwise Comparisons | P(at least one false positive) |
|---|---|---|
| 3 | 3 | 14.3% |
| 4 | 6 | 26.5% |
| 5 | 10 | 40.1% |
| 6 | 15 | 53.7% |
| 10 | 45 | 90.1% |

ANOVA controls this by testing all groups in a single hypothesis test.

## Assumptions of One-Way ANOVA

1. **Independence**: Observations are independent within and between groups
2. **Normality**: Data within each group are approximately normally distributed
3. **Homoscedasticity**: All groups have equal variances (check with Levene's test)

> **Common pitfall:** ANOVA is robust to mild violations of normality when group sizes are equal and n > 10 per group. But it is sensitive to unequal variances, especially with unequal group sizes. When Levene's test is significant, consider Welch's ANOVA or the Kruskal-Wallis alternative.

## Post-Hoc Tests: Which Groups Differ?

### Tukey's Honestly Significant Difference (HSD)

The gold standard post-hoc test. Compares all pairs of group means while controlling the family-wise error rate.

- Tests all k(k-1)/2 pairwise differences
- Provides adjusted p-values and confidence intervals
- Assumes equal variances and equal (or similar) group sizes

<div style="text-align: center; margin: 2em 0;">
<svg width="650" height="340" viewBox="0 0 650 340" xmlns="http://www.w3.org/2000/svg" style="background: #fafbfc; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="325" y="28" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">Tukey HSD: Pairwise Comparison Matrix</text>
  <text x="325" y="46" text-anchor="middle" font-size="12" fill="#6b7280">Which dose pairs are significantly different?</text>
  <!-- Matrix grid -->
  <!-- Column headers -->
  <text x="260" y="80" text-anchor="middle" font-size="12" fill="#374151" font-weight="bold">Placebo</text>
  <text x="360" y="80" text-anchor="middle" font-size="12" fill="#374151" font-weight="bold">25 mg</text>
  <text x="460" y="80" text-anchor="middle" font-size="12" fill="#374151" font-weight="bold">50 mg</text>
  <text x="560" y="80" text-anchor="middle" font-size="12" fill="#374151" font-weight="bold">100 mg</text>
  <!-- Row headers -->
  <text x="155" y="130" text-anchor="end" font-size="12" fill="#374151" font-weight="bold">Placebo</text>
  <text x="155" y="190" text-anchor="end" font-size="12" fill="#374151" font-weight="bold">25 mg</text>
  <text x="155" y="250" text-anchor="end" font-size="12" fill="#374151" font-weight="bold">50 mg</text>
  <text x="155" y="310" text-anchor="end" font-size="12" fill="#374151" font-weight="bold">100 mg</text>
  <!-- Diagonal (self) -->
  <rect x="215" y="105" width="90" height="45" rx="4" fill="#f3f4f6" stroke="#d1d5db" stroke-width="1"/>
  <text x="260" y="132" text-anchor="middle" font-size="11" fill="#9ca3af">---</text>
  <rect x="315" y="165" width="90" height="45" rx="4" fill="#f3f4f6" stroke="#d1d5db" stroke-width="1"/>
  <text x="360" y="192" text-anchor="middle" font-size="11" fill="#9ca3af">---</text>
  <rect x="415" y="225" width="90" height="45" rx="4" fill="#f3f4f6" stroke="#d1d5db" stroke-width="1"/>
  <text x="460" y="252" text-anchor="middle" font-size="11" fill="#9ca3af">---</text>
  <rect x="515" y="285" width="90" height="45" rx="4" fill="#f3f4f6" stroke="#d1d5db" stroke-width="1"/>
  <text x="560" y="312" text-anchor="middle" font-size="11" fill="#9ca3af">---</text>
  <!-- Significant pairs (red/green fill) -->
  <!-- Placebo vs 25mg: significant but smaller diff -->
  <rect x="315" y="105" width="90" height="45" rx="4" fill="#dcfce7" stroke="#16a34a" stroke-width="1.5"/>
  <text x="360" y="125" text-anchor="middle" font-size="10" fill="#16a34a" font-weight="bold">p &lt; 0.001</text>
  <text x="360" y="140" text-anchor="middle" font-size="9" fill="#16a34a">diff = 67 ***</text>
  <!-- Placebo vs 50mg: highly significant -->
  <rect x="415" y="105" width="90" height="45" rx="4" fill="#dcfce7" stroke="#16a34a" stroke-width="2"/>
  <text x="460" y="125" text-anchor="middle" font-size="10" fill="#16a34a" font-weight="bold">p &lt; 0.001</text>
  <text x="460" y="140" text-anchor="middle" font-size="9" fill="#16a34a">diff = 177 ***</text>
  <!-- Placebo vs 100mg: most significant -->
  <rect x="515" y="105" width="90" height="45" rx="4" fill="#bbf7d0" stroke="#16a34a" stroke-width="2"/>
  <text x="560" y="125" text-anchor="middle" font-size="10" fill="#16a34a" font-weight="bold">p &lt; 0.001</text>
  <text x="560" y="140" text-anchor="middle" font-size="9" fill="#16a34a">diff = 304 ***</text>
  <!-- 25mg vs 50mg -->
  <rect x="415" y="165" width="90" height="45" rx="4" fill="#dcfce7" stroke="#16a34a" stroke-width="1.5"/>
  <text x="460" y="185" text-anchor="middle" font-size="10" fill="#16a34a" font-weight="bold">p &lt; 0.001</text>
  <text x="460" y="200" text-anchor="middle" font-size="9" fill="#16a34a">diff = 110 ***</text>
  <!-- 25mg vs 100mg -->
  <rect x="515" y="165" width="90" height="45" rx="4" fill="#dcfce7" stroke="#16a34a" stroke-width="2"/>
  <text x="560" y="185" text-anchor="middle" font-size="10" fill="#16a34a" font-weight="bold">p &lt; 0.001</text>
  <text x="560" y="200" text-anchor="middle" font-size="9" fill="#16a34a">diff = 237 ***</text>
  <!-- 50mg vs 100mg -->
  <rect x="515" y="225" width="90" height="45" rx="4" fill="#dcfce7" stroke="#16a34a" stroke-width="1.5"/>
  <text x="560" y="245" text-anchor="middle" font-size="10" fill="#16a34a" font-weight="bold">p &lt; 0.001</text>
  <text x="560" y="260" text-anchor="middle" font-size="9" fill="#16a34a">diff = 127 ***</text>
  <!-- Lower triangle (mirror) -->
  <rect x="215" y="165" width="90" height="45" rx="4" fill="#dcfce7" stroke="#16a34a" stroke-width="1.5" opacity="0.5"/>
  <text x="260" y="192" text-anchor="middle" font-size="9" fill="#6b7280">symmetric</text>
  <rect x="215" y="225" width="90" height="45" rx="4" fill="#dcfce7" stroke="#16a34a" stroke-width="1.5" opacity="0.5"/>
  <text x="260" y="252" text-anchor="middle" font-size="9" fill="#6b7280">symmetric</text>
  <rect x="315" y="225" width="90" height="45" rx="4" fill="#dcfce7" stroke="#16a34a" stroke-width="1.5" opacity="0.5"/>
  <text x="360" y="252" text-anchor="middle" font-size="9" fill="#6b7280">symmetric</text>
  <rect x="215" y="285" width="90" height="45" rx="4" fill="#bbf7d0" stroke="#16a34a" stroke-width="2" opacity="0.5"/>
  <text x="260" y="312" text-anchor="middle" font-size="9" fill="#6b7280">symmetric</text>
  <rect x="315" y="285" width="90" height="45" rx="4" fill="#dcfce7" stroke="#16a34a" stroke-width="2" opacity="0.5"/>
  <text x="360" y="312" text-anchor="middle" font-size="9" fill="#6b7280">symmetric</text>
  <rect x="415" y="285" width="90" height="45" rx="4" fill="#dcfce7" stroke="#16a34a" stroke-width="1.5" opacity="0.5"/>
  <text x="460" y="312" text-anchor="middle" font-size="9" fill="#6b7280">symmetric</text>
  <!-- Key -->
  <rect x="30" y="95" width="12" height="12" rx="2" fill="#dcfce7" stroke="#16a34a" stroke-width="1"/>
  <text x="48" y="106" font-size="10" fill="#374151">Significant (p &lt; 0.05)</text>
  <text x="30" y="124" font-size="10" fill="#6b7280">All 6 pairs differ: clear dose-response</text>
</svg>
</div>

### Other Post-Hoc Options

| Method | When to Use |
|---|---|
| **Tukey HSD** | All pairwise comparisons needed, balanced design |
| **Bonferroni** | Conservative; fewer planned comparisons |
| **Dunnett** | Compare all groups to a single control |
| **Games-Howell** | Unequal variances or unequal group sizes |

## Effect Size: Eta-Squared

Just as Cohen's d quantifies the effect for two groups, **eta-squared** quantifies it for ANOVA:

**eta-squared = SS_between / SS_total**

| Eta-squared | Interpretation |
|---|---|
| 0.01 | Small — group explains 1% of total variance |
| 0.06 | Medium — group explains 6% |
| 0.14 | Large — group explains 14%+ |

## Non-Parametric Alternatives

| Parametric | Non-Parametric | Use When |
|---|---|---|
| One-way ANOVA | Kruskal-Wallis | Groups are independent, normality violated |
| Repeated measures ANOVA | Friedman test | Same subjects measured under all conditions |

## ANOVA in BioLang

### One-Way ANOVA: Dose-Response

```bio
# Tumor volume (mm^3) at 4 dose levels
let placebo = [485, 512, 468, 530, 495, 478, 521, 503]
let dose_25 = [420, 445, 398, 461, 432, 410, 452, 438]
let dose_50 = [310, 335, 288, 352, 321, 298, 345, 328]
let dose_100 = [180, 210, 165, 225, 195, 172, 218, 198]

# One-way ANOVA
let result = anova([placebo, dose_25, dose_50, dose_100])
print("=== One-Way ANOVA: Tumor Volume by Dose ===")
print("F-statistic: {result.statistic:.4}")
print("p-value: {result.p_value:.2e}")
print("df between: {result.df_between}, df within: {result.df_within}")

# Effect size: eta-squared = SS_between / SS_total (inline from anova output)
let eta2 = result.ss_between / (result.ss_between + result.ss_within)
print("Eta-squared: {eta2:.4} ({eta2*100:.1}% of variance explained)")

# Check assumptions: compare variances per group
print("\nVariances: placebo={variance(placebo):.1}, 25mg={variance(dose_25):.1}, 50mg={variance(dose_50):.1}, 100mg={variance(dose_100):.1}")
```

### Tukey HSD Post-Hoc

```bio
let placebo  = [485, 512, 468, 530, 495, 478, 521, 503]
let dose_25  = [420, 445, 398, 461, 432, 410, 452, 438]
let dose_50  = [310, 335, 288, 352, 321, 298, 345, 328]
let dose_100 = [180, 210, 165, 225, 195, 172, 218, 198]

# Tukey HSD: pairwise ttest() + p_adjust()
let groups = [placebo, dose_25, dose_50, dose_100]
let labels = ["Placebo", "25mg", "50mg", "100mg"]
let pairwise_p = []
let pair_labels = []
for i in 0..len(groups) {
  for j in (i+1)..len(groups) {
    let r = ttest(groups[i], groups[j])
    pairwise_p = append(pairwise_p, r.p_value)
    pair_labels = append(pair_labels, "{labels[i]} vs {labels[j]}")
  }
}
let adj_p = p_adjust(pairwise_p, "bonferroni")

print("=== Pairwise t-tests (Bonferroni-adjusted) ===")
print("Comparison          | Diff     | p-adj")
print("--------------------|----------|--------")
for k in 0..len(pair_labels) {
  print("{pair_labels[k]:<20}| {adj_p[k]:.4}")
}
```

### Grouped Boxplot Visualization

```bio
let groups = {
  "Placebo": [485, 512, 468, 530, 495, 478, 521, 503],
  "25 mg":   [420, 445, 398, 461, 432, 410, 452, 438],
  "50 mg":   [310, 335, 288, 352, 321, 298, 345, 328],
  "100 mg":  [180, 210, 165, 225, 195, 172, 218, 198]
}

boxplot(groups, {title: "Tumor Volume by Treatment Dose", y_label: "Tumor Volume (mm^3)", x_label: "Dose Group", show_points: true})
```

### Kruskal-Wallis: When ANOVA Assumptions Fail

```bio
# Cytokine levels across three disease stages — heavily skewed
let stage_I   = [2.1, 1.5, 3.8, 0.9, 12.5, 1.8, 2.4, 0.7, 4.2, 1.1]
let stage_II  = [8.5, 15.2, 5.3, 22.1, 9.8, 45.0, 7.2, 12.8, 6.1, 18.5]
let stage_III = [35.2, 88.1, 42.5, 120.0, 55.3, 78.9, 95.2, 48.7, 110.5, 65.8]

# Check normality
# Visual normality check — all stages are right-skewed
qq_plot(stage_I, {title: "QQ: Stage I"})
qq_plot(stage_II, {title: "QQ: Stage II"})
qq_plot(stage_III, {title: "QQ: Stage III"})
print("Normality violated -> use Kruskal-Wallis (anova on ranks)\n")

let result = anova([stage_I, stage_II, stage_III])
print("=== Kruskal-Wallis Test ===")
print("H statistic: {result.statistic:.4}")
print("p-value: {result.p_value:.2e}")

# Pairwise follow-up with Bonferroni correction
if result.p_value < 0.05 {
  let p12 = wilcoxon(stage_I, stage_II).p_value
  let p13 = wilcoxon(stage_I, stage_III).p_value
  let p23 = wilcoxon(stage_II, stage_III).p_value
  let adj = p_adjust([p12, p13, p23], "bonferroni")

  print("\nPairwise Mann-Whitney (Bonferroni-adjusted):")
  print("  Stage I vs II:   p = {adj[0]:.4}")
  print("  Stage I vs III:  p = {adj[1]:.4}")
  print("  Stage II vs III: p = {adj[2]:.4}")
}

let bp_table = table({"Stage I": stage_I, "Stage II": stage_II, "Stage III": stage_III})
boxplot(bp_table, {title: "IL-6 Levels by Disease Stage"})
```

### Friedman Test: Repeated Measures

```bio
# Pain scores (1-10) for 8 patients under 3 analgesics
# Same patients tested with each drug (crossover design)
let drug_a = [7, 5, 8, 6, 4, 7, 5, 6]
let drug_b = [4, 3, 5, 4, 2, 5, 3, 4]
let drug_c = [3, 2, 4, 3, 1, 3, 2, 3]

# Friedman test: anova() on ranks for repeated measures
let result = anova([drug_a, drug_b, drug_c])
print("=== Friedman Test: Pain Scores Across Analgesics ===")
print("Chi-squared: {result.statistic:.4}")
print("p-value: {result.p_value:.6}")

if result.p_value < 0.05 {
  print("At least one analgesic differs in pain reduction")
  print("Medians: Drug A={median(drug_a)}, Drug B={median(drug_b)}, Drug C={median(drug_c)}")
}
```

### Complete Workflow: Gene Expression Across Tissues

```bio
# FOXP3 expression across immune cell types
let t_reg  = [8.5, 9.2, 8.8, 9.5, 8.1, 9.0, 8.7, 9.3]
let t_eff  = [3.2, 3.8, 3.5, 4.1, 2.9, 3.6, 3.3, 3.9]
let b_cell = [1.5, 1.8, 1.2, 2.0, 1.4, 1.7, 1.3, 1.9]
let nk     = [0.8, 1.1, 0.6, 1.3, 0.9, 1.0, 0.7, 1.2]

# Step 1: Check assumptions
print("=== Assumption Checks ===")
# Compare variances across groups
print("Variances: T-reg={variance(t_reg):.3}, T-eff={variance(t_eff):.3}, B cell={variance(b_cell):.3}, NK={variance(nk):.3}")
# Visual normality check
for name, data in [["T-reg", t_reg], ["T-eff", t_eff], ["B cell", b_cell], ["NK", nk]] {
  qq_plot(data, {title: "QQ Plot: {name}"})
}

# Step 2: ANOVA
let result = anova([t_reg, t_eff, b_cell, nk])
print("\n=== One-Way ANOVA ===")
print("F = {result.statistic:.2}, p = {result.p_value:.2e}")

# Step 3: Effect size (eta-squared from anova output)
let eta2 = result.ss_between / (result.ss_between + result.ss_within)
print("Eta-squared = {eta2:.3} (cell type explains {eta2*100:.1}% of variance)")

# Step 4: Post-hoc — pairwise ttest() + p_adjust()
let cell_groups = [t_reg, t_eff, b_cell, nk]
let cell_labels = ["T-reg", "T-eff", "B cell", "NK"]
let pw_pvals = []
let pw_labels = []
for i in 0..len(cell_groups) {
  for j in (i+1)..len(cell_groups) {
    pw_pvals = append(pw_pvals, ttest(cell_groups[i], cell_groups[j]).p_value)
    pw_labels = append(pw_labels, "{cell_labels[i]} vs {cell_labels[j]}")
  }
}
let pw_adj = p_adjust(pw_pvals, "bonferroni")

print("\n=== Pairwise t-tests (Bonferroni) ===")
for k in 0..len(pw_labels) {
  let sig = if pw_adj[k] < 0.001 then "***" else if pw_adj[k] < 0.01 then "**" else if pw_adj[k] < 0.05 then "*" else "ns"
  print("{pw_labels[k]:<20} p={pw_adj[k]:.4} {sig}")
}

# Step 5: Visualize
let bp_table = table({"T-reg": t_reg, "T-eff": t_eff, "B cell": b_cell, "NK": nk})
boxplot(bp_table, {title: "FOXP3 Expression Across Immune Cell Types", show_points: true})
```

**Python:**

```python
from scipy import stats
import scikit_posthocs as sp

placebo  = [485, 512, 468, 530, 495, 478, 521, 503]
dose_25  = [420, 445, 398, 461, 432, 410, 452, 438]
dose_50  = [310, 335, 288, 352, 321, 298, 345, 328]
dose_100 = [180, 210, 165, 225, 195, 172, 218, 198]

# One-way ANOVA
f, p = stats.f_oneway(placebo, dose_25, dose_50, dose_100)
print(f"F = {f:.4f}, p = {p:.2e}")

# Tukey HSD
from statsmodels.stats.multicomp import pairwise_tukeyhsd
import numpy as np
data = placebo + dose_25 + dose_50 + dose_100
groups = ['P']*8 + ['25']*8 + ['50']*8 + ['100']*8
print(pairwise_tukeyhsd(data, groups))

# Kruskal-Wallis
h, p = stats.kruskal(placebo, dose_25, dose_50, dose_100)

# Friedman
stats.friedmanchisquare(drug_a, drug_b, drug_c)
```

**R:**

```r
# One-way ANOVA
data <- data.frame(
  volume = c(485,512,468,530,495,478,521,503,
             420,445,398,461,432,410,452,438,
             310,335,288,352,321,298,345,328,
             180,210,165,225,195,172,218,198),
  dose = factor(rep(c("Placebo","25mg","50mg","100mg"), each=8))
)
result <- aov(volume ~ dose, data = data)
summary(result)
TukeyHSD(result)

# Eta-squared
library(effectsize)
eta_squared(result)

# Kruskal-Wallis
kruskal.test(volume ~ dose, data = data)

# Friedman
friedman.test(matrix(c(drug_a, drug_b, drug_c), ncol=3))
```

## Exercises

**Exercise 1: FWER Calculation**

You have 5 experimental groups and want to compare all pairs. Calculate: (a) how many pairwise comparisons there are, (b) the probability of at least one false positive at alpha = 0.05, (c) what the Bonferroni-adjusted alpha would be.

**Exercise 2: Full ANOVA Workflow**

Three fertilizer treatments were tested on plant growth (cm):

```bio
let control   = [12.5, 14.2, 11.8, 13.5, 12.9, 14.8, 11.2, 13.1]
let fert_a    = [18.3, 20.1, 17.5, 19.8, 18.9, 21.2, 17.0, 19.5]
let fert_b    = [15.8, 17.2, 14.5, 16.9, 15.3, 17.8, 14.1, 16.5]

# TODO: Check normality and equal variances
# TODO: Run one-way ANOVA
# TODO: Compute eta-squared
# TODO: If significant, run Tukey HSD
# TODO: Create boxplot
# TODO: Interpret: which fertilizer is best?
```

**Exercise 3: Parametric vs Non-Parametric ANOVA**

Run both ANOVA and Kruskal-Wallis on the cytokine data. Compare p-values. Which is more appropriate?

```bio
let mild     = [5.2, 3.8, 8.1, 2.5, 12.0, 4.3, 6.7, 1.9]
let moderate = [25.1, 45.0, 18.3, 52.8, 30.5, 15.2, 38.7, 22.4]
let severe   = [120, 250, 85, 310, 180, 95, 275, 145]

# TODO: Check normality
# TODO: Run both ANOVA and Kruskal-Wallis
# TODO: Which test is more appropriate for this data? Why?
```

**Exercise 4: Repeated Measures Design**

Five patients had their blood pressure measured under three conditions (rest, mild exercise, intense exercise):

```bio
let rest     = [120, 135, 118, 142, 125]
let mild_ex  = [130, 148, 128, 155, 138]
let intense  = [155, 172, 148, 180, 162]

# TODO: Use Friedman test (non-parametric repeated measures)
# TODO: If significant, perform pairwise Wilcoxon signed-rank tests
# TODO: Apply Bonferroni correction to the pairwise p-values
```

## Key Takeaways

- **Multiple t-tests** inflate the false positive rate — the family-wise error rate grows rapidly with the number of comparisons
- **ANOVA** tests whether any group differs from the others in a single F-test, controlling the overall error rate
- The **F-statistic** compares between-group variance to within-group variance: F much greater than 1 suggests real differences
- A significant ANOVA tells you "at least one group differs" — use **Tukey HSD** post-hoc to find which pairs differ
- **Eta-squared** measures effect size: the proportion of total variance explained by group membership
- **Kruskal-Wallis** is the non-parametric alternative when normality is violated
- **Friedman test** handles repeated measures designs non-parametrically
- Always check assumptions (normality, equal variances) before interpreting ANOVA results

## What's Next

Tomorrow we shift from continuous to categorical outcomes. When your data consist of counts in categories — genotypes, disease status, response/non-response — you need the chi-square test and Fisher's exact test. These tools are essential for testing genetic associations, evaluating Hardy-Weinberg equilibrium, and computing odds ratios in case-control studies.
