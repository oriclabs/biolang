# Day 26: Meta-Analysis — Combining Studies

<div class="day-meta">
<span class="badge">Day 26 of 30</span>
<span class="badge">Prerequisites: Days 6-8, 14, 19</span>
<span class="badge">~60 min reading</span>
<span class="badge">Evidence Synthesis</span>
</div>

## The Problem

Three independent clinical trials have tested PCSK9 inhibitors for lowering LDL cholesterol in patients with familial hypercholesterolemia:

| Study | N | Mean LDL Reduction (mg/dL) | SE |
|---|---|---|---|
| Trial A (Europe, 2019) | 250 | -52.3 | 4.1 |
| Trial B (USA, 2020) | 180 | -48.7 | 5.3 |
| Trial C (Asia, 2021) | 320 | -55.1 | 3.8 |

Each study alone has a confidence interval that overlaps with the others. None is definitive. But together, the evidence is overwhelming. The question is: how do you formally combine them? You cannot just average the means — the studies have different sample sizes, different precisions, and were conducted in different populations. You need a method that respects these differences.

Meta-analysis is that method. It provides a rigorous framework for pooling results across studies, weighting each by its precision, quantifying heterogeneity, and assessing whether the pooled estimate is trustworthy. It sits at the top of the evidence hierarchy — above individual RCTs — because it synthesizes all available evidence.

## What Is Meta-Analysis?

Meta-analysis is the statistical combination of results from two or more separate studies to produce a single, more precise estimate of an effect. It is not simply "averaging" — it is a weighted combination that accounts for each study's precision.

Think of it as a vote among experts. If three experts estimate a quantity, you would give more weight to the expert who measured most precisely (smallest uncertainty), and less weight to the expert whose estimate is vague. Meta-analysis formalizes this intuition.

> **Key insight:** Meta-analysis does not combine raw data — it combines summary statistics (effect sizes and their standard errors). This means you can conduct a meta-analysis using published results alone, without accessing any original data.

## Why Combine Studies?

1. **Increased precision**: Pooling 750 patients across three trials gives a tighter CI than any single trial.
2. **Resolving contradictions**: If Study A finds an effect and Study B does not, meta-analysis can determine whether this reflects true heterogeneity or sampling variability.
3. **Generalizability**: Studies from Europe, USA, and Asia together provide evidence across populations.
4. **Detecting small effects**: An individual study may be underpowered; the pooled analysis may cross the significance threshold.

## Fixed-Effects Model

The fixed-effects model assumes that all studies estimate the **same true effect**. Differences between study results are due to sampling variability alone.

### Weighting

Each study is weighted by the inverse of its variance:

**w_i = 1 / SE_i^2**

The pooled estimate is the weighted mean:

**Pooled = Sum(w_i x estimate_i) / Sum(w_i)**

The pooled SE is:

**SE_pooled = 1 / sqrt(Sum(w_i))**

### When to Use Fixed Effects

Use fixed effects when you believe the true effect is the same across all studies — for instance, highly standardized lab assays or studies using identical protocols. In practice, this assumption is often too strong.

```bio
# Fixed-effects meta-analysis
let studies = ["Trial A", "Trial B", "Trial C"]
let effects = [-52.3, -48.7, -55.1]
let se = [4.1, 5.3, 3.8]

# Manual calculation
let weights = se |> map(|s| 1.0 / (s * s))
let total_weight = sum(weights)
let pooled_effect = sum(zip(weights, effects) |> map(|we| we[0] * we[1])) / total_weight
let pooled_se = 1.0 / sqrt(total_weight)

print("=== Fixed-Effects Meta-Analysis ===")
print("Study weights: " + str(weights |> map(|w| round(w, 2))))
print("Pooled effect: " + str(round(pooled_effect, 2)) + " mg/dL")
print("Pooled SE: " + str(round(pooled_se, 2)))
print("95% CI: [" +
  str(round(pooled_effect - 1.96 * pooled_se, 2)) + ", " +
  str(round(pooled_effect + 1.96 * pooled_se, 2)) + "]")
```

## Random-Effects Model

The random-effects model assumes that studies estimate **different but related true effects**. Each study's true effect is drawn from a distribution of effects. The between-study variance (tau-squared) captures how much the true effects vary.

### When to Use Random Effects

Use random effects when studies differ in population, dosing, protocol, or outcome definition — which is almost always in biomedical research. Random effects produces wider CIs than fixed effects, reflecting the additional uncertainty from between-study variability.

| Model | Assumption | CIs | When to use |
|---|---|---|---|
| Fixed-effects | Same true effect across studies | Narrower | Identical protocols, homogeneous studies |
| Random-effects | True effects vary across studies | Wider | Different populations, protocols, settings |

> **Common pitfall:** Some researchers choose between fixed and random effects based on which gives a smaller p-value. This is a form of p-hacking. Choose the model before seeing the results, based on the study designs and populations.

## Heterogeneity: Q and I-Squared

Heterogeneity quantifies how much the studies disagree beyond what sampling variability would explain.

### Cochran's Q Statistic

Q = Sum(w_i x (estimate_i - pooled)^2)

Under the null hypothesis of no heterogeneity, Q follows a chi-square distribution with k-1 degrees of freedom (where k is the number of studies). A significant Q (p < 0.10, using a lenient threshold because the test has low power) suggests heterogeneity.

### I-Squared

I-squared quantifies the proportion of total variation due to between-study heterogeneity:

**I^2 = max(0, (Q - df) / Q) x 100%**

| I^2 | Heterogeneity |
|---|---|
| 0-25% | Low — studies are consistent |
| 25-50% | Moderate — some inconsistency |
| 50-75% | Substantial — investigate sources |
| 75-100% | Considerable — pooling may be inappropriate |

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="280" viewBox="0 0 680 280" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <!-- Title -->
  <text x="340" y="25" text-anchor="middle" font-size="14" font-weight="bold" fill="#1e293b">I-Squared: Low vs High Heterogeneity</text>
  <!-- Left panel: Low I² -->
  <text x="170" y="50" text-anchor="middle" font-size="12" font-weight="bold" fill="#16a34a">Low I² (10%) — Studies Agree</text>
  <!-- Pooled line -->
  <line x1="170" y1="65" x2="170" y2="230" stroke="#16a34a" stroke-width="2" stroke-dasharray="5,3"/>
  <text x="170" y="248" text-anchor="middle" font-size="10" fill="#16a34a">Pooled = -52.5</text>
  <!-- Study CIs - tightly clustered -->
  <line x1="148" y1="85" x2="192" y2="85" stroke="#2563eb" stroke-width="2"/>
  <rect x="166" y="80" width="8" height="10" fill="#2563eb" rx="1"/>
  <text x="58" y="89" font-size="10" fill="#374151">Study A</text>
  <line x1="152" y1="115" x2="196" y2="115" stroke="#2563eb" stroke-width="2"/>
  <rect x="170" y="110" width="8" height="10" fill="#2563eb" rx="1"/>
  <text x="58" y="119" font-size="10" fill="#374151">Study B</text>
  <line x1="145" y1="145" x2="190" y2="145" stroke="#2563eb" stroke-width="2"/>
  <rect x="163" y="140" width="8" height="10" fill="#2563eb" rx="1"/>
  <text x="58" y="149" font-size="10" fill="#374151">Study C</text>
  <line x1="150" y1="175" x2="195" y2="175" stroke="#2563eb" stroke-width="2"/>
  <rect x="168" y="170" width="8" height="10" fill="#2563eb" rx="1"/>
  <text x="58" y="179" font-size="10" fill="#374151">Study D</text>
  <line x1="155" y1="205" x2="198" y2="205" stroke="#2563eb" stroke-width="2"/>
  <rect x="172" y="200" width="8" height="10" fill="#2563eb" rx="1"/>
  <text x="58" y="209" font-size="10" fill="#374151">Study E</text>
  <!-- Annotation -->
  <text x="170" y="268" text-anchor="middle" font-size="10" fill="#6b7280">CIs overlap tightly</text>
  <!-- Border -->
  <line x1="340" y1="40" x2="340" y2="260" stroke="#e5e7eb" stroke-width="1" stroke-dasharray="4,4"/>
  <!-- Right panel: High I² -->
  <text x="510" y="50" text-anchor="middle" font-size="12" font-weight="bold" fill="#dc2626">High I² (82%) — Studies Disagree</text>
  <!-- Pooled line -->
  <line x1="510" y1="65" x2="510" y2="230" stroke="#dc2626" stroke-width="2" stroke-dasharray="5,3"/>
  <text x="510" y="248" text-anchor="middle" font-size="10" fill="#dc2626">Pooled = -48.0</text>
  <!-- Study CIs - widely spread -->
  <line x1="430" y1="85" x2="510" y2="85" stroke="#ef4444" stroke-width="2"/>
  <rect x="466" y="80" width="8" height="10" fill="#ef4444" rx="1"/>
  <text x="398" y="89" font-size="10" fill="#374151">Study A</text>
  <line x1="500" y1="115" x2="605" y2="115" stroke="#ef4444" stroke-width="2"/>
  <rect x="548" y="110" width="8" height="10" fill="#ef4444" rx="1"/>
  <text x="398" y="119" font-size="10" fill="#374151">Study B</text>
  <line x1="440" y1="145" x2="530" y2="145" stroke="#ef4444" stroke-width="2"/>
  <rect x="481" y="140" width="8" height="10" fill="#ef4444" rx="1"/>
  <text x="398" y="149" font-size="10" fill="#374151">Study C</text>
  <line x1="475" y1="175" x2="580" y2="175" stroke="#ef4444" stroke-width="2"/>
  <rect x="523" y="170" width="8" height="10" fill="#ef4444" rx="1"/>
  <text x="398" y="179" font-size="10" fill="#374151">Study D</text>
  <line x1="410" y1="205" x2="490" y2="205" stroke="#ef4444" stroke-width="2"/>
  <rect x="446" y="200" width="8" height="10" fill="#ef4444" rx="1"/>
  <text x="398" y="209" font-size="10" fill="#374151">Study E</text>
  <!-- Annotation -->
  <text x="510" y="268" text-anchor="middle" font-size="10" fill="#6b7280">CIs are spread widely</text>
</svg>
</div>

```bio
# Heterogeneity assessment
let Q = sum(zip(weights, effects) |> map(|we| we[0] * (we[1] - pooled_effect) * (we[1] - pooled_effect)))
let df = len(studies) - 1
let I_squared = max(0, (Q - df) / Q) * 100

print("=== Heterogeneity ===")
print("Q statistic: " + str(round(Q, 2)) + " (df=" + str(df) + ")")
print("I-squared: " + str(round(I_squared, 1)) + "%")
```

## The Forest Plot

The forest plot is the signature visualization of meta-analysis. Each study is a row showing its point estimate (square, sized by weight) and confidence interval (horizontal line). The pooled estimate is a diamond at the bottom. A vertical line at the null (0 for mean differences, 1 for ratios) allows quick assessment of significance.

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="380" viewBox="0 0 680 380" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <!-- Title -->
  <text x="340" y="28" text-anchor="middle" font-size="14" font-weight="bold" fill="#1e293b">Forest Plot — PCSK9 Inhibitor Meta-Analysis</text>
  <!-- Axis area -->
  <line x1="280" y1="50" x2="280" y2="310" stroke="#dc2626" stroke-width="1.5" stroke-dasharray="6,3"/>
  <text x="280" y="340" text-anchor="middle" font-size="11" fill="#dc2626">No Effect (0)</text>
  <!-- X-axis labels -->
  <text x="160" y="355" text-anchor="middle" font-size="10" fill="#6b7280">-70</text>
  <text x="220" y="355" text-anchor="middle" font-size="10" fill="#6b7280">-60</text>
  <text x="280" y="355" text-anchor="middle" font-size="10" fill="#6b7280">0</text>
  <text x="340" y="355" text-anchor="middle" font-size="10" fill="#6b7280">-40</text>
  <text x="400" y="355" text-anchor="middle" font-size="10" fill="#6b7280">-30</text>
  <text x="490" y="370" text-anchor="middle" font-size="11" fill="#6b7280">Mean LDL Reduction (mg/dL)</text>
  <!-- Column headers -->
  <text x="60" y="55" font-size="11" font-weight="bold" fill="#374151">Study</text>
  <text x="540" y="55" font-size="11" font-weight="bold" fill="#374151">Weight</text>
  <text x="610" y="55" font-size="11" font-weight="bold" fill="#374151">Effect</text>
  <!-- Study 1: Hoffmann 2019 -->
  <text x="10" y="90" font-size="11" fill="#374151">Hoffmann 2019</text>
  <line x1="198" y1="87" x2="245" y2="87" stroke="#2563eb" stroke-width="2"/>
  <rect x="215" y="81" width="12" height="12" fill="#2563eb" rx="1"/>
  <text x="548" y="91" font-size="11" fill="#6b7280">24%</text>
  <text x="605" y="91" font-size="11" fill="#374151">-52.3</text>
  <!-- Study 2: Martinez 2020 -->
  <text x="10" y="130" font-size="11" fill="#374151">Martinez 2020</text>
  <line x1="182" y1="127" x2="260" y2="127" stroke="#2563eb" stroke-width="2"/>
  <rect x="218" y="122" width="9" height="9" fill="#2563eb" rx="1"/>
  <text x="548" y="131" font-size="11" fill="#6b7280">14%</text>
  <text x="605" y="131" font-size="11" fill="#374151">-48.7</text>
  <!-- Study 3: Chen 2020 -->
  <text x="10" y="170" font-size="11" fill="#374151">Chen 2020</text>
  <line x1="190" y1="167" x2="238" y2="167" stroke="#2563eb" stroke-width="2"/>
  <rect x="206" y="160" width="14" height="14" fill="#2563eb" rx="1"/>
  <text x="548" y="171" font-size="11" fill="#6b7280">28%</text>
  <text x="605" y="171" font-size="11" fill="#374151">-55.1</text>
  <!-- Study 4: Kumar 2021 -->
  <text x="10" y="210" font-size="11" fill="#374151">Kumar 2021</text>
  <line x1="196" y1="207" x2="250" y2="207" stroke="#2563eb" stroke-width="2"/>
  <rect x="217" y="201" width="11" height="11" fill="#2563eb" rx="1"/>
  <text x="548" y="211" font-size="11" fill="#6b7280">18%</text>
  <text x="605" y="211" font-size="11" fill="#374151">-50.2</text>
  <!-- Study 5: Larsson 2021 -->
  <text x="10" y="250" font-size="11" fill="#374151">Larsson 2021</text>
  <line x1="192" y1="247" x2="244" y2="247" stroke="#2563eb" stroke-width="2"/>
  <rect x="212" y="241" width="10" height="10" fill="#2563eb" rx="1"/>
  <text x="548" y="251" font-size="11" fill="#6b7280">16%</text>
  <text x="605" y="251" font-size="11" fill="#374151">-53.8</text>
  <!-- Separator -->
  <line x1="10" y1="272" x2="520" y2="272" stroke="#9ca3af" stroke-width="0.5" stroke-dasharray="3,3"/>
  <!-- Pooled estimate diamond -->
  <text x="10" y="300" font-size="11" font-weight="bold" fill="#374151">Pooled (RE)</text>
  <polygon points="210,297 218,290 226,297 218,304" fill="#16a34a" stroke="#16a34a" stroke-width="1"/>
  <text x="548" y="301" font-size="11" font-weight="bold" fill="#6b7280">100%</text>
  <text x="605" y="301" font-size="11" font-weight="bold" fill="#374151">-52.5</text>
  <!-- Legend -->
  <rect x="440" y="46" width="8" height="8" fill="#2563eb" rx="1"/>
  <text x="452" y="54" font-size="10" fill="#6b7280">Study estimate</text>
  <polygon points="440,72 444,66 448,72 444,78" fill="#16a34a"/>
  <text x="452" y="74" font-size="10" fill="#6b7280">Pooled estimate</text>
  <!-- Favors labels -->
  <text x="180" y="330" text-anchor="middle" font-size="10" fill="#16a34a">Favors Drug</text>
  <text x="380" y="330" text-anchor="middle" font-size="10" fill="#6b7280">Favors Placebo</text>
</svg>
</div>

```bio
let studies = ["Trial A (2019)", "Trial B (2020)", "Trial C (2021)",
               "Chen (2020)", "Kumar (2021)", "Pooled"]
let effects = [-52.3, -48.7, -55.1, -50.2, -53.8, -52.5]
let ci_lower = [-60.3, -59.1, -62.5, -57.4, -61.2, -55.8]
let ci_upper = [-44.3, -38.3, -47.7, -43.0, -46.4, -49.2]
let weights = [24, 14, 28, 18, 16, 100]

let forest_tbl = zip(studies, effects, ci_lower, ci_upper, weights) |> map(|r| {
  study: r[0], estimate: r[1], ci_lower: r[2], ci_upper: r[3], weight: r[4]
}) |> to_table()

forest_plot(forest_tbl,
  {null_value: 0,
  title: "PCSK9 Inhibitor — LDL Reduction (mg/dL)",
  xlabel: "Mean LDL Reduction (95% CI)"})
```

**Reading the forest plot:**
- If a study's CI does not cross the null line, that study alone is significant.
- If the pooled diamond does not cross the null, the combined evidence is significant.
- Study squares vary in size — larger squares mean more weight (more precise studies).
- The diamond width shows the CI of the pooled estimate.

## Publication Bias and the Funnel Plot

Publication bias occurs when studies with significant results are more likely to be published than studies with null results. This biases meta-analyses toward overestimating effects.

The funnel plot detects this. It plots each study's effect size (x-axis) against its precision (y-axis, typically 1/SE or sample size). In the absence of bias, the plot should look like an inverted funnel — symmetric around the pooled estimate, with more scatter at the bottom (less precise studies).

Asymmetry suggests bias. If small studies with negative or null results are missing (they were not published), the funnel will be asymmetric — missing studies from the lower-left.

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="340" viewBox="0 0 680 340" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <!-- Title -->
  <text x="340" y="25" text-anchor="middle" font-size="14" font-weight="bold" fill="#1e293b">Funnel Plots — Detecting Publication Bias</text>
  <!-- Left panel: Symmetric (no bias) -->
  <text x="170" y="48" text-anchor="middle" font-size="12" font-weight="bold" fill="#16a34a">No Bias (Symmetric)</text>
  <!-- Axes -->
  <line x1="50" y1="60" x2="50" y2="280" stroke="#374151" stroke-width="1.5"/>
  <line x1="50" y1="280" x2="310" y2="280" stroke="#374151" stroke-width="1.5"/>
  <text x="180" y="305" text-anchor="middle" font-size="10" fill="#6b7280">Effect Size</text>
  <text x="22" y="170" text-anchor="middle" font-size="10" fill="#6b7280" transform="rotate(-90 22 170)">Standard Error</text>
  <!-- SE axis labels (inverted: small SE at top) -->
  <text x="42" y="72" text-anchor="end" font-size="9" fill="#9ca3af">2</text>
  <text x="42" y="170" text-anchor="end" font-size="9" fill="#9ca3af">4</text>
  <text x="42" y="270" text-anchor="end" font-size="9" fill="#9ca3af">6</text>
  <!-- Funnel outline -->
  <line x1="180" y1="65" x2="100" y2="275" stroke="#93c5fd" stroke-width="1" stroke-dasharray="4,3"/>
  <line x1="180" y1="65" x2="260" y2="275" stroke="#93c5fd" stroke-width="1" stroke-dasharray="4,3"/>
  <!-- Pooled estimate line -->
  <line x1="180" y1="60" x2="180" y2="280" stroke="#9ca3af" stroke-width="1" stroke-dasharray="3,3"/>
  <!-- Dots: symmetric scatter -->
  <circle cx="178" cy="80" r="4" fill="#2563eb" opacity="0.8"/>
  <circle cx="184" cy="90" r="4" fill="#2563eb" opacity="0.8"/>
  <circle cx="170" cy="120" r="4" fill="#2563eb" opacity="0.8"/>
  <circle cx="192" cy="115" r="4" fill="#2563eb" opacity="0.8"/>
  <circle cx="160" cy="155" r="4" fill="#2563eb" opacity="0.8"/>
  <circle cx="200" cy="150" r="4" fill="#2563eb" opacity="0.8"/>
  <circle cx="175" cy="145" r="4" fill="#2563eb" opacity="0.8"/>
  <circle cx="145" cy="190" r="4" fill="#2563eb" opacity="0.8"/>
  <circle cx="210" cy="185" r="4" fill="#2563eb" opacity="0.8"/>
  <circle cx="165" cy="200" r="4" fill="#2563eb" opacity="0.8"/>
  <circle cx="195" cy="195" r="4" fill="#2563eb" opacity="0.8"/>
  <circle cx="130" cy="230" r="4" fill="#2563eb" opacity="0.8"/>
  <circle cx="225" cy="235" r="4" fill="#2563eb" opacity="0.8"/>
  <circle cx="155" cy="245" r="4" fill="#2563eb" opacity="0.8"/>
  <circle cx="205" cy="250" r="4" fill="#2563eb" opacity="0.8"/>
  <!-- Right panel: Asymmetric (bias) -->
  <text x="510" y="48" text-anchor="middle" font-size="12" font-weight="bold" fill="#dc2626">Publication Bias (Asymmetric)</text>
  <!-- Axes -->
  <line x1="380" y1="60" x2="380" y2="280" stroke="#374151" stroke-width="1.5"/>
  <line x1="380" y1="280" x2="640" y2="280" stroke="#374151" stroke-width="1.5"/>
  <text x="510" y="305" text-anchor="middle" font-size="10" fill="#6b7280">Effect Size</text>
  <!-- Funnel outline -->
  <line x1="510" y1="65" x2="430" y2="275" stroke="#93c5fd" stroke-width="1" stroke-dasharray="4,3"/>
  <line x1="510" y1="65" x2="590" y2="275" stroke="#93c5fd" stroke-width="1" stroke-dasharray="4,3"/>
  <!-- Pooled estimate line -->
  <line x1="510" y1="60" x2="510" y2="280" stroke="#9ca3af" stroke-width="1" stroke-dasharray="3,3"/>
  <!-- Dots: asymmetric - missing lower-left -->
  <circle cx="508" cy="80" r="4" fill="#2563eb" opacity="0.8"/>
  <circle cx="515" cy="88" r="4" fill="#2563eb" opacity="0.8"/>
  <circle cx="520" cy="118" r="4" fill="#2563eb" opacity="0.8"/>
  <circle cx="500" cy="125" r="4" fill="#2563eb" opacity="0.8"/>
  <circle cx="530" cy="150" r="4" fill="#2563eb" opacity="0.8"/>
  <circle cx="505" cy="148" r="4" fill="#2563eb" opacity="0.8"/>
  <circle cx="540" cy="185" r="4" fill="#2563eb" opacity="0.8"/>
  <circle cx="525" cy="195" r="4" fill="#2563eb" opacity="0.8"/>
  <circle cx="555" cy="225" r="4" fill="#2563eb" opacity="0.8"/>
  <circle cx="545" cy="245" r="4" fill="#2563eb" opacity="0.8"/>
  <circle cx="515" cy="210" r="4" fill="#2563eb" opacity="0.8"/>
  <!-- Missing region annotation -->
  <text x="430" y="230" font-size="10" fill="#dc2626" font-style="italic">Missing</text>
  <text x="430" y="243" font-size="10" fill="#dc2626" font-style="italic">studies</text>
  <path d="M 430 248 Q 445 265 465 260" fill="none" stroke="#dc2626" stroke-width="1" marker-end="url(#arrowRed26)"/>
  <defs>
    <marker id="arrowRed26" markerWidth="6" markerHeight="6" refX="5" refY="3" orient="auto">
      <path d="M0,0 L6,3 L0,6 Z" fill="#dc2626"/>
    </marker>
  </defs>
  <!-- Border between panels -->
  <line x1="340" y1="40" x2="340" y2="300" stroke="#e5e7eb" stroke-width="1" stroke-dasharray="4,4"/>
</svg>
</div>

```bio
let effect_sizes = [-52.3, -48.7, -55.1, -50.2, -53.8]
let standard_errors = [4.1, 5.3, 3.8, 3.7, 3.9]

# Funnel plot: scatter of effect size vs SE
scatter(effect_sizes, standard_errors,
  {title: "Funnel Plot — Publication Bias Assessment",
  xlabel: "LDL Reduction (mg/dL)",
  ylabel: "Standard Error"})
```

> **Clinical relevance:** Publication bias is a serious concern in pharmaceutical research. A meta-analysis of published antidepressant trials found a pooled effect size of 0.37 (moderate). When unpublished trials obtained through FDA records were included, the effect dropped to 0.15 (small). Publication bias had inflated the apparent efficacy by more than double.

## When Meta-Analysis Is Inappropriate

Meta-analysis is not appropriate when:

1. **Studies measure fundamentally different things**: Combining a study of aspirin with a study of statins because both are "cardiovascular interventions" is meaningless.
2. **Heterogeneity is too high** (I^2 > 75%): If studies genuinely disagree, pooling them hides important differences. Investigate subgroups instead.
3. **Studies are not independent**: If three papers report on overlapping patient cohorts, they are not independent studies.
4. **Publication bias is severe**: A pooled estimate from biased studies is itself biased — garbage in, garbage out.
5. **Too few studies**: Meta-analysis of 2 studies with opposite results tells you very little. At minimum, 3-5 studies are needed.

> **Common pitfall:** "Combining apples and oranges" is the classic criticism. Meta-analysis is appropriate when studies address the same question with similar methods. If studies differ fundamentally, no amount of statistical sophistication makes the pooled result meaningful.

## Meta-Analysis in BioLang — Complete Pipeline

```bio

# ============================================
# Five studies of PCSK9 inhibitor effect on LDL
# ============================================

let studies = ["Hoffmann 2019", "Martinez 2020", "Chen 2020",
               "Kumar 2021", "Larsson 2021"]
let effects = [-52.3, -48.7, -55.1, -50.2, -53.8]
let se = [4.1, 5.3, 3.8, 3.7, 3.9]
let n_patients = [250, 180, 320, 290, 260]

# ============================================
# 1. Fixed-effects meta-analysis
# ============================================
let weights_fe = se |> map(|s| 1.0 / (s * s))
let total_w = sum(weights_fe)
let pooled_fe = sum(zip(weights_fe, effects) |> map(|p| p[0] * p[1])) / total_w
let se_fe = 1.0 / sqrt(total_w)

print("=== Fixed-Effects ===")
print("Pooled: " + str(round(pooled_fe, 2)) + " [" +
  str(round(pooled_fe - 1.96 * se_fe, 2)) + ", " +
  str(round(pooled_fe + 1.96 * se_fe, 2)) + "]")

# ============================================
# 2. Heterogeneity
# ============================================
let Q = sum(zip(weights_fe, effects) |>
  map(|p| p[0] * (p[1] - pooled_fe) * (p[1] - pooled_fe)))
let df = len(studies) - 1
let I_sq = max(0, (Q - df) / Q) * 100

print("\n=== Heterogeneity ===")
print("Q = " + str(round(Q, 2)) + ", df = " + str(df))
print("I-squared = " + str(round(I_sq, 1)) + "%")

# Estimate tau-squared (between-study variance)
let C = total_w - sum(weights_fe |> map(|w| w * w)) / total_w
let tau_sq = max(0, (Q - df) / C)
print("tau-squared = " + str(round(tau_sq, 2)))

# ============================================
# 3. Random-effects meta-analysis
# ============================================
let weights_re = se |> map(|s| 1.0 / (s * s + tau_sq))
let total_w_re = sum(weights_re)
let pooled_re = sum(zip(weights_re, effects) |> map(|p| p[0] * p[1])) / total_w_re
let se_re = 1.0 / sqrt(total_w_re)

print("\n=== Random-Effects ===")
print("Pooled: " + str(round(pooled_re, 2)) + " [" +
  str(round(pooled_re - 1.96 * se_re, 2)) + ", " +
  str(round(pooled_re + 1.96 * se_re, 2)) + "]")

# ============================================
# 4. Forest plot
# ============================================
let all_ci_lo = range(0, len(effects)) |> map(|i| effects[i] - 1.96 * se[i])
let all_ci_hi = range(0, len(effects)) |> map(|i| effects[i] + 1.96 * se[i])
let all_w_pct = weights_re |> map(|w| round(w / total_w_re * 100, 1))

# Build forest plot table
let rows = range(0, len(studies)) |> map(|i| {
  study: studies[i], estimate: effects[i],
  ci_lower: all_ci_lo[i], ci_upper: all_ci_hi[i], weight: all_w_pct[i]
})
let pooled_row = [{study: "Pooled (RE)", estimate: pooled_re,
  ci_lower: pooled_re - 1.96 * se_re,
  ci_upper: pooled_re + 1.96 * se_re, weight: 100}]
let forest_tbl = concat(rows, pooled_row) |> to_table()

forest_plot(forest_tbl,
  {null_value: 0,
  title: "PCSK9 Inhibitor Meta-Analysis — LDL Reduction",
  xlabel: "Mean LDL Reduction, mg/dL (95% CI)"})

# ============================================
# 5. Funnel plot
# ============================================
# Funnel plot: scatter of effect vs SE
scatter(effects, se,
  {title: "Funnel Plot — Publication Bias",
  xlabel: "LDL Reduction (mg/dL)",
  ylabel: "Standard Error"})

# ============================================
# 6. Study-level summary
# ============================================
print("\n=== Study Summary ===")
print("Study                | Effect  | SE   | Weight(RE)")
print("---------------------|---------|------|----------")
for i in 0..len(studies) {
  let w_pct = round(weights_re[i] / total_w_re * 100, 1)
  print(studies[i] + " | " + str(effects[i]) + " | " +
    str(se[i]) + " | " + str(w_pct) + "%")
}

# ============================================
# 7. Interpretation
# ============================================
print("\n=== Interpretation ===")
if I_sq < 25 {
  print("Heterogeneity is low (I^2 = " + str(round(I_sq, 1)) + "%).")
  print("Studies are consistent. Fixed and random effects agree.")
} else if I_sq < 50 {
  print("Moderate heterogeneity (I^2 = " + str(round(I_sq, 1)) + "%).")
  print("Random-effects model is preferred.")
} else {
  print("Substantial heterogeneity (I^2 = " + str(round(I_sq, 1)) + "%).")
  print("Investigate sources of heterogeneity before trusting the pooled estimate.")
}
```

**Python:**

```python
import numpy as np
import matplotlib.pyplot as plt

effects = np.array([-52.3, -48.7, -55.1, -50.2, -53.8])
se = np.array([4.1, 5.3, 3.8, 3.7, 3.9])

# Fixed effects
w = 1 / se**2
pooled_fe = np.average(effects, weights=w)
se_fe = 1 / np.sqrt(w.sum())

# Heterogeneity
Q = np.sum(w * (effects - pooled_fe)**2)
df = len(effects) - 1
I2 = max(0, (Q - df) / Q) * 100

# Random effects (DerSimonian-Laird)
C = w.sum() - (w**2).sum() / w.sum()
tau2 = max(0, (Q - df) / C)
w_re = 1 / (se**2 + tau2)
pooled_re = np.average(effects, weights=w_re)
se_re = 1 / np.sqrt(w_re.sum())

print(f"Fixed: {pooled_fe:.1f} [{pooled_fe-1.96*se_fe:.1f}, {pooled_fe+1.96*se_fe:.1f}]")
print(f"Random: {pooled_re:.1f} [{pooled_re-1.96*se_re:.1f}, {pooled_re+1.96*se_re:.1f}]")
print(f"I²: {I2:.1f}%")
```

**R:**

```r
library(meta)

m <- metagen(TE = c(-52.3, -48.7, -55.1, -50.2, -53.8),
             seTE = c(4.1, 5.3, 3.8, 3.7, 3.9),
             studlab = c("Hoffmann", "Martinez", "Chen", "Kumar", "Larsson"),
             sm = "MD")
summary(m)
forest(m)
funnel(m)

# Alternative with metafor
library(metafor)
res <- rma(yi = effects, sei = se, method = "DL")
summary(res)
forest(res)
funnel(res)
```

## Exercises

1. **Fixed vs random.** Given the five PCSK9 studies above, compute both fixed-effects and random-effects pooled estimates. How different are they? Based on I-squared, which model is more appropriate?

```bio
# Your code: both models, compare, interpret I-squared
```

2. **Adding a contradictory study.** A sixth study (Nakamura 2022, N=150) finds a much smaller effect: -30.5 mg/dL, SE=6.2. Add it to the meta-analysis. How do the pooled estimate, CI width, and I-squared change? Create the updated forest plot.

```bio
# Your code: add study, re-run meta-analysis, compare
```

3. **Publication bias simulation.** Simulate 20 studies: true effect = -50, SE drawn from Uniform(3, 8). Then "suppress" all studies with p > 0.05 (simulating publication bias). Run meta-analysis on the remaining studies. Is the pooled estimate biased? Check with a funnel plot.

```bio
# Your code: simulate, suppress, meta-analyze, funnel plot
```

4. **Subgroup analysis.** The five studies come from different continents (Europe, USA, Asia). Compute pooled estimates separately for Western (Trials A, B) and Asian (Trials C, D, E) studies. Is there a meaningful difference?

```bio
# Your code: subgroup meta-analysis, compare pooled estimates
```

5. **Hazard ratio meta-analysis.** Five survival studies report hazard ratios (log scale) for a new chemotherapy vs standard-of-care. Combine them using random effects and create a forest plot.

```bio
let log_hr = [-0.33, -0.22, -0.41, -0.28, -0.35]
let se_log_hr = [0.12, 0.15, 0.10, 0.11, 0.13]
# Your code: meta-analysis on log(HR), forest plot, back-transform to HR
```

## Key Takeaways

- Meta-analysis formally combines results across studies to produce a more precise pooled estimate, weighted by each study's precision.
- Fixed-effects models assume one true effect across studies; random-effects models allow the true effect to vary. Random effects is almost always more appropriate in biomedical research.
- Cochran's Q tests for heterogeneity; I-squared quantifies its magnitude. I-squared above 50% warrants investigation before pooling.
- The forest plot is the standard meta-analysis visualization: study estimates with CIs arranged vertically, pooled estimate as a diamond.
- The funnel plot assesses publication bias: asymmetry suggests that small negative studies are missing from the literature.
- Meta-analysis is inappropriate when studies measure different things, heterogeneity is extreme, studies are not independent, or publication bias is severe.
- Meta-analysis sits at the top of the evidence hierarchy because it synthesizes all available evidence — but it is only as good as the studies it includes.

## What's Next

We have learned to analyze data, but can we trust our analysis? Tomorrow we confront the reproducibility crisis head-on: how to structure your statistical analysis so that it can be re-run perfectly by anyone, at any time. Random seeds, modular scripts, parameter files, and version tracking — the practices that separate publishable science from a pile of scattered scripts.
