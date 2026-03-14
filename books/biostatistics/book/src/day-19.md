# Day 19: Effect Sizes — Beyond p-Values

## The Problem

Two papers land on Dr. Rachel Nguyen's desk the same morning.

**Paper A:** "GENE_X is significantly differentially expressed between tumor and normal tissue (p = 0.00001, n = 5,000)." She looks at the supplementary data: the mean difference is **0.02 FPKM** on a scale where genes range from 0 to 50,000 FPKM. The fold change is 1.001.

**Paper B:** "DRUG_Y showed a non-significant trend toward tumor reduction (p = 0.08, n = 24)." She looks at the data: the median tumor volume shrank by **40%** in the treatment arm.

Paper A is "highly significant" but biologically meaningless — the difference is lost in measurement noise. Paper B fails the significance threshold but describes a potentially life-changing clinical effect that just needs more patients.

**The p-value alone tells you almost nothing.** You need **effect sizes**.

<div style="text-align: center; margin: 2em 0;">
<svg width="660" height="360" viewBox="0 0 660 360" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="330" y="24" text-anchor="middle" font-size="14" font-weight="bold" fill="#1e293b">P-Value vs. Effect Size: They Measure Different Things</text>
  <g transform="translate(80, 45)">
    <!-- Axes -->
    <line x1="60" y1="260" x2="480" y2="260" stroke="#6b7280" stroke-width="1.5"/>
    <line x1="60" y1="260" x2="60" y2="20" stroke="#6b7280" stroke-width="1.5"/>
    <text x="270" y="295" text-anchor="middle" font-size="12" fill="#6b7280">Effect Size (Cohen's d)</text>
    <text x="15" y="140" text-anchor="middle" font-size="12" fill="#6b7280" transform="rotate(-90, 15, 140)">-log10(p-value)</text>
    <!-- Y-axis: significance scale -->
    <text x="52" y="264" text-anchor="end" font-size="10" fill="#6b7280">0</text>
    <text x="52" y="212" text-anchor="end" font-size="10" fill="#6b7280">1</text>
    <text x="52" y="164" text-anchor="end" font-size="10" fill="#6b7280">2</text>
    <text x="52" y="116" text-anchor="end" font-size="10" fill="#6b7280">3</text>
    <text x="52" y="68" text-anchor="end" font-size="10" fill="#6b7280">4</text>
    <text x="52" y="24" text-anchor="end" font-size="10" fill="#6b7280">5</text>
    <!-- X-axis -->
    <text x="60" y="278" text-anchor="middle" font-size="10" fill="#6b7280">0</text>
    <text x="165" y="278" text-anchor="middle" font-size="10" fill="#6b7280">0.25</text>
    <text x="270" y="278" text-anchor="middle" font-size="10" fill="#6b7280">0.50</text>
    <text x="375" y="278" text-anchor="middle" font-size="10" fill="#6b7280">0.75</text>
    <text x="480" y="278" text-anchor="middle" font-size="10" fill="#6b7280">1.00</text>
    <!-- Significance threshold line (p=0.05 => -log10 = 1.3) -->
    <line x1="60" y1="197" x2="480" y2="197" stroke="#dc2626" stroke-width="1" stroke-dasharray="5,3"/>
    <text x="485" y="195" font-size="9" fill="#dc2626">p = 0.05</text>
    <!-- Quadrant shading -->
    <!-- Top-left: significant but small effect (BAD) -->
    <rect x="60" y="20" width="150" height="177" fill="#fef2f2" opacity="0.3"/>
    <!-- Top-right: significant and large effect (GOOD) -->
    <rect x="210" y="20" width="270" height="177" fill="#f0fdf4" opacity="0.3"/>
    <!-- Bottom-right: not significant but large effect (underpowered) -->
    <rect x="210" y="197" width="270" height="63" fill="#fffbeb" opacity="0.3"/>
    <!-- Quadrant labels -->
    <text x="135" y="50" text-anchor="middle" font-size="10" fill="#dc2626" font-weight="bold">Significant</text>
    <text x="135" y="62" text-anchor="middle" font-size="10" fill="#dc2626" font-weight="bold">but trivial</text>
    <text x="135" y="77" text-anchor="middle" font-size="9" fill="#dc2626">(large n inflates p)</text>
    <text x="380" y="50" text-anchor="middle" font-size="10" fill="#16a34a" font-weight="bold">Significant AND</text>
    <text x="380" y="62" text-anchor="middle" font-size="10" fill="#16a34a" font-weight="bold">meaningful</text>
    <text x="380" y="230" text-anchor="middle" font-size="10" fill="#b45309" font-weight="bold">Large effect but</text>
    <text x="380" y="242" text-anchor="middle" font-size="10" fill="#b45309" font-weight="bold">not significant</text>
    <text x="380" y="257" text-anchor="middle" font-size="9" fill="#b45309">(needs more samples)</text>
    <!-- Scatter: large n, tiny effect (top-left) -->
    <circle cx="85" cy="60" r="7" fill="#dc2626" opacity="0.5"/>
    <text x="100" y="58" font-size="8" fill="#dc2626">n=5000</text>
    <circle cx="105" cy="80" r="6" fill="#dc2626" opacity="0.4"/>
    <circle cx="130" cy="100" r="5" fill="#dc2626" opacity="0.4"/>
    <!-- Scatter: good studies (top-right) -->
    <circle cx="310" cy="90" r="6" fill="#16a34a" opacity="0.5"/>
    <circle cx="370" cy="50" r="7" fill="#16a34a" opacity="0.5"/>
    <circle cx="420" cy="70" r="6" fill="#16a34a" opacity="0.5"/>
    <circle cx="280" cy="120" r="5" fill="#16a34a" opacity="0.5"/>
    <circle cx="350" cy="110" r="5" fill="#16a34a" opacity="0.5"/>
    <!-- Scatter: underpowered (bottom-right) -->
    <circle cx="400" cy="230" r="6" fill="#b45309" opacity="0.5"/>
    <text x="412" y="220" font-size="8" fill="#b45309">n=12</text>
    <circle cx="320" cy="240" r="5" fill="#b45309" opacity="0.4"/>
    <circle cx="450" cy="220" r="5" fill="#b45309" opacity="0.4"/>
  </g>
</svg>
</div>

## The Tyranny of p-Values

In 2016, the American Statistical Association took the extraordinary step of issuing a formal statement on p-values — the first time in its 177-year history. Key points:

1. P-values do NOT measure the probability that the hypothesis is true
2. P-values do NOT measure the size or importance of an effect
3. Scientific conclusions should NOT be based only on whether p < 0.05
4. A p-value near 0.05 provides only weak evidence against the null

> **Key insight:** A p-value is a function of **effect size** AND **sample size**. With enough data, any trivial difference becomes "significant." With too few data, any real effect becomes "non-significant." The p-value alone is fundamentally incomplete.

### The Problem of Large n

| True Effect Size | n per group | p-value | Significant? | Meaningful? |
|-----------------|-------------|---------|-------------|------------|
| d = 0.01 (trivial) | 50,000 | 0.02 | Yes | No |
| d = 0.8 (large) | 5 | 0.12 | No | Yes |
| d = 0.5 (medium) | 64 | 0.04 | Yes | Likely |
| d = 0.3 (small) | 30 | 0.15 | No | Maybe |

## What Are Effect Sizes?

An effect size quantifies **how large** a difference or association is, independent of sample size. There are two families:

### Standardized Effect Sizes (unit-free)

| Metric | Used For | Scale |
|--------|----------|-------|
| Cohen's d | Two-group mean difference | 0.2 small, 0.5 medium, 0.8 large |
| Odds Ratio (OR) | Binary outcome association | 1.0 = no effect |
| Relative Risk (RR) | Binary outcome risk | 1.0 = no effect |
| Cramér's V | Categorical association | 0 to 1 |
| Eta-squared (η²) | ANOVA variance explained | 0 to 1 |
| R² | Regression variance explained | 0 to 1 |

### Unstandardized Effect Sizes (original units)

| Metric | Example |
|--------|---------|
| Mean difference | "Treatment reduces tumor volume by 2.3 cm³" |
| Regression slope | "Each year of age increases risk by 3%" |
| Median survival difference | "Treatment arm survived 6 months longer" |

> **Key insight:** Unstandardized effect sizes are often MORE useful than standardized ones because they're in meaningful units. "The drug reduced blood pressure by 8 mmHg" is more interpretable than "Cohen's d = 0.5."

## Cohen's d: Standardized Mean Difference

$$d = \frac{\bar{X}_1 - \bar{X}_2}{s_{pooled}}$$

where the pooled standard deviation is:

$$s_{pooled} = \sqrt{\frac{(n_1-1)s_1^2 + (n_2-1)s_2^2}{n_1 + n_2 - 2}}$$

Cohen's benchmarks (widely used, sometimes criticized as arbitrary):

| d | Label | What It Means |
|---|-------|---------------|
| 0.2 | Small | Groups overlap ~85% |
| 0.5 | Medium | Groups overlap ~67% |
| 0.8 | Large | Groups overlap ~53% |
| 1.2 | Very large | Groups overlap ~40% |
| 2.0 | Huge | Groups barely overlap |

> **Common pitfall:** Cohen himself warned that "small/medium/large" are context-dependent. In pharmacogenomics, d = 0.3 might be clinically important. In quality control, d = 2.0 might be necessary. Always judge effect sizes in your domain context.

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="340" viewBox="0 0 680 340" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="24" text-anchor="middle" font-size="14" font-weight="bold" fill="#1e293b">Cohen's d: How Much Do Distributions Overlap?</text>
  <!-- Top row: d=0.2 (small) -->
  <g transform="translate(30, 45)">
    <text x="150" y="12" text-anchor="middle" font-size="12" font-weight="bold" fill="#6b7280">d = 0.2 (Small Effect)</text>
    <!-- Axis -->
    <line x1="20" y1="120" x2="280" y2="120" stroke="#9ca3af" stroke-width="0.5"/>
    <!-- Distribution A -->
    <path d="M 20,120 C 40,118 60,108 80,88 C 100,62 120,40 140,32 C 160,40 180,62 200,88 C 220,108 240,118 260,120" fill="#2563eb" opacity="0.2" stroke="#2563eb" stroke-width="1.5"/>
    <!-- Distribution B (shifted slightly) -->
    <path d="M 40,120 C 60,118 80,108 100,88 C 120,62 140,40 160,32 C 180,40 200,62 220,88 C 240,108 260,118 280,120" fill="#dc2626" opacity="0.2" stroke="#dc2626" stroke-width="1.5"/>
    <!-- Overlap annotation -->
    <text x="150" y="140" text-anchor="middle" font-size="10" fill="#6b7280">~85% overlap</text>
    <!-- Means -->
    <line x1="140" y1="28" x2="140" y2="120" stroke="#2563eb" stroke-width="1" stroke-dasharray="3,2" opacity="0.5"/>
    <line x1="160" y1="28" x2="160" y2="120" stroke="#dc2626" stroke-width="1" stroke-dasharray="3,2" opacity="0.5"/>
  </g>
  <!-- Top row: d=0.8 (large) -->
  <g transform="translate(370, 45)">
    <text x="150" y="12" text-anchor="middle" font-size="12" font-weight="bold" fill="#6b7280">d = 0.8 (Large Effect)</text>
    <!-- Axis -->
    <line x1="0" y1="120" x2="300" y2="120" stroke="#9ca3af" stroke-width="0.5"/>
    <!-- Distribution A -->
    <path d="M 0,120 C 15,118 30,108 50,88 C 70,62 90,40 110,32 C 130,40 150,62 170,88 C 190,108 210,118 230,120" fill="#2563eb" opacity="0.2" stroke="#2563eb" stroke-width="1.5"/>
    <!-- Distribution B (shifted far) -->
    <path d="M 70,120 C 85,118 100,108 120,88 C 140,62 160,40 180,32 C 200,40 220,62 240,88 C 260,108 275,118 300,120" fill="#dc2626" opacity="0.2" stroke="#dc2626" stroke-width="1.5"/>
    <!-- Overlap annotation -->
    <text x="150" y="140" text-anchor="middle" font-size="10" fill="#6b7280">~53% overlap</text>
    <!-- Means -->
    <line x1="110" y1="28" x2="110" y2="120" stroke="#2563eb" stroke-width="1" stroke-dasharray="3,2" opacity="0.5"/>
    <line x1="180" y1="28" x2="180" y2="120" stroke="#dc2626" stroke-width="1" stroke-dasharray="3,2" opacity="0.5"/>
  </g>
  <!-- Legend -->
  <g transform="translate(180, 200)">
    <rect x="0" y="0" width="320" height="24" rx="4" fill="white" stroke="#e5e7eb" stroke-width="0.5"/>
    <rect x="20" y="6" width="16" height="12" fill="#2563eb" opacity="0.4" rx="2"/>
    <text x="44" y="16" font-size="10" fill="#1e293b">Control group</text>
    <rect x="160" y="6" width="16" height="12" fill="#dc2626" opacity="0.4" rx="2"/>
    <text x="184" y="16" font-size="10" fill="#1e293b">Treatment group</text>
  </g>
  <!-- Bottom scale -->
  <g transform="translate(80, 238)">
    <line x1="0" y1="20" x2="520" y2="20" stroke="#6b7280" stroke-width="1.5"/>
    <line x1="0" y1="16" x2="0" y2="24" stroke="#6b7280" stroke-width="1.5"/>
    <line x1="130" y1="16" x2="130" y2="24" stroke="#6b7280" stroke-width="1.5"/>
    <line x1="260" y1="16" x2="260" y2="24" stroke="#6b7280" stroke-width="1.5"/>
    <line x1="390" y1="16" x2="390" y2="24" stroke="#6b7280" stroke-width="1.5"/>
    <line x1="520" y1="16" x2="520" y2="24" stroke="#6b7280" stroke-width="1.5"/>
    <text x="0" y="40" text-anchor="middle" font-size="10" fill="#6b7280">0.0</text>
    <text x="130" y="40" text-anchor="middle" font-size="10" fill="#6b7280">0.2</text>
    <text x="260" y="40" text-anchor="middle" font-size="10" fill="#6b7280">0.5</text>
    <text x="390" y="40" text-anchor="middle" font-size="10" fill="#6b7280">0.8</text>
    <text x="520" y="40" text-anchor="middle" font-size="10" fill="#6b7280">1.2+</text>
    <text x="65" y="55" text-anchor="middle" font-size="9" fill="#9ca3af">Negligible</text>
    <text x="195" y="55" text-anchor="middle" font-size="9" fill="#9ca3af">Small</text>
    <text x="325" y="55" text-anchor="middle" font-size="9" fill="#7c3aed" font-weight="bold">Medium</text>
    <text x="455" y="55" text-anchor="middle" font-size="9" fill="#2563eb" font-weight="bold">Large</text>
    <text x="260" y="72" text-anchor="middle" font-size="11" fill="#6b7280">Cohen's d Scale</text>
  </g>
</svg>
</div>

## Odds Ratio and Relative Risk

For binary outcomes (response/no response, mutation/wild-type), two key measures:

### Odds Ratio (OR)

$$OR = \frac{a \cdot d}{b \cdot c}$$

From a 2x2 table:

| | Outcome + | Outcome - |
|---|---|---|
| Exposed | a | b |
| Unexposed | c | d |

### Relative Risk (RR)

$$RR = \frac{a/(a+b)}{c/(c+d)}$$

### OR vs. RR: Why They Differ

| Baseline Risk | OR | RR | Interpretation |
|--------------|----|----|----------------|
| 1% (rare) | 2.0 | 2.0 | Nearly identical (rare disease approximation) |
| 10% | 2.0 | 1.8 | Starting to diverge |
| 30% | 2.0 | 1.5 | Substantial difference |
| 50% | 2.0 | 1.3 | OR greatly overstates RR |

> **Clinical relevance:** Case-control studies can only estimate OR (not RR). Cohort studies and RCTs can estimate both. When reporting to patients, RR or absolute risk difference is more intuitive: "Your risk goes from 10% to 15%" is clearer than "OR = 1.6."

## Cramér's V: Categorical Associations

For larger contingency tables (beyond 2x2), Cramér's V measures association strength:

$$V = \sqrt{\frac{\chi^2}{n \cdot (k-1)}}$$

where k = min(rows, columns).

| V | Interpretation |
|---|---------------|
| 0.0 | No association |
| 0.1 | Weak |
| 0.3 | Moderate |
| 0.5 | Strong |

## Eta-squared: ANOVA Effect Size

For ANOVA (comparing means across multiple groups):

$$\eta^2 = \frac{SS_{between}}{SS_{total}}$$

| η² | Interpretation |
|----|---------------|
| 0.01 | Small |
| 0.06 | Medium |
| 0.14 | Large |

Eta-squared tells you what fraction of total variance is explained by group membership.

## Forest Plots: The Standard Display

**Forest plots** are the gold standard for displaying effect sizes across multiple comparisons. Each row shows:
- A point estimate (the effect size)
- A horizontal line (the confidence interval)
- A vertical reference line (null effect: 0 for differences, 1 for ratios)

They're essential for:
- Meta-analyses (combining studies)
- Multi-gene comparisons (e.g., DE genes ranked by effect size)
- Subgroup analyses (effect by age, sex, stage)
- Cox regression hazard ratios

<div style="text-align: center; margin: 2em 0;">
<svg width="660" height="320" viewBox="0 0 660 320" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="330" y="24" text-anchor="middle" font-size="14" font-weight="bold" fill="#1e293b">Anatomy of a Forest Plot</text>
  <g transform="translate(20, 45)">
    <!-- Headers -->
    <text x="80" y="14" text-anchor="end" font-size="11" fill="#6b7280" font-weight="bold">Study</text>
    <text x="340" y="14" text-anchor="middle" font-size="11" fill="#6b7280" font-weight="bold">Effect Size (95% CI)</text>
    <text x="540" y="14" text-anchor="middle" font-size="11" fill="#6b7280" font-weight="bold">d [CI]</text>
    <line x1="0" y1="22" x2="620" y2="22" stroke="#e5e7eb" stroke-width="1"/>
    <!-- No-effect reference line -->
    <line x1="340" y1="25" x2="340" y2="218" stroke="#1e293b" stroke-width="1.5" stroke-dasharray="4,3"/>
    <!-- Axis -->
    <line x1="180" y1="218" x2="500" y2="218" stroke="#9ca3af" stroke-width="0.5"/>
    <text x="180" y="234" text-anchor="middle" font-size="9" fill="#9ca3af">-1.0</text>
    <text x="260" y="234" text-anchor="middle" font-size="9" fill="#9ca3af">-0.5</text>
    <text x="340" y="234" text-anchor="middle" font-size="9" fill="#9ca3af">0</text>
    <text x="420" y="234" text-anchor="middle" font-size="9" fill="#9ca3af">+0.5</text>
    <text x="500" y="234" text-anchor="middle" font-size="9" fill="#9ca3af">+1.0</text>
    <!-- Annotations -->
    <!-- Point estimate annotation -->
    <line x1="445" y1="45" x2="480" y2="32" stroke="#6b7280" stroke-width="0.5"/>
    <text x="485" y="35" font-size="9" fill="#6b7280" font-style="italic">Point estimate</text>
    <!-- CI line annotation -->
    <line x1="285" y1="45" x2="260" y2="32" stroke="#6b7280" stroke-width="0.5"/>
    <text x="220" y="35" font-size="9" fill="#6b7280" font-style="italic">Confidence interval</text>
    <!-- Study 1: BRCA1, d=0.65 [0.2, 1.1] -->
    <text x="80" y="52" text-anchor="end" font-size="11" fill="#1e293b">BRCA1</text>
    <line x1="372" y1="48" x2="516" y2="48" stroke="#2563eb" stroke-width="2"/>
    <rect x="440" y="42" width="10" height="10" fill="#2563eb" rx="1"/>
    <text x="540" y="52" text-anchor="middle" font-size="10" fill="#1e293b">0.65 [0.20, 1.10]</text>
    <!-- Study 2: TP53, d=0.82 [0.45, 1.19] -->
    <text x="80" y="82" text-anchor="end" font-size="11" fill="#1e293b">TP53</text>
    <line x1="412" y1="78" x2="530" y2="78" stroke="#2563eb" stroke-width="2"/>
    <rect x="471" y="72" width="12" height="12" fill="#2563eb" rx="1"/>
    <text x="540" y="82" text-anchor="middle" font-size="10" fill="#1e293b">0.82 [0.45, 1.19]</text>
    <!-- Study 3: EGFR, d=0.15 [-0.3, 0.6] — crosses zero -->
    <text x="80" y="112" text-anchor="end" font-size="11" fill="#1e293b">EGFR</text>
    <line x1="292" y1="108" x2="436" y2="108" stroke="#9ca3af" stroke-width="2"/>
    <rect x="362" y="103" width="8" height="8" fill="#9ca3af" rx="1"/>
    <text x="540" y="112" text-anchor="middle" font-size="10" fill="#9ca3af">0.15 [-0.30, 0.60]</text>
    <!-- Study 4: KRAS, d=0.45 [0.05, 0.85] -->
    <text x="80" y="142" text-anchor="end" font-size="11" fill="#1e293b">KRAS</text>
    <line x1="348" y1="138" x2="476" y2="138" stroke="#2563eb" stroke-width="2"/>
    <rect x="412" y="133" width="9" height="9" fill="#2563eb" rx="1"/>
    <text x="540" y="142" text-anchor="middle" font-size="10" fill="#1e293b">0.45 [0.05, 0.85]</text>
    <!-- Study 5: MYC, d=-0.10 [-0.55, 0.35] — crosses zero -->
    <text x="80" y="172" text-anchor="end" font-size="11" fill="#1e293b">MYC</text>
    <line x1="252" y1="168" x2="396" y2="168" stroke="#9ca3af" stroke-width="2"/>
    <rect x="322" y="163" width="8" height="8" fill="#9ca3af" rx="1"/>
    <text x="540" y="172" text-anchor="middle" font-size="10" fill="#9ca3af">-0.10 [-0.55, 0.35]</text>
    <!-- Pooled diamond -->
    <polygon points="340,200 380,208 340,216 300,208" fill="#7c3aed" opacity="0.6" stroke="#7c3aed" stroke-width="1"/>
    <text x="80" y="212" text-anchor="end" font-size="11" fill="#7c3aed" font-weight="bold">Pooled</text>
    <text x="540" y="212" text-anchor="middle" font-size="10" fill="#7c3aed" font-weight="bold">0.40 [0.20, 0.60]</text>
    <!-- Labels below -->
    <text x="340" y="252" text-anchor="middle" font-size="10" fill="#1e293b">No effect (d = 0)</text>
    <text x="240" y="262" text-anchor="middle" font-size="9" fill="#6b7280">Favors control</text>
    <text x="440" y="262" text-anchor="middle" font-size="9" fill="#6b7280">Favors treatment</text>
  </g>
</svg>
</div>

## The Reporting Checklist

Every statistical result should include all four pieces:

| Element | What It Answers | Example |
|---------|----------------|---------|
| **Effect size** | How large? | d = 0.72 |
| **Confidence interval** | How precise? | 95% CI [0.35, 1.09] |
| **p-value** | Could it be chance? | p = 0.003 |
| **Sample size** | How much data? | n = 45 per group |

Omitting any one of these leaves the reader unable to fully evaluate the finding.

## Effect Sizes in BioLang

### Cohen's d for Gene Expression

```bio
set_seed(42)
# Compare expression of a gene between tumor and normal
let n = 50

let tumor = rnorm(n, 12.5, 3.0)
let normal_expr = rnorm(n, 10.0, 2.8)

# Cohen's d — compute inline
let pooled_sd = sqrt((variance(tumor) + variance(normal_expr)) / 2.0)
let d = (mean(tumor) - mean(normal_expr)) / pooled_sd
print("=== Cohen's d ===")
print("d = {d |> round(3)}")

let interpretation = if abs(d) >= 0.8 { "large" }
    else if abs(d) >= 0.5 { "medium" }
    else if abs(d) >= 0.2 { "small" }
    else { "negligible" }

print("Interpretation: {interpretation} effect")
print("")

# Also report the raw difference
let mean_diff = mean(tumor) - mean(normal_expr)
print("Mean difference: {mean_diff |> round(2)} FPKM")
print("This means tumor expression is ~{(mean_diff / mean(normal_expr) * 100) |> round(0)}% higher")

# Complete report: effect + CI + p + n
let t = ttest(tumor, normal_expr)
print("\n=== Complete Report ===")
print("Cohen's d = {d |> round(2)}")
print("p = {t.p_value |> round(4)}, n = {n} per group")
```

### Odds Ratio and Relative Risk

```bio
# Immunotherapy response by PD-L1 status

# 2x2 table:
#              Respond  Non-respond
# PD-L1 high     35        15        (70% response)
# PD-L1 low      20        30        (40% response)

let a = 35  # PD-L1 high + respond
let b = 15  # PD-L1 high + non-respond
let c = 20  # PD-L1 low + respond
let d_val = 30  # PD-L1 low + non-respond

# Odds ratio — compute inline
let or_val = (a * d_val) / (b * c)
print("=== Odds Ratio ===")
print("OR = {or_val |> round(2)}")

# Relative risk — compute inline
let risk_high = a / (a + b)
let risk_low = c / (c + d_val)
let rr_val = risk_high / risk_low
print("\n=== Relative Risk ===")
print("RR = {rr_val |> round(2)}")

# Absolute risk difference
let ard = risk_high - risk_low
print("\n=== Absolute Risk Difference ===")
print("Risk (PD-L1 high): {(risk_high * 100) |> round(1)}%")
print("Risk (PD-L1 low):  {(risk_low * 100) |> round(1)}%")
print("Absolute difference: {(ard * 100) |> round(1)} percentage points")
print("NNT: {(1 / ard) |> round(1)} (treat this many to get 1 extra responder)")

# Fisher's exact test for significance
let fe = fisher_exact(a, b, c, d_val)
print("Fisher's exact p-value: {fe.p_value |> round(4)}")

# Note: OR overstates the relative risk when the outcome is common
```

### Cramér's V for Categorical Data

```bio
# Association between tumor subtype (4 types) and treatment response (3 levels)
# observed counts in a 4x3 contingency table
let observed = [
    [30, 15, 5],   # Luminal A
    [20, 20, 10],  # Luminal B
    [10, 15, 25],  # HER2+
    [5, 10, 35]    # Triple-neg
]

# Flatten for chi-square test
let obs_flat = [30, 15, 5, 20, 20, 10, 10, 15, 25, 5, 10, 35]
let total = obs_flat |> sum
let chi2 = chi_square(obs_flat, obs_flat |> map(|x| total / len(obs_flat)))

# Compute Cramer's V inline
let n_obs = total
let k = 3  # min(rows, cols)
let v = sqrt(chi2.statistic / (n_obs * (k - 1)))

print("=== Cramer's V ===")
print("Chi-square: {chi2.statistic |> round(2)}")
print("p-value: {chi2.p_value |> round(4)}")
print("Cramer's V: {v |> round(3)}")
print("Interpretation: {if v > 0.3 { "moderate to strong" } else { "weak" }} association")
```

### Eta-squared for ANOVA

```bio
set_seed(42)
# Expression differences across 4 cancer subtypes
let subtype_a = rnorm(30, 10, 3)
let subtype_b = rnorm(30, 12, 3)
let subtype_c = rnorm(30, 11, 3)
let subtype_d = rnorm(30, 15, 3)

let aov = anova([subtype_a, subtype_b, subtype_c, subtype_d])

# Compute eta-squared inline from ANOVA output
# eta2 = SS_between / SS_total
let eta2 = aov.ss_between / aov.ss_total

print("=== Eta-squared (ANOVA Effect Size) ===")
print("F = {aov.f_statistic |> round(2)}, p = {aov.p_value |> round(4)}")
print("eta2 = {eta2 |> round(3)}")
print("{(eta2 * 100) |> round(1)}% of expression variance is explained by subtype")

let eta_interp = if eta2 >= 0.14 { "large" }
    else if eta2 >= 0.06 { "medium" }
    else { "small" }
print("Interpretation: {eta_interp} effect")
```

### Forest Plot for Multiple Genes

```bio
set_seed(42)
# Cohen's d for 10 differentially expressed genes
let gene_names = ["BRCA1", "TP53", "EGFR", "KRAS", "MYC",
                  "PTEN", "PIK3CA", "BRAF", "CDH1", "RB1"]
let effect_sizes = []
let ci_lowers = []
let ci_uppers = []

for i in 0..10 {
    let true_effect = rnorm(1, 0.5, 0.4)[0]
    let tumor_g = rnorm(40, 10 + true_effect, 2)
    let normal_g = rnorm(40, 10, 2)

    # Compute Cohen's d inline
    let pooled = sqrt((variance(tumor_g) + variance(normal_g)) / 2.0)
    let d = (mean(tumor_g) - mean(normal_g)) / pooled
    let se_d = sqrt(2 / 40 + d ** 2 / (4 * 40))

    effect_sizes = effect_sizes + [d]
    ci_lowers = ci_lowers + [d - 1.96 * se_d]
    ci_uppers = ci_uppers + [d + 1.96 * se_d]
}

# Forest plot — the standard effect size visualization
let forest_data = table({
    "gene": gene_names,
    "effect": effect_sizes,
    "ci_lower": ci_lowers,
    "ci_upper": ci_uppers
})
forest_plot(forest_data)
```

### Contrasting "Significant Tiny" vs. "Non-Significant Large"

```bio
set_seed(42)
# The key lesson: significance does not equal importance

# Scenario 1: huge n, tiny effect
let n_large = 5000
let group1_large = rnorm(n_large, 10.000, 2)
let group2_large = rnorm(n_large, 10.020, 2)

let t_large = ttest(group1_large, group2_large)
let pooled_lg = sqrt((variance(group1_large) + variance(group2_large)) / 2.0)
let d_large = (mean(group1_large) - mean(group2_large)) / pooled_lg

print("=== Scenario 1: Large n, Tiny Effect ===")
print("n = {n_large} per group")
print("Mean difference: {(mean(group1_large) - mean(group2_large)) |> abs |> round(4)}")
print("Cohen's d: {d_large |> round(4)}")
print("p-value: {t_large.p_value |> round(6)}")
print("Significant? {if t_large.p_value < 0.05 { "YES" } else { "NO" }}")
print("Biologically meaningful? VERY UNLIKELY (d ~ 0.01)")

# Scenario 2: small n, large effect
let n_small = 12
let group1_small = rnorm(n_small, 10, 3)
let group2_small = rnorm(n_small, 13, 3)

let t_small = ttest(group1_small, group2_small)
let pooled_sm = sqrt((variance(group1_small) + variance(group2_small)) / 2.0)
let d_small = (mean(group1_small) - mean(group2_small)) / pooled_sm

print("\n=== Scenario 2: Small n, Large Effect ===")
print("n = {n_small} per group")
print("Mean difference: {(mean(group1_small) - mean(group2_small)) |> abs |> round(2)}")
print("Cohen's d: {d_small |> round(2)}")
print("p-value: {t_small.p_value |> round(4)}")
print("Significant? {if t_small.p_value < 0.05 { "YES" } else { "NO" }}")
print("Biologically meaningful? LIKELY (d ~ 1.0) — needs more samples")

print("\n=== Lesson ===")
print("Scenario 1 is 'significant' but meaningless")
print("Scenario 2 is 'non-significant' but potentially important")
print("ALWAYS report effect sizes alongside p-values")
```

**Python:**

```python
from scipy.stats import norm
import numpy as np

# Cohen's d
def cohens_d(x, y):
    nx, ny = len(x), len(y)
    pooled_sd = np.sqrt(((nx-1)*np.var(x, ddof=1) + (ny-1)*np.var(y, ddof=1)) / (nx+ny-2))
    return (np.mean(x) - np.mean(y)) / pooled_sd

# Odds ratio (from statsmodels)
from statsmodels.stats.contingency_tables import Table2x2
table = np.array([[35, 15], [20, 30]])
t = Table2x2(table)
print(f"OR = {t.oddsratio:.2f}, 95% CI: {t.oddsratio_confint()}")
print(f"RR = {t.riskratio:.2f}")

# Cramér's V
from scipy.stats import chi2_contingency
chi2, p, dof, expected = chi2_contingency(table)
n = table.sum()
cramers_v = np.sqrt(chi2 / (n * (min(table.shape) - 1)))

# Forest plot (using matplotlib)
import matplotlib.pyplot as plt
plt.errorbar(effect_sizes, range(len(genes)), xerr=ci_widths, fmt='o')
plt.axvline(0, color='black', linestyle='--')
```

**R:**

```r
# Cohen's d
library(effectsize)
cohens_d(tumor ~ group)

# Odds ratio
library(epitools)
oddsratio(table)
riskratio(table)

# Cramér's V
library(rcompanion)
cramerV(table)

# Eta-squared
library(effectsize)
eta_squared(aov_result)

# Forest plot
library(forestplot)
forestplot(labeltext = gene_names,
           mean = effect_sizes,
           lower = ci_lower,
           upper = ci_upper)
```

## Exercises

### Exercise 1: Cohen's d Across Genes

Compute Cohen's d for 20 genes comparing tumor vs. normal. Rank them by effect size and create a forest plot. Which genes have the largest biological impact (not just the smallest p-value)?

```bio
let n = 30

# Generate 20 genes with varying true effects
# Some have large effects, some small, some zero
# 1. Compute Cohen's d inline for each gene
# 2. Compute p-value via ttest() for each gene
# 3. Create a forest_plot() sorted by effect size
# 4. Find a gene that is significant (p < 0.05) but has d < 0.2
# 5. Find a gene that is non-significant but has d > 0.5
```

### Exercise 2: OR vs. RR

Create three 2x2 tables where the outcome prevalence is 5%, 30%, and 60%. Set the true relative risk to 2.0 in all three. Show that OR ≈ RR when prevalence is low but OR >> RR when prevalence is high.

```bio
# Table 1: rare outcome (5% baseline)
# Table 2: moderate outcome (30% baseline)
# Table 3: common outcome (60% baseline)

# For each: compute OR = (a*d)/(b*c) and RR = (a/(a+b))/(c/(c+d))
# Show the divergence as prevalence increases
# Which metric should you report to patients?
```

### Exercise 3: Complete Reporting

Given the following analysis, write a complete results paragraph with effect size, CI, p-value, and sample size. Then write it again omitting the effect size — notice how much information is lost.

```bio
set_seed(42)
let treatment = rnorm(45, 15, 4)
let control = rnorm(45, 12, 4)

# Compute: ttest(), Cohen's d inline, mean difference
# Write: "Treatment group showed significantly higher X
# (mean = __, SD = __) compared to control (mean = __, SD = __),
# with a [small/medium/large] effect (d = __),
# p = __."
```

### Exercise 4: The Winner's Curse Revisited

Run 100 simulations of an underpowered study (n=10, true d=0.3). For the significant results only, compute the observed d. Show that "published" effect sizes are inflated.

```bio
set_seed(42)
let true_d = 0.3
let n = 10
let n_sims = 100

# 1. Simulate 100 ttest() calls
# 2. Record which are significant (p < 0.05)
# 3. For significant studies, compute observed Cohen's d inline
# 4. Compare mean "published d" to true d = 0.3
# 5. Plot histogram of published d values
```

### Exercise 5: Forest Plot for a Meta-Analysis

Combine results from 8 "studies" of the same drug effect (true d = 0.5) with varying sample sizes (n = 10 to 200). Show that larger studies give more precise estimates (narrower CIs) and cluster closer to the true effect.

```bio
set_seed(42)
let study_sizes = [10, 15, 25, 40, 60, 100, 150, 200]

# 1. Simulate each study
# 2. Compute Cohen's d and 95% CI inline for each
# 3. Create a forest_plot()
# 4. Which studies have CIs that include the true d = 0.5?
# 5. Do larger studies provide better estimates?
```

## Key Takeaways

- **P-values conflate effect size with sample size** — a tiny effect can be "significant" with enough data, and a large effect can be "non-significant" with too few data
- **Always report effect sizes** with confidence intervals alongside p-values: this is the modern reporting standard
- **Cohen's d** quantifies standardized mean differences: 0.2 small, 0.5 medium, 0.8 large (but context matters)
- **Odds ratio ≠ relative risk** when the outcome is common (>10% prevalence) — OR overstates RR
- **Cramér's V** measures categorical association strength; **eta-squared** measures variance explained in ANOVA
- **Forest plots** are the standard visualization for effect sizes across multiple comparisons or studies
- The **winner's curse**: underpowered studies that reach significance overestimate the true effect
- A complete result reports **effect size + confidence interval + p-value + sample size** — omitting any element is incomplete reporting

## What's Next

We've learned to quantify and test effects. But what happens when technical artifacts overwhelm biology? Day 20 tackles the critical problem of **batch effects and confounders** — when your PCA reveals that the dominant signal in your data is the lab that processed the sample, not the biology you're studying.
