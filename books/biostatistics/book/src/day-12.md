# Day 12: The Multiple Testing Crisis — FDR and Correction

## The Problem

Dr. Rachel Kim is a genomicist analyzing differential gene expression between tumor and normal tissue. She runs a Welch's t-test on each of 20,000 genes and finds 1,200 with p < 0.05. Exciting — until she does the arithmetic: 20,000 genes multiplied by a 5% false positive rate equals 1,000 genes that would appear "significant" purely by chance, even if not a single gene were truly differentially expressed.

Her 1,200 "hits" likely contain about 1,000 false positives and perhaps 200 true findings. That is a false discovery rate of over 80%. If she publishes this list and a validation lab tries to confirm the top 50 genes, roughly 40 will fail to replicate. Her scientific reputation — and the lab's funding — depends on solving this problem.

This is the **multiple testing crisis**, and it is the single most important statistical concept in genomics. Every differential expression study, every GWAS, every proteomics screen must address it. The solutions — Bonferroni correction, Benjamini-Hochberg FDR, and their relatives — are what make genome-scale analysis possible.

## Making It Visceral: A Simulation

Before discussing theory, let us see the problem with our own eyes.

```bio
set_seed(42)
# Simulate 20,000 genes where NONE are truly different (complete null)

let p_values = []
for i in 1..20000 {
  # Both groups drawn from the same distribution — no real differences
  let group1 = rnorm(10, 0, 1)
  let group2 = rnorm(10, 0, 1)
  let result = ttest(group1, group2)
  p_values = append(p_values, result.p_value)
}

# Count "discoveries" at various thresholds
let sig_05 = p_values |> filter(|p| p < 0.05) |> len()
let sig_01 = p_values |> filter(|p| p < 0.01) |> len()
let sig_001 = p_values |> filter(|p| p < 0.001) |> len()

print("=== 20,000 Null Genes (No True Differences) ===")
print("'Significant' at p < 0.05:  {sig_05} (expected: ~1000)")
print("'Significant' at p < 0.01:  {sig_01} (expected: ~200)")
print("'Significant' at p < 0.001: {sig_001} (expected: ~20)")
print("\nEvery single one is a false positive!")

histogram(p_values, {title: "p-Value Distribution Under Complete Null", x_label: "p-value", bins: 50})
```

Under the null hypothesis, p-values are **uniformly distributed** between 0 and 1. Exactly 5% will fall below 0.05 by definition. With 20,000 tests, that is 1,000 false alarms.

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="340" viewBox="0 0 680 340" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="24" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">The Multiple Testing Disaster: 20,000 Null Tests</text>
  <text x="340" y="44" text-anchor="middle" font-size="12" fill="#6b7280">Every "significant" result is a false positive</text>
  <!-- Grid area -->
  <rect x="40" y="60" width="600" height="200" rx="4" fill="#f8fafc" stroke="#e2e8f0" stroke-width="1"/>
  <!-- Plot ~200 dots representing genes, some highlighted as false positives -->
  <!-- Null genes (gray) - scattered across the grid -->
  <circle cx="65" cy="135" r="3" fill="#d1d5db"/><circle cx="85" cy="180" r="3" fill="#d1d5db"/>
  <circle cx="105" cy="120" r="3" fill="#d1d5db"/><circle cx="125" cy="200" r="3" fill="#d1d5db"/>
  <circle cx="145" cy="155" r="3" fill="#d1d5db"/><circle cx="165" cy="90" r="3" fill="#d1d5db"/>
  <circle cx="185" cy="175" r="3" fill="#d1d5db"/><circle cx="205" cy="110" r="3" fill="#d1d5db"/>
  <circle cx="225" cy="195" r="3" fill="#d1d5db"/><circle cx="245" cy="140" r="3" fill="#d1d5db"/>
  <circle cx="265" cy="165" r="3" fill="#d1d5db"/><circle cx="285" cy="85" r="3" fill="#d1d5db"/>
  <circle cx="305" cy="210" r="3" fill="#d1d5db"/><circle cx="325" cy="130" r="3" fill="#d1d5db"/>
  <circle cx="345" cy="185" r="3" fill="#d1d5db"/><circle cx="365" cy="100" r="3" fill="#d1d5db"/>
  <circle cx="385" cy="150" r="3" fill="#d1d5db"/><circle cx="405" cy="220" r="3" fill="#d1d5db"/>
  <circle cx="425" cy="95" r="3" fill="#d1d5db"/><circle cx="445" cy="170" r="3" fill="#d1d5db"/>
  <circle cx="465" cy="125" r="3" fill="#d1d5db"/><circle cx="485" cy="200" r="3" fill="#d1d5db"/>
  <circle cx="505" cy="145" r="3" fill="#d1d5db"/><circle cx="525" cy="80" r="3" fill="#d1d5db"/>
  <circle cx="545" cy="190" r="3" fill="#d1d5db"/><circle cx="565" cy="115" r="3" fill="#d1d5db"/>
  <circle cx="585" cy="160" r="3" fill="#d1d5db"/><circle cx="605" cy="205" r="3" fill="#d1d5db"/>
  <circle cx="75" cy="105" r="3" fill="#d1d5db"/><circle cx="155" cy="215" r="3" fill="#d1d5db"/>
  <circle cx="235" cy="85" r="3" fill="#d1d5db"/><circle cx="315" cy="175" r="3" fill="#d1d5db"/>
  <circle cx="395" cy="105" r="3" fill="#d1d5db"/><circle cx="475" cy="225" r="3" fill="#d1d5db"/>
  <circle cx="555" cy="95" r="3" fill="#d1d5db"/><circle cx="615" cy="140" r="3" fill="#d1d5db"/>
  <circle cx="95" cy="230" r="3" fill="#d1d5db"/><circle cx="195" cy="75" r="3" fill="#d1d5db"/>
  <circle cx="295" cy="230" r="3" fill="#d1d5db"/><circle cx="435" cy="240" r="3" fill="#d1d5db"/>
  <circle cx="535" cy="235" r="3" fill="#d1d5db"/><circle cx="355" cy="245" r="3" fill="#d1d5db"/>
  <!-- False positives (red) - ~5% highlighted -->
  <circle cx="72" cy="68" r="4.5" fill="#ef4444"/><circle cx="138" cy="72" r="4.5" fill="#ef4444"/>
  <circle cx="212" cy="65" r="4.5" fill="#ef4444"/><circle cx="278" cy="70" r="4.5" fill="#ef4444"/>
  <circle cx="342" cy="66" r="4.5" fill="#ef4444"/><circle cx="418" cy="73" r="4.5" fill="#ef4444"/>
  <circle cx="488" cy="68" r="4.5" fill="#ef4444"/><circle cx="558" cy="71" r="4.5" fill="#ef4444"/>
  <circle cx="108" cy="75" r="4.5" fill="#ef4444"/><circle cx="382" cy="69" r="4.5" fill="#ef4444"/>
  <circle cx="250" cy="74" r="4.5" fill="#ef4444"/><circle cx="520" cy="66" r="4.5" fill="#ef4444"/>
  <circle cx="600" cy="72" r="4.5" fill="#ef4444"/>
  <!-- Threshold line -->
  <line x1="40" y1="78" x2="640" y2="78" stroke="#ef4444" stroke-width="1.5" stroke-dasharray="6,3"/>
  <text x="645" y="82" text-anchor="start" font-size="10" fill="#ef4444">p = 0.05</text>
  <!-- Labels -->
  <text x="340" y="290" text-anchor="middle" font-size="13" fill="#1e293b">20,000 genes (all truly null — no real differences)</text>
  <!-- Legend -->
  <circle cx="160" cy="315" r="5" fill="#ef4444"/>
  <text x="172" y="319" font-size="12" fill="#dc2626">~1,000 false positives (p &lt; 0.05 by chance)</text>
  <circle cx="430" cy="315" r="4" fill="#d1d5db"/>
  <text x="440" y="319" font-size="12" fill="#6b7280">~19,000 true negatives</text>
</svg>
</div>

## Family-Wise Error Rate (FWER)

The **family-wise error rate** is the probability of making at least one Type I error across all tests:

**FWER = 1 - (1 - alpha)^m**

Where m is the number of tests.

| Number of Tests (m) | FWER at alpha = 0.05 |
|---|---|
| 1 | 5.0% |
| 10 | 40.1% |
| 100 | 99.4% |
| 1,000 | ~100% |
| 20,000 | ~100% |

## Bonferroni Correction

The simplest and most conservative approach: divide alpha by the number of tests.

**Adjusted alpha = alpha / m**

For 20,000 tests at alpha = 0.05: adjusted alpha = 0.05 / 20,000 = 2.5 x 10^-6.

Equivalently, multiply each p-value by m and compare to the original alpha:

**p_adjusted = min(p x m, 1.0)**

### Strengths and Weaknesses

| Property | Assessment |
|---|---|
| Controls FWER | Yes, strongly |
| Simple to compute | Yes |
| Power | Very low — misses many true effects |
| Assumes independence | Works regardless, but overly conservative for correlated tests |

> **Common pitfall:** Bonferroni is often TOO conservative for genomics. With 20,000 correlated gene expression measurements, it throws out far too many true positives. It is appropriate when you need to be extremely cautious (drug safety) or when you have few tests.

## Holm's Step-Down Correction

Holm's method is uniformly more powerful than Bonferroni while still controlling FWER.

**Procedure:**
1. Sort p-values from smallest to largest: p(1) <= p(2) <= ... <= p(m)
2. For the i-th smallest p-value, compute adjusted p: p(i) x (m - i + 1)
3. Enforce monotonicity: each adjusted p must be >= the previous
4. Reject all hypotheses whose adjusted p < alpha

Holm's method is *always* at least as powerful as Bonferroni, and sometimes substantially more so.

## False Discovery Rate (FDR): The Breakthrough

In 1995, Benjamini and Hochberg introduced a paradigm shift. Instead of controlling the probability of *any* false positive (FWER), they controlled the expected *proportion* of false positives among rejected hypotheses.

**FDR = E[V / R]**

Where V = number of false positives and R = total rejections.

If you set FDR = 0.05, you accept that about 5% of your "discoveries" may be false — a much more practical threshold for genomics than guaranteeing zero false positives.

### Benjamini-Hochberg (BH) Procedure

**Procedure:**
1. Sort p-values from smallest to largest: p(1) <= p(2) <= ... <= p(m)
2. For each p(i), compute the BH threshold: (i / m) x q, where q is the desired FDR level
3. Find the largest i where p(i) <= (i / m) x q
4. Reject all hypotheses 1, 2, ..., i

Equivalently, the BH-adjusted p-value (q-value) is:

**q(i) = min(p(i) x m / i, 1.0)** (with monotonicity enforced)

### Why BH Changed Genomics

| Method | Controls | Typical Threshold | Power |
|---|---|---|---|
| Bonferroni | FWER | p < 2.5e-6 (for 20K tests) | Very low |
| Holm | FWER | Similar to Bonferroni | Slightly higher |
| **BH (FDR)** | **FDR** | **q < 0.05** | **Much higher** |

> **Key insight:** BH-FDR is the standard for differential expression, GWAS, proteomics, and almost all high-throughput biology. When a paper reports "genes with FDR < 0.05," they almost always mean Benjamini-Hochberg adjusted p-values.

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="360" viewBox="0 0 680 360" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="24" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">Sorted p-Values: Bonferroni vs BH Threshold</text>
  <text x="340" y="44" text-anchor="middle" font-size="12" fill="#6b7280">BH's ascending line finds more true discoveries</text>
  <!-- Axes -->
  <line x1="80" y1="300" x2="620" y2="300" stroke="#6b7280" stroke-width="1.5"/>
  <line x1="80" y1="300" x2="80" y2="60" stroke="#6b7280" stroke-width="1.5"/>
  <text x="350" y="340" text-anchor="middle" font-size="13" fill="#6b7280">Rank (sorted by p-value)</text>
  <text x="30" y="180" text-anchor="middle" font-size="13" fill="#6b7280" transform="rotate(-90,30,180)">p-value</text>
  <!-- Y-axis ticks -->
  <text x="72" y="304" text-anchor="end" font-size="11" fill="#6b7280">0</text>
  <text x="72" y="244" text-anchor="end" font-size="11" fill="#6b7280">0.01</text>
  <line x1="75" y1="240" x2="85" y2="240" stroke="#d1d5db" stroke-width="1"/>
  <text x="72" y="184" text-anchor="end" font-size="11" fill="#6b7280">0.02</text>
  <line x1="75" y1="180" x2="85" y2="180" stroke="#d1d5db" stroke-width="1"/>
  <text x="72" y="124" text-anchor="end" font-size="11" fill="#6b7280">0.03</text>
  <line x1="75" y1="120" x2="85" y2="120" stroke="#d1d5db" stroke-width="1"/>
  <text x="72" y="64" text-anchor="end" font-size="11" fill="#6b7280">0.05</text>
  <line x1="75" y1="60" x2="620" y2="60" stroke="#d1d5db" stroke-width="1" stroke-dasharray="3,3"/>
  <!-- Bonferroni threshold: flat line at alpha/m (very low) -->
  <line x1="80" y1="288" x2="620" y2="288" stroke="#dc2626" stroke-width="2.5"/>
  <text x="625" y="292" text-anchor="start" font-size="11" font-weight="bold" fill="#dc2626">Bonferroni</text>
  <text x="625" y="305" text-anchor="start" font-size="10" fill="#dc2626">(alpha/m)</text>
  <!-- BH threshold: ascending line from 0 to q -->
  <line x1="80" y1="300" x2="620" y2="60" stroke="#2563eb" stroke-width="2.5"/>
  <text x="625" y="64" text-anchor="start" font-size="11" font-weight="bold" fill="#2563eb">BH (FDR)</text>
  <text x="625" y="77" text-anchor="start" font-size="10" fill="#2563eb">(i/m x q)</text>
  <!-- Sorted p-values (dots rising) -->
  <circle cx="95" cy="296" r="3" fill="#1e293b"/><circle cx="110" cy="294" r="3" fill="#1e293b"/>
  <circle cx="125" cy="291" r="3" fill="#1e293b"/><circle cx="140" cy="287" r="3" fill="#1e293b"/>
  <circle cx="155" cy="283" r="3" fill="#1e293b"/><circle cx="170" cy="278" r="3" fill="#1e293b"/>
  <circle cx="185" cy="273" r="3" fill="#1e293b"/><circle cx="200" cy="267" r="3" fill="#1e293b"/>
  <circle cx="215" cy="260" r="3" fill="#1e293b"/><circle cx="230" cy="252" r="3" fill="#1e293b"/>
  <circle cx="245" cy="243" r="3" fill="#1e293b"/><circle cx="260" cy="235" r="3" fill="#1e293b"/>
  <circle cx="275" cy="225" r="3" fill="#1e293b"/><circle cx="290" cy="215" r="3" fill="#1e293b"/>
  <circle cx="305" cy="205" r="3" fill="#1e293b"/><circle cx="320" cy="193" r="3" fill="#1e293b"/>
  <circle cx="335" cy="180" r="3" fill="#1e293b"/><circle cx="350" cy="168" r="3" fill="#1e293b"/>
  <circle cx="365" cy="155" r="3" fill="#1e293b"/><circle cx="380" cy="140" r="3" fill="#1e293b"/>
  <circle cx="395" cy="125" r="3" fill="#1e293b"/><circle cx="410" cy="110" r="3" fill="#1e293b"/>
  <circle cx="425" cy="95" r="3" fill="#1e293b"/><circle cx="440" cy="85" r="3" fill="#1e293b"/>
  <circle cx="455" cy="78" r="3" fill="#1e293b"/>
  <!-- BH finds more: bracket -->
  <line x1="260" y1="232" x2="260" y2="212" stroke="#2563eb" stroke-width="1" stroke-dasharray="3,2"/>
  <text x="265" y="222" font-size="10" fill="#2563eb">BH rejects these</text>
  <!-- Bonferroni annotation -->
  <text x="180" y="280" text-anchor="middle" font-size="10" fill="#dc2626">Bonferroni rejects only these few</text>
  <line x1="95" y1="283" x2="155" y2="283" stroke="#dc2626" stroke-width="1" stroke-dasharray="3,2"/>
</svg>
</div>

## Other Correction Methods

### Hochberg's Step-Up

Similar to Holm but slightly more powerful; assumes independence or positive dependence among tests.

### Benjamini-Yekutieli (BY)

A conservative FDR procedure that works under *any* dependency structure between tests. Use when you have strong correlations (e.g., genes in the same pathway).

## Choosing a Method

| Method | Controls | Best For |
|---|---|---|
| **Bonferroni** | FWER | Few tests, safety-critical decisions |
| **Holm** | FWER | Same as Bonferroni but always more powerful |
| **BH** | FDR | Standard genomics, proteomics, any large-scale screen |
| **Hochberg** | FWER | Independent or positively dependent tests |
| **BY** | FDR | Strongly correlated tests, conservative FDR |

## The Volcano Plot

The volcano plot is the most iconic visualization in differential expression analysis. It plots:
- **x-axis**: log2 fold change (effect size)
- **y-axis**: -log10(adjusted p-value) (statistical significance)

Genes in the upper corners are both statistically significant AND biologically meaningful — the ones you actually care about.

| Region | Interpretation |
|---|---|
| Upper-right | Significantly upregulated (high FC, low p) |
| Upper-left | Significantly downregulated (high FC, low p) |
| Bottom center | Not significant (high p, any FC) |
| Upper center | Significant but small effect (low FC, low p) |

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="420" viewBox="0 0 680 420" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="24" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">Anatomy of a Volcano Plot</text>
  <!-- Axes -->
  <line x1="80" y1="320" x2="600" y2="320" stroke="#6b7280" stroke-width="1.5"/>
  <line x1="340" y1="320" x2="340" y2="50" stroke="#6b7280" stroke-width="1" stroke-dasharray="4,3"/>
  <line x1="80" y1="320" x2="80" y2="50" stroke="#6b7280" stroke-width="1.5"/>
  <!-- Axis labels -->
  <text x="340" y="355" text-anchor="middle" font-size="13" font-weight="bold" fill="#1e293b">log2 Fold Change</text>
  <text x="25" y="185" text-anchor="middle" font-size="13" font-weight="bold" fill="#1e293b" transform="rotate(-90,25,185)">-log10(adj. p-value)</text>
  <!-- X-axis ticks -->
  <text x="130" y="338" text-anchor="middle" font-size="11" fill="#6b7280">-3</text>
  <text x="200" y="338" text-anchor="middle" font-size="11" fill="#6b7280">-2</text>
  <text x="270" y="338" text-anchor="middle" font-size="11" fill="#6b7280">-1</text>
  <text x="340" y="338" text-anchor="middle" font-size="11" fill="#6b7280">0</text>
  <text x="410" y="338" text-anchor="middle" font-size="11" fill="#6b7280">1</text>
  <text x="480" y="338" text-anchor="middle" font-size="11" fill="#6b7280">2</text>
  <text x="550" y="338" text-anchor="middle" font-size="11" fill="#6b7280">3</text>
  <!-- Significance threshold line (horizontal) -->
  <line x1="80" y1="200" x2="600" y2="200" stroke="#9ca3af" stroke-width="1.5" stroke-dasharray="6,3"/>
  <text x="608" y="204" text-anchor="start" font-size="10" fill="#9ca3af">p = 0.05</text>
  <!-- FC threshold lines (vertical) -->
  <line x1="270" y1="50" x2="270" y2="320" stroke="#9ca3af" stroke-width="1.5" stroke-dasharray="6,3"/>
  <line x1="410" y1="50" x2="410" y2="320" stroke="#9ca3af" stroke-width="1.5" stroke-dasharray="6,3"/>
  <text x="270" y="46" text-anchor="middle" font-size="10" fill="#9ca3af">FC = -1</text>
  <text x="410" y="46" text-anchor="middle" font-size="10" fill="#9ca3af">FC = +1</text>
  <!-- Not significant dots (gray, center bottom) -->
  <circle cx="290" cy="280" r="3" fill="#d1d5db"/><circle cx="310" cy="260" r="3" fill="#d1d5db"/>
  <circle cx="330" cy="290" r="3" fill="#d1d5db"/><circle cx="350" cy="270" r="3" fill="#d1d5db"/>
  <circle cx="370" cy="285" r="3" fill="#d1d5db"/><circle cx="300" cy="300" r="3" fill="#d1d5db"/>
  <circle cx="320" cy="240" r="3" fill="#d1d5db"/><circle cx="340" cy="310" r="3" fill="#d1d5db"/>
  <circle cx="360" cy="250" r="3" fill="#d1d5db"/><circle cx="380" cy="295" r="3" fill="#d1d5db"/>
  <circle cx="315" cy="220" r="3" fill="#d1d5db"/><circle cx="355" cy="230" r="3" fill="#d1d5db"/>
  <circle cx="335" cy="215" r="3" fill="#d1d5db"/><circle cx="285" cy="245" r="3" fill="#d1d5db"/>
  <circle cx="395" cy="265" r="3" fill="#d1d5db"/><circle cx="275" cy="275" r="3" fill="#d1d5db"/>
  <circle cx="345" cy="305" r="3" fill="#d1d5db"/><circle cx="325" cy="275" r="3" fill="#d1d5db"/>
  <!-- Significant upregulated (red, upper right) -->
  <circle cx="430" cy="120" r="4.5" fill="#dc2626"/><circle cx="460" cy="90" r="4.5" fill="#dc2626"/>
  <circle cx="490" cy="75" r="4.5" fill="#dc2626"/><circle cx="445" cy="140" r="4.5" fill="#dc2626"/>
  <circle cx="510" cy="105" r="4.5" fill="#dc2626"/><circle cx="470" cy="155" r="4.5" fill="#dc2626"/>
  <circle cx="530" cy="130" r="4.5" fill="#dc2626"/><circle cx="500" cy="165" r="4.5" fill="#dc2626"/>
  <circle cx="550" cy="95" r="4.5" fill="#dc2626"/><circle cx="420" cy="175" r="4.5" fill="#dc2626"/>
  <circle cx="480" cy="110" r="4.5" fill="#dc2626"/>
  <!-- Significant downregulated (blue, upper left) -->
  <circle cx="250" cy="115" r="4.5" fill="#2563eb"/><circle cx="220" cy="85" r="4.5" fill="#2563eb"/>
  <circle cx="190" cy="100" r="4.5" fill="#2563eb"/><circle cx="235" cy="145" r="4.5" fill="#2563eb"/>
  <circle cx="170" cy="125" r="4.5" fill="#2563eb"/><circle cx="205" cy="165" r="4.5" fill="#2563eb"/>
  <circle cx="150" cy="90" r="4.5" fill="#2563eb"/><circle cx="260" cy="170" r="4.5" fill="#2563eb"/>
  <circle cx="130" cy="110" r="4.5" fill="#2563eb"/><circle cx="240" cy="180" r="4.5" fill="#2563eb"/>
  <!-- Region labels -->
  <rect x="102" y="56" width="130" height="24" rx="4" fill="#2563eb" fill-opacity="0.12"/>
  <text x="167" y="73" text-anchor="middle" font-size="12" font-weight="bold" fill="#2563eb">Downregulated</text>
  <rect x="448" y="56" width="120" height="24" rx="4" fill="#dc2626" fill-opacity="0.12"/>
  <text x="508" y="73" text-anchor="middle" font-size="12" font-weight="bold" fill="#dc2626">Upregulated</text>
  <rect x="290" y="296" width="100" height="20" rx="4" fill="#d1d5db" fill-opacity="0.4"/>
  <text x="340" y="310" text-anchor="middle" font-size="11" fill="#6b7280">Not Significant</text>
  <!-- Legend -->
  <rect x="120" y="375" width="440" height="35" rx="6" fill="#f1f5f9" stroke="#e2e8f0" stroke-width="1"/>
  <circle cx="155" cy="393" r="5" fill="#2563eb"/>
  <text x="165" y="397" font-size="11" fill="#1e293b">Sig. down (FC&lt;-1, p&lt;0.05)</text>
  <circle cx="325" cy="393" r="5" fill="#dc2626"/>
  <text x="335" y="397" font-size="11" fill="#1e293b">Sig. up (FC&gt;1, p&lt;0.05)</text>
  <circle cx="480" cy="393" r="4" fill="#d1d5db"/>
  <text x="490" y="397" font-size="11" fill="#1e293b">NS</text>
</svg>
</div>

## Multiple Testing Correction in BioLang

### Simulating the Crisis and Applying Corrections

```bio
set_seed(42)
# Simulate 20,000 genes: 18,000 null + 2,000 truly differential

let p_values = []
let is_true = []  # Ground truth: 1 = truly differential, 0 = null
let fold_changes = []

for i in 1..20000 {
  let group1 = rnorm(10, 0, 1)
  if i <= 2000 {
    # True differential: shifted mean
    let shift = rnorm(1, 2.0, 0.5)[0]
    let group2 = rnorm(10, shift, 1)
    is_true = append(is_true, 1)
  } else {
    # Null gene: no difference
    let group2 = rnorm(10, 0, 1)
    is_true = append(is_true, 0)
  }
  let result = ttest(group1, group2)
  p_values = append(p_values, result.p_value)
  fold_changes = append(fold_changes, mean(group2) - mean(group1))
}

print("Total genes: {len(p_values)}")
print("True DE genes: {is_true |> filter(|x| x == 1) |> len()}")
print("Null genes: {is_true |> filter(|x| x == 0) |> len()}")
```

### Applying All Correction Methods

```bio
# Apply multiple correction methods
let p_bonf = p_adjust(p_values, "bonferroni")
let p_holm = p_adjust(p_values, "holm")
let p_bh   = p_adjust(p_values, "BH")
let p_hoch = p_adjust(p_values, "hochberg")
let p_by   = p_adjust(p_values, "BY")

# Count discoveries at adjusted p < 0.05
let count_sig = |adj_p| adj_p |> filter(|p| p < 0.05) |> len()

print("=== Discoveries at Adjusted p < 0.05 ===")
print("Method       | Total Discoveries | True Pos | False Pos | FDR")
print("-------------|-------------------|----------|-----------|--------")

for method_name, adj_p in [
  ["Unadjusted ", p_values],
  ["Bonferroni ", p_bonf],
  ["Holm       ", p_holm],
  ["BH (FDR)   ", p_bh],
  ["Hochberg   ", p_hoch],
  ["BY         ", p_by]
] {
  let discoveries = []
  let true_pos = 0
  let false_pos = 0
  for j in 0..len(adj_p) {
    if adj_p[j] < 0.05 {
      discoveries = append(discoveries, j)
      if is_true[j] == 1 { true_pos = true_pos + 1 }
      else { false_pos = false_pos + 1 }
    }
  }
  let total = len(discoveries)
  let fdr = if total > 0 then false_pos / total else 0.0
  print("{method_name} | {total:>17} | {true_pos:>8} | {false_pos:>9} | {fdr:>6.3}")
}
```

### Drawing a Volcano Plot

```bio
# Volcano plot: the most important visualization in DE analysis
let log2_fc = fold_changes
let neg_log10_p = p_bh |> map(|p| if p > 0 then -log10(p) else 10)

# Classify genes
let colors = []
for i in 0..len(p_bh) {
  if p_bh[i] < 0.05 and abs(log2_fc[i]) > 1.0 {
    colors = append(colors, "significant")
  } else {
    colors = append(colors, "not_significant")
  }
}

volcano(log2_fc, neg_log10_p, {title: "Differential Expression: Tumor vs Normal", x_label: "log2 Fold Change", y_label: "-log10(FDR-adjusted p-value)", fc_threshold: 1.0, p_threshold: 0.05, highlight: colors})

# Count genes in each quadrant
let sig_up = 0
let sig_down = 0
let not_sig = 0
for i in 0..len(p_bh) {
  if p_bh[i] < 0.05 and log2_fc[i] > 1.0 { sig_up = sig_up + 1 }
  else if p_bh[i] < 0.05 and log2_fc[i] < -1.0 { sig_down = sig_down + 1 }
  else { not_sig = not_sig + 1 }
}
print("Significantly upregulated:   {sig_up}")
print("Significantly downregulated: {sig_down}")
print("Not significant:             {not_sig}")
```

### Visualizing the BH Procedure

```bio
set_seed(42)
# Step-by-step BH procedure visualization

# Smaller example for clarity: 100 tests, 10 truly different
let p_vals = []
for i in 1..100 {
  let g1 = rnorm(10, 0, 1)
  let g2 = if i <= 10 {
    rnorm(10, 3, 1)
  } else {
    rnorm(10, 0, 1)
  }
  p_vals = append(p_vals, ttest(g1, g2).p_value)
}

# Sort p-values
let sorted_p = sort(p_vals)
let bh_thresholds = range(1, 101) |> map(|i| i / 100 * 0.05)

# Plot sorted p-values against BH thresholds
scatter(range(1, 101), sorted_p, {title: "BH Procedure: Sorted p-values vs Threshold Line", x_label: "Rank", y_label: "p-value", overlay_lines: [[range(1, 101), bh_thresholds]]})

let bh_adjusted = p_adjust(p_vals, "BH")
let n_sig = bh_adjusted |> filter(|p| p < 0.05) |> len()
print("BH discoveries (FDR < 0.05): {n_sig}")
```

### p-Value Histograms: Diagnostic Tool

```bio
set_seed(42)
# A well-behaved p-value histogram tells you a lot
# Uniform = all null; spike at 0 = true signal exists


# Scenario 1: All null (should be uniform)
let null_ps = []
for i in 1..5000 {
  let g1 = rnorm(10, 0, 1)
  let g2 = rnorm(10, 0, 1)
  null_ps = append(null_ps, ttest(g1, g2).p_value)
}

# Scenario 2: 20% true signal (spike near 0 + uniform)
let mixed_ps = []
for i in 1..5000 {
  let g1 = rnorm(10, 0, 1)
  let g2 = if i <= 1000 {
    rnorm(10, 2, 1)
  } else {
    rnorm(10, 0, 1)
  }
  mixed_ps = append(mixed_ps, ttest(g1, g2).p_value)
}

histogram(null_ps, {title: "All Null: Uniform p-value Distribution", x_label: "p-value", bins: 20})

histogram(mixed_ps, {title: "20% True Signal: Spike Near Zero + Uniform Background", x_label: "p-value", bins: 20})
```

**Python:**

```python
from statsmodels.stats.multitest import multipletests
import numpy as np

# Simulate p-values (example with 1000 tests)
np.random.seed(42)
p_values = np.random.uniform(0, 1, 1000)
p_values[:100] = np.random.uniform(0, 0.01, 100)  # 100 true signals

# Apply corrections
reject_bonf, padj_bonf, _, _ = multipletests(p_values, method='bonferroni')
reject_holm, padj_holm, _, _ = multipletests(p_values, method='holm')
reject_bh, padj_bh, _, _ = multipletests(p_values, method='fdr_bh')

print(f"Bonferroni: {reject_bonf.sum()} discoveries")
print(f"Holm:       {reject_holm.sum()} discoveries")
print(f"BH (FDR):   {reject_bh.sum()} discoveries")
```

**R:**

```r
# Simulate p-values
set.seed(42)
p_values <- c(runif(100, 0, 0.01), runif(900, 0, 1))

# Apply corrections (all built-in)
p_bonf <- p.adjust(p_values, method = "bonferroni")
p_holm <- p.adjust(p_values, method = "holm")
p_bh   <- p.adjust(p_values, method = "BH")
p_by   <- p.adjust(p_values, method = "BY")

cat("Bonferroni:", sum(p_bonf < 0.05), "\n")
cat("Holm:      ", sum(p_holm < 0.05), "\n")
cat("BH (FDR):  ", sum(p_bh < 0.05), "\n")
cat("BY:        ", sum(p_by < 0.05), "\n")

# Volcano plot (using EnhancedVolcano)
library(EnhancedVolcano)
EnhancedVolcano(results, lab = rownames(results),
  x = 'log2FoldChange', y = 'padj',
  pCutoff = 0.05, FCcutoff = 1.0)
```

## Exercises

**Exercise 1: Simulate and Count**

Simulate 10,000 tests where all genes are null (no true differences). Verify that approximately 500 have p < 0.05, 100 have p < 0.01, and 10 have p < 0.001.

```bio

# TODO: Simulate 10,000 null t-tests
# TODO: Count p < 0.05, p < 0.01, p < 0.001
# TODO: Plot the p-value histogram (should be uniform)
```

**Exercise 2: Compare Correction Power**

Simulate 5,000 genes (500 truly DE with log2FC = 1.5, 4,500 null). Apply Bonferroni, Holm, and BH. For each, compute: (a) total discoveries, (b) true positives, (c) false positives, (d) actual FDR.

```bio

# TODO: Simulate 5,000 genes (500 DE + 4,500 null)
# TODO: Apply all three corrections
# TODO: Build a comparison table
# TODO: Which method gives the best balance of power and error control?
```

**Exercise 3: Build a Volcano Plot**

Using the simulation from Exercise 2, create a volcano plot. Color genes as "significant" (BH-adjusted p < 0.05 AND |log2FC| > 1) versus "not significant."

```bio
# TODO: Use fold changes and BH-adjusted p-values from Exercise 2
# TODO: Create a volcano plot with appropriate thresholds
# TODO: Count genes in each quadrant
```

**Exercise 4: p-Value Histogram Diagnostics**

Generate p-value histograms for three scenarios: (a) 100% null genes, (b) 10% true DE genes, (c) 50% true DE genes. Describe the characteristic shape of each and explain what you would look for in real data.

```bio

# TODO: Scenario A — all null
# TODO: Scenario B — 10% DE
# TODO: Scenario C — 50% DE
# TODO: Create histograms for each
# TODO: Describe the shape and what it tells you
```

**Exercise 5: When Does Bonferroni Make Sense?**

You are testing 5 pre-specified candidate genes (not genome-wide). Apply Bonferroni and BH to these 5 p-values: [0.008, 0.023, 0.041, 0.062, 0.110]. How do the results differ? When would you prefer Bonferroni here?

```bio
let p_vals = [0.008, 0.023, 0.041, 0.062, 0.110]

# TODO: Apply Bonferroni and BH
# TODO: Which genes are significant under each method?
# TODO: Argue for which method is more appropriate for 5 candidate genes
```

## Key Takeaways

- Testing m hypotheses at alpha = 0.05 yields approximately m x 0.05 false positives — devastating for genomics
- **Bonferroni** correction (p x m) controls FWER but is often too conservative for large-scale studies
- **Holm's** step-down method is always at least as powerful as Bonferroni and should be preferred
- **Benjamini-Hochberg (BH)** FDR correction is the standard for genomics: it controls the expected proportion of false discoveries rather than the probability of any false discovery
- At FDR = 0.05, you accept that about 5% of discoveries may be false — a practical trade-off for high-throughput biology
- **p-value histograms** are essential diagnostics: uniform = all null, spike at zero = true signal present
- The **volcano plot** (-log10 adjusted p vs log2 fold change) is the standard visualization for differential expression: genes in the upper corners are both significant and biologically meaningful
- Always report which correction method you used — "p < 0.05" means very different things with and without adjustment

## What's Next

With the multiple testing crisis solved, we have completed the core toolkit of statistical hypothesis testing. Week 3 shifts to modeling and relationships: tomorrow we explore correlation — how to measure and test whether two continuous variables move together, from gene expression co-regulation to dose-response curves. This transition from "are groups different?" to "how are variables related?" opens the door to regression, prediction, and the modeling approaches that power modern biostatistics.
