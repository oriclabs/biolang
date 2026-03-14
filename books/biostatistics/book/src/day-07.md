# Day 7: Hypothesis Testing — Asking Precise Questions

## The Problem

Dr. Kenji Nakamura has spent three years developing a blood-based biomarker panel for early Alzheimer's detection. His team measures plasma levels of phosphorylated tau (p-tau217) in 40 cognitively normal individuals and 40 patients with confirmed early-stage Alzheimer's. The mean p-tau217 level in the Alzheimer's group is 3.8 pg/mL, compared to 2.9 pg/mL in controls. The difference looks promising — nearly 30% higher.

But when Dr. Nakamura submits to the FDA for breakthrough device designation, the reviewer's response is blunt: "Your biomarker shows a numerical difference. Can you demonstrate this isn't just sampling noise? What is the probability of seeing a difference this large if the biomarker has no real diagnostic value?" This is the fundamental question that hypothesis testing answers.

The stakes are enormous. If the biomarker works, millions of patients could be diagnosed years earlier, when interventions are most effective. If it doesn't — if the observed difference is just statistical noise — pursuing it wastes hundreds of millions in development costs and, worse, could lead to false diagnoses.

## What Is Hypothesis Testing?

Think of hypothesis testing as a courtroom trial for your scientific claim.

- The **defendant** is the null hypothesis (H0): "There is no effect." In the courtroom, the defendant is presumed innocent.
- The **prosecution's evidence** is your data. You are trying to show the evidence is so overwhelming that the "innocence" explanation is implausible.
- The **verdict** is either "guilty" (reject H0) or "not proven" (fail to reject H0). Notice: the jury never declares the defendant "innocent" — just that the evidence was insufficient.

> **Key insight:** Hypothesis testing never proves your theory is true. It only tells you whether the data are inconsistent enough with "no effect" that you can reject that explanation with a specified level of confidence.

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="340" viewBox="0 0 680 340" xmlns="http://www.w3.org/2000/svg" style="background: #fafbfc; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="28" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">Null Distribution with Rejection Regions (Two-Tailed Test)</text>
  <!-- X axis -->
  <line x1="40" y1="260" x2="640" y2="260" stroke="#374151" stroke-width="1.5"/>
  <text x="340" y="290" text-anchor="middle" font-size="12" fill="#374151">z-statistic</text>
  <!-- X axis ticks -->
  <text x="80" y="278" text-anchor="middle" font-size="11" fill="#6b7280">-3</text>
  <text x="167" y="278" text-anchor="middle" font-size="11" fill="#6b7280">-2</text>
  <text x="253" y="278" text-anchor="middle" font-size="11" fill="#6b7280">-1</text>
  <text x="340" y="278" text-anchor="middle" font-size="11" fill="#6b7280">0</text>
  <text x="427" y="278" text-anchor="middle" font-size="11" fill="#6b7280">1</text>
  <text x="513" y="278" text-anchor="middle" font-size="11" fill="#6b7280">2</text>
  <text x="600" y="278" text-anchor="middle" font-size="11" fill="#6b7280">3</text>
  <!-- Bell curve (standard normal) -->
  <path d="M 40,260 C 40,260 60,258 80,255 C 100,250 120,240 140,225 C 160,205 180,180 200,150 C 220,115 240,85 260,65 C 280,50 300,42 320,40 C 340,39 340,39 340,39 C 360,40 380,42 400,50 C 420,65 440,85 460,115 C 480,150 500,180 520,205 C 540,225 560,240 580,250 C 600,255 620,258 640,260" fill="none" stroke="#3b82f6" stroke-width="2.5"/>
  <!-- Fill acceptance region (light blue) -->
  <path d="M 167,240 C 180,225 195,200 210,170 C 230,130 250,90 270,65 C 290,48 310,42 330,40 C 350,40 370,48 390,65 C 410,90 430,130 450,170 C 465,200 480,225 493,240 Z" fill="#dbeafe" opacity="0.6"/>
  <!-- Fill left rejection region (red) -->
  <path d="M 40,260 C 40,260 60,258 80,255 C 100,250 120,242 140,232 C 155,224 163,218 167,240 L 40,260 Z" fill="#fecaca" opacity="0.7"/>
  <!-- Fill right rejection region (red) -->
  <path d="M 513,240 C 517,218 525,224 540,232 C 560,242 580,250 600,255 C 620,258 640,260 640,260 L 513,260 Z" fill="#fecaca" opacity="0.7"/>
  <!-- Critical value lines -->
  <line x1="167" y1="240" x2="167" y2="265" stroke="#dc2626" stroke-width="1.5" stroke-dasharray="4,2"/>
  <text x="167" y="308" text-anchor="middle" font-size="11" fill="#dc2626" font-weight="bold">-1.96</text>
  <line x1="513" y1="240" x2="513" y2="265" stroke="#dc2626" stroke-width="1.5" stroke-dasharray="4,2"/>
  <text x="513" y="308" text-anchor="middle" font-size="11" fill="#dc2626" font-weight="bold">+1.96</text>
  <!-- Labels -->
  <text x="100" y="232" text-anchor="middle" font-size="11" fill="#dc2626" font-weight="bold">Reject H0</text>
  <text x="100" y="246" text-anchor="middle" font-size="10" fill="#dc2626">(alpha/2 = 2.5%)</text>
  <text x="580" y="232" text-anchor="middle" font-size="11" fill="#dc2626" font-weight="bold">Reject H0</text>
  <text x="580" y="246" text-anchor="middle" font-size="10" fill="#dc2626">(alpha/2 = 2.5%)</text>
  <text x="340" y="155" text-anchor="middle" font-size="12" fill="#2563eb" font-weight="bold">Fail to reject H0</text>
  <text x="340" y="170" text-anchor="middle" font-size="11" fill="#2563eb">(95% of area)</text>
  <!-- Legend -->
  <rect x="180" y="316" width="14" height="10" fill="#fecaca" stroke="#dc2626" stroke-width="0.5" rx="2"/>
  <text x="200" y="326" font-size="11" fill="#6b7280">Rejection regions (alpha = 0.05)</text>
  <rect x="400" y="316" width="14" height="10" fill="#dbeafe" stroke="#3b82f6" stroke-width="0.5" rx="2"/>
  <text x="420" y="326" font-size="11" fill="#6b7280">Acceptance region</text>
</svg>
</div>

## The Five Steps of Hypothesis Testing

| Step | Description | Alzheimer's Example |
|---|---|---|
| 1. State H0 and H1 | Define the null and alternative | H0: mu_AD = mu_control; H1: mu_AD > mu_control |
| 2. Choose alpha | Set significance threshold | alpha = 0.05 |
| 3. Compute test statistic | Summarize evidence against H0 | z = (x_bar1 - x_bar2) / SE |
| 4. Find p-value | Probability of seeing this extreme a result under H0 | p = P(Z >= z_obs) |
| 5. Make decision | Compare p to alpha | If p < 0.05, reject H0 |

## The Null Hypothesis (H0)

The null hypothesis is the "boring" explanation — the default assumption of no effect, no difference, no relationship. It is what you assume until the data force you to abandon it.

| Research Question | Null Hypothesis (H0) |
|---|---|
| Does the drug reduce tumor size? | Mean tumor size is the same with and without drug |
| Is this SNP associated with diabetes? | Allele frequencies are the same in cases and controls |
| Does expression differ between tissues? | Mean expression is equal in both tissues |
| Is the mutation rate elevated? | Mutation rate equals the background rate |

## The Alternative Hypothesis (H1)

The alternative is what you actually believe — the "interesting" claim.

- **Two-tailed:** H1: mu1 != mu2 (the groups differ in either direction)
- **One-tailed:** H1: mu1 > mu2 (specifically higher) or H1: mu1 < mu2 (specifically lower)

> **Common pitfall:** Do not choose one-tailed vs two-tailed after looking at your data. This decision must be made before the experiment, based on your scientific question. Switching from two-tailed to one-tailed after seeing results halves your p-value — that is scientific fraud.

## The p-Value: Most Misunderstood Number in Science

The p-value is the probability of observing data as extreme as (or more extreme than) what you got, **assuming H0 is true**.

### What the p-value IS:
- A measure of how surprising your data are under the null hypothesis
- A continuous measure of evidence — smaller p = more evidence against H0
- The probability of the data given H0: P(data | H0)

### What the p-value IS NOT:
- The probability that H0 is true: NOT P(H0 | data)
- The probability your result is due to chance
- The probability of making an error
- A measure of effect size or practical importance

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="320" viewBox="0 0 680 320" xmlns="http://www.w3.org/2000/svg" style="background: #fafbfc; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="28" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">The p-Value: Area Beyond the Observed Statistic</text>
  <!-- X axis -->
  <line x1="40" y1="240" x2="640" y2="240" stroke="#374151" stroke-width="1.5"/>
  <text x="340" y="268" text-anchor="middle" font-size="12" fill="#374151">z-statistic</text>
  <!-- Axis ticks -->
  <text x="80" y="256" text-anchor="middle" font-size="10" fill="#6b7280">-3</text>
  <text x="167" y="256" text-anchor="middle" font-size="10" fill="#6b7280">-2</text>
  <text x="253" y="256" text-anchor="middle" font-size="10" fill="#6b7280">-1</text>
  <text x="340" y="256" text-anchor="middle" font-size="10" fill="#6b7280">0</text>
  <text x="427" y="256" text-anchor="middle" font-size="10" fill="#6b7280">1</text>
  <text x="513" y="256" text-anchor="middle" font-size="10" fill="#6b7280">2</text>
  <text x="600" y="256" text-anchor="middle" font-size="10" fill="#6b7280">3</text>
  <!-- Bell curve -->
  <path d="M 40,240 C 40,240 60,238 80,235 C 100,230 120,220 140,205 C 160,185 180,160 200,130 C 220,95 240,70 260,52 C 280,40 300,34 320,32 C 340,31 340,31 340,31 C 360,32 380,34 400,40 C 420,52 440,70 460,95 C 480,130 500,160 520,185 C 540,205 560,220 580,230 C 600,235 620,238 640,240" fill="none" stroke="#3b82f6" stroke-width="2.5"/>
  <!-- Shade the p-value area (right tail beyond z_obs = 2.4) -->
  <path d="M 548,210 C 555,215 565,222 575,228 C 590,233 605,236 620,238 C 630,239 640,240 640,240 L 548,240 Z" fill="#ef4444" opacity="0.5"/>
  <!-- Mirror on left tail for two-tailed -->
  <path d="M 132,210 C 125,215 115,222 105,228 C 90,233 75,236 60,238 C 50,239 40,240 40,240 L 132,240 Z" fill="#ef4444" opacity="0.5"/>
  <!-- Observed z line -->
  <line x1="548" y1="210" x2="548" y2="245" stroke="#dc2626" stroke-width="2" stroke-dasharray="4,2"/>
  <text x="548" y="285" text-anchor="middle" font-size="12" fill="#dc2626" font-weight="bold">z_obs = 2.4</text>
  <!-- Mirror line -->
  <line x1="132" y1="210" x2="132" y2="245" stroke="#dc2626" stroke-width="2" stroke-dasharray="4,2"/>
  <text x="132" y="285" text-anchor="middle" font-size="12" fill="#dc2626" font-weight="bold">-2.4</text>
  <!-- Arrows and labels for p-value areas -->
  <path d="M 590,195 L 600,210" fill="none" stroke="#dc2626" stroke-width="1.5"/>
  <text x="608" y="190" font-size="12" fill="#dc2626" font-weight="bold">p/2</text>
  <path d="M 90,195 L 80,210" fill="none" stroke="#dc2626" stroke-width="1.5"/>
  <text x="62" y="190" font-size="12" fill="#dc2626" font-weight="bold">p/2</text>
  <!-- Annotation -->
  <text x="340" y="130" text-anchor="middle" font-size="12" fill="#374151">The shaded area = total p-value</text>
  <text x="340" y="146" text-anchor="middle" font-size="11" fill="#6b7280">p = P(|Z| >= 2.4 | H0 true) = 0.016</text>
  <text x="340" y="162" text-anchor="middle" font-size="11" fill="#16a34a" font-weight="bold">p &lt; 0.05: Reject H0</text>
</svg>
</div>

| p-value | Informal Interpretation |
|---|---|
| p > 0.10 | Little evidence against H0 |
| 0.05 < p < 0.10 | Weak evidence against H0 |
| 0.01 < p < 0.05 | Moderate evidence against H0 |
| 0.001 < p < 0.01 | Strong evidence against H0 |
| p < 0.001 | Very strong evidence against H0 |

## Type I and Type II Errors

Every decision carries the risk of being wrong:

| | H0 is True | H0 is False |
|---|---|---|
| **Reject H0** | Type I error (false positive), probability = alpha | Correct (true positive), probability = 1 - beta = power |
| **Fail to reject H0** | Correct (true negative) | Type II error (false negative), probability = beta |

- **Type I error (alpha):** You claim a drug works when it doesn't. A patient receives ineffective treatment.
- **Type II error (beta):** You miss a real effect. An effective drug gets shelved.

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="380" viewBox="0 0 680 380" xmlns="http://www.w3.org/2000/svg" style="background: #fafbfc; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="28" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">Type I and Type II Errors: Two Overlapping Distributions</text>
  <!-- X axis -->
  <line x1="30" y1="250" x2="650" y2="250" stroke="#374151" stroke-width="1.5"/>
  <text x="340" y="275" text-anchor="middle" font-size="12" fill="#374151">Test statistic value</text>
  <!-- H0 distribution (centered at 200) -->
  <path d="M 30,250 C 50,248 70,244 90,236 C 110,224 130,206 150,182 C 170,152 190,120 210,94 C 230,72 250,58 270,50 C 290,46 300,45 310,45 C 320,46 330,50 340,58 C 360,72 370,86 380,105 C 395,130 405,155 415,178 C 430,206 440,224 455,236 C 468,243 478,247 490,249 L 30,250 Z" fill="#93c5fd" opacity="0.35" stroke="#2563eb" stroke-width="2"/>
  <!-- H1 distribution (centered at 420, shifted right) -->
  <path d="M 190,249 C 210,247 230,242 250,234 C 270,220 290,200 310,174 C 330,144 350,115 370,90 C 390,70 410,56 430,50 C 450,47 460,47 470,48 C 490,52 510,62 530,80 C 550,100 560,120 570,145 C 580,170 590,195 600,215 C 610,230 625,242 640,248 L 190,250 Z" fill="#bbf7d0" opacity="0.35" stroke="#16a34a" stroke-width="2"/>
  <!-- Critical value line -->
  <line x1="415" y1="40" x2="415" y2="255" stroke="#dc2626" stroke-width="2" stroke-dasharray="6,3"/>
  <text x="415" y="290" text-anchor="middle" font-size="11" fill="#dc2626" font-weight="bold">Critical value</text>
  <!-- Alpha region (right tail of H0 beyond critical value) -->
  <path d="M 415,178 C 430,206 440,224 455,236 C 468,243 478,247 490,249 L 415,249 Z" fill="#dc2626" opacity="0.4"/>
  <!-- Beta region (left tail of H1 below critical value) -->
  <path d="M 190,249 C 210,247 230,242 250,234 C 270,220 290,200 310,174 C 330,144 350,115 370,90 C 390,70 405,60 415,56 L 415,249 L 190,249 Z" fill="#f59e0b" opacity="0.3"/>
  <!-- Labels for distributions -->
  <text x="260" y="38" font-size="13" fill="#2563eb" font-weight="bold">H0 (null)</text>
  <text x="260" y="52" font-size="10" fill="#2563eb">"No effect"</text>
  <text x="470" y="38" font-size="13" fill="#16a34a" font-weight="bold">H1 (alternative)</text>
  <text x="470" y="52" font-size="10" fill="#16a34a">"Real effect"</text>
  <!-- Alpha label -->
  <path d="M 455,200 L 445,215" fill="none" stroke="#dc2626" stroke-width="1.2"/>
  <text x="460" y="195" font-size="12" fill="#dc2626" font-weight="bold">alpha</text>
  <text x="460" y="208" font-size="10" fill="#dc2626">(Type I)</text>
  <!-- Beta label -->
  <path d="M 320,190 L 340,200" fill="none" stroke="#b45309" stroke-width="1.2"/>
  <text x="300" y="185" font-size="12" fill="#b45309" font-weight="bold">beta</text>
  <text x="300" y="198" font-size="10" fill="#b45309">(Type II)</text>
  <!-- Power label -->
  <text x="500" y="140" font-size="12" fill="#16a34a" font-weight="bold">Power = 1 - beta</text>
  <text x="500" y="155" font-size="10" fill="#16a34a">(correct detection)</text>
  <!-- Legend -->
  <rect x="100" y="310" width="14" height="10" fill="#dc2626" opacity="0.5" rx="2"/>
  <text x="120" y="320" font-size="11" fill="#6b7280">alpha: False positive (reject true H0)</text>
  <rect x="100" y="330" width="14" height="10" fill="#f59e0b" opacity="0.4" rx="2"/>
  <text x="120" y="340" font-size="11" fill="#6b7280">beta: False negative (miss real effect)</text>
  <rect x="380" y="310" width="14" height="10" fill="#bbf7d0" stroke="#16a34a" stroke-width="0.5" rx="2"/>
  <text x="400" y="320" font-size="11" fill="#6b7280">Power: Correctly detect effect (1 - beta)</text>
  <text x="340" y="368" text-anchor="middle" font-size="11" fill="#6b7280">Reducing alpha (fewer false positives) increases beta (more false negatives) -- a fundamental trade-off</text>
</svg>
</div>

> **Clinical relevance:** In drug safety testing, alpha is typically set very low (0.01 or even 0.001) because a Type I error means approving a dangerous drug. In exploratory genomics, higher alpha (0.05 or even 0.10) is acceptable because you will validate hits in follow-up experiments.

## Statistical vs Practical Significance

A p-value of 0.001 does not mean the effect is large or important. With enough data, trivially small effects become "statistically significant."

| Scenario | p-value | Effect Size | Verdict |
|---|---|---|---|
| Gene expression differs by 0.01% (n=100,000) | p < 0.001 | Negligible | Statistically significant, practically meaningless |
| Drug reduces tumor by 40% (n=12) | p = 0.03 | Large | Both statistically and practically significant |
| Biomarker differs by 5% (n=20) | p = 0.08 | Moderate | Not significant — but maybe underpowered |

Always report effect sizes alongside p-values. We will dedicate Day 19 entirely to this topic.

## The z-Test: The Simplest Hypothesis Test

When the population standard deviation sigma is known (rare, but a good starting point), the z-test compares a sample mean to a known value:

**z = (x-bar - mu0) / (sigma / sqrt(n))**

Under H0, z follows a standard normal distribution N(0, 1).

## One-Tailed vs Two-Tailed Tests

| Test Type | H1 | p-value Calculation | Use When |
|---|---|---|---|
| Two-tailed | mu != mu0 | 2 x P(Z > abs(z)) | You care about differences in either direction |
| Right-tailed | mu > mu0 | P(Z > z) | You only care if the value is higher |
| Left-tailed | mu < mu0 | P(Z < z) | You only care if the value is lower |

## Hypothesis Testing in BioLang

### z-Test on Biomarker Data

```bio
# Alzheimer's biomarker study
# Known population SD from large reference database: sigma = 1.2 pg/mL
# Expected normal level: mu0 = 2.9 pg/mL
# Observed in 40 Alzheimer's patients:
let ad_levels = [3.2, 4.1, 3.8, 2.7, 4.5, 3.3, 3.9, 4.2, 3.1, 3.6,
                 4.0, 3.5, 4.3, 3.7, 2.9, 3.8, 4.1, 3.4, 3.6, 4.4,
                 3.0, 3.9, 4.2, 3.3, 3.7, 4.0, 3.5, 3.8, 4.1, 3.2,
                 3.6, 3.4, 4.3, 3.1, 3.9, 3.7, 4.0, 3.5, 3.8, 3.3]

# Compute z-test manually (known sigma)
let n = len(ad_levels)
let x_bar = mean(ad_levels)
let se = 1.2 / sqrt(n)
let z_stat = (x_bar - 2.9) / se
let p_val = 2.0 * (1.0 - pnorm(abs(z_stat), 0, 1))

print("z-statistic: {z_stat:.4}")
print("p-value (two-tailed): {p_val:.6}")
print("Mean observed: {x_bar:.3} pg/mL")

if p_val < 0.05 {
  print("Reject H0: Alzheimer's group significantly differs from normal level")
} else {
  print("Fail to reject H0: Insufficient evidence of difference")
}
```

### Visualizing the Null Distribution

```bio
# Show where our test statistic falls on the null distribution
let z_obs = 4.71  # from the z-test above

# Generate the null distribution (standard normal)
let x_vals = range(-4.0, 4.0, 0.01)
let y_vals = x_vals |> map(|x| dnorm(x, 0, 1))

# Plot the null distribution with our observed z marked
density(x_vals, {title: "Null Distribution (Standard Normal)", x_label: "z-statistic", y_label: "Density", vlines: [z_obs], shade_above: 1.96, shade_below: -1.96})

print("Critical value (two-tailed, alpha=0.05): +/- 1.96")
print("Our z = {z_obs} falls far in the rejection region")
```

### Binomial Test: Is This Mutation Rate Elevated?

```bio
# In a cancer cohort, 18 of 100 patients carry a specific BRCA2 variant
# Population frequency is known to be 8%
# Compute binomial test using dbinom: P(X >= 18) when X ~ Binom(100, 0.08)
let p_val = 0.0
for k in range(18, 101) {
    p_val = p_val + dbinom(k, 100, 0.08)
}

print("Observed proportion: 18/100 = 18%")
print("Expected under H0: 8%")
print("p-value (one-sided): {p_val:.6}")

if p_val < 0.05 {
  print("Reject H0: Mutation rate is significantly elevated in this cohort")
} else {
  print("Fail to reject H0")
}
```

### Complete Hypothesis Test Workflow

```bio
# Full workflow: Is mean platelet count elevated in a disease cohort?
# Reference population: mu = 250 (x10^3/uL), sigma = 50
# Our 25 patients:
let platelets = [280, 310, 265, 295, 275, 320, 290, 305, 260, 285,
                 300, 270, 315, 288, 292, 278, 308, 282, 298, 272,
                 310, 295, 268, 302, 288]

# Step 1: State hypotheses
print("H0: mu = 250 (platelet count is normal)")
print("H1: mu > 250 (platelet count is elevated)")
print("alpha = 0.05, one-tailed test\n")

# Step 2: Compute test statistic
let n = len(platelets)
let x_bar = mean(platelets)
let se = 50 / sqrt(n)  # sigma is known
let z = (x_bar - 250) / se

# Step 3: Find p-value (one-tailed)
let p = 1.0 - pnorm(z, 0, 1)

# Step 4: Decision
print("Sample mean: {x_bar:.1}")
print("z-statistic: {z:.4}")
print("p-value (one-tailed): {p:.6}")

if p < 0.05 {
  print("\nDecision: Reject H0 at alpha = 0.05")
  print("Conclusion: Platelet count is significantly elevated in this cohort")
} else {
  print("\nDecision: Fail to reject H0")
}
```

### Interpreting p-Values with Simulated Data

```bio
set_seed(42)
# Demonstrate: under H0 (no effect), p-values are uniformly distributed

let p_values = []
for i in 1..1000 {
  # Generate two samples from the SAME distribution (H0 is true)
  let group1 = rnorm(20, 10, 3)
  let group2 = rnorm(20, 10, 3)
  let z_stat = (mean(group1) - mean(group2)) / (3.0 / sqrt(20))
  let p_val = 2.0 * (1.0 - pnorm(abs(z_stat), 0, 1))
  p_values = append(p_values, p_val)
}

# Count false positives at alpha = 0.05
let false_pos = p_values |> filter(|p| p < 0.05) |> len()
print("False positives out of 1000 null tests: {false_pos}")
print("Expected: ~50 (5% of 1000)")

histogram(p_values, {title: "p-Value Distribution Under the Null", x_label: "p-value", bins: 20})
```

### Connecting CIs and Hypothesis Tests

```bio
# Demonstrate: a 95% CI that excludes the null value corresponds to p < 0.05
let ad_levels = [3.2, 4.1, 3.8, 2.7, 4.5, 3.3, 3.9, 4.2, 3.1, 3.6,
                 4.0, 3.5, 4.3, 3.7, 2.9, 3.8, 4.1, 3.4, 3.6, 4.4]

# z-test: is the mean different from 2.9?
let n = len(ad_levels)
let x_bar = mean(ad_levels)
let se = 1.2 / sqrt(n)
let z_stat = (x_bar - 2.9) / se
let z_p = 2.0 * (1.0 - pnorm(abs(z_stat), 0, 1))
print("z-test p-value: {z_p:.6}")

# 95% CI for the mean (using known sigma)
let se2 = 1.2 / sqrt(n)
let ci_lower = x_bar - 1.96 * se2
let ci_upper = x_bar + 1.96 * se2
print("95% CI: [{ci_lower:.3}, {ci_upper:.3}]")
print("Null value (2.9) is {'outside' if ci_lower > 2.9 or ci_upper < 2.9 else 'inside'} the CI")
print("This matches the hypothesis test: p < 0.05 <=> CI excludes null value")
```

**Python:**

```python
import numpy as np
from scipy import stats

ad_levels = [3.2, 4.1, 3.8, 2.7, 4.5, 3.3, 3.9, 4.2, 3.1, 3.6,
             4.0, 3.5, 4.3, 3.7, 2.9, 3.8, 4.1, 3.4, 3.6, 4.4,
             3.0, 3.9, 4.2, 3.3, 3.7, 4.0, 3.5, 3.8, 4.1, 3.2,
             3.6, 3.4, 4.3, 3.1, 3.9, 3.7, 4.0, 3.5, 3.8, 3.3]

# z-test (manual — scipy doesn't have a built-in z-test for means)
z = (np.mean(ad_levels) - 2.9) / (1.2 / np.sqrt(len(ad_levels)))
p = 2 * (1 - stats.norm.cdf(abs(z)))
print(f"z = {z:.4f}, p = {p:.6f}")

# Binomial test
result = stats.binomtest(18, 100, 0.08, alternative='greater')
print(f"Binomial test p = {result.pvalue:.6f}")
```

**R:**

```r
ad_levels <- c(3.2, 4.1, 3.8, 2.7, 4.5, 3.3, 3.9, 4.2, 3.1, 3.6,
               4.0, 3.5, 4.3, 3.7, 2.9, 3.8, 4.1, 3.4, 3.6, 4.4,
               3.0, 3.9, 4.2, 3.3, 3.7, 4.0, 3.5, 3.8, 4.1, 3.2,
               3.6, 3.4, 4.3, 3.1, 3.9, 3.7, 4.0, 3.5, 3.8, 3.3)

# z-test (using BSDA package, or manual)
z <- (mean(ad_levels) - 2.9) / (1.2 / sqrt(length(ad_levels)))
p <- 2 * pnorm(-abs(z))
cat(sprintf("z = %.4f, p = %.6f\n", z, p))

# Binomial test
binom.test(18, 100, p = 0.08, alternative = "greater")
```

## Exercises

**Exercise 1: Formulate Hypotheses**

For each scenario, write the null and alternative hypotheses. State whether you would use a one-tailed or two-tailed test and why.

a) Does a new antibiotic reduce bacterial colony counts compared to placebo?
b) Is the GC content of a newly sequenced genome different from the expected 42%?
c) Do patients with the variant allele have higher LDL cholesterol?

**Exercise 2: z-Test on Gene Expression**

A reference database reports the mean expression of housekeeping gene GAPDH as 8.5 log2-CPM with sigma = 0.8 across thousands of samples. Your RNA-seq experiment on 15 samples yields a mean of 7.9. Is your experiment's GAPDH level significantly different?

```bio
let gapdh_expression = [7.5, 8.1, 7.8, 7.6, 8.3, 7.2, 8.0, 7.9,
                        8.2, 7.4, 7.7, 8.1, 7.6, 8.4, 7.3]

# TODO: Perform z-test with mu=8.5, sigma=0.8
# TODO: Interpret the result — what might explain a difference?
```

**Exercise 3: Simulate Type I Error Rate**

Run 10,000 z-tests where H0 is true (both groups from the same distribution). Count what fraction of p-values fall below 0.05, 0.01, and 0.001. Do these match expectations?

```bio

# TODO: Simulate 10,000 null tests
# TODO: Count p < 0.05, p < 0.01, p < 0.001
# TODO: Compare to expected rates (5%, 1%, 0.1%)
```

**Exercise 4: One-Tailed vs Two-Tailed**

Using the Alzheimer's biomarker data, compute the p-value for both a one-tailed test (H1: AD levels are *higher*) and a two-tailed test. What is the relationship between the two p-values?

```bio
let ad_levels = [3.2, 4.1, 3.8, 2.7, 4.5, 3.3, 3.9, 4.2, 3.1, 3.6,
                 4.0, 3.5, 4.3, 3.7, 2.9, 3.8, 4.1, 3.4, 3.6, 4.4]

# TODO: Compute z-stat manually, then get two-tailed and one-tailed p-values
# Two-tailed: 2 * (1 - pnorm(abs(z), 0, 1))
# One-tailed: 1 - pnorm(z, 0, 1)
# TODO: What is the mathematical relationship?
```

## Key Takeaways

- Hypothesis testing uses the **courtroom analogy**: H0 (innocence) is assumed until the evidence (data) is overwhelming
- The **p-value** is the probability of data this extreme under H0 — it is NOT the probability H0 is true
- **Type I error** (false positive) is controlled by alpha; **Type II error** (false negative) is controlled by power
- **Statistical significance** (small p) does not imply **practical significance** (large effect)
- The **z-test** is the simplest hypothesis test, applicable when sigma is known
- Always state hypotheses and choose alpha **before** looking at data
- Under the null, p-values are uniformly distributed: at alpha = 0.05, exactly 5% of null tests will be "significant" by chance

## What's Next

Tomorrow we move from the z-test (which requires known sigma) to the workhorse of biological research: the t-test. You will learn independent, paired, and Welch's versions, check assumptions with Shapiro-Wilk and Levene's tests, and quantify effect sizes with Cohen's d. If hypothesis testing is the question, the t-test is the answer for two-group comparisons.
