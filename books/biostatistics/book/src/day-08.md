# Day 8: Comparing Two Groups — The t-Test

## The Problem

Dr. Sofia Reyes is a cancer biologist studying BRCA1 expression in breast tissue. She has RNA-seq data from 12 tumor samples and 12 matched normal samples from the same patients. The mean BRCA1 expression in tumors is 4.2 log2-CPM versus 6.8 log2-CPM in normals — a 2.6-fold reduction. But with only 12 samples per group and considerable biological variability, can she confidently claim BRCA1 is downregulated in tumors?

She cannot use a z-test because the population standard deviation is unknown — she must estimate it from the data itself. She needs the **t-test**, the most widely used statistical test in biomedical research. But which version? Her samples are paired (tumor and normal from the same patient), which adds another consideration. And before running any test, she should verify that the data meet the test's assumptions.

This chapter covers the t-test in all its forms: independent, Welch's, paired, and one-sample. You will learn when each is appropriate, how to check assumptions, and how to quantify the magnitude of differences with Cohen's d.

## What Is the t-Test?

The t-test asks: "Is the difference between two group means larger than what we would expect from random sampling variation alone?"

Think of it this way: you have two piles of measurements. The t-test weighs how far apart the piles' centers are, relative to how spread out each pile is. If the piles are far apart and tight, the difference is convincing. If they overlap substantially, it is not.

**The t-statistic** = (difference in means) / (standard error of the difference)

A larger t-statistic means more evidence of a real difference.

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="320" viewBox="0 0 680 320" xmlns="http://www.w3.org/2000/svg" style="background: #fafbfc; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="28" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">Easy vs Hard to Distinguish: Overlap Determines Significance</text>
  <!-- LEFT PANEL: Large effect (easy) -->
  <text x="175" y="55" text-anchor="middle" font-size="13" fill="#16a34a" font-weight="bold">Large Effect (Easy)</text>
  <!-- Left group A -->
  <path d="M 40,220 C 55,218 65,210 75,196 C 85,178 95,155 105,135 C 115,118 125,106 135,100 C 145,98 148,98 150,98 C 155,100 165,106 175,120 C 185,138 195,160 205,182 C 215,200 225,212 240,218 L 40,220 Z" fill="#93c5fd" opacity="0.5" stroke="#2563eb" stroke-width="2"/>
  <!-- Left group B -->
  <path d="M 200,220 C 215,218 225,210 235,196 C 245,178 255,155 265,135 C 275,118 285,106 295,100 C 305,98 308,98 310,98 C 315,100 325,106 335,120 L 335,220 Z" fill="#bbf7d0" opacity="0.5" stroke="#16a34a" stroke-width="2"/>
  <text x="140" y="88" text-anchor="middle" font-size="11" fill="#2563eb">Group A</text>
  <text x="280" y="88" text-anchor="middle" font-size="11" fill="#16a34a">Group B</text>
  <!-- Axis for left -->
  <line x1="30" y1="220" x2="340" y2="220" stroke="#374151" stroke-width="1"/>
  <!-- Arrow showing big gap -->
  <line x1="150" y1="240" x2="300" y2="240" stroke="#374151" stroke-width="1.5" marker-end="url(#arrow)"/>
  <line x1="300" y1="240" x2="150" y2="240" stroke="#374151" stroke-width="1.5" marker-end="url(#arrow)"/>
  <text x="225" y="260" text-anchor="middle" font-size="11" fill="#374151" font-weight="bold">Large d</text>
  <text x="175" y="280" text-anchor="middle" font-size="10" fill="#6b7280">Small overlap, small p-value</text>
  <!-- RIGHT PANEL: Small effect (hard) -->
  <text x="510" y="55" text-anchor="middle" font-size="13" fill="#dc2626" font-weight="bold">Small Effect (Hard)</text>
  <!-- Right group A -->
  <path d="M 370,220 C 385,218 400,208 420,190 C 440,165 455,138 470,115 C 485,100 495,92 505,90 C 510,89 512,89 515,90 C 525,94 535,105 545,122 C 555,142 565,168 575,192 C 585,208 595,216 610,219 L 370,220 Z" fill="#93c5fd" opacity="0.5" stroke="#2563eb" stroke-width="2"/>
  <!-- Right group B (heavily overlapping, shifted just a bit) -->
  <path d="M 400,220 C 415,218 430,208 450,190 C 470,165 485,138 500,115 C 515,100 525,92 535,90 C 540,89 542,89 545,90 C 555,94 565,105 575,122 C 585,142 595,168 605,192 C 615,208 625,216 640,219 L 400,220 Z" fill="#bbf7d0" opacity="0.5" stroke="#16a34a" stroke-width="2"/>
  <text x="490" y="82" text-anchor="middle" font-size="11" fill="#2563eb">A</text>
  <text x="530" y="82" text-anchor="middle" font-size="11" fill="#16a34a">B</text>
  <!-- Axis for right -->
  <line x1="360" y1="220" x2="650" y2="220" stroke="#374151" stroke-width="1"/>
  <!-- Arrow showing small gap -->
  <line x1="505" y1="240" x2="535" y2="240" stroke="#374151" stroke-width="1.5" marker-end="url(#arrow)"/>
  <line x1="535" y1="240" x2="505" y2="240" stroke="#374151" stroke-width="1.5" marker-end="url(#arrow)"/>
  <text x="520" y="260" text-anchor="middle" font-size="11" fill="#374151" font-weight="bold">Small d</text>
  <text x="510" y="280" text-anchor="middle" font-size="10" fill="#6b7280">Large overlap, large p-value</text>
  <!-- Arrow marker definition -->
  <defs>
    <marker id="arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto">
      <path d="M 0 0 L 10 5 L 0 10 z" fill="#374151"/>
    </marker>
  </defs>
  <text x="340" y="305" text-anchor="middle" font-size="11" fill="#6b7280">Cohen's d = difference in means / pooled standard deviation</text>
</svg>
</div>

## The Four Flavors of t-Test

| Test | When to Use | Formula |
|---|---|---|
| **One-sample** | Compare sample mean to a known value | t = (x-bar - mu0) / (s / sqrt(n)) |
| **Independent two-sample** | Compare means of two unrelated groups | t = (x-bar1 - x-bar2) / (s_p x sqrt(1/n1 + 1/n2)) |
| **Welch's** | Two unrelated groups, unequal variances | t = (x-bar1 - x-bar2) / sqrt(s1^2/n1 + s2^2/n2) |
| **Paired** | Matched or before/after measurements | t = d-bar / (s_d / sqrt(n)) |

## Independent Two-Sample t-Test

### Assumptions

1. **Independence**: Observations within and between groups are independent
2. **Normality**: Data in each group are approximately normally distributed
3. **Equal variances**: Both groups have similar spread (homoscedasticity)

### The Pooled Standard Error

When variances are assumed equal, we pool them for a better estimate:

**s_p = sqrt(((n1-1)s1^2 + (n2-1)s2^2) / (n1 + n2 - 2))**

Degrees of freedom: **df = n1 + n2 - 2**

## Welch's t-Test: The Safer Default

Welch's t-test does not assume equal variances. It uses each group's own variance estimate and adjusts the degrees of freedom downward with the Welch-Satterthwaite equation.

> **Key insight:** Welch's t-test is almost always the better default choice. It performs nearly as well as the pooled t-test when variances ARE equal, and much better when they are not. Most modern statistical software (including R's `t.test()`) uses Welch's version by default.

## Paired t-Test: Matched Samples

When observations are naturally paired — tumor/normal from the same patient, before/after treatment on the same subject — the paired t-test is far more powerful because it controls for inter-subject variability.

The trick: compute the **difference** for each pair, then perform a one-sample t-test on the differences:

**t = d-bar / (s_d / sqrt(n))**

Where d-bar is the mean of the paired differences and s_d is their standard deviation.

| Design | Pairing | Correct Test |
|---|---|---|
| Tumor vs normal from same patient | Paired | Paired t-test |
| Drug vs placebo in different patients | Independent | Welch's t-test |
| Before vs after treatment, same patients | Paired | Paired t-test |
| Wild-type vs knockout mice | Independent | Welch's t-test |
| Left eye vs right eye of same individuals | Paired | Paired t-test |

<div style="text-align: center; margin: 2em 0;">
<svg width="650" height="380" viewBox="0 0 650 380" xmlns="http://www.w3.org/2000/svg" style="background: #fafbfc; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="325" y="28" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">Paired Design: Before/After Connected by Patient</text>
  <text x="325" y="46" text-anchor="middle" font-size="12" fill="#6b7280">Each arrow shows one patient's change -- every patient improved</text>
  <!-- Column labels -->
  <text x="180" y="75" text-anchor="middle" font-size="13" fill="#dc2626" font-weight="bold">Before Treatment</text>
  <text x="470" y="75" text-anchor="middle" font-size="13" fill="#16a34a" font-weight="bold">After Treatment</text>
  <!-- Y axis scale reference -->
  <text x="60" y="105" text-anchor="end" font-size="10" fill="#9ca3af">400+</text>
  <text x="60" y="175" text-anchor="end" font-size="10" fill="#9ca3af">300</text>
  <text x="60" y="245" text-anchor="end" font-size="10" fill="#9ca3af">200</text>
  <text x="60" y="315" text-anchor="end" font-size="10" fill="#9ca3af">100</text>
  <text x="325" y="365" text-anchor="middle" font-size="11" fill="#6b7280">Tumor volume (mm^3)</text>
  <!-- Patient 1: 245 -> 180 -->
  <circle cx="180" cy="220" r="7" fill="#ef4444" opacity="0.8"/>
  <circle cx="470" cy="268" r="7" fill="#22c55e" opacity="0.8"/>
  <line x1="187" y1="222" x2="463" y2="266" stroke="#6b7280" stroke-width="1.5" stroke-dasharray="4,2"/>
  <text x="570" y="246" font-size="10" fill="#374151">-65</text>
  <!-- Patient 2: 312 -> 245 -->
  <circle cx="180" cy="170" r="7" fill="#ef4444" opacity="0.8"/>
  <circle cx="470" cy="220" r="7" fill="#22c55e" opacity="0.8"/>
  <line x1="187" y1="172" x2="463" y2="218" stroke="#6b7280" stroke-width="1.5" stroke-dasharray="4,2"/>
  <text x="570" y="198" font-size="10" fill="#374151">-67</text>
  <!-- Patient 3: 198 -> 165 -->
  <circle cx="180" cy="255" r="7" fill="#ef4444" opacity="0.8"/>
  <circle cx="470" cy="279" r="7" fill="#22c55e" opacity="0.8"/>
  <line x1="187" y1="257" x2="463" y2="278" stroke="#6b7280" stroke-width="1.5" stroke-dasharray="4,2"/>
  <text x="570" y="270" font-size="10" fill="#374151">-33</text>
  <!-- Patient 4: 367 -> 298 -->
  <circle cx="180" cy="129" r="7" fill="#ef4444" opacity="0.8"/>
  <circle cx="470" cy="180" r="7" fill="#22c55e" opacity="0.8"/>
  <line x1="187" y1="132" x2="463" y2="178" stroke="#6b7280" stroke-width="1.5" stroke-dasharray="4,2"/>
  <text x="570" y="158" font-size="10" fill="#374151">-69</text>
  <!-- Patient 5: 421 -> 350 -->
  <circle cx="180" cy="90" r="7" fill="#ef4444" opacity="0.8"/>
  <circle cx="470" cy="143" r="7" fill="#22c55e" opacity="0.8"/>
  <line x1="187" y1="93" x2="463" y2="141" stroke="#6b7280" stroke-width="1.5" stroke-dasharray="4,2"/>
  <text x="570" y="120" font-size="10" fill="#374151">-71</text>
  <!-- Patient 6: 289 -> 220 -->
  <circle cx="180" cy="187" r="7" fill="#ef4444" opacity="0.8"/>
  <circle cx="470" cy="238" r="7" fill="#22c55e" opacity="0.8"/>
  <line x1="187" y1="190" x2="463" y2="236" stroke="#6b7280" stroke-width="1.5" stroke-dasharray="4,2"/>
  <text x="570" y="216" font-size="10" fill="#374151">-69</text>
  <!-- Difference column label -->
  <text x="570" y="80" text-anchor="middle" font-size="11" fill="#7c3aed" font-weight="bold">Diff</text>
  <!-- Summary -->
  <text x="325" y="345" text-anchor="middle" font-size="12" fill="#374151">Paired test analyzes the <tspan font-weight="bold">differences</tspan>, removing between-patient variability</text>
</svg>
</div>

> **Common pitfall:** Using an independent t-test on paired data wastes statistical power. If you have natural pairs, always use the paired test. Conversely, using a paired test on unpaired data gives wrong results.

<div style="text-align: center; margin: 2em 0;">
<svg width="650" height="360" viewBox="0 0 650 360" xmlns="http://www.w3.org/2000/svg" style="background: #fafbfc; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="325" y="28" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">Which t-Test? Decision Flowchart</text>
  <!-- Start node -->
  <rect x="225" y="50" width="200" height="40" rx="20" fill="#dbeafe" stroke="#2563eb" stroke-width="2"/>
  <text x="325" y="75" text-anchor="middle" font-size="13" fill="#1e293b" font-weight="bold">Comparing 2 groups?</text>
  <!-- Arrow down -->
  <line x1="325" y1="90" x2="325" y2="115" stroke="#374151" stroke-width="1.5" marker-end="url(#arrowF)"/>
  <!-- Paired? -->
  <rect x="215" y="115" width="220" height="40" rx="8" fill="#f3f4f6" stroke="#6b7280" stroke-width="1.5"/>
  <text x="325" y="140" text-anchor="middle" font-size="12" fill="#1e293b">Are observations <tspan font-weight="bold">paired</tspan>?</text>
  <text x="325" y="153" text-anchor="middle" font-size="10" fill="#6b7280">(same subject, before/after, matched)</text>
  <!-- YES branch: paired -->
  <line x1="435" y1="135" x2="510" y2="135" stroke="#16a34a" stroke-width="1.5" marker-end="url(#arrowF)"/>
  <text x="465" y="128" font-size="11" fill="#16a34a" font-weight="bold">Yes</text>
  <rect x="510" y="115" width="120" height="40" rx="8" fill="#dcfce7" stroke="#16a34a" stroke-width="2"/>
  <text x="570" y="138" text-anchor="middle" font-size="13" fill="#16a34a" font-weight="bold">Paired t-test</text>
  <!-- NO branch: unpaired -->
  <line x1="325" y1="155" x2="325" y2="195" stroke="#dc2626" stroke-width="1.5" marker-end="url(#arrowF)"/>
  <text x="335" y="180" font-size="11" fill="#dc2626" font-weight="bold">No</text>
  <!-- Normal? -->
  <rect x="215" y="195" width="220" height="40" rx="8" fill="#f3f4f6" stroke="#6b7280" stroke-width="1.5"/>
  <text x="325" y="218" text-anchor="middle" font-size="12" fill="#1e293b">Data approximately <tspan font-weight="bold">normal</tspan>?</text>
  <!-- NO branch: non-parametric -->
  <line x1="215" y1="215" x2="110" y2="215" stroke="#dc2626" stroke-width="1.5" marker-end="url(#arrowF)"/>
  <text x="170" y="208" font-size="11" fill="#dc2626" font-weight="bold">No</text>
  <rect x="10" y="198" width="100" height="36" rx="8" fill="#fef3c7" stroke="#f59e0b" stroke-width="2"/>
  <text x="60" y="215" text-anchor="middle" font-size="11" fill="#92400e" font-weight="bold">Mann-Whitney</text>
  <text x="60" y="228" text-anchor="middle" font-size="9" fill="#92400e">(Day 9)</text>
  <!-- YES branch: equal variances? -->
  <line x1="325" y1="235" x2="325" y2="270" stroke="#16a34a" stroke-width="1.5" marker-end="url(#arrowF)"/>
  <text x="335" y="255" font-size="11" fill="#16a34a" font-weight="bold">Yes</text>
  <rect x="195" y="270" width="260" height="40" rx="8" fill="#f3f4f6" stroke="#6b7280" stroke-width="1.5"/>
  <text x="325" y="293" text-anchor="middle" font-size="12" fill="#1e293b">Variances approximately <tspan font-weight="bold">equal</tspan>?</text>
  <!-- YES: pooled -->
  <line x1="195" y1="290" x2="110" y2="290" stroke="#16a34a" stroke-width="1.5" marker-end="url(#arrowF)"/>
  <text x="160" y="283" font-size="11" fill="#16a34a" font-weight="bold">Yes</text>
  <rect x="10" y="272" width="100" height="36" rx="8" fill="#dcfce7" stroke="#16a34a" stroke-width="2"/>
  <text x="60" y="294" text-anchor="middle" font-size="11" fill="#16a34a" font-weight="bold">Pooled t-test</text>
  <!-- NO: Welch's -->
  <line x1="455" y1="290" x2="510" y2="290" stroke="#dc2626" stroke-width="1.5" marker-end="url(#arrowF)"/>
  <text x="475" y="283" font-size="11" fill="#dc2626" font-weight="bold">No</text>
  <rect x="510" y="272" width="120" height="36" rx="8" fill="#dbeafe" stroke="#2563eb" stroke-width="2"/>
  <text x="570" y="293" text-anchor="middle" font-size="13" fill="#2563eb" font-weight="bold">Welch's t-test</text>
  <!-- Recommendation -->
  <text x="570" y="325" text-anchor="middle" font-size="10" fill="#7c3aed" font-weight="bold">Recommended default</text>
  <rect x="515" y="315" width="110" height="16" rx="4" fill="none" stroke="#7c3aed" stroke-width="1" stroke-dasharray="3,2"/>
  <!-- Arrow defs -->
  <defs>
    <marker id="arrowF" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto">
      <path d="M 0 0 L 10 5 L 0 10 z" fill="#374151"/>
    </marker>
  </defs>
</svg>
</div>

## Checking Assumptions

### Normality: Shapiro-Wilk Test

The Shapiro-Wilk test checks whether data could have come from a normal distribution.

- H0: Data are normally distributed
- If p > 0.05, normality assumption is reasonable
- If p < 0.05, data are significantly non-normal

Also use **QQ plots**: if points fall along the diagonal line, data are approximately normal.

### Equal Variances: Levene's Test

Levene's test checks whether two groups have equal variances.

- H0: Variances are equal
- If p > 0.05, equal variance assumption is reasonable
- If p < 0.05, use Welch's t-test (or just always use Welch's)

## Cohen's d: Quantifying Effect Size

A p-value tells you *whether* a difference exists. Cohen's d tells you *how large* it is, in standard deviation units:

**d = (x-bar1 - x-bar2) / s_pooled**

| Cohen's d | Interpretation | Biological Example |
|---|---|---|
| 0.2 | Small | Subtle expression change |
| 0.5 | Medium | Moderate drug effect |
| 0.8 | Large | Strong phenotypic difference |
| > 1.2 | Very large | Knockout vs wild-type |

> **Key insight:** A large p-value with a large Cohen's d suggests you are underpowered — you may have a real effect but too few samples to detect it. A small p-value with a tiny Cohen's d suggests the effect, while real, may not be biologically meaningful.

## The t-Test in BioLang

### Independent Two-Sample t-Test: Gene Expression

```bio
# BRCA1 expression (log2-CPM) in tumor vs normal breast tissue
let tumor  = [3.8, 4.5, 4.1, 3.2, 4.8, 3.9, 4.3, 5.1, 3.6, 4.0, 4.7, 3.5]
let normal = [6.2, 7.1, 6.5, 7.4, 6.8, 7.0, 6.3, 7.2, 6.9, 6.6, 7.3, 6.1]

# Default: Welch's t-test (unequal variances)
let result = ttest(tumor, normal)
print("=== Welch's t-test: BRCA1 Tumor vs Normal ===")
print("t-statistic: {result.statistic:.4}")
print("p-value: {result.p_value:.2e}")
print("Degrees of freedom: {result.df:.1}")
print("Mean tumor: {mean(tumor):.2}, Mean normal: {mean(normal):.2}")
print("Difference: {mean(tumor) - mean(normal):.2} log2-CPM")

# Effect size (Cohen's d inline)
let d = (mean(tumor) - mean(normal)) / sqrt((variance(tumor) + variance(normal)) / 2.0)
print("Cohen's d: {d:.3}")

# Visualize
let bp_table = table({"Tumor": tumor, "Normal": normal})
boxplot(bp_table, {title: "BRCA1 Expression: Tumor vs Normal"})
```

### Checking Assumptions

```bio
let tumor  = [3.8, 4.5, 4.1, 3.2, 4.8, 3.9, 4.3, 5.1, 3.6, 4.0, 4.7, 3.5]
let normal = [6.2, 7.1, 6.5, 7.4, 6.8, 7.0, 6.3, 7.2, 6.9, 6.6, 7.3, 6.1]

# 1. Normality check: use QQ plots for visual assessment
# (no built-in Shapiro-Wilk; use QQ plots + summary stats)
let s_tumor = summary(tumor)
let s_normal = summary(normal)
print("Tumor summary:  {s_tumor}")
print("Normal summary: {s_normal}")

# 2. Equal variance check: compare variances from summary()
let var_ratio = variance(tumor) / variance(normal)
print("Variance ratio (tumor/normal): {var_ratio:.3}")
if var_ratio > 2.0 or var_ratio < 0.5 {
  print("Variances appear unequal -> use Welch's t-test (the default)")
} else {
  print("Variances appear similar -> pooled t-test is also valid")
}

# 3. QQ plots for visual normality assessment
qq_plot(tumor, {title: "QQ Plot: Tumor BRCA1 Expression"})
qq_plot(normal, {title: "QQ Plot: Normal BRCA1 Expression"})
```

### Paired t-Test: Before/After Treatment

```bio
# Tumor volume (mm^3) before and after 6 weeks of treatment
# Same 10 patients measured at both time points
let before = [245, 312, 198, 367, 289, 421, 156, 334, 278, 305]
let after  = [180, 245, 165, 298, 220, 350, 132, 270, 210, 248]

# Paired t-test: accounts for patient-to-patient variability
let result = ttest_paired(before, after)
print("=== Paired t-test: Tumor Volume Before vs After Treatment ===")
print("t-statistic: {result.statistic:.4}")
print("p-value: {result.p_value:.6}")

# Show the paired differences
let diffs = zip(before, after) |> map(|pair| pair[0] - pair[1])
print("Mean reduction: {mean(diffs):.1} mm^3")
print("Individual reductions: {diffs}")

# Compare: what if we wrongly used an independent t-test?
let wrong_result = ttest(before, after)
print("\nWrong (independent) t-test p-value: {wrong_result.p_value:.6}")
print("Correct (paired) t-test p-value: {result.p_value:.6}")
print("Paired test is more powerful because it removes inter-patient variability")

# Visualize paired differences
histogram(diffs, {title: "Distribution of Tumor Volume Reductions", x_label: "Reduction (mm^3)", bins: 8})
```

### One-Sample t-Test

```bio
# Is the GC content of our assembled genome different from the expected 41%?
let gc_per_contig = [40.2, 41.5, 39.8, 42.1, 40.7, 41.3, 39.5, 42.4,
                     40.1, 41.8, 40.5, 41.0, 39.9, 41.6, 40.3]

let result = ttest_one(gc_per_contig, 41.0)
print("=== One-sample t-test: GC Content vs Expected 41% ===")
print("Sample mean: {mean(gc_per_contig):.2}%")
print("t-statistic: {result.statistic:.4}")
print("p-value: {result.p_value:.4}")

if result.p_value > 0.05 {
  print("No significant deviation from expected GC content")
}
```

### Complete Workflow: Multiple Genes

```bio
# Test multiple genes at once
let genes = ["BRCA1", "TP53", "MYC", "GAPDH", "EGFR"]

let tumor_expr = [
  [3.8, 4.5, 4.1, 3.2, 4.8, 3.9, 4.3, 5.1, 3.6, 4.0, 4.7, 3.5],
  [2.1, 1.8, 2.5, 1.4, 2.2, 1.9, 2.0, 1.6, 2.3, 1.7, 2.4, 1.5],
  [9.2, 10.1, 8.8, 9.5, 10.3, 9.7, 8.6, 9.9, 10.5, 9.1, 9.8, 10.2],
  [8.1, 8.3, 7.9, 8.2, 8.0, 8.4, 7.8, 8.1, 8.3, 8.0, 8.2, 7.9],
  [7.5, 8.2, 7.8, 8.5, 7.1, 8.0, 7.6, 8.3, 7.9, 8.1, 7.4, 8.4]
]

let normal_expr = [
  [6.2, 7.1, 6.5, 7.4, 6.8, 7.0, 6.3, 7.2, 6.9, 6.6, 7.3, 6.1],
  [5.8, 6.2, 5.5, 6.0, 5.9, 6.3, 5.7, 6.1, 5.6, 6.4, 5.8, 6.0],
  [5.1, 5.4, 4.9, 5.3, 5.6, 5.0, 5.2, 5.5, 4.8, 5.7, 5.1, 5.4],
  [8.0, 8.2, 7.8, 8.3, 8.1, 8.0, 8.4, 7.9, 8.2, 8.1, 8.3, 8.0],
  [5.0, 5.3, 4.8, 5.1, 5.5, 4.9, 5.2, 5.4, 4.7, 5.6, 5.0, 5.3]
]

print("Gene       | t-stat | p-value    | Cohen's d | Interpretation")
print("-----------|--------|------------|-----------|---------------")

for i in 0..len(genes) {
  let result = ttest(tumor_expr[i], normal_expr[i])
  let d = (mean(tumor_expr[i]) - mean(normal_expr[i])) / sqrt((variance(tumor_expr[i]) + variance(normal_expr[i])) / 2.0)
  let interp = if abs(d) > 0.8 then "Large" else if abs(d) > 0.5 then "Medium" else "Small"
  print("{genes[i]:<10} | {result.statistic:>6.2} | {result.p_value:>10.2e} | {d:>9.3} | {interp}")
}
```

**Python:**

```python
from scipy import stats
import numpy as np

tumor  = [3.8, 4.5, 4.1, 3.2, 4.8, 3.9, 4.3, 5.1, 3.6, 4.0, 4.7, 3.5]
normal = [6.2, 7.1, 6.5, 7.4, 6.8, 7.0, 6.3, 7.2, 6.9, 6.6, 7.3, 6.1]

# Welch's t-test (default)
t, p = stats.ttest_ind(tumor, normal, equal_var=False)
print(f"Welch's t = {t:.4f}, p = {p:.2e}")

# Paired t-test
before = [245, 312, 198, 367, 289, 421, 156, 334, 278, 305]
after  = [180, 245, 165, 298, 220, 350, 132, 270, 210, 248]
t, p = stats.ttest_rel(before, after)
print(f"Paired t = {t:.4f}, p = {p:.6f}")

# Cohen's d (manual)
pooled_std = np.sqrt((np.std(tumor, ddof=1)**2 + np.std(normal, ddof=1)**2) / 2)
d = (np.mean(tumor) - np.mean(normal)) / pooled_std
print(f"Cohen's d = {d:.3f}")
```

**R:**

```r
tumor  <- c(3.8, 4.5, 4.1, 3.2, 4.8, 3.9, 4.3, 5.1, 3.6, 4.0, 4.7, 3.5)
normal <- c(6.2, 7.1, 6.5, 7.4, 6.8, 7.0, 6.3, 7.2, 6.9, 6.6, 7.3, 6.1)

# Welch's t-test (default in R)
t.test(tumor, normal)

# Paired t-test
before <- c(245, 312, 198, 367, 289, 421, 156, 334, 278, 305)
after  <- c(180, 245, 165, 298, 220, 350, 132, 270, 210, 248)
t.test(before, after, paired = TRUE)

# Cohen's d
library(effsize)
cohen.d(tumor, normal)
```

## Exercises

**Exercise 1: Choose the Right t-Test**

For each scenario, state which t-test variant is appropriate and why:

a) Comparing white blood cell counts between 20 patients with sepsis and 25 healthy volunteers
b) Measuring gene expression in liver biopsies taken before and after drug treatment (same 15 patients)
c) Testing whether mean read length from your sequencer matches the expected 150 bp

**Exercise 2: Full t-Test Workflow**

Hemoglobin levels (g/dL) in two groups:
- Anemia patients: [9.2, 8.8, 10.1, 9.5, 8.3, 9.7, 8.6, 9.0, 10.3, 8.9]
- Healthy controls: [13.5, 14.2, 12.8, 13.9, 14.5, 13.1, 14.0, 13.6, 12.9, 14.3]

```bio
let anemia  = [9.2, 8.8, 10.1, 9.5, 8.3, 9.7, 8.6, 9.0, 10.3, 8.9]
let healthy = [13.5, 14.2, 12.8, 13.9, 14.5, 13.1, 14.0, 13.6, 12.9, 14.3]

# TODO: 1. Check normality with qq_plot() on each group
# TODO: 2. Check equal variances by comparing variance() per group
# TODO: 3. Run the appropriate t-test with ttest()
# TODO: 4. Compute Cohen's d inline: (mean(a)-mean(b)) / sqrt((variance(a)+variance(b))/2)
# TODO: 5. Create a boxplot
# TODO: 6. Interpret results in a biological context
```

**Exercise 3: Paired vs Independent**

Run both a paired and independent t-test on the tumor volume data below. Compare the p-values and explain why they differ.

```bio
let before = [245, 312, 198, 367, 289, 421, 156, 334, 278, 305]
let after  = [180, 245, 165, 298, 220, 350, 132, 270, 210, 248]

# TODO: Run ttest_paired and ttest
# TODO: Which gives a smaller p-value? Why?
# TODO: What does the paired test "remove" that the independent test cannot?
```

**Exercise 4: When Assumptions Fail**

The following data are highly skewed (as often seen in cytokine measurements):

```bio
let treatment = [2.1, 1.8, 45.2, 3.5, 2.9, 1.2, 38.7, 4.1, 2.3, 1.5]
let control   = [0.8, 0.5, 0.9, 0.3, 1.1, 0.7, 0.4, 0.6, 1.0, 0.2]

# TODO: Test normality with qq_plot()
# TODO: Run the t-test with ttest() anyway — what does it say?
# TODO: Try log-transforming the data and re-testing
# TODO: Preview: tomorrow we'll learn non-parametric alternatives
```

## Key Takeaways

- The **t-test** compares two group means, accounting for variability and sample size
- **Welch's t-test** (unequal variances) should be the default — it is robust even when variances are equal
- **Paired t-tests** are more powerful when observations are naturally matched (same patient, same timepoint)
- Always **check assumptions**: Shapiro-Wilk for normality, Levene's for equal variances, QQ plots for visual inspection
- **Cohen's d** quantifies effect size independently of sample size: 0.2 = small, 0.5 = medium, 0.8 = large
- A significant t-test with a small Cohen's d may not be biologically meaningful
- A non-significant t-test with a large Cohen's d suggests you need more samples

## What's Next

What happens when your data violate the normality assumption? Cytokine levels, bacterial abundances, and many other biological measurements are wildly skewed. Tomorrow we introduce non-parametric tests — rank-based alternatives to the t-test that make no assumptions about the shape of your data distribution.
