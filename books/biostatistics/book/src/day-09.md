# Day 9: When Normality Fails — Non-Parametric Tests

## The Problem

Dr. Maria Gonzalez studies the gut microbiome in inflammatory bowel disease (IBD). She has 16S rRNA sequencing data from 15 IBD patients and 15 healthy controls, measuring the relative abundance of *Faecalibacterium prausnitzii*, a key anti-inflammatory bacterium. Looking at the data, she sees a mess: most values cluster near zero, a few patients have moderate levels, and one healthy individual has an enormous abundance of 45%. The histogram looks nothing like a bell curve — it is right-skewed with a long tail.

She runs a Shapiro-Wilk test on each group: both return p < 0.001, firmly rejecting normality. The t-test assumes normally distributed data. With data this skewed, the t-test's p-value could be wildly inaccurate — too liberal or too conservative, depending on the specific pattern. She needs tests that work without any assumptions about the shape of the distribution.

These are **non-parametric tests**: methods that operate on the *ranks* of data rather than the raw values, making them robust to skewness, outliers, and any distributional shape.

## What Are Non-Parametric Tests?

Imagine you are judging a cooking competition. A parametric judge scores each dish on a precise 1-100 scale and compares average scores. A non-parametric judge simply ranks the dishes from best to worst — first place, second place, third place. The ranking approach is less precise when scores are reliable, but it is far more robust when one judge has an eccentric scoring system.

Non-parametric tests replace raw data values with their **ranks** (1st smallest, 2nd smallest, ...) and then analyze the ranks. This has powerful consequences:

| Property | Parametric (t-test) | Non-parametric (rank-based) |
|---|---|---|
| Assumes normality | Yes | No |
| Sensitive to outliers | Very | Resistant |
| Uses raw values | Yes | Uses ranks |
| Power (normal data) | Highest | Slightly lower (~95%) |
| Power (non-normal data) | Unreliable | Reliable |
| Handles ordinal data | No | Yes |

> **Key insight:** Non-parametric tests are not "worse" versions of parametric tests. They are the correct choice when distributional assumptions are violated. Using a t-test on heavily skewed data is like measuring temperature with a ruler — you might get a number, but it doesn't mean anything.

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="300" viewBox="0 0 680 300" xmlns="http://www.w3.org/2000/svg" style="background: #fafbfc; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="28" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">Normal Data vs Skewed Microbiome Data</text>
  <!-- LEFT: Normal distribution -->
  <text x="170" y="55" text-anchor="middle" font-size="13" fill="#2563eb" font-weight="bold">Normal (Gene Expression)</text>
  <text x="170" y="70" text-anchor="middle" font-size="10" fill="#6b7280">Symmetric, bell-shaped</text>
  <line x1="30" y1="220" x2="310" y2="220" stroke="#374151" stroke-width="1"/>
  <!-- Normal bell curve filled -->
  <path d="M 30,220 C 40,219 50,217 60,213 C 80,202 100,182 120,155 C 140,120 155,92 165,78 C 172,70 175,68 178,68 C 181,68 184,70 190,78 C 200,92 215,120 230,155 C 250,182 270,202 290,213 C 300,217 305,219 310,220 Z" fill="#dbeafe" stroke="#2563eb" stroke-width="2"/>
  <!-- Mean line -->
  <line x1="170" y1="68" x2="170" y2="225" stroke="#2563eb" stroke-width="1.5" stroke-dasharray="4,2"/>
  <text x="170" y="238" text-anchor="middle" font-size="10" fill="#2563eb">mean = median</text>
  <text x="170" y="260" text-anchor="middle" font-size="10" fill="#16a34a" font-weight="bold">t-test works great</text>
  <!-- RIGHT: Skewed microbiome distribution -->
  <text x="510" y="55" text-anchor="middle" font-size="13" fill="#dc2626" font-weight="bold">Skewed (Microbiome Abundance)</text>
  <text x="510" y="70" text-anchor="middle" font-size="10" fill="#6b7280">Many zeros, long right tail</text>
  <line x1="370" y1="220" x2="650" y2="220" stroke="#374151" stroke-width="1"/>
  <!-- Skewed distribution: spike near zero, long tail -->
  <path d="M 370,220 L 380,218 L 390,82 L 400,78 L 410,100 L 420,140 L 430,168 L 440,186 L 450,198 L 460,206 L 470,210 L 490,214 L 510,216 L 540,217 L 570,218 L 600,219 L 630,219.5 L 650,220 Z" fill="#fecaca" stroke="#dc2626" stroke-width="2"/>
  <!-- Median near left -->
  <line x1="405" y1="80" x2="405" y2="225" stroke="#7c3aed" stroke-width="1.5" stroke-dasharray="4,2"/>
  <text x="405" y="238" text-anchor="middle" font-size="10" fill="#7c3aed">median</text>
  <!-- Mean further right -->
  <line x1="475" y1="208" x2="475" y2="225" stroke="#f59e0b" stroke-width="1.5" stroke-dasharray="4,2"/>
  <text x="475" y="238" text-anchor="middle" font-size="10" fill="#f59e0b">mean</text>
  <!-- Outlier annotation -->
  <text x="590" y="210" font-size="9" fill="#dc2626">rare high values</text>
  <text x="510" y="260" text-anchor="middle" font-size="10" fill="#dc2626" font-weight="bold">t-test unreliable!</text>
  <text x="510" y="275" text-anchor="middle" font-size="10" fill="#16a34a" font-weight="bold">Use Mann-Whitney</text>
</svg>
</div>

## When to Choose Non-Parametric

Use non-parametric tests when:
- **Shapiro-Wilk** rejects normality (p < 0.05) and sample size is small
- Data are **ordinal** (pain scale 1-10, tumor grade I-IV)
- Data have **heavy outliers** that cannot be removed
- **Sample sizes are very small** (n < 10 per group)
- Data are **bounded** or have floor/ceiling effects (many zeros)

## The Rank Transformation

The foundation of all non-parametric tests is replacing values with ranks:

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="320" viewBox="0 0 680 320" xmlns="http://www.w3.org/2000/svg" style="background: #fafbfc; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="28" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">Raw Values vs Ranks: Taming Outliers</text>
  <!-- LEFT: Raw values -->
  <text x="170" y="58" text-anchor="middle" font-size="13" fill="#2563eb" font-weight="bold">Raw Values</text>
  <!-- Bar chart of raw values -->
  <line x1="50" y1="240" x2="300" y2="240" stroke="#374151" stroke-width="1"/>
  <line x1="50" y1="70" x2="50" y2="240" stroke="#374151" stroke-width="1"/>
  <!-- Scale: 0 to 45 -->
  <text x="45" y="244" text-anchor="end" font-size="9" fill="#9ca3af">0</text>
  <text x="45" y="198" text-anchor="end" font-size="9" fill="#9ca3af">10</text>
  <text x="45" y="148" text-anchor="end" font-size="9" fill="#9ca3af">20</text>
  <text x="45" y="98" text-anchor="end" font-size="9" fill="#9ca3af">30</text>
  <text x="45" y="78" text-anchor="end" font-size="9" fill="#9ca3af">40</text>
  <!-- Bars: 0.1, 0.3, 0.8, 1.5, 3.2, 45.0 -->
  <rect x="65" y="239" width="30" height="1" fill="#93c5fd" stroke="#2563eb" stroke-width="0.5" rx="2"/>
  <text x="80" y="254" text-anchor="middle" font-size="9" fill="#6b7280">0.1%</text>
  <rect x="105" y="239" width="30" height="1" fill="#93c5fd" stroke="#2563eb" stroke-width="0.5" rx="2"/>
  <text x="120" y="254" text-anchor="middle" font-size="9" fill="#6b7280">0.3%</text>
  <rect x="145" y="237" width="30" height="3" fill="#93c5fd" stroke="#2563eb" stroke-width="0.5" rx="2"/>
  <text x="160" y="254" text-anchor="middle" font-size="9" fill="#6b7280">0.8%</text>
  <rect x="185" y="234" width="30" height="6" fill="#93c5fd" stroke="#2563eb" stroke-width="0.5" rx="2"/>
  <text x="200" y="254" text-anchor="middle" font-size="9" fill="#6b7280">1.5%</text>
  <rect x="225" y="228" width="30" height="12" fill="#93c5fd" stroke="#2563eb" stroke-width="0.5" rx="2"/>
  <text x="240" y="254" text-anchor="middle" font-size="9" fill="#6b7280">3.2%</text>
  <rect x="265" y="72" width="30" height="168" fill="#ef4444" stroke="#dc2626" stroke-width="0.5" rx="2"/>
  <text x="280" y="254" text-anchor="middle" font-size="9" fill="#dc2626" font-weight="bold">45%!</text>
  <!-- Arrow -->
  <text x="340" y="160" text-anchor="middle" font-size="24" fill="#7c3aed" font-weight="bold">--></text>
  <text x="340" y="180" text-anchor="middle" font-size="10" fill="#7c3aed">Rank</text>
  <text x="340" y="192" text-anchor="middle" font-size="10" fill="#7c3aed">transform</text>
  <!-- RIGHT: Ranked values -->
  <text x="510" y="58" text-anchor="middle" font-size="13" fill="#16a34a" font-weight="bold">Ranked Values</text>
  <line x1="380" y1="240" x2="650" y2="240" stroke="#374151" stroke-width="1"/>
  <line x1="380" y1="70" x2="380" y2="240" stroke="#374151" stroke-width="1"/>
  <!-- Scale: 0 to 6 -->
  <text x="375" y="244" text-anchor="end" font-size="9" fill="#9ca3af">0</text>
  <text x="375" y="212" text-anchor="end" font-size="9" fill="#9ca3af">1</text>
  <text x="375" y="184" text-anchor="end" font-size="9" fill="#9ca3af">2</text>
  <text x="375" y="156" text-anchor="end" font-size="9" fill="#9ca3af">3</text>
  <text x="375" y="128" text-anchor="end" font-size="9" fill="#9ca3af">4</text>
  <text x="375" y="100" text-anchor="end" font-size="9" fill="#9ca3af">5</text>
  <text x="375" y="78" text-anchor="end" font-size="9" fill="#9ca3af">6</text>
  <!-- Even bars for ranks 1-6 -->
  <rect x="395" y="212" width="30" height="28" fill="#bbf7d0" stroke="#16a34a" stroke-width="0.5" rx="2"/>
  <text x="410" y="254" text-anchor="middle" font-size="9" fill="#6b7280">Rank 1</text>
  <rect x="435" y="184" width="30" height="56" fill="#bbf7d0" stroke="#16a34a" stroke-width="0.5" rx="2"/>
  <text x="450" y="254" text-anchor="middle" font-size="9" fill="#6b7280">Rank 2</text>
  <rect x="475" y="156" width="30" height="84" fill="#bbf7d0" stroke="#16a34a" stroke-width="0.5" rx="2"/>
  <text x="490" y="254" text-anchor="middle" font-size="9" fill="#6b7280">Rank 3</text>
  <rect x="515" y="128" width="30" height="112" fill="#bbf7d0" stroke="#16a34a" stroke-width="0.5" rx="2"/>
  <text x="530" y="254" text-anchor="middle" font-size="9" fill="#6b7280">Rank 4</text>
  <rect x="555" y="100" width="30" height="140" fill="#bbf7d0" stroke="#16a34a" stroke-width="0.5" rx="2"/>
  <text x="570" y="254" text-anchor="middle" font-size="9" fill="#6b7280">Rank 5</text>
  <rect x="595" y="72" width="30" height="168" fill="#86efac" stroke="#16a34a" stroke-width="1" rx="2"/>
  <text x="610" y="254" text-anchor="middle" font-size="9" fill="#16a34a" font-weight="bold">Rank 6</text>
  <!-- Annotation -->
  <text x="340" y="290" text-anchor="middle" font-size="12" fill="#374151">The outlier (45%) gets rank 6 -- just one step above 3.2%</text>
  <text x="340" y="308" text-anchor="middle" font-size="11" fill="#6b7280">Its extreme magnitude no longer dominates the analysis</text>
</svg>
</div>


| Patient | Abundance | Rank |
|---|---|---|
| P1 | 0.1% | 1 |
| P2 | 0.3% | 2 |
| P3 | 0.8% | 3 |
| P4 | 1.5% | 4 |
| P5 | 3.2% | 5 |
| P6 | 45.0% | 6 |

Notice: the outlier (45%) gets rank 6 — just one rank above 3.2%. Its extreme value no longer dominates the analysis.

## Wilcoxon Rank-Sum Test (Mann-Whitney U)

The non-parametric counterpart of the independent two-sample t-test.

**Procedure:**
1. Combine all observations and rank them 1 through N
2. Sum the ranks in each group separately
3. If one group consistently has higher ranks, the rank sum will be extreme
4. Compare to the expected rank sum under H0 (no difference)

**H0:** The two groups have identical distributions
**H1:** One group tends to have larger values

> **Common pitfall:** The Wilcoxon rank-sum and Mann-Whitney U are the same test, just computed differently. U = W - n1(n1+1)/2. Different software uses different names, but the p-value is identical.

## Wilcoxon Signed-Rank Test

The non-parametric counterpart of the paired t-test.

**Procedure:**
1. Compute the difference for each pair
2. Rank the absolute differences (ignoring zeros)
3. Sum ranks of positive differences (W+) and negative differences (W-)
4. If the treatment consistently increases (or decreases), one sum will dominate

## Sign Test

Even simpler than Wilcoxon signed-rank — only considers the *direction* of differences, not their magnitude.

**Procedure:**
1. For each pair, note whether the difference is positive, negative, or zero
2. Count positives and negatives (discard zeros)
3. Under H0, positives and negatives should be equally likely (binomial test with p = 0.5)

The sign test has less power than Wilcoxon signed-rank but makes even fewer assumptions.

## Kruskal-Wallis Test

The non-parametric counterpart of one-way ANOVA, for comparing three or more groups.

**H0:** All groups have the same distribution
**H1:** At least one group differs

If significant, follow up with pairwise Wilcoxon tests (with multiple testing correction).

## Kolmogorov-Smirnov (KS) Test

Compares two entire distributions, not just their centers. Detects differences in shape, spread, or location.

**H0:** The two samples come from the same distribution
**H1:** The distributions differ in any way

> **Clinical relevance:** The KS test is useful when you suspect groups differ not just in average abundance, but in the entire pattern of their distribution — for example, one group might be bimodal while the other is unimodal.

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="380" viewBox="0 0 680 380" xmlns="http://www.w3.org/2000/svg" style="background: #fafbfc; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="28" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">Decision Flowchart: Parametric or Non-Parametric?</text>
  <defs>
    <marker id="arrowNP" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto">
      <path d="M 0 0 L 10 5 L 0 10 z" fill="#374151"/>
    </marker>
  </defs>
  <!-- Start: What type of data? -->
  <rect x="230" y="50" width="220" height="38" rx="19" fill="#dbeafe" stroke="#2563eb" stroke-width="2"/>
  <text x="340" y="74" text-anchor="middle" font-size="12" fill="#1e293b" font-weight="bold">What type of data?</text>
  <!-- Branch: Ordinal -->
  <line x1="230" y1="69" x2="100" y2="69" stroke="#374151" stroke-width="1.5" marker-end="url(#arrowNP)"/>
  <text x="170" y="62" font-size="10" fill="#6b7280">Ordinal/ranked</text>
  <rect x="20" y="55" width="80" height="30" rx="6" fill="#fef3c7" stroke="#f59e0b" stroke-width="2"/>
  <text x="60" y="74" text-anchor="middle" font-size="11" fill="#92400e" font-weight="bold">Non-param</text>
  <!-- Branch: Continuous -->
  <line x1="340" y1="88" x2="340" y2="120" stroke="#374151" stroke-width="1.5" marker-end="url(#arrowNP)"/>
  <text x="355" y="108" font-size="10" fill="#6b7280">Continuous</text>
  <!-- Normal? -->
  <rect x="228" y="120" width="224" height="42" rx="8" fill="#f3f4f6" stroke="#6b7280" stroke-width="1.5"/>
  <text x="340" y="140" text-anchor="middle" font-size="12" fill="#1e293b">Data approximately normal?</text>
  <text x="340" y="155" text-anchor="middle" font-size="10" fill="#6b7280">(Shapiro-Wilk, QQ plot, n > 30)</text>
  <!-- YES: check further -->
  <line x1="452" y1="141" x2="520" y2="141" stroke="#16a34a" stroke-width="1.5" marker-end="url(#arrowNP)"/>
  <text x="480" y="134" font-size="11" fill="#16a34a" font-weight="bold">Yes</text>
  <!-- Parametric box -->
  <rect x="520" y="120" width="140" height="42" rx="8" fill="#dcfce7" stroke="#16a34a" stroke-width="2"/>
  <text x="590" y="140" text-anchor="middle" font-size="12" fill="#16a34a" font-weight="bold">Parametric</text>
  <text x="590" y="155" text-anchor="middle" font-size="10" fill="#16a34a">t-test / ANOVA</text>
  <!-- NO: skewed path -->
  <line x1="340" y1="162" x2="340" y2="195" stroke="#dc2626" stroke-width="1.5" marker-end="url(#arrowNP)"/>
  <text x="355" y="182" font-size="11" fill="#dc2626" font-weight="bold">No</text>
  <!-- Can transform? -->
  <rect x="228" y="195" width="224" height="42" rx="8" fill="#f3f4f6" stroke="#6b7280" stroke-width="1.5"/>
  <text x="340" y="215" text-anchor="middle" font-size="12" fill="#1e293b">Log-transform fixes it?</text>
  <text x="340" y="230" text-anchor="middle" font-size="10" fill="#6b7280">(common for fold-changes, concentrations)</text>
  <!-- YES: transform then parametric -->
  <line x1="452" y1="216" x2="520" y2="216" stroke="#16a34a" stroke-width="1.5" marker-end="url(#arrowNP)"/>
  <text x="480" y="209" font-size="11" fill="#16a34a" font-weight="bold">Yes</text>
  <rect x="520" y="198" width="140" height="36" rx="8" fill="#dcfce7" stroke="#16a34a" stroke-width="1.5"/>
  <text x="590" y="215" text-anchor="middle" font-size="11" fill="#16a34a" font-weight="bold">Transform, then</text>
  <text x="590" y="228" text-anchor="middle" font-size="11" fill="#16a34a">parametric</text>
  <!-- NO: non-parametric -->
  <line x1="340" y1="237" x2="340" y2="270" stroke="#dc2626" stroke-width="1.5" marker-end="url(#arrowNP)"/>
  <text x="355" y="258" font-size="11" fill="#dc2626" font-weight="bold">No</text>
  <!-- How many groups? -->
  <rect x="228" y="270" width="224" height="36" rx="8" fill="#f3f4f6" stroke="#6b7280" stroke-width="1.5"/>
  <text x="340" y="293" text-anchor="middle" font-size="12" fill="#1e293b">How many groups?</text>
  <!-- 2 groups -->
  <line x1="228" y1="288" x2="130" y2="288" stroke="#374151" stroke-width="1.5" marker-end="url(#arrowNP)"/>
  <text x="185" y="282" font-size="10" fill="#6b7280">2 groups</text>
  <rect x="20" y="270" width="110" height="50" rx="8" fill="#fef3c7" stroke="#f59e0b" stroke-width="2"/>
  <text x="75" y="290" text-anchor="middle" font-size="11" fill="#92400e" font-weight="bold">Mann-Whitney</text>
  <text x="75" y="304" text-anchor="middle" font-size="10" fill="#92400e">(Wilcoxon signed-</text>
  <text x="75" y="315" text-anchor="middle" font-size="10" fill="#92400e">rank if paired)</text>
  <!-- 3+ groups -->
  <line x1="452" y1="288" x2="520" y2="288" stroke="#374151" stroke-width="1.5" marker-end="url(#arrowNP)"/>
  <text x="480" y="282" font-size="10" fill="#6b7280">3+ groups</text>
  <rect x="520" y="270" width="140" height="36" rx="8" fill="#fef3c7" stroke="#f59e0b" stroke-width="2"/>
  <text x="590" y="293" text-anchor="middle" font-size="11" fill="#92400e" font-weight="bold">Kruskal-Wallis</text>
  <!-- Small n? annotation -->
  <rect x="20" y="340" width="640" height="28" rx="6" fill="#f5f3ff" stroke="#7c3aed" stroke-width="1"/>
  <text x="340" y="358" text-anchor="middle" font-size="11" fill="#7c3aed">Tip: With very small n (&lt; 10), non-parametric tests are often safer regardless of apparent normality</text>
</svg>
</div>

## Decision Guide: Parametric vs Non-Parametric

| Comparison | Parametric | Non-Parametric |
|---|---|---|
| One sample vs known value | One-sample t-test | Wilcoxon signed-rank (one sample) |
| Two independent groups | Welch's t-test | Mann-Whitney U / Wilcoxon rank-sum |
| Two paired groups | Paired t-test | Wilcoxon signed-rank |
| Three+ independent groups | One-way ANOVA | Kruskal-Wallis |
| Three+ paired groups | Repeated measures ANOVA | Friedman test |
| Compare distributions | — | KS test |

## Non-Parametric Tests in BioLang

### Mann-Whitney U: Microbiome Abundance

```bio
# F. prausnitzii relative abundance (%) in IBD vs healthy
let ibd = [0.1, 0.3, 0.0, 0.8, 0.2, 0.0, 1.5, 0.4, 0.1, 0.0,
           3.2, 0.5, 0.1, 0.7, 0.0]
let healthy = [2.1, 5.4, 1.8, 8.2, 3.5, 12.1, 4.7, 6.3, 2.9, 45.0,
               7.1, 3.8, 9.5, 4.2, 6.8]

# First, demonstrate why t-test is inappropriate
# Check normality visually — both distributions are right-skewed
qq_plot(ibd, {title: "QQ Plot: IBD"})
qq_plot(healthy, {title: "QQ Plot: Healthy"})
print("Both groups are heavily skewed — normality violated!\n")

# Mann-Whitney U test (non-parametric)
let result = wilcoxon(ibd, healthy)
print("=== Mann-Whitney U Test ===")
print("U statistic: {result.statistic:.1}")
print("p-value: {result.p_value:.2e}")

# Compare to (inappropriate) t-test
let t_result = ttest(ibd, healthy)
print("\n(Inappropriate) Welch's t-test p-value: {t_result.p_value:.2e}")
print("Mann-Whitney p-value: {result.p_value:.2e}")
print("Results may differ substantially with skewed data")

# Visualize the skewed distributions
let bp_table = table({"IBD": ibd, "Healthy": healthy})
boxplot(bp_table, {title: "F. prausnitzii Abundance"})
```

### Wilcoxon Signed-Rank: Paired Treatment Data

```bio
# Inflammatory cytokine IL-6 (pg/mL) before and after anti-TNF therapy
# Same 12 patients measured twice — highly skewed cytokine data
let before = [245, 18, 892, 45, 32, 1250, 67, 128, 15, 543, 78, 2100]
let after  = [120, 12, 340, 22, 28, 450,  35,  65, 10, 210, 42,  890]

# Normality check on differences
let diffs = zip(before, after) |> map(|p| p[0] - p[1])
qq_plot(diffs, {title: "QQ Plot: Paired Differences"})
print("Differences are non-normal -> use Wilcoxon signed-rank\n")

# Wilcoxon signed-rank test
let result = wilcoxon(before, after)
print("=== Wilcoxon Signed-Rank Test ===")
print("V statistic: {result.statistic:.1}")
print("p-value: {result.p_value:.6}")
print("All 12 patients showed reduction in IL-6")

# For comparison: the sign test via dbinom (even more robust, less powerful)
# Count how many differences are positive
let n_pos = diffs |> filter(|d| d > 0) |> len()
let n_nonzero = diffs |> filter(|d| d != 0) |> len()
# Under H0, n_pos ~ Binomial(n_nonzero, 0.5)
let sign_p = 0.0
for k in range(n_pos, n_nonzero + 1) {
    sign_p = sign_p + dbinom(k, n_nonzero, 0.5)
}
let sign_p = 2.0 * min(sign_p, 1.0 - sign_p)  # two-tailed
print("\nSign test p-value: {sign_p:.6}")
```

### Kruskal-Wallis: Multiple Body Sites

```bio
# Bacterial diversity (Shannon index) across three gut regions
let ileum   = [1.2, 0.8, 1.5, 0.3, 2.1, 0.9, 1.4, 0.6, 1.8, 0.4]
let cecum   = [2.5, 3.1, 2.8, 2.2, 3.4, 2.7, 3.0, 2.3, 2.9, 3.2]
let rectum  = [3.8, 4.2, 3.5, 4.5, 3.9, 4.1, 3.6, 4.3, 3.7, 4.0]

# Kruskal-Wallis: use anova() on rank-transformed data
let result = anova([ileum, cecum, rectum])
print("=== Kruskal-Wallis Test: Diversity Across Body Sites ===")
print("H statistic: {result.statistic:.4}")
print("p-value: {result.p_value:.2e}")
print("Degrees of freedom: {result.df}")

if result.p_value < 0.05 {
  print("\nAt least one body site differs. Running pairwise comparisons...")

  let p1 = wilcoxon(ileum, cecum).p_value
  let p2 = wilcoxon(ileum, rectum).p_value
  let p3 = wilcoxon(cecum, rectum).p_value

  # Bonferroni correction for 3 comparisons
  let adjusted = p_adjust([p1, p2, p3], "bonferroni")
  print("Ileum vs Cecum:  p = {adjusted[0]:.4}")
  print("Ileum vs Rectum: p = {adjusted[1]:.4}")
  print("Cecum vs Rectum: p = {adjusted[2]:.4}")
}

let bp_table = table({"Ileum": ileum, "Cecum": cecum, "Rectum": rectum})
boxplot(bp_table, {title: "Microbial Diversity by Gut Region"})
```

### KS Test: Comparing Distributions

```bio
# Do tumor suppressor genes and oncogenes have different
# expression distributions (not just different means)?
let tumor_suppressors = [2.1, 3.4, 1.8, 4.2, 2.9, 3.1, 2.5, 3.8, 1.5, 4.0,
                         2.7, 3.3, 2.0, 3.6, 2.3, 3.9, 1.9, 4.1, 2.6, 3.5]
let oncogenes = [5.2, 8.1, 6.3, 12.4, 7.5, 5.8, 9.2, 6.7, 11.3, 7.0,
                 5.5, 8.8, 6.1, 10.5, 7.3, 5.9, 9.7, 6.5, 11.8, 7.8]

let result = ks_test(tumor_suppressors, oncogenes)
print("=== KS Test: Expression Distributions ===")
print("D statistic: {result.statistic:.4}")
print("p-value: {result.p_value:.2e}")
print("Maximum distance between cumulative distributions: {result.statistic:.4}")

histogram([tumor_suppressors, oncogenes], {labels: ["Tumor Suppressors", "Oncogenes"], title: "Expression Distributions by Gene Class", x_label: "Expression (log2-CPM)", bins: 12})
```

### Comparing t-Test vs Wilcoxon on the Same Data

```bio
set_seed(42)
# Demonstrate: with normal data, both tests agree
# With skewed data, they can disagree

print("=== Normal Data: Both Tests Agree ===")
let norm_a = rnorm(20, 5.0, 1.0)
let norm_b = rnorm(20, 6.0, 1.0)
let t_p = ttest(norm_a, norm_b).p_value
let w_p = wilcoxon(norm_a, norm_b).p_value
print("t-test p = {t_p:.4}, Mann-Whitney p = {w_p:.4}")

print("\n=== Skewed Data with Outlier: Tests May Disagree ===")
let skew_a = [1.2, 1.5, 1.8, 1.1, 1.4, 1.6, 1.3, 1.7, 1.9, 50.0]
let skew_b = [2.1, 2.3, 2.5, 2.0, 2.4, 2.2, 2.6, 2.1, 2.3, 2.5]
let t_p2 = ttest(skew_a, skew_b).p_value
let w_p2 = wilcoxon(skew_a, skew_b).p_value
print("t-test p = {t_p2:.4}, Mann-Whitney p = {w_p2:.4}")
print("The outlier inflates the t-test mean, masking the real pattern")
```

**Python:**

```python
from scipy import stats

ibd = [0.1, 0.3, 0.0, 0.8, 0.2, 0.0, 1.5, 0.4, 0.1, 0.0,
       3.2, 0.5, 0.1, 0.7, 0.0]
healthy = [2.1, 5.4, 1.8, 8.2, 3.5, 12.1, 4.7, 6.3, 2.9, 45.0,
           7.1, 3.8, 9.5, 4.2, 6.8]

# Mann-Whitney U
u, p = stats.mannwhitneyu(ibd, healthy, alternative='two-sided')
print(f"U = {u}, p = {p:.2e}")

# Wilcoxon signed-rank (paired)
before = [245, 18, 892, 45, 32, 1250, 67, 128, 15, 543, 78, 2100]
after  = [120, 12, 340, 22, 28, 450,  35,  65, 10, 210, 42,  890]
w, p = stats.wilcoxon(before, after)
print(f"W = {w}, p = {p:.6f}")

# Kruskal-Wallis
ileum  = [1.2, 0.8, 1.5, 0.3, 2.1, 0.9, 1.4, 0.6, 1.8, 0.4]
cecum  = [2.5, 3.1, 2.8, 2.2, 3.4, 2.7, 3.0, 2.3, 2.9, 3.2]
rectum = [3.8, 4.2, 3.5, 4.5, 3.9, 4.1, 3.6, 4.3, 3.7, 4.0]
h, p = stats.kruskal(ileum, cecum, rectum)
print(f"H = {h:.4f}, p = {p:.2e}")
```

**R:**

```r
ibd <- c(0.1, 0.3, 0.0, 0.8, 0.2, 0.0, 1.5, 0.4, 0.1, 0.0,
         3.2, 0.5, 0.1, 0.7, 0.0)
healthy <- c(2.1, 5.4, 1.8, 8.2, 3.5, 12.1, 4.7, 6.3, 2.9, 45.0,
             7.1, 3.8, 9.5, 4.2, 6.8)

wilcox.test(ibd, healthy)           # Mann-Whitney
wilcox.test(before, after, paired = TRUE)  # Wilcoxon signed-rank
kruskal.test(list(ileum, cecum, rectum))   # Kruskal-Wallis
ks.test(tumor_suppressors, oncogenes)      # KS test
```

## Exercises

**Exercise 1: Choose the Right Test**

For each dataset, decide whether a parametric or non-parametric test is more appropriate:

a) Pain scores (0-10 scale) in drug vs placebo groups
b) Blood pressure measurements in 30 patients (continuous, approximately normal)
c) Number of bacterial colonies per plate (many zeros, some very high counts)
d) Survival time in days (typically right-skewed)

**Exercise 2: Microbiome Comparison**

Two diets were compared for their effect on *Bacteroides* abundance:

```bio
let high_fiber = [8.2, 12.5, 6.3, 15.1, 9.8, 22.4, 7.5, 11.2, 14.8, 5.9]
let low_fiber  = [1.2, 0.5, 3.1, 0.8, 2.4, 0.3, 1.8, 0.9, 2.7, 0.6]

# TODO: Test normality of each group
# TODO: Run Mann-Whitney U test
# TODO: Also run a t-test and compare results
# TODO: Create a boxplot
```

**Exercise 3: Multiple Body Sites with Post-Hoc**

OTU richness from four body sites (oral, gut, skin, vaginal). Run Kruskal-Wallis and, if significant, perform all pairwise comparisons with Bonferroni correction.

```bio
let oral    = [120, 95, 145, 110, 88, 132, 105, 98, 140, 115]
let gut     = [350, 420, 280, 390, 310, 445, 360, 295, 410, 380]
let skin    = [180, 210, 165, 195, 220, 175, 200, 185, 230, 190]
let vaginal = [45, 30, 55, 38, 25, 50, 42, 35, 48, 28]

# TODO: Kruskal-Wallis test
# TODO: If significant, pairwise Mann-Whitney with Bonferroni correction
# TODO: Which sites differ from which?
```

**Exercise 4: The Power Trade-Off**

Generate 1000 simulations where both groups are truly normal with different means. Compare how often the t-test and Mann-Whitney detect the difference (power). Then repeat with skewed data (e.g., exponential).

```bio

# TODO: Simulate normal data, compare t-test vs Mann-Whitney power
# TODO: Simulate skewed data, compare again
# TODO: Which test wins in each scenario?
```

## Key Takeaways

- **Non-parametric tests** use ranks instead of raw values, making them robust to skewness and outliers
- The **Mann-Whitney U** (Wilcoxon rank-sum) is the non-parametric alternative to the independent t-test
- The **Wilcoxon signed-rank** test is the non-parametric alternative to the paired t-test
- The **Kruskal-Wallis** test extends to three or more groups (non-parametric ANOVA)
- The **KS test** compares entire distributions, not just central tendency
- Non-parametric tests have about 95% of the power of parametric tests when data ARE normal, but are far more reliable when data are NOT normal
- Microbiome data, cytokine levels, survival times, and ordinal scales almost always require non-parametric methods
- Always check normality first (Shapiro-Wilk, QQ plots) — let the data guide your choice of test

## What's Next

So far we have compared two groups. But what if you have three, four, or ten groups — different drug doses, tissue types, or experimental conditions? Running all pairwise t-tests inflates false positives dramatically. Tomorrow we introduce ANOVA, the principled way to compare many groups simultaneously, along with post-hoc tests that identify *which* groups differ.
