# Day 16: Logistic Regression — Binary Outcomes

## The Problem

Dr. Priya Sharma is an immuno-oncologist analyzing data from 180 melanoma patients who received anti-PD-1 immunotherapy. For each patient, she has three biomarkers — **tumor mutational burden (TMB)**, **PD-L1 expression**, and **microsatellite instability (MSI) status** — and one outcome: **response** (tumor shrank ≥30%) or **non-response**.

Her first instinct is to use linear regression, predicting response (coded 1/0) from the biomarkers. But the predictions come out as 1.3 for one patient and -0.2 for another. Probabilities can't be greater than 1 or less than 0.

She needs a method designed for **binary outcomes** — logistic regression.

## Why Linear Regression Fails for Binary Outcomes

When Y is binary (0 or 1), linear regression has fundamental problems:

| Problem | Consequence |
|---------|-------------|
| Predictions outside [0, 1] | Impossible probabilities (negative or > 100%) |
| Non-normal residuals | Residuals are binary, violating normality assumption |
| Heteroscedasticity | Variance depends on the predicted value |
| Non-linear relationship | The true probability follows an S-curve, not a line |

> **Key insight:** We need a function that maps any real number to the range [0, 1]. The **logistic (sigmoid) function** does exactly this.

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="340" viewBox="0 0 680 340" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="24" text-anchor="middle" font-size="14" font-weight="bold" fill="#1e293b">Why Linear Regression Fails for Binary Data</text>
  <!-- Left panel: Linear -->
  <g transform="translate(30, 40)">
    <text x="140" y="14" text-anchor="middle" font-size="12" font-weight="bold" fill="#dc2626">Linear Regression (Bad)</text>
    <!-- Axes -->
    <line x1="40" y1="240" x2="280" y2="240" stroke="#6b7280" stroke-width="1.5"/>
    <line x1="40" y1="240" x2="40" y2="30" stroke="#6b7280" stroke-width="1.5"/>
    <text x="160" y="268" text-anchor="middle" font-size="11" fill="#6b7280">Predictor (X)</text>
    <text x="12" y="135" text-anchor="middle" font-size="11" fill="#6b7280" transform="rotate(-90, 12, 135)">P(Y=1)</text>
    <!-- Y-axis labels -->
    <text x="34" y="244" text-anchor="end" font-size="10" fill="#6b7280">0</text>
    <text x="34" y="140" text-anchor="end" font-size="10" fill="#6b7280">0.5</text>
    <text x="34" y="36" text-anchor="end" font-size="10" fill="#6b7280">1.0</text>
    <line x1="37" y1="136" x2="43" y2="136" stroke="#9ca3af" stroke-width="1"/>
    <line x1="37" y1="32" x2="43" y2="32" stroke="#9ca3af" stroke-width="1"/>
    <!-- Scatter points (0/1) -->
    <circle cx="60" cy="238" r="4" fill="#3b82f6" opacity="0.5"/><circle cx="75" cy="238" r="4" fill="#3b82f6" opacity="0.5"/>
    <circle cx="85" cy="32" r="4" fill="#3b82f6" opacity="0.5"/><circle cx="95" cy="238" r="4" fill="#3b82f6" opacity="0.5"/>
    <circle cx="110" cy="238" r="4" fill="#3b82f6" opacity="0.5"/><circle cx="125" cy="32" r="4" fill="#3b82f6" opacity="0.5"/>
    <circle cx="140" cy="238" r="4" fill="#3b82f6" opacity="0.5"/><circle cx="155" cy="32" r="4" fill="#3b82f6" opacity="0.5"/>
    <circle cx="165" cy="32" r="4" fill="#3b82f6" opacity="0.5"/><circle cx="180" cy="238" r="4" fill="#3b82f6" opacity="0.5"/>
    <circle cx="195" cy="32" r="4" fill="#3b82f6" opacity="0.5"/><circle cx="210" cy="32" r="4" fill="#3b82f6" opacity="0.5"/>
    <circle cx="225" cy="32" r="4" fill="#3b82f6" opacity="0.5"/><circle cx="240" cy="32" r="4" fill="#3b82f6" opacity="0.5"/>
    <circle cx="255" cy="238" r="4" fill="#3b82f6" opacity="0.5"/><circle cx="265" cy="32" r="4" fill="#3b82f6" opacity="0.5"/>
    <!-- Linear fit line (goes below 0 and above 1) -->
    <line x1="40" y1="260" x2="280" y2="10" stroke="#dc2626" stroke-width="2.5" stroke-dasharray="6,3"/>
    <!-- Problem annotations -->
    <rect x="220" y="4" width="65" height="18" rx="3" fill="#fef2f2" stroke="#dc2626" stroke-width="0.5"/>
    <text x="252" y="16" text-anchor="middle" font-size="9" fill="#dc2626" font-weight="bold">P > 1.0 !</text>
    <rect x="42" y="252" width="65" height="18" rx="3" fill="#fef2f2" stroke="#dc2626" stroke-width="0.5"/>
    <text x="74" y="264" text-anchor="middle" font-size="9" fill="#dc2626" font-weight="bold">P < 0.0 !</text>
  </g>
  <!-- Right panel: Logistic -->
  <g transform="translate(360, 40)">
    <text x="140" y="14" text-anchor="middle" font-size="12" font-weight="bold" fill="#16a34a">Logistic Regression (Good)</text>
    <!-- Axes -->
    <line x1="40" y1="240" x2="280" y2="240" stroke="#6b7280" stroke-width="1.5"/>
    <line x1="40" y1="240" x2="40" y2="30" stroke="#6b7280" stroke-width="1.5"/>
    <text x="160" y="268" text-anchor="middle" font-size="11" fill="#6b7280">Predictor (X)</text>
    <text x="34" y="244" text-anchor="end" font-size="10" fill="#6b7280">0</text>
    <text x="34" y="140" text-anchor="end" font-size="10" fill="#6b7280">0.5</text>
    <text x="34" y="36" text-anchor="end" font-size="10" fill="#6b7280">1.0</text>
    <line x1="37" y1="136" x2="43" y2="136" stroke="#9ca3af" stroke-width="1"/>
    <line x1="37" y1="32" x2="43" y2="32" stroke="#9ca3af" stroke-width="1"/>
    <!-- Scatter points (same) -->
    <circle cx="60" cy="238" r="4" fill="#3b82f6" opacity="0.5"/><circle cx="75" cy="238" r="4" fill="#3b82f6" opacity="0.5"/>
    <circle cx="85" cy="32" r="4" fill="#3b82f6" opacity="0.5"/><circle cx="95" cy="238" r="4" fill="#3b82f6" opacity="0.5"/>
    <circle cx="110" cy="238" r="4" fill="#3b82f6" opacity="0.5"/><circle cx="125" cy="32" r="4" fill="#3b82f6" opacity="0.5"/>
    <circle cx="140" cy="238" r="4" fill="#3b82f6" opacity="0.5"/><circle cx="155" cy="32" r="4" fill="#3b82f6" opacity="0.5"/>
    <circle cx="165" cy="32" r="4" fill="#3b82f6" opacity="0.5"/><circle cx="180" cy="238" r="4" fill="#3b82f6" opacity="0.5"/>
    <circle cx="195" cy="32" r="4" fill="#3b82f6" opacity="0.5"/><circle cx="210" cy="32" r="4" fill="#3b82f6" opacity="0.5"/>
    <circle cx="225" cy="32" r="4" fill="#3b82f6" opacity="0.5"/><circle cx="240" cy="32" r="4" fill="#3b82f6" opacity="0.5"/>
    <circle cx="255" cy="238" r="4" fill="#3b82f6" opacity="0.5"/><circle cx="265" cy="32" r="4" fill="#3b82f6" opacity="0.5"/>
    <!-- Sigmoid curve -->
    <path d="M 40,236 C 60,236 80,234 100,228 C 120,218 130,200 145,170 C 155,148 160,125 165,136 C 170,120 180,90 200,60 C 220,42 240,36 260,34 C 270,33 275,32 280,32" stroke="#16a34a" stroke-width="2.5" fill="none"/>
    <!-- Bounds -->
    <line x1="40" y1="32" x2="280" y2="32" stroke="#16a34a" stroke-width="0.5" stroke-dasharray="4,4" opacity="0.5"/>
    <line x1="40" y1="240" x2="280" y2="240" stroke="#16a34a" stroke-width="0.5" stroke-dasharray="4,4" opacity="0.5"/>
    <rect x="195" y="28" width="90" height="18" rx="3" fill="#f0fdf4" stroke="#16a34a" stroke-width="0.5"/>
    <text x="240" y="40" text-anchor="middle" font-size="9" fill="#16a34a" font-weight="bold">Always in [0, 1]</text>
  </g>
</svg>
</div>

## The Logistic Function

The logistic regression model predicts the **probability** of the outcome being 1:

$$P(Y=1|X) = \frac{1}{1 + e^{-(\beta_0 + \beta_1 X_1 + \cdots + \beta_p X_p)}}$$

The sigmoid function transforms the linear predictor (which ranges from -∞ to +∞) into a probability (which ranges from 0 to 1):

| Linear predictor (β₀ + β₁X) | Probability P(Y=1) |
|------|------|
| -5 | 0.007 |
| -2 | 0.12 |
| 0 | 0.50 |
| +2 | 0.88 |
| +5 | 0.993 |

The curve is steepest at P = 0.5 (the decision boundary) and flattens at the extremes — exactly how biological responses behave.

<div style="text-align: center; margin: 2em 0;">
<svg width="650" height="320" viewBox="0 0 650 320" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="325" y="24" text-anchor="middle" font-size="14" font-weight="bold" fill="#1e293b">The Sigmoid (Logistic) Function</text>
  <g transform="translate(80, 40)">
    <!-- Grid lines -->
    <line x1="60" y1="50" x2="480" y2="50" stroke="#e5e7eb" stroke-width="0.5"/>
    <line x1="60" y1="125" x2="480" y2="125" stroke="#e5e7eb" stroke-width="0.5"/>
    <line x1="60" y1="200" x2="480" y2="200" stroke="#e5e7eb" stroke-width="0.5"/>
    <line x1="270" y1="30" x2="270" y2="250" stroke="#e5e7eb" stroke-width="0.5"/>
    <!-- Axes -->
    <line x1="60" y1="250" x2="480" y2="250" stroke="#6b7280" stroke-width="1.5"/>
    <line x1="60" y1="250" x2="60" y2="30" stroke="#6b7280" stroke-width="1.5"/>
    <!-- Y-axis labels -->
    <text x="52" y="254" text-anchor="end" font-size="11" fill="#6b7280">0.0</text>
    <text x="52" y="204" text-anchor="end" font-size="11" fill="#6b7280">0.25</text>
    <text x="52" y="129" text-anchor="end" font-size="11" fill="#6b7280">0.50</text>
    <text x="52" y="54" text-anchor="end" font-size="11" fill="#6b7280">1.0</text>
    <!-- X-axis labels -->
    <text x="60" y="270" text-anchor="middle" font-size="11" fill="#6b7280">-6</text>
    <text x="130" y="270" text-anchor="middle" font-size="11" fill="#6b7280">-4</text>
    <text x="200" y="270" text-anchor="middle" font-size="11" fill="#6b7280">-2</text>
    <text x="270" y="270" text-anchor="middle" font-size="11" fill="#6b7280">0</text>
    <text x="340" y="270" text-anchor="middle" font-size="11" fill="#6b7280">+2</text>
    <text x="410" y="270" text-anchor="middle" font-size="11" fill="#6b7280">+4</text>
    <text x="480" y="270" text-anchor="middle" font-size="11" fill="#6b7280">+6</text>
    <text x="270" y="290" text-anchor="middle" font-size="12" fill="#6b7280">Linear predictor (z = B0 + B1*X)</text>
    <text x="20" y="140" text-anchor="middle" font-size="12" fill="#6b7280" transform="rotate(-90, 20, 140)">P(Y = 1)</text>
    <!-- Sigmoid curve -->
    <path d="M 60,247 C 80,246 100,245 120,242 C 140,237 160,228 180,215 C 200,195 220,170 240,150 C 250,138 260,130 270,125 C 280,120 290,112 300,100 C 320,80 340,65 360,55 C 380,50 400,48 420,47 C 460,46 480,45 480,45" stroke="#2563eb" stroke-width="3" fill="none"/>
    <!-- Inflection point at (0, 0.5) -->
    <circle cx="270" cy="125" r="6" fill="#2563eb" stroke="white" stroke-width="2"/>
    <line x1="270" y1="125" x2="270" y2="250" stroke="#2563eb" stroke-width="1" stroke-dasharray="4,3" opacity="0.4"/>
    <line x1="60" y1="125" x2="270" y2="125" stroke="#2563eb" stroke-width="1" stroke-dasharray="4,3" opacity="0.4"/>
    <!-- Annotation: inflection point -->
    <rect x="280" y="110" width="130" height="20" rx="4" fill="#eff6ff" stroke="#2563eb" stroke-width="0.5"/>
    <text x="345" y="124" text-anchor="middle" font-size="10" fill="#2563eb" font-weight="bold">Inflection at P = 0.5</text>
    <!-- Asymptote labels -->
    <line x1="60" y1="50" x2="480" y2="50" stroke="#9ca3af" stroke-width="0.5" stroke-dasharray="3,3"/>
    <line x1="60" y1="250" x2="480" y2="250" stroke="#9ca3af" stroke-width="0.5" stroke-dasharray="3,3"/>
    <text x="488" y="48" font-size="9" fill="#9ca3af">P = 1</text>
    <text x="488" y="248" font-size="9" fill="#9ca3af">P = 0</text>
    <!-- Region labels -->
    <text x="110" y="220" text-anchor="middle" font-size="10" fill="#6b7280" font-style="italic">Low risk</text>
    <text x="440" y="75" text-anchor="middle" font-size="10" fill="#6b7280" font-style="italic">High risk</text>
  </g>
</svg>
</div>

## Interpreting Coefficients: Log-Odds and Odds Ratios

Logistic regression coefficients are on the **log-odds** scale:

$$\ln\left(\frac{P}{1-P}\right) = \beta_0 + \beta_1 X_1 + \beta_2 X_2 + \cdots$$

This is less intuitive than linear regression. The key transformation:

$$\text{Odds Ratio} = e^{\beta}$$

| β (log-odds) | OR = e^β | Interpretation |
|------|------|------|
| 0 | 1.0 | No effect |
| 0.5 | 1.65 | 65% higher odds per unit increase |
| 1.0 | 2.72 | 2.7× higher odds |
| -0.5 | 0.61 | 39% lower odds |
| -1.0 | 0.37 | 63% lower odds |

> **Key insight:** An odds ratio of 2.0 means the odds of response **double** for each 1-unit increase in the predictor. It does NOT mean the probability doubles — that depends on the baseline probability.

### Odds vs. Probability

| Probability | Odds | Interpretation |
|------------|------|----------------|
| 0.50 | 1:1 | Equal chance either way |
| 0.75 | 3:1 | Three times as likely to respond |
| 0.20 | 1:4 | Four times as likely NOT to respond |
| 0.90 | 9:1 | Strong favoring response |

> **Common pitfall:** When the outcome is common (prevalence > 10%), odds ratios substantially overestimate relative risks. An OR of 3.0 for a 50% baseline event means the probability goes from 50% to 75% — a relative risk of only 1.5. Report both OR and absolute probabilities.

## ROC Curves and AUC

The model outputs probabilities. To make yes/no predictions, you need a **threshold** — above it, predict "responder." But which threshold?

A **Receiver Operating Characteristic (ROC) curve** plots sensitivity vs. (1 - specificity) across all possible thresholds:

| Metric | Definition | Trade-off |
|--------|-----------|-----------|
| Sensitivity (TPR) | True responders correctly identified | Higher = catch more responders |
| Specificity (TNR) | True non-responders correctly identified | Higher = fewer false alarms |
| PPV (Precision) | Positive predictions that are correct | Higher = trust positive results |
| NPV | Negative predictions that are correct | Higher = trust negative results |

The **Area Under the Curve (AUC)** summarizes overall discrimination:

| AUC | Performance |
|-----|-------------|
| 0.50 | Random guessing (useless) |
| 0.60-0.70 | Poor |
| 0.70-0.80 | Acceptable |
| 0.80-0.90 | Good |
| 0.90-1.00 | Excellent |

> **Clinical relevance:** In cancer diagnostics, the threshold choice depends on context. Screening tests favor **high sensitivity** (don't miss cancers), while confirmatory tests favor **high specificity** (don't cause unnecessary biopsies).

<div style="text-align: center; margin: 2em 0;">
<svg width="650" height="380" viewBox="0 0 650 380" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="325" y="24" text-anchor="middle" font-size="14" font-weight="bold" fill="#1e293b">ROC Curve and AUC</text>
  <g transform="translate(100, 40)">
    <!-- Axes -->
    <line x1="60" y1="290" x2="420" y2="290" stroke="#6b7280" stroke-width="1.5"/>
    <line x1="60" y1="290" x2="60" y2="20" stroke="#6b7280" stroke-width="1.5"/>
    <!-- Grid -->
    <line x1="60" y1="20" x2="420" y2="20" stroke="#e5e7eb" stroke-width="0.5"/>
    <line x1="60" y1="155" x2="420" y2="155" stroke="#e5e7eb" stroke-width="0.5"/>
    <line x1="240" y1="20" x2="240" y2="290" stroke="#e5e7eb" stroke-width="0.5"/>
    <line x1="420" y1="20" x2="420" y2="290" stroke="#e5e7eb" stroke-width="0.5"/>
    <!-- Axis labels -->
    <text x="240" y="318" text-anchor="middle" font-size="12" fill="#6b7280">1 - Specificity (False Positive Rate)</text>
    <text x="15" y="155" text-anchor="middle" font-size="12" fill="#6b7280" transform="rotate(-90, 15, 155)">Sensitivity (True Positive Rate)</text>
    <!-- Tick labels -->
    <text x="60" y="306" text-anchor="middle" font-size="10" fill="#6b7280">0.0</text>
    <text x="150" y="306" text-anchor="middle" font-size="10" fill="#6b7280">0.25</text>
    <text x="240" y="306" text-anchor="middle" font-size="10" fill="#6b7280">0.50</text>
    <text x="330" y="306" text-anchor="middle" font-size="10" fill="#6b7280">0.75</text>
    <text x="420" y="306" text-anchor="middle" font-size="10" fill="#6b7280">1.0</text>
    <text x="52" y="294" text-anchor="end" font-size="10" fill="#6b7280">0.0</text>
    <text x="52" y="159" text-anchor="end" font-size="10" fill="#6b7280">0.5</text>
    <text x="52" y="24" text-anchor="end" font-size="10" fill="#6b7280">1.0</text>
    <!-- AUC shaded area -->
    <path d="M 60,290 L 60,260 C 80,220 100,160 130,110 C 160,70 200,45 240,35 C 280,28 340,23 420,20 L 420,290 Z" fill="#2563eb" opacity="0.12"/>
    <!-- Diagonal (random classifier) -->
    <line x1="60" y1="290" x2="420" y2="20" stroke="#9ca3af" stroke-width="1.5" stroke-dasharray="6,4"/>
    <text x="280" y="185" font-size="10" fill="#9ca3af" transform="rotate(-37, 280, 185)">Random (AUC = 0.50)</text>
    <!-- ROC curve -->
    <path d="M 60,290 C 60,260 65,230 75,200 C 85,170 100,140 120,115 C 140,90 165,68 200,50 C 230,38 270,30 310,26 C 350,23 390,21 420,20" stroke="#2563eb" stroke-width="3" fill="none"/>
    <!-- AUC label -->
    <rect x="200" y="170" width="120" height="40" rx="6" fill="white" stroke="#2563eb" stroke-width="1"/>
    <text x="260" y="188" text-anchor="middle" font-size="11" fill="#2563eb" font-weight="bold">AUC = 0.85</text>
    <text x="260" y="203" text-anchor="middle" font-size="10" fill="#6b7280">(Good discrimination)</text>
    <!-- Optimal threshold point -->
    <circle cx="120" cy="115" r="6" fill="#dc2626" stroke="white" stroke-width="2"/>
    <line x1="128" y1="108" x2="185" y2="80" stroke="#dc2626" stroke-width="0.8"/>
    <rect x="185" y="68" width="108" height="18" rx="3" fill="#fef2f2" stroke="#dc2626" stroke-width="0.5"/>
    <text x="239" y="80" text-anchor="middle" font-size="9" fill="#dc2626" font-weight="bold">Optimal threshold</text>
    <!-- Legend -->
    <line x1="300" y1="250" x2="325" y2="250" stroke="#2563eb" stroke-width="2.5"/>
    <text x="330" y="254" font-size="10" fill="#1e293b">Model ROC</text>
    <line x1="300" y1="268" x2="325" y2="268" stroke="#9ca3af" stroke-width="1.5" stroke-dasharray="6,4"/>
    <text x="330" y="272" font-size="10" fill="#1e293b">Random</text>
  </g>
</svg>
</div>

## The GLM Framework

Logistic regression is a special case of the **Generalized Linear Model (GLM)** — a flexible framework for non-normal outcomes:

| Outcome Type | Distribution | Link Function | Name |
|-------------|-------------|---------------|------|
| Continuous | Normal | Identity | Linear regression |
| Binary (0/1) | Binomial | Logit | Logistic regression |
| Counts | Poisson | Log | Poisson regression |
| Counts (overdispersed) | Negative binomial | Log | NB regression |
| Positive continuous | Gamma | Inverse | Gamma regression |

The GLM framework unifies these under one interface, letting you choose the appropriate model for your data type.

> **Clinical relevance:** RNA-seq read counts follow a negative binomial distribution. Tools like DESeq2 use GLMs with a negative binomial family — the same framework as logistic regression, just with a different distribution assumption.

## Logistic Regression in BioLang

### Fitting a Logistic Model

```bio
set_seed(42)
# Immunotherapy response prediction
let n = 180

# Simulate predictors
let tmb = rnorm(n, 10, 5) |> map(|x| max(x, 0))
let pdl1 = rnorm(n, 30, 20) |> map(|x| max(min(x, 100), 0))
let msi = rnorm(n, 0, 1) |> map(|x| if x > 1.0 { 1 } else { 0 })  # ~15% MSI-high

# True response probability (logistic model)
let log_odds = -3 + 0.15 * tmb + 0.02 * pdl1 + 1.5 * msi
let prob = log_odds |> map(|x| 1.0 / (1.0 + exp(-x)))
let response = prob |> map(|p| if rnorm(1, 0, 1)[0] < p { 1 } else { 0 })

print("Response rate: {(response |> sum) / n * 100 |> round(1)}%")

# Fit logistic regression
let glm_data = table({"response": response, "TMB": tmb, "PDL1": pdl1, "MSI": msi})
let model = glm("response ~ TMB + PDL1 + MSI", glm_data, "binomial")

print("=== Logistic Regression Results ===")
print("Intercept: {model.intercept |> round(3)}")
```

### Interpreting Odds Ratios

```bio
# Convert coefficients to odds ratios
let coef_names = ["TMB", "PD-L1", "MSI"]
let coefficients = model.coefficients

print("=== Odds Ratios ===")
for i in 0..3 {
    let beta = coefficients[i]
    let or_val = exp(beta)
    print("{coef_names[i]}: β = {beta |> round(3)}, OR = {or_val |> round(2)}")
}

# Interpretation:
# TMB OR = 1.16 means each additional mut/Mb increases odds of response by 16%
# MSI OR = 4.5 means MSI-high patients have 4.5x the odds of responding
```

### ROC Curve and AUC

```bio
# Compute predicted probabilities from model
let pred_prob = []
for i in 0..n {
    let lp = model.intercept + model.coefficients[0] * tmb[i]
        + model.coefficients[1] * pdl1[i] + model.coefficients[2] * msi[i]
    pred_prob = pred_prob + [1.0 / (1.0 + exp(-lp))]
}

# ROC curve
let roc_data = table({"actual": response, "predicted": pred_prob})
roc_curve(roc_data)

# Compute AUC
let auc_val = model.auc
print("AUC: {auc_val |> round(3)}")

# Interpretation
if auc_val >= 0.80 {
    print("Good discrimination")
} else if auc_val >= 0.70 {
    print("Acceptable discrimination")
} else {
    print("Poor discrimination — model needs additional predictors")
}
```

### Finding the Optimal Threshold

```bio
# Sensitivity/specificity at different thresholds
let thresholds = [0.2, 0.3, 0.4, 0.5, 0.6, 0.7]

print("=== Threshold Analysis ===")
print("Threshold  Sensitivity  Specificity  PPV      NPV")

for t in thresholds {
    let predicted_class = pred_prob |> map(|p| if p >= t { 1 } else { 0 })
    let tp = 0
    let fp = 0
    let tn = 0
    let fn = 0

    for i in 0..n {
        if predicted_class[i] == 1 && response[i] == 1 { tp = tp + 1 }
        if predicted_class[i] == 1 && response[i] == 0 { fp = fp + 1 }
        if predicted_class[i] == 0 && response[i] == 0 { tn = tn + 1 }
        if predicted_class[i] == 0 && response[i] == 1 { fn = fn + 1 }
    }

    let sens = tp / (tp + fn)
    let spec = tn / (tn + fp)
    let ppv = if tp + fp > 0 { tp / (tp + fp) } else { 0 }
    let npv = if tn + fn > 0 { tn / (tn + fn) } else { 0 }

    print("{t}        {sens |> round(3)}       {spec |> round(3)}      {ppv |> round(3)}    {npv |> round(3)}")
}

# Youden's J: optimal balance of sensitivity and specificity
```

### Using the GLM Framework

```bio
# Logistic regression via the general GLM interface
let model_glm = glm("response ~ TMB + PDL1 + MSI", glm_data, "binomial")

# Same results, but now you can swap families:

# Poisson regression for count data (e.g., number of mutations)
# let count_data = table({"mutations": mutation_count, "exposure": exposure, "age": age})
# let count_model = glm("mutations ~ exposure + age", count_data, "poisson")
```

### Visualizing Predicted Probabilities

```bio
# Box plot: predicted probabilities by actual outcome
let resp_probs = []
let nonresp_probs = []

for i in 0..n {
    if response[i] == 1 {
        resp_probs = resp_probs + [pred_prob[i]]
    } else {
        nonresp_probs = nonresp_probs + [pred_prob[i]]
    }
}

let bp_table = table({"Non-Responders": nonresp_probs, "Responders": resp_probs})
boxplot(bp_table, {title: "Predicted Probabilities by Actual Outcome"})

# Good model: minimal overlap between the two boxes
```

### Effect of Individual Predictors

```bio
# Show how each predictor shifts the probability curve
# Fix other predictors at their means
let tmb_range = [0, 5, 10, 15, 20, 25, 30]
let pdl1_mean = 30
let msi_0 = 0

print("=== TMB Effect on Response Probability ===")
print("(PD-L1 = {pdl1_mean}, MSI = negative)")

for t in tmb_range {
    let lp = model.intercept + model.coefficients[0] * t
        + model.coefficients[1] * pdl1_mean + model.coefficients[2] * msi_0
    let p = 1.0 / (1.0 + exp(-lp))
    print("  TMB = {t}: P(response) = {(p * 100) |> round(1)}%")
}
```

**Python:**

```python
from sklearn.linear_model import LogisticRegression
from sklearn.metrics import roc_curve, auc, classification_report
import statsmodels.api as sm

# Statsmodels (full inference)
X = sm.add_constant(df[['TMB', 'PDL1', 'MSI']])
model = sm.Logit(response, X).fit()
print(model.summary())

# Odds ratios
print(np.exp(model.params))
print(np.exp(model.conf_int()))

# ROC curve
fpr, tpr, thresholds = roc_curve(response, model.predict(X))
roc_auc = auc(fpr, tpr)
plt.plot(fpr, tpr, label=f'AUC = {roc_auc:.2f}')

# Scikit-learn (prediction-focused)
clf = LogisticRegression().fit(X_train, y_train)
y_prob = clf.predict_proba(X_test)[:, 1]
```

**R:**

```r
# Logistic regression
model <- glm(response ~ TMB + PDL1 + MSI, family = binomial)
summary(model)

# Odds ratios with CI
exp(cbind(OR = coef(model), confint(model)))

# ROC curve
library(pROC)
roc_obj <- roc(response, fitted(model))
plot(roc_obj, print.auc = TRUE)

# Optimal threshold (Youden's J)
coords(roc_obj, "best", best.method = "youden")
```

## Exercises

### Exercise 1: Build and Interpret a Logistic Model

Predict cancer diagnosis (1 = cancer, 0 = benign) from three biomarkers. Interpret each odds ratio in clinical terms.

```bio
set_seed(42)
let n = 200

let biomarker_a = rnorm(n, 50, 15)
let biomarker_b = rnorm(n, 10, 4)
let age = rnorm(n, 55, 12)

let log_odds = -5 + 0.04 * biomarker_a + 0.2 * biomarker_b + 0.03 * age
let prob = log_odds |> map(|x| 1.0 / (1.0 + exp(-x)))
let cancer = prob |> map(|p| if rnorm(1, 0, 1)[0] < p { 1 } else { 0 })

# 1. Fit glm("cancer ~ biomarker_a + biomarker_b + age", data, "binomial")
# 2. Compute and interpret odds ratios: exp(coefficient) for each predictor
# 3. Which biomarker has the strongest effect?
# 4. What does the OR for age mean clinically?
```

### Exercise 2: ROC Analysis

Build a logistic model and evaluate it with an ROC curve. Find the threshold that maximizes Youden's J index (sensitivity + specificity - 1).

```bio
# Use the model from Exercise 1
# 1. Generate predicted probabilities from model coefficients
# 2. Plot ROC curve with roc_curve(table)
# 3. Compute sensitivity and specificity at thresholds 0.3, 0.5, 0.7
# 4. Which threshold maximizes Youden's J?
# 5. If this is a screening test, would you prefer a different threshold? Why?
```

### Exercise 3: Comparing Two Models

Build two logistic models: one with TMB alone, another with TMB + PD-L1 + MSI. Compare their AUC values. Does adding predictors improve discrimination?

```bio
set_seed(42)
let n = 150

# Simulate data where TMB is a moderate predictor and MSI adds substantial value
# 1. Fit model_simple = glm("response ~ TMB", data, "binomial")
# 2. Fit model_full = glm("response ~ TMB + PDL1 + MSI", data, "binomial")
# 3. Compare AUC values
# 4. Plot both ROC curves
# 5. Is the improvement worth the added complexity?
```

### Exercise 4: The Separation Problem

What happens when a predictor perfectly separates outcomes? Simulate MSI status that perfectly predicts response and observe what logistic regression does.

```bio
set_seed(42)
let n = 100
let msi = rnorm(n, 0, 1) |> map(|x| if x > 0.84 { 1 } else { 0 })
let response = msi  # perfect separation!

# 1. Try fitting glm("response ~ msi", data, "binomial")
# 2. What happens to the coefficient and its standard error?
# 3. Why is this a problem? (Hint: the MLE doesn't exist)
# 4. How would you handle this in practice?
```

## Key Takeaways

- **Logistic regression** models binary outcomes by predicting probabilities via the sigmoid function — use it instead of linear regression when Y is 0/1
- Coefficients are on the **log-odds** scale; exponentiate to get **odds ratios** (OR): e^β
- An OR of 2.0 means the odds double per unit increase — but this is NOT the same as doubling the probability
- **ROC curves** show the sensitivity/specificity trade-off across all thresholds; **AUC** summarizes overall discrimination
- The **optimal threshold** depends on clinical context: screening favors sensitivity, confirmation favors specificity
- Logistic regression is a special case of the **GLM framework** — the same approach extends to count data (Poisson), survival data, and more
- Always report odds ratios with **confidence intervals**, not just p-values
- Watch for **separation** (a predictor perfectly predicts the outcome), which causes coefficient inflation

## What's Next

Sometimes the outcome isn't just binary — it's **time-to-event**: how long until a patient relapses, how long a cell line survives treatment. Day 17 introduces **survival analysis**, where censoring makes standard methods fail and Kaplan-Meier curves become essential.
