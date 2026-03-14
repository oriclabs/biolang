# Day 14: Linear Regression — Prediction from Data

## The Problem

Dr. James Park is a pharmacogenomicist working with the NCI-60 cell line panel — 60 cancer cell lines spanning 9 tissue types. He has gene expression data and drug sensitivity measurements (IC50 values) for each line. His question: **Can we predict how sensitive a cell line will be to a new kinase inhibitor based on its expression of the drug's target gene?**

He knows the target gene and IC50 seem correlated (Day 13 confirmed r = -0.72). But correlation just says "they move together." James needs to go further: given a **new cell line** with a known expression level, what IC50 should he **predict**? And how confident should he be?

This is the leap from **association** to **prediction** — the domain of linear regression.

## What Is Linear Regression?

Linear regression fits a straight line through data points to model the relationship between a **predictor** (X) and a **response** (Y):

$$Y = \beta_0 + \beta_1 X + \varepsilon$$

| Term | Meaning | Biological Example |
|------|---------|-------------------|
| Y | Response variable | Drug IC50 |
| X | Predictor variable | Target gene expression |
| β₀ | Intercept (Y when X = 0) | Baseline sensitivity |
| β₁ | Slope (change in Y per unit X) | Sensitivity per expression unit |
| ε | Error (noise) | Biological + technical variation |

The method of **least squares** finds the β₀ and β₁ that minimize the sum of squared residuals:

$$\min_{\beta_0, \beta_1} \sum_{i=1}^{n} (y_i - \beta_0 - \beta_1 x_i)^2$$

> **Key insight:** Regression has a clear asymmetry that correlation lacks. X predicts Y — there is a designated predictor and a designated outcome. The regression of Y on X is NOT the same as X on Y.

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="380" viewBox="0 0 680 380" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="24" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">Linear Regression: Fitted Line and Residuals</text>
  <text x="340" y="44" text-anchor="middle" font-size="12" fill="#6b7280">Residuals = vertical distance from each point to the line</text>
  <!-- Axes -->
  <line x1="80" y1="310" x2="620" y2="310" stroke="#6b7280" stroke-width="1.5"/>
  <line x1="80" y1="310" x2="80" y2="60" stroke="#6b7280" stroke-width="1.5"/>
  <text x="350" y="345" text-anchor="middle" font-size="13" fill="#6b7280">Target Gene Expression (log2 FPKM)</text>
  <text x="30" y="185" text-anchor="middle" font-size="13" fill="#6b7280" transform="rotate(-90,30,185)">Drug IC50 (uM)</text>
  <!-- Regression line (negative slope) -->
  <line x1="100" y1="100" x2="600" y2="280" stroke="#2563eb" stroke-width="2.5"/>
  <!-- Data points with residual lines -->
  <!-- Point 1: above line -->
  <circle cx="140" cy="100" r="5" fill="#1e293b"/>
  <line x1="140" y1="100" x2="140" y2="128" stroke="#dc2626" stroke-width="1.5" stroke-dasharray="4,2"/>
  <!-- Point 2: below line -->
  <circle cx="180" cy="170" r="5" fill="#1e293b"/>
  <line x1="180" y1="170" x2="180" y2="148" stroke="#dc2626" stroke-width="1.5" stroke-dasharray="4,2"/>
  <!-- Point 3: above line -->
  <circle cx="230" cy="140" r="5" fill="#1e293b"/>
  <line x1="230" y1="140" x2="230" y2="165" stroke="#dc2626" stroke-width="1.5" stroke-dasharray="4,2"/>
  <!-- Point 4: below line -->
  <circle cx="280" cy="215" r="5" fill="#1e293b"/>
  <line x1="280" y1="215" x2="280" y2="185" stroke="#dc2626" stroke-width="1.5" stroke-dasharray="4,2"/>
  <!-- Point 5: above line -->
  <circle cx="320" cy="175" r="5" fill="#1e293b"/>
  <line x1="320" y1="175" x2="320" y2="199" stroke="#dc2626" stroke-width="1.5" stroke-dasharray="4,2"/>
  <!-- Point 6: below line -->
  <circle cx="370" cy="240" r="5" fill="#1e293b"/>
  <line x1="370" y1="240" x2="370" y2="217" stroke="#dc2626" stroke-width="1.5" stroke-dasharray="4,2"/>
  <!-- Point 7: above line -->
  <circle cx="410" cy="215" r="5" fill="#1e293b"/>
  <line x1="410" y1="215" x2="410" y2="231" stroke="#dc2626" stroke-width="1.5" stroke-dasharray="4,2"/>
  <!-- Point 8: below line -->
  <circle cx="460" cy="275" r="5" fill="#1e293b"/>
  <line x1="460" y1="275" x2="460" y2="249" stroke="#dc2626" stroke-width="1.5" stroke-dasharray="4,2"/>
  <!-- Point 9: above line -->
  <circle cx="510" cy="245" r="5" fill="#1e293b"/>
  <line x1="510" y1="245" x2="510" y2="267" stroke="#dc2626" stroke-width="1.5" stroke-dasharray="4,2"/>
  <!-- Point 10: below line -->
  <circle cx="560" cy="295" r="5" fill="#1e293b"/>
  <line x1="560" y1="295" x2="560" y2="278" stroke="#dc2626" stroke-width="1.5" stroke-dasharray="4,2"/>
  <!-- Annotations -->
  <text x="600" y="95" text-anchor="start" font-size="12" fill="#2563eb" font-weight="bold">Y = b0 + b1*X</text>
  <!-- Residual label -->
  <line x1="285" y1="185" x2="285" y2="215" stroke="#dc2626" stroke-width="2"/>
  <text x="295" y="205" font-size="11" fill="#dc2626">residual</text>
  <!-- Legend -->
  <rect x="140" y="355" width="400" height="22" rx="4" fill="#f1f5f9" stroke="#e2e8f0"/>
  <line x1="160" y1="366" x2="180" y2="366" stroke="#2563eb" stroke-width="2.5"/>
  <text x="188" y="370" font-size="11" fill="#1e293b">Regression line (least squares)</text>
  <line x1="360" y1="366" x2="380" y2="366" stroke="#dc2626" stroke-width="1.5" stroke-dasharray="4,2"/>
  <text x="388" y="370" font-size="11" fill="#1e293b">Residuals (errors)</text>
</svg>
</div>

## From Correlation to Regression

Correlation and simple regression are intimately linked:

| Metric | What It Tells You |
|--------|-------------------|
| Pearson r | Strength and direction of linear association |
| R² = r² | Proportion of variance in Y explained by X |
| β₁ (slope) | How much Y changes per unit change in X |
| p-value of β₁ | Whether the slope is significantly different from zero |

If r = -0.72, then R² = 0.52, meaning 52% of the variation in IC50 is explained by target expression. The remaining 48% is unexplained — due to other genes, pathway redundancy, or noise.

## Interpreting Regression Output

A regression produces a table of coefficients:

| Term | Estimate | Std. Error | t-value | p-value |
|------|----------|-----------|---------|---------|
| Intercept (β₀) | 85.3 | 6.2 | 13.8 | < 0.001 |
| Expression (β₁) | -4.7 | 0.8 | -5.9 | < 0.001 |

**Reading this:**
- **Intercept:** When expression = 0, predicted IC50 is 85.3 μM (often not biologically meaningful)
- **Slope:** Each 1-unit increase in expression decreases IC50 by 4.7 μM (more expression → more sensitive)
- **p-value:** The slope is significantly different from zero — expression truly predicts sensitivity
- **R²:** How well the line fits overall

> **Common pitfall:** The intercept often represents an extrapolation beyond the data range. If expression ranges from 5-15, interpreting "IC50 when expression = 0" is meaningless. Focus on the slope.

## Prediction: Point Estimates and Intervals

Once we have a fitted model, we can predict Y for new X values. But predictions come with two types of uncertainty:

**Confidence Interval (for the mean response):**
"We're 95% confident the **average** IC50 for all cell lines with expression = 10 lies in this range."

**Prediction Interval (for a single new observation):**
"We're 95% confident the IC50 for **one specific new cell line** with expression = 10 lies in this range."

Prediction intervals are always wider than confidence intervals because they include individual-level noise (ε).

> **Clinical relevance:** In precision medicine, you're usually predicting for **one patient** (prediction interval), not the population average (confidence interval). The prediction interval honestly reflects your uncertainty.

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="360" viewBox="0 0 680 360" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="24" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">Confidence Interval vs Prediction Interval</text>
  <text x="340" y="44" text-anchor="middle" font-size="12" fill="#6b7280">Prediction intervals are always wider (include individual noise)</text>
  <!-- Axes -->
  <line x1="80" y1="300" x2="600" y2="300" stroke="#6b7280" stroke-width="1.5"/>
  <line x1="80" y1="300" x2="80" y2="60" stroke="#6b7280" stroke-width="1.5"/>
  <text x="340" y="330" text-anchor="middle" font-size="13" fill="#6b7280">X (predictor)</text>
  <text x="35" y="180" text-anchor="middle" font-size="13" fill="#6b7280" transform="rotate(-90,35,180)">Y (response)</text>
  <!-- Prediction interval band (wider, lighter) -->
  <polygon points="110,240 200,215 300,180 400,150 500,120 580,100 580,270 500,248 400,218 300,190 200,225 110,260" fill="#dc2626" fill-opacity="0.08" stroke="none"/>
  <line x1="110" y1="240" x2="580" y2="100" stroke="#dc2626" stroke-width="1" stroke-dasharray="5,3" stroke-opacity="0.5"/>
  <line x1="110" y1="260" x2="580" y2="270" stroke="#dc2626" stroke-width="1" stroke-dasharray="5,3" stroke-opacity="0.5"/>
  <!-- Confidence interval band (narrower, darker) -->
  <polygon points="110,248 200,222 300,186 400,160 500,134 580,118 580,152 500,168 400,192 300,195 200,232 110,256" fill="#2563eb" fill-opacity="0.15" stroke="none"/>
  <line x1="110" y1="248" x2="580" y2="118" stroke="#2563eb" stroke-width="1.5" stroke-opacity="0.5"/>
  <line x1="110" y1="256" x2="580" y2="152" stroke="#2563eb" stroke-width="1.5" stroke-opacity="0.5"/>
  <!-- Regression line -->
  <line x1="110" y1="252" x2="580" y2="135" stroke="#1e293b" stroke-width="2.5"/>
  <!-- Data points -->
  <circle cx="130" cy="245" r="3.5" fill="#6b7280"/><circle cx="160" cy="255" r="3.5" fill="#6b7280"/>
  <circle cx="190" cy="235" r="3.5" fill="#6b7280"/><circle cx="220" cy="220" r="3.5" fill="#6b7280"/>
  <circle cx="250" cy="225" r="3.5" fill="#6b7280"/><circle cx="280" cy="200" r="3.5" fill="#6b7280"/>
  <circle cx="310" cy="195" r="3.5" fill="#6b7280"/><circle cx="340" cy="185" r="3.5" fill="#6b7280"/>
  <circle cx="370" cy="175" r="3.5" fill="#6b7280"/><circle cx="400" cy="168" r="3.5" fill="#6b7280"/>
  <circle cx="430" cy="160" r="3.5" fill="#6b7280"/><circle cx="460" cy="155" r="3.5" fill="#6b7280"/>
  <circle cx="490" cy="148" r="3.5" fill="#6b7280"/><circle cx="520" cy="140" r="3.5" fill="#6b7280"/>
  <circle cx="550" cy="130" r="3.5" fill="#6b7280"/><circle cx="570" cy="145" r="3.5" fill="#6b7280"/>
  <!-- Labels for bands -->
  <line x1="585" y1="100" x2="620" y2="85" stroke="#dc2626" stroke-width="1"/>
  <text x="622" y="82" font-size="11" fill="#dc2626" font-weight="bold">Prediction</text>
  <text x="622" y="95" font-size="11" fill="#dc2626">Interval (95%)</text>
  <line x1="585" y1="122" x2="620" y2="122" stroke="#2563eb" stroke-width="1"/>
  <text x="622" y="120" font-size="11" fill="#2563eb" font-weight="bold">Confidence</text>
  <text x="622" y="133" font-size="11" fill="#2563eb">Interval (95%)</text>
  <!-- Annotation: narrowest at mean of X -->
  <line x1="340" y1="300" x2="340" y2="165" stroke="#7c3aed" stroke-width="1" stroke-dasharray="3,2"/>
  <text x="340" y="315" text-anchor="middle" font-size="10" fill="#7c3aed">mean of X (narrowest)</text>
</svg>
</div>

## Residual Analysis: Checking Your Model

A regression model makes assumptions. **Residuals** (observed - predicted) reveal violations:

| Plot | What to Check | Warning Sign |
|------|---------------|-------------|
| Residuals vs. Fitted | Constant variance (homoscedasticity) | Fan/funnel shape |
| Q-Q plot of residuals | Normality of errors | Curved pattern |
| Residuals vs. order | Independence | Systematic trend |
| Scale-Location | Variance trend | Upward slope |

**The four assumptions of linear regression:**
1. **Linearity:** Y is a linear function of X (check scatter plot)
2. **Independence:** Observations are independent (study design)
3. **Normality:** Residuals are normally distributed (Q-Q plot)
4. **Homoscedasticity:** Constant error variance (residuals vs. fitted)

<div style="text-align: center; margin: 2em 0;">
<svg width="680" height="230" viewBox="0 0 680 230" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="340" y="22" text-anchor="middle" font-size="14" font-weight="bold" fill="#1e293b">Residual Diagnostics: Good vs. Bad Patterns</text>
  <!-- Panel 1: Good - random scatter -->
  <rect x="18" y="38" width="200" height="140" rx="4" fill="#f8fafc" stroke="#e2e8f0"/>
  <text x="118" y="54" text-anchor="middle" font-size="12" font-weight="bold" fill="#16a34a">Good: Random</text>
  <line x1="35" y1="115" x2="200" y2="115" stroke="#9ca3af" stroke-width="1" stroke-dasharray="3,2"/>
  <line x1="35" y1="170" x2="200" y2="170" stroke="#d1d5db" stroke-width="0.5"/>
  <line x1="35" y1="170" x2="35" y2="60" stroke="#d1d5db" stroke-width="0.5"/>
  <text x="118" y="184" text-anchor="middle" font-size="10" fill="#6b7280">Fitted values</text>
  <text x="30" y="115" text-anchor="end" font-size="9" fill="#6b7280">0</text>
  <!-- Random dots around zero line -->
  <circle cx="50" cy="100" r="2.5" fill="#16a34a"/><circle cx="62" cy="125" r="2.5" fill="#16a34a"/>
  <circle cx="74" cy="108" r="2.5" fill="#16a34a"/><circle cx="86" cy="130" r="2.5" fill="#16a34a"/>
  <circle cx="98" cy="105" r="2.5" fill="#16a34a"/><circle cx="110" cy="120" r="2.5" fill="#16a34a"/>
  <circle cx="122" cy="98" r="2.5" fill="#16a34a"/><circle cx="134" cy="128" r="2.5" fill="#16a34a"/>
  <circle cx="146" cy="110" r="2.5" fill="#16a34a"/><circle cx="158" cy="95" r="2.5" fill="#16a34a"/>
  <circle cx="170" cy="122" r="2.5" fill="#16a34a"/><circle cx="182" cy="108" r="2.5" fill="#16a34a"/>
  <circle cx="55" cy="132" r="2.5" fill="#16a34a"/><circle cx="92" cy="95" r="2.5" fill="#16a34a"/>
  <circle cx="140" cy="100" r="2.5" fill="#16a34a"/><circle cx="190" cy="118" r="2.5" fill="#16a34a"/>
  <!-- Panel 2: Bad - curved pattern -->
  <rect x="240" y="38" width="200" height="140" rx="4" fill="#f8fafc" stroke="#e2e8f0"/>
  <text x="340" y="54" text-anchor="middle" font-size="12" font-weight="bold" fill="#dc2626">Bad: Curved</text>
  <line x1="257" y1="115" x2="422" y2="115" stroke="#9ca3af" stroke-width="1" stroke-dasharray="3,2"/>
  <line x1="257" y1="170" x2="422" y2="170" stroke="#d1d5db" stroke-width="0.5"/>
  <line x1="257" y1="170" x2="257" y2="60" stroke="#d1d5db" stroke-width="0.5"/>
  <text x="340" y="184" text-anchor="middle" font-size="10" fill="#6b7280">Fitted values</text>
  <!-- U-shaped pattern -->
  <circle cx="270" cy="90" r="2.5" fill="#dc2626"/><circle cx="282" cy="95" r="2.5" fill="#dc2626"/>
  <circle cx="294" cy="105" r="2.5" fill="#dc2626"/><circle cx="306" cy="115" r="2.5" fill="#dc2626"/>
  <circle cx="318" cy="128" r="2.5" fill="#dc2626"/><circle cx="330" cy="135" r="2.5" fill="#dc2626"/>
  <circle cx="342" cy="138" r="2.5" fill="#dc2626"/><circle cx="354" cy="135" r="2.5" fill="#dc2626"/>
  <circle cx="366" cy="125" r="2.5" fill="#dc2626"/><circle cx="378" cy="115" r="2.5" fill="#dc2626"/>
  <circle cx="390" cy="100" r="2.5" fill="#dc2626"/><circle cx="402" cy="88" r="2.5" fill="#dc2626"/>
  <path d="M 265 88 Q 340 145 415 85" fill="none" stroke="#dc2626" stroke-width="1.5" stroke-opacity="0.4"/>
  <!-- Panel 3: Bad - fan shape (heteroscedasticity) -->
  <rect x="462" y="38" width="200" height="140" rx="4" fill="#f8fafc" stroke="#e2e8f0"/>
  <text x="562" y="54" text-anchor="middle" font-size="12" font-weight="bold" fill="#f97316">Bad: Fan Shape</text>
  <line x1="479" y1="115" x2="644" y2="115" stroke="#9ca3af" stroke-width="1" stroke-dasharray="3,2"/>
  <line x1="479" y1="170" x2="644" y2="170" stroke="#d1d5db" stroke-width="0.5"/>
  <line x1="479" y1="170" x2="479" y2="60" stroke="#d1d5db" stroke-width="0.5"/>
  <text x="562" y="184" text-anchor="middle" font-size="10" fill="#6b7280">Fitted values</text>
  <!-- Fan/funnel pattern: spread increases -->
  <circle cx="492" cy="113" r="2.5" fill="#f97316"/><circle cx="500" cy="117" r="2.5" fill="#f97316"/>
  <circle cx="512" cy="110" r="2.5" fill="#f97316"/><circle cx="524" cy="122" r="2.5" fill="#f97316"/>
  <circle cx="536" cy="105" r="2.5" fill="#f97316"/><circle cx="548" cy="130" r="2.5" fill="#f97316"/>
  <circle cx="560" cy="95" r="2.5" fill="#f97316"/><circle cx="572" cy="140" r="2.5" fill="#f97316"/>
  <circle cx="584" cy="85" r="2.5" fill="#f97316"/><circle cx="596" cy="148" r="2.5" fill="#f97316"/>
  <circle cx="608" cy="75" r="2.5" fill="#f97316"/><circle cx="620" cy="155" r="2.5" fill="#f97316"/>
  <circle cx="632" cy="70" r="2.5" fill="#f97316"/><circle cx="638" cy="160" r="2.5" fill="#f97316"/>
  <!-- Fan lines -->
  <line x1="485" y1="114" x2="645" y2="68" stroke="#f97316" stroke-width="0.8" stroke-opacity="0.3"/>
  <line x1="485" y1="116" x2="645" y2="162" stroke="#f97316" stroke-width="0.8" stroke-opacity="0.3"/>
  <!-- Bottom labels -->
  <text x="118" y="210" text-anchor="middle" font-size="11" fill="#16a34a">Assumptions met</text>
  <text x="340" y="210" text-anchor="middle" font-size="11" fill="#dc2626">Non-linear relationship</text>
  <text x="562" y="210" text-anchor="middle" font-size="11" fill="#f97316">Heteroscedasticity</text>
</svg>
</div>

> **Common pitfall:** Many biological relationships are not linear on the original scale but become linear after log-transformation. Always consider transforming IC50 or expression values before fitting.

## When Regression Goes Wrong

### Extrapolation
Predicting beyond the range of your data is dangerous. If expression ranges from 5-15 in your training data, predicting IC50 for expression = 25 assumes the linear trend continues — it may not.

### Confounding
A significant regression doesn't prove causation. The apparent effect of expression on IC50 could be mediated by a third variable (e.g., tissue type).

### Non-linearity
If the true relationship is curved (threshold effect, saturation), a line fits poorly. Residual plots expose this.

### Influential Points
A single extreme observation can dramatically change the fitted line. Leverage and Cook's distance help identify such points.

## Linear Regression in BioLang

### Simple Linear Regression

```bio
set_seed(42)
# NCI-60 pharmacogenomics: predict IC50 from target expression
let n = 60

# Simulate expression and drug sensitivity
let expression = rnorm(n, 10, 3)
let ic50 = 85 - 4.5 * expression + rnorm(n, 0, 8)

# Fit simple linear regression
let model = lm(ic50, expression)

# Print model summary
print("=== Linear Regression Summary ===")
print("Intercept: {model.intercept}")
print("Slope: {model.slope}")
print("R²: {model.r_squared}")
print("Adjusted R²: {model.adj_r_squared}")
print("F-statistic p-value: {model.p_value}")
```

### Interpreting Coefficients

```bio
# Detailed coefficient table
print("=== Coefficients ===")
print("  Intercept: {model.intercept}")
print("  Expression β: {model.slope}")
print("  p-value: {model.p_value}")

# Interpretation
let slope = model.slope
print("\nFor each 1-unit increase in expression,")
print("IC50 decreases by {slope |> abs |> round(2)} μM")
print("R² = {model.r_squared |> round(3)}: expression explains")
print("{(model.r_squared * 100) |> round(1)}% of IC50 variation")
```

### Prediction with Intervals

```bio
# Predict IC50 for new cell lines
let new_expression = [7.0, 10.0, 13.0]

# Point predictions using model coefficients
print("=== Predictions ===")
for i in 0..3 {
    let predicted = model.slope * new_expression[i] + model.intercept
    print("Expression = {new_expression[i]}: Predicted IC50 = {predicted |> round(1)} μM")
}
```

### Residual Analysis

```bio
# Compute residuals and fitted values
let fitted = expression |> map(|x| model.slope * x + model.intercept)
let resid = []
for i in 0..n {
    resid = resid + [ic50[i] - fitted[i]]
}

# 1. Residuals vs Fitted — check for patterns
let resid_table = table({"Fitted": fitted, "Residual": resid})
plot(resid_table, {type: "scatter", x: "Fitted", y: "Residual",
    title: "Residuals vs Fitted"})

# 2. Check residual distribution
print("Residual summary:")
print(summary(resid))
```

### Visualization: Scatter with Regression Line

```bio
# Scatter plot with regression line
let plot_data = table({"Expression": expression, "IC50": ic50})
plot(plot_data, {type: "scatter", x: "Expression", y: "IC50",
    title: "Drug Sensitivity vs Target Expression (NCI-60)",
    x_label: "Target Gene Expression (log2 FPKM)",
    y_label: "Drug IC50 (μM)"})
```

### Demonstrating Problems: Extrapolation

```bio
set_seed(42)
# Show danger of extrapolation
let x = rnorm(50, 10, 3)
let y = 100 - 3 * x + 0.2 * x ** 2 + rnorm(50, 0, 3)

# Fit linear model in observed range
let model_extrap = lm(y, x)
print("R² in training range: {model_extrap.r_squared |> round(3)}")

# Predict within range — reasonable
let pred_in = model_extrap.slope * 10.0 + model_extrap.intercept
print("Prediction at x=10 (in range): {pred_in |> round(1)}")

# Predict outside range — dangerous!
let pred_out = model_extrap.slope * 25.0 + model_extrap.intercept
print("Prediction at x=25 (extrapolation): {pred_out |> round(1)}")
print("True value at x=25 would be: {(100 - 3*25 + 0.2*25**2) |> round(1)}")
print("Extrapolation error demonstrates the danger!")
```

### Working with Log-Transformed Data

```bio
set_seed(42)
# Many biological relationships are linear on the log scale
let dose = rnorm(40, 50, 25) |> map(|d| max(d, 0.1))
let response = 50 / (1 + (dose / 10) ** 1.5) + rnorm(40, 0, 3)

# Linear model on raw scale — poor fit
let model_raw = lm(response, dose)
print("R² (raw scale): {model_raw.r_squared |> round(3)}")

# Log-transform dose — much better
let log_dose = dose |> map(|d| log2(d))
let model_log = lm(response, log_dose)
print("R² (log scale): {model_log.r_squared |> round(3)}")

# Compare residual plots to see the difference
```

**Python:**

```python
import numpy as np
from scipy import stats
import statsmodels.api as sm

# Simple linear regression
X = sm.add_constant(expression)  # adds intercept
model = sm.OLS(ic50, X).fit()
print(model.summary())

# Prediction with intervals
predictions = model.get_prediction(sm.add_constant([7, 10, 13]))
pred_summary = predictions.summary_frame(alpha=0.05)
# obs_ci_lower/obs_ci_upper = prediction interval
# mean_ci_lower/mean_ci_upper = confidence interval

# Residual analysis
fig, axes = plt.subplots(1, 2, figsize=(10, 4))
axes[0].scatter(model.fittedvalues, model.resid)
axes[0].axhline(0, color='red')
stats.probplot(model.resid, plot=axes[1])
```

**R:**

```r
# Simple linear regression
model <- lm(ic50 ~ expression)
summary(model)

# Prediction with intervals
new_data <- data.frame(expression = c(7, 10, 13))
predict(model, new_data, interval = "confidence")
predict(model, new_data, interval = "prediction")

# Residual diagnostics — 4 diagnostic plots
par(mfrow = c(2, 2))
plot(model)
```

## Exercises

### Exercise 1: Fit and Interpret

A study measures tumor mutation burden (TMB) and neoantigen count across 80 melanoma samples. Fit a linear regression and answer: How many additional neoantigens does each additional mutation predict?

```bio
set_seed(42)
let n = 80
let tmb = rnorm(n, 200, 80)
let neoantigens = 5 + 0.15 * tmb + rnorm(n, 0, 12)

# 1. Fit lm(neoantigens, tmb)
# 2. What is the slope? Interpret it biologically
# 3. What is model.r_squared? Is TMB a good predictor of neoantigen count?
# 4. Predict neoantigen count for TMB = 100, 200, 400
```

### Exercise 2: Residual Diagnostics

Fit a linear model to the dose-response data below and use residual plots to determine whether the linear assumption holds. If not, what transformation improves the fit?

```bio
set_seed(42)
let dose = rnorm(60, 25, 12) |> map(|d| max(d, 1))
let effect = 20 * log2(dose) + rnorm(60, 0, 5)

# 1. Fit lm(effect, dose)
# 2. Create residuals vs fitted plot — what pattern do you see?
# 3. Try log-transforming dose and re-fitting
# 4. Compare R² values and residual patterns
```

### Exercise 3: Confidence vs. Prediction Intervals

Demonstrate why prediction intervals are always wider than confidence intervals. Generate data, fit a model, and plot both intervals across the predictor range.

```bio
set_seed(42)
let x = rnorm(50, 10, 5)
let y = 10 + 2 * x + rnorm(50, 0, 4)

# 1. Fit lm(y, x)
# 2. Generate predictions for x = 0, 2, 4, ..., 20
# 3. Use model.slope * xi + model.intercept for each
# 4. Compare point estimates at different x values
```

### Exercise 4: The Outlier Effect

Add a single influential outlier to well-behaved data and show how it changes the slope, R², and predictions. Then remove it and compare.

```bio
set_seed(42)
let x_clean = rnorm(49, 10, 2)
let y_clean = 5 + 1.5 * x_clean + rnorm(49, 0, 2)

# Add one extreme outlier at x=10, y=50 (should be ~20)
# 1. Fit model with and without the outlier
# 2. Compare slopes and R² values
# 3. How much does prediction at x=12 change?
```

## Key Takeaways

- Linear regression models Y = β₀ + β₁X + ε, using least squares to find the best-fit line
- **R²** tells you the proportion of variance explained; the **slope** tells you the rate of change
- **Prediction intervals** (for individuals) are always wider than **confidence intervals** (for the mean) — use the right one for your question
- **Residual analysis** is mandatory: check linearity, normality, and constant variance before trusting results
- **Log-transforming** variables often linearizes biological relationships (dose-response, expression-phenotype)
- Beware **extrapolation** — the linear trend may not continue beyond your data range
- A significant regression does not prove causation — confounders may drive the relationship
- Always visualize your data with a scatter plot before and after fitting

## What's Next

One predictor is rarely enough. Day 15 introduces **multiple regression**, where we predict outcomes from many variables simultaneously — and face new challenges like multicollinearity, model selection, and regularization.
