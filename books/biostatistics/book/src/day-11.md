# Day 11: Categorical Data — Chi-Square and Fisher's Exact

## The Problem

Dr. Elena Vasquez is an epidemiologist studying the genetics of Alzheimer's disease. She genotypes a SNP near the APOE gene in 1,000 participants: 500 with Alzheimer's and 500 age-matched controls. Among Alzheimer's patients, 180 carry at least one copy of the risk allele. Among controls, 120 carry it. The numbers look different — 36% versus 24% — but these are proportions, not measurements on a continuous scale. She cannot compute a mean or standard deviation. She cannot run a t-test.

When your data are counts in categories — disease yes/no, genotype AA/AG/GG, response/non-response — you need tests designed for categorical data. The chi-square test and Fisher's exact test are the workhorses. They also underpin some of the most important calculations in genetics: Hardy-Weinberg equilibrium, odds ratios for case-control studies, and allelic association tests.

This chapter covers the full toolkit for analyzing categorical data, from contingency tables to effect size measures like odds ratios and Cramer's V.

## What Are Categorical Data?

Categorical variables place observations into discrete groups rather than measuring them on a continuous scale.

| Type | Examples | Key Property |
|---|---|---|
| **Nominal** | Blood type (A, B, AB, O), genotype (AA, AG, GG) | No natural ordering |
| **Ordinal** | Tumor grade (I, II, III, IV), pain scale (1-10) | Ordered but not equal intervals |
| **Binary** | Disease (yes/no), mutation (present/absent) | Two categories |

The fundamental data structure for categorical analysis is the **contingency table** (also called a cross-tabulation):

|  | Risk Allele | No Risk Allele | Total |
|---|---|---|---|
| **Alzheimer's** | 180 | 320 | 500 |
| **Control** | 120 | 380 | 500 |
| **Total** | 300 | 700 | 1000 |

<div style="text-align: center; margin: 2em 0;">
<svg width="660" height="340" viewBox="0 0 660 340" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="330" y="28" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">2x2 Contingency Table: Observed (Expected)</text>
  <!-- Column headers -->
  <text x="330" y="58" text-anchor="middle" font-size="13" font-weight="bold" fill="#2563eb">Risk Allele</text>
  <text x="490" y="58" text-anchor="middle" font-size="13" font-weight="bold" fill="#2563eb">No Risk Allele</text>
  <text x="610" y="58" text-anchor="middle" font-size="13" font-weight="bold" fill="#6b7280">Total</text>
  <!-- Row headers -->
  <text x="140" y="108" text-anchor="middle" font-size="13" font-weight="bold" fill="#dc2626">Alzheimer's</text>
  <text x="140" y="178" text-anchor="middle" font-size="13" font-weight="bold" fill="#16a34a">Control</text>
  <text x="140" y="248" text-anchor="middle" font-size="13" font-weight="bold" fill="#6b7280">Total</text>
  <!-- Grid lines -->
  <line x1="220" y1="40" x2="220" y2="270" stroke="#d1d5db" stroke-width="1"/>
  <line x1="410" y1="40" x2="410" y2="270" stroke="#d1d5db" stroke-width="1"/>
  <line x1="570" y1="40" x2="570" y2="270" stroke="#d1d5db" stroke-width="1"/>
  <line x1="80" y1="68" x2="650" y2="68" stroke="#d1d5db" stroke-width="1"/>
  <line x1="80" y1="138" x2="650" y2="138" stroke="#d1d5db" stroke-width="1"/>
  <line x1="80" y1="208" x2="650" y2="208" stroke="#d1d5db" stroke-width="1"/>
  <!-- Cell values: Observed -->
  <text x="330" y="100" text-anchor="middle" font-size="18" font-weight="bold" fill="#1e293b">180</text>
  <text x="330" y="120" text-anchor="middle" font-size="12" fill="#9ca3af">(150.0)</text>
  <text x="490" y="100" text-anchor="middle" font-size="18" font-weight="bold" fill="#1e293b">320</text>
  <text x="490" y="120" text-anchor="middle" font-size="12" fill="#9ca3af">(350.0)</text>
  <text x="330" y="170" text-anchor="middle" font-size="18" font-weight="bold" fill="#1e293b">120</text>
  <text x="330" y="190" text-anchor="middle" font-size="12" fill="#9ca3af">(150.0)</text>
  <text x="490" y="170" text-anchor="middle" font-size="18" font-weight="bold" fill="#1e293b">380</text>
  <text x="490" y="190" text-anchor="middle" font-size="12" fill="#9ca3af">(350.0)</text>
  <!-- Totals -->
  <text x="610" y="105" text-anchor="middle" font-size="15" font-weight="bold" fill="#6b7280">500</text>
  <text x="610" y="175" text-anchor="middle" font-size="15" font-weight="bold" fill="#6b7280">500</text>
  <text x="330" y="245" text-anchor="middle" font-size="15" font-weight="bold" fill="#6b7280">300</text>
  <text x="490" y="245" text-anchor="middle" font-size="15" font-weight="bold" fill="#6b7280">700</text>
  <text x="610" y="245" text-anchor="middle" font-size="15" font-weight="bold" fill="#6b7280">1000</text>
  <!-- Formula annotation -->
  <text x="330" y="290" text-anchor="middle" font-size="12" fill="#7c3aed">Expected = (Row Total x Column Total) / Grand Total</text>
  <text x="330" y="310" text-anchor="middle" font-size="12" fill="#7c3aed">E.g., E(AD, Risk) = (500 x 300) / 1000 = 150</text>
  <!-- Highlight cells that deviate most -->
  <rect x="260" y="78" width="140" height="50" rx="4" fill="#dc2626" fill-opacity="0.08" stroke="#dc2626" stroke-width="1" stroke-opacity="0.3"/>
  <rect x="260" y="148" width="140" height="50" rx="4" fill="#16a34a" fill-opacity="0.08" stroke="#16a34a" stroke-width="1" stroke-opacity="0.3"/>
</svg>
</div>

## Chi-Square Test of Independence

The chi-square test asks: "Are these two categorical variables independent, or is there an association?"

### How It Works

1. Compute **expected counts** under independence: E = (row total x column total) / grand total
2. For each cell, compute (Observed - Expected)^2 / Expected
3. Sum across all cells to get the chi-square statistic
4. Compare to the chi-square distribution with df = (rows - 1)(cols - 1)

For the Alzheimer's example:

| | Risk Allele | No Risk Allele |
|---|---|---|
| **Expected (AD)** | 500 x 300 / 1000 = 150 | 500 x 700 / 1000 = 350 |
| **Expected (Ctrl)** | 500 x 300 / 1000 = 150 | 500 x 700 / 1000 = 350 |

The chi-square statistic measures how far the observed counts deviate from what independence predicts.

### Assumptions

- Observations are independent
- Expected count in each cell is at least 5 (if not, use Fisher's exact test)
- Sample is reasonably large

> **Common pitfall:** The chi-square test requires *expected* counts of at least 5 in each cell, not *observed* counts. Check expected counts before interpreting results. When they are too small, Fisher's exact test is the safe alternative.

## Chi-Square Goodness of Fit: Hardy-Weinberg

The chi-square test can also compare observed frequencies to a theoretical distribution. A critical application in genetics is testing **Hardy-Weinberg Equilibrium** (HWE).

For a biallelic locus with allele frequencies p (major) and q (minor), HWE predicts:
- AA: p^2
- Ag: 2pq
- gg: q^2

If observed genotype counts deviate significantly from HWE expectations, it may indicate selection, population structure, genotyping error, or non-random mating.

<div style="text-align: center; margin: 2em 0;">
<svg width="660" height="300" viewBox="0 0 660 300" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="330" y="28" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">Hardy-Weinberg Equilibrium: Expected Genotype Frequencies</text>
  <text x="330" y="48" text-anchor="middle" font-size="12" fill="#6b7280">p(A) = 0.6, q(G) = 0.4</text>
  <!-- Axis -->
  <line x1="100" y1="240" x2="560" y2="240" stroke="#6b7280" stroke-width="1.5"/>
  <line x1="100" y1="240" x2="100" y2="60" stroke="#6b7280" stroke-width="1.5"/>
  <!-- Y-axis label -->
  <text x="40" y="150" text-anchor="middle" font-size="12" fill="#6b7280" transform="rotate(-90,40,150)">Frequency</text>
  <!-- Y-axis ticks -->
  <text x="90" y="244" text-anchor="end" font-size="11" fill="#6b7280">0</text>
  <text x="90" y="204" text-anchor="end" font-size="11" fill="#6b7280">0.1</text>
  <line x1="95" y1="200" x2="105" y2="200" stroke="#6b7280" stroke-width="1"/>
  <text x="90" y="164" text-anchor="end" font-size="11" fill="#6b7280">0.2</text>
  <line x1="95" y1="160" x2="105" y2="160" stroke="#6b7280" stroke-width="1"/>
  <text x="90" y="124" text-anchor="end" font-size="11" fill="#6b7280">0.3</text>
  <line x1="95" y1="120" x2="105" y2="120" stroke="#6b7280" stroke-width="1"/>
  <text x="90" y="84" text-anchor="end" font-size="11" fill="#6b7280">0.4</text>
  <line x1="95" y1="80" x2="105" y2="80" stroke="#6b7280" stroke-width="1"/>
  <!-- AA bar: p^2 = 0.36 -->
  <rect x="145" y="96" width="100" height="144" rx="3" fill="#2563eb" fill-opacity="0.75"/>
  <text x="195" y="88" text-anchor="middle" font-size="14" font-weight="bold" fill="#2563eb">0.36</text>
  <text x="195" y="260" text-anchor="middle" font-size="14" font-weight="bold" fill="#1e293b">AA</text>
  <text x="195" y="278" text-anchor="middle" font-size="11" fill="#6b7280">p² = 0.6²</text>
  <!-- AG bar: 2pq = 0.48 -->
  <rect x="280" y="48" width="100" height="192" rx="3" fill="#7c3aed" fill-opacity="0.7"/>
  <text x="330" y="40" text-anchor="middle" font-size="14" font-weight="bold" fill="#7c3aed">0.48</text>
  <text x="330" y="260" text-anchor="middle" font-size="14" font-weight="bold" fill="#1e293b">AG</text>
  <text x="330" y="278" text-anchor="middle" font-size="11" fill="#6b7280">2pq = 2(0.6)(0.4)</text>
  <!-- GG bar: q^2 = 0.16 -->
  <rect x="415" y="176" width="100" height="64" rx="3" fill="#dc2626" fill-opacity="0.65"/>
  <text x="465" y="168" text-anchor="middle" font-size="14" font-weight="bold" fill="#dc2626">0.16</text>
  <text x="465" y="260" text-anchor="middle" font-size="14" font-weight="bold" fill="#1e293b">GG</text>
  <text x="465" y="278" text-anchor="middle" font-size="11" fill="#6b7280">q² = 0.4²</text>
</svg>
</div>

## Fisher's Exact Test

When sample sizes are small or expected counts fall below 5, the chi-square approximation is unreliable. Fisher's exact test computes the exact probability using the hypergeometric distribution.

Fisher's exact test is computationally expensive for large tables but is the gold standard for 2x2 tables with small counts — common in rare variant studies and pilot experiments.

> **Clinical relevance:** Regulatory agencies (FDA, EMA) often prefer Fisher's exact test for safety analyses where adverse events are rare and sample sizes are modest.

## McNemar's Test: Paired Categorical Data

When observations are paired — the same patients tested before and after treatment, or the same samples tested with two diagnostic methods — McNemar's test is the correct choice.

| | Test B Positive | Test B Negative |
|---|---|---|
| **Test A Positive** | a (both positive) | b (A+, B-) |
| **Test A Negative** | c (A-, B+) | d (both negative) |

McNemar's test focuses on the **discordant pairs** (b and c):

**chi-square = (b - c)^2 / (b + c)**

## Measures of Association

### Odds Ratio (OR)

The odds ratio quantifies the strength of association in a 2x2 table:

**OR = (a x d) / (b x c)**

| OR | Interpretation |
|---|---|
| OR = 1 | No association |
| OR > 1 | Exposure increases odds of outcome |
| OR < 1 | Exposure decreases odds of outcome |
| OR = 2.5 | Exposed group has 2.5x the odds |

<div style="text-align: center; margin: 2em 0;">
<svg width="660" height="200" viewBox="0 0 660 200" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="330" y="28" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">Odds Ratio Interpretation</text>
  <!-- Number line -->
  <line x1="60" y1="100" x2="600" y2="100" stroke="#6b7280" stroke-width="2"/>
  <!-- Tick marks -->
  <line x1="120" y1="92" x2="120" y2="108" stroke="#6b7280" stroke-width="2"/>
  <text x="120" y="125" text-anchor="middle" font-size="13" font-weight="bold" fill="#6b7280">0.25</text>
  <line x1="220" y1="92" x2="220" y2="108" stroke="#6b7280" stroke-width="2"/>
  <text x="220" y="125" text-anchor="middle" font-size="13" font-weight="bold" fill="#6b7280">0.5</text>
  <line x1="330" y1="88" x2="330" y2="112" stroke="#1e293b" stroke-width="3"/>
  <text x="330" y="128" text-anchor="middle" font-size="14" font-weight="bold" fill="#1e293b">1.0</text>
  <line x1="440" y1="92" x2="440" y2="108" stroke="#6b7280" stroke-width="2"/>
  <text x="440" y="125" text-anchor="middle" font-size="13" font-weight="bold" fill="#6b7280">2.0</text>
  <line x1="540" y1="92" x2="540" y2="108" stroke="#6b7280" stroke-width="2"/>
  <text x="540" y="125" text-anchor="middle" font-size="13" font-weight="bold" fill="#6b7280">4.0</text>
  <!-- Protective zone -->
  <rect x="60" y="58" width="268" height="30" rx="4" fill="#16a34a" fill-opacity="0.15"/>
  <text x="168" y="78" text-anchor="middle" font-size="13" font-weight="bold" fill="#16a34a">Protective (OR &lt; 1)</text>
  <!-- Risk zone -->
  <rect x="332" y="58" width="268" height="30" rx="4" fill="#dc2626" fill-opacity="0.15"/>
  <text x="472" y="78" text-anchor="middle" font-size="13" font-weight="bold" fill="#dc2626">Risk Factor (OR &gt; 1)</text>
  <!-- No effect label -->
  <text x="330" y="55" text-anchor="middle" font-size="12" fill="#7c3aed">No Effect</text>
  <!-- Example point with CI -->
  <circle cx="465" cy="155" r="6" fill="#2563eb"/>
  <line x1="385" y1="155" x2="545" y2="155" stroke="#2563eb" stroke-width="2"/>
  <line x1="385" y1="148" x2="385" y2="162" stroke="#2563eb" stroke-width="2"/>
  <line x1="545" y1="148" x2="545" y2="162" stroke="#2563eb" stroke-width="2"/>
  <text x="465" y="175" text-anchor="middle" font-size="11" fill="#2563eb">OR = 2.25 (95% CI: 1.5 - 3.4)</text>
  <!-- Arrow heads -->
  <polygon points="60,100 70,95 70,105" fill="#16a34a"/>
  <polygon points="600,100 590,95 590,105" fill="#dc2626"/>
</svg>
</div>

### Relative Risk (RR)

In prospective studies (cohorts), relative risk is often preferred:

**RR = [a / (a+b)] / [c / (c+d)]**

> **Key insight:** Odds ratios approximate relative risk only when the outcome is rare (< 10%). For common outcomes, OR overestimates RR. Case-control studies can only estimate OR, not RR.

### Cramer's V

A measure of association strength for tables larger than 2x2:

**V = sqrt(chi-square / (n x min(r-1, c-1)))**

V ranges from 0 (no association) to 1 (perfect association).

| V | Interpretation |
|---|---|
| 0.1 | Small association |
| 0.3 | Medium association |
| 0.5 | Large association |

## Proportion Test

Tests whether an observed proportion differs from an expected value, or whether two proportions differ from each other. Useful for comparing mutation frequencies, response rates, or allele frequencies between populations.

## Categorical Tests in BioLang

### Chi-Square Test: SNP-Disease Association

```bio
# Observed genotype counts near APOE
# Rows: Alzheimer's, Control
# Columns: Risk allele present, Risk allele absent
let observed = [[180, 320], [120, 380]]

let result = chi_square(observed)
print("=== Chi-Square Test of Independence ===")
print("Chi-square statistic: {result.statistic:.4}")
print("p-value: {result.p_value:.2e}")
print("Degrees of freedom: {result.df}")

# Effect size: Cramer's V from chi-square output
let n_total = 180 + 320 + 120 + 380
let v = sqrt(result.statistic / (n_total * min(2 - 1, 2 - 1)))
print("Cramer's V: {v:.4}")

# Odds ratio (inline: (a*d) / (b*c))
let a = 180
let b = 320
let c = 120
let d = 380
let or_val = (a * d) / (b * c)
print("\nOdds Ratio: {or_val:.3}")

# Relative risk (inline)
let rr = (a / (a + b)) / (c / (c + d))
print("Relative Risk: {rr:.3}")

if result.p_value < 0.05 {
  print("\nSignificant association between risk allele and Alzheimer's")
}
```

### Fisher's Exact Test: Rare Variant Study

```bio
# Rare loss-of-function variant in 200 cases and 200 controls
# Very small expected counts -> Fisher's exact test
let observed = [[8, 192], [2, 198]]

print("=== Fisher's Exact Test: Rare Variant ===")
print("Observed: 8/200 cases vs 2/200 controls carry the variant\n")

# Chi-square would be unreliable here
let chi_result = chi_square(observed)
print("Chi-square p-value: {chi_result.p_value:.4} (unreliable — low expected counts)")

# Fisher's exact is the correct choice
let fisher_result = fisher_exact(observed)
print("Fisher's exact p-value: {fisher_result.p_value:.4}")

# Odds ratio (inline)
let or_val = (8 * 198) / (192 * 2)
print("Odds Ratio: {or_val:.2}")

# Note: CI includes 1.0, so despite the apparent 4x difference,
# the sample size is too small for significance
```

### Hardy-Weinberg Equilibrium Test

```bio
# Genotype counts at a SNP locus
# Observed: AA=280, AG=430, GG=290 (total = 1000)
let obs_AA = 280
let obs_AG = 430
let obs_GG = 290
let n = obs_AA + obs_AG + obs_GG

# Estimate allele frequencies
let p = (2 * obs_AA + obs_AG) / (2 * n)  # freq of A
let q = 1.0 - p                           # freq of G

print("Allele frequencies: p(A) = {p:.4}, q(G) = {q:.4}")

# Expected counts under HWE
let exp_AA = p * p * n
let exp_AG = 2 * p * q * n
let exp_GG = q * q * n

print("\nGenotype     | Observed | Expected (HWE)")
print("-------------|----------|----------------")
print("AA           | {obs_AA:>8} | {exp_AA:>14.1}")
print("AG           | {obs_AG:>8} | {exp_AG:>14.1}")
print("GG           | {obs_GG:>8} | {exp_GG:>14.1}")

# Chi-square goodness of fit (df=1 for HWE with 2 alleles)
let chi_sq = (obs_AA - exp_AA)^2 / exp_AA +
             (obs_AG - exp_AG)^2 / exp_AG +
             (obs_GG - exp_GG)^2 / exp_GG

# Use chi_square with expected frequencies
let observed = [obs_AA, obs_AG, obs_GG]
let expected = [exp_AA, exp_AG, exp_GG]
let result = chi_square(observed, expected)

print("\nChi-square = {result.statistic:.4}")
print("p-value = {result.p_value:.4}")

if result.p_value > 0.05 {
  print("Genotype frequencies are consistent with Hardy-Weinberg Equilibrium")
} else {
  print("Significant deviation from HWE — investigate possible causes")
}
```

### McNemar's Test: Diagnostic Agreement

```bio
# Two diagnostic tests for TB applied to same 200 patients
# Test A (culture) vs Test B (PCR)
let table = [[85, 15], [10, 90]]
# 85: both positive, 90: both negative
# 15: A+/B-, 10: A-/B+

# McNemar's test: use chi_square() on discordant cells
let b_disc = 15  # A+/B-
let c_disc = 10  # A-/B+
let mcnemar_chi2 = (b_disc - c_disc) * (b_disc - c_disc) / (b_disc + c_disc)
let mcnemar_p = 1.0 - pnorm(sqrt(mcnemar_chi2), 0, 1) * 2.0  # approximate
# Or use chi_square on discordant cells
let result = chi_square([b_disc, c_disc], [12.5, 12.5])
print("=== McNemar's Test: Culture vs PCR ===")
print("Discordant pairs: A+/B- = 15, A-/B+ = 10")
print("Chi-square: {result.statistic:.4}")
print("p-value: {result.p_value:.4}")

if result.p_value > 0.05 {
  print("No significant difference between the two diagnostic tests")
} else {
  print("The tests have significantly different detection rates")
}
```

### Proportion Test: Comparing Mutation Frequencies

```bio
# EGFR mutation frequency in two populations
# Asian cohort: 120/300 (40%), European cohort: 45/300 (15%)
# Two-proportion test via chi_square
let observed = [[120, 180], [45, 255]]
let result = chi_square(observed)

print("=== Two-Proportion Test: EGFR Mutation Frequency ===")
print("Asian: 120/300 = 40%")
print("European: 45/300 = 15%")
print("Chi-square: {result.statistic:.4}")
print("p-value: {result.p_value:.2e}")
print("Difference: 25 percentage points")

# Visualize
bar_chart(["Asian", "European"], [40.0, 15.0], {title: "EGFR Mutation Frequency by Population", y_label: "Mutation Frequency (%)"})
```

### Complete Workflow: Multi-Allelic Association

```bio
# Three genotypes at a pharmacogenomics locus
# and three drug response categories
let observed = [
  [45, 30, 25],   # Poor metabolizer
  [35, 55, 60],   # Intermediate
  [20, 15, 15]    # Ultra-rapid
]
let row_labels = ["Poor", "Intermediate", "Ultra-rapid"]
let col_labels = ["AA", "AG", "GG"]

let result = chi_square(observed)
print("=== Chi-Square: Genotype vs Drug Response ===")
print("Chi-square: {result.statistic:.4}")
print("p-value: {result.p_value:.4}")
print("df: {result.df}")

# Cramer's V from chi-square output
let n_total = 45 + 30 + 25 + 35 + 55 + 60 + 20 + 15 + 15
let v = sqrt(result.statistic / (n_total * min(3 - 1, 3 - 1)))
print("Cramer's V: {v:.4} (effect size)")

# Display the contingency table
print("\n           | AA   | AG   | GG   | Total")
print("-----------|------|------|------|------")
for i in 0..3 {
  let total = observed[i][0] + observed[i][1] + observed[i][2]
  print("{row_labels[i]:<11}| {observed[i][0]:>4} | {observed[i][1]:>4} | {observed[i][2]:>4} | {total:>4}")
}
```

**Python:**

```python
from scipy import stats
import numpy as np

# Chi-square test of independence
observed = np.array([[180, 320], [120, 380]])
chi2, p, dof, expected = stats.chi2_contingency(observed)
print(f"Chi-square = {chi2:.4f}, p = {p:.2e}")

# Fisher's exact test
odds, p = stats.fisher_exact([[8, 192], [2, 198]])
print(f"OR = {odds:.2f}, p = {p:.4f}")

# McNemar's test
from statsmodels.stats.contingency_tables import mcnemar
result = mcnemar(np.array([[85, 15], [10, 90]]), exact=False)
print(f"McNemar chi2 = {result.statistic:.4f}, p = {result.pvalue:.4f}")

# Proportion test
from statsmodels.stats.proportion import proportions_ztest
z, p = proportions_ztest([120, 45], [300, 300])
print(f"z = {z:.4f}, p = {p:.2e}")
```

**R:**

```r
# Chi-square test
observed <- matrix(c(180, 120, 320, 380), nrow = 2)
chisq.test(observed)

# Fisher's exact test
fisher.test(matrix(c(8, 2, 192, 198), nrow = 2))

# McNemar's test
mcnemar.test(matrix(c(85, 10, 15, 90), nrow = 2))

# Odds ratio
library(epitools)
oddsratio(observed)

# Proportion test
prop.test(c(120, 45), c(300, 300))
```

## Exercises

**Exercise 1: SNP Association Study**

A GWAS hit is validated in a replication cohort. Genotype counts:

|  | AA | AG | GG |
|---|---|---|---|
| Cases | 150 | 200 | 50 |
| Controls | 180 | 160 | 60 |

```bio
let observed = [[150, 200, 50], [180, 160, 60]]

# TODO: Run chi-square test
# TODO: Compute Cramer's V
# TODO: Test HWE separately in cases and controls
# TODO: Interpret the results
```

**Exercise 2: Fisher's Exact on Rare Mutations**

In a rare disease study: 5/50 patients carry a mutation vs 1/100 controls.

```bio
let table = [[5, 45], [1, 99]]

# TODO: Why is Fisher's exact preferred here?
# TODO: Compute p-value and odds ratio
# TODO: What does the wide CI on the OR tell you?
```

**Exercise 3: McNemar's for Treatment Response**

A tumor is biopsied before and after chemotherapy. Response to an immunostaining marker:

|  | After: Positive | After: Negative |
|---|---|---|
| Before: Positive | 40 | 25 |
| Before: Negative | 5 | 30 |

```bio
let table = [[40, 25], [5, 30]]

# TODO: Run McNemar's test
# TODO: What do the discordant pairs tell you?
# TODO: Is the marker significantly altered by chemotherapy?
```

**Exercise 4: Multi-Population Allele Comparison**

Compare allele frequencies of a pharmacogenomics variant across three populations. Use chi-square and Cramer's V, then create a bar chart of frequencies.

```bio
# Observed carrier counts out of 500 per population
let observed = [[210, 290], [150, 350], [85, 415]]
let populations = ["East Asian", "European", "African"]

# TODO: Chi-square test of independence
# TODO: Cramer's V
# TODO: Pairwise proportion tests with Bonferroni correction
# TODO: Bar chart of carrier frequencies
```

## Key Takeaways

- The **chi-square test** evaluates whether two categorical variables are independent by comparing observed to expected counts
- **Fisher's exact test** is preferred when expected cell counts are below 5, common with rare variants
- **McNemar's test** handles paired categorical data (same subjects, two conditions)
- The **odds ratio** quantifies association strength; OR = 1 means no association
- **Relative risk** is preferred in cohort studies but cannot be computed from case-control designs
- **Cramer's V** measures association strength for tables larger than 2x2
- **Hardy-Weinberg equilibrium** testing uses chi-square goodness of fit to check for genotyping artifacts or population structure
- The **proportion test** compares frequencies between populations or against expected values
- Always check expected cell counts before using chi-square — use Fisher's exact when they are small

## What's Next

You now have a powerful toolkit for individual tests. But in genomics, we never run just one test — we run thousands or millions simultaneously. Testing 20,000 genes means 1,000 false positives at alpha = 0.05. Tomorrow we confront the multiple testing crisis head-on and learn the corrections that make genome-scale analysis possible, culminating in the volcano plot that has become the icon of differential expression analysis.
