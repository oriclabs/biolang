# Day 15: Multiple Regression and Model Selection

## The Problem

Dr. Maria Chen is a clinical researcher studying pancreatic cancer. She has tumor samples from 120 patients, each profiled with 10 biomarkers: CA19-9, CEA, MKI67, TP53 status, tumor size, age, albumin, CRP, neutrophil-lymphocyte ratio (NLR), and platelet count. She wants to predict **tumor stage** (a continuous composite score from 1.0 to 4.0) from these biomarkers.

But there's a problem. CA19-9 and CEA are highly correlated (r = 0.88) — they measure overlapping biology. Including both inflates standard errors and makes coefficients uninterpretable. And with 10 potential predictors, how does she find the **best subset** without overfitting?

She needs multiple regression with careful **model selection**.

## What Is Multiple Regression?

Multiple regression extends simple regression to multiple predictors:

$$Y = \beta_0 + \beta_1 X_1 + \beta_2 X_2 + \cdots + \beta_p X_p + \varepsilon$$

Each coefficient βⱼ represents the effect of Xⱼ **holding all other predictors constant**. This is fundamentally different from running p separate simple regressions.

| Simple Regression | Multiple Regression |
|-------------------|---------------------|
| β₁ = total effect of X₁ on Y | β₁ = effect of X₁ **after accounting for** X₂...Xₚ |
| One predictor at a time | All predictors simultaneously |
| Can't separate correlated effects | Separates correlated effects (when possible) |
| May show spurious associations | Controls for confounders |

> **Key insight:** In simple regression, tumor size might predict stage with β = 0.8. In multiple regression controlling for CA19-9, the tumor size coefficient might drop to β = 0.3 — because CA19-9 captures much of the same information.

## Multicollinearity: When Predictors Are Too Similar

**Multicollinearity** occurs when predictors are highly correlated with each other. It doesn't bias predictions, but it wreaks havoc on coefficient interpretation:

| Effect | Consequence |
|--------|-------------|
| Inflated standard errors | Coefficients appear non-significant when they are |
| Unstable coefficients | Small data changes cause wild coefficient swings |
| Sign flipping | A predictor with a positive true effect can get a negative coefficient |
| Uninterpretable | "Effect of X₁ holding X₂ constant" is meaningless if X₁ and X₂ always move together |

### Detecting Multicollinearity: VIF

The **Variance Inflation Factor** quantifies how much each predictor is explained by the others:

$$VIF_j = \frac{1}{1 - R_j^2}$$

where R²ⱼ is the R² from regressing Xⱼ on all other predictors.

| VIF | Interpretation | Action |
|-----|---------------|--------|
| 1 | No collinearity | Good |
| 1-5 | Moderate | Usually acceptable |
| 5-10 | High | Investigate |
| > 10 | Severe | Remove or combine predictors |

> **Common pitfall:** VIF > 10 doesn't always mean "drop the variable." In genomics, biologically meaningful predictors may be correlated. Consider combining them (e.g., principal component) or using regularization instead.

<div style="text-align: center; margin: 2em 0;">
<svg width="660" height="280" viewBox="0 0 660 280" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="330" y="24" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">Multicollinearity: Overlapping Explained Variance</text>
  <text x="330" y="44" text-anchor="middle" font-size="12" fill="#6b7280">When predictors share information, their individual effects become ambiguous</text>
  <!-- Low collinearity (left) -->
  <text x="180" y="70" text-anchor="middle" font-size="13" font-weight="bold" fill="#16a34a">Low Collinearity (VIF ~ 1)</text>
  <circle cx="140" cy="155" r="60" fill="#2563eb" fill-opacity="0.2" stroke="#2563eb" stroke-width="2"/>
  <circle cx="220" cy="155" r="60" fill="#dc2626" fill-opacity="0.2" stroke="#dc2626" stroke-width="2"/>
  <text x="120" y="155" text-anchor="middle" font-size="12" font-weight="bold" fill="#2563eb">CA19-9</text>
  <text x="240" y="155" text-anchor="middle" font-size="12" font-weight="bold" fill="#dc2626">MKI67</text>
  <text x="180" y="160" text-anchor="middle" font-size="10" fill="#6b7280">small</text>
  <text x="180" y="172" text-anchor="middle" font-size="10" fill="#6b7280">overlap</text>
  <text x="180" y="235" text-anchor="middle" font-size="11" fill="#16a34a">Each predictor contributes</text>
  <text x="180" y="250" text-anchor="middle" font-size="11" fill="#16a34a">unique information</text>
  <!-- High collinearity (right) -->
  <text x="490" y="70" text-anchor="middle" font-size="13" font-weight="bold" fill="#dc2626">High Collinearity (VIF > 10)</text>
  <circle cx="460" cy="155" r="65" fill="#2563eb" fill-opacity="0.2" stroke="#2563eb" stroke-width="2"/>
  <circle cx="520" cy="155" r="65" fill="#dc2626" fill-opacity="0.2" stroke="#dc2626" stroke-width="2"/>
  <text x="430" y="145" text-anchor="middle" font-size="12" font-weight="bold" fill="#2563eb">CA19-9</text>
  <text x="550" y="145" text-anchor="middle" font-size="12" font-weight="bold" fill="#dc2626">CEA</text>
  <!-- Large overlap shading -->
  <text x="490" y="160" text-anchor="middle" font-size="11" font-weight="bold" fill="#7c3aed">LARGE</text>
  <text x="490" y="175" text-anchor="middle" font-size="11" font-weight="bold" fill="#7c3aed">overlap</text>
  <text x="490" y="235" text-anchor="middle" font-size="11" fill="#dc2626">Hard to separate effects;</text>
  <text x="490" y="250" text-anchor="middle" font-size="11" fill="#dc2626">coefficients unstable</text>
  <!-- Divider -->
  <line x1="330" y1="60" x2="330" y2="260" stroke="#e2e8f0" stroke-width="1" stroke-dasharray="4,3"/>
</svg>
</div>

## Model Selection: Finding the Best Model

With p predictors, there are 2ᵖ possible models. For p = 10, that's 1024 models. We need principled ways to choose.

### Information Criteria

| Criterion | Formula | Preference |
|-----------|---------|------------|
| AIC (Akaike) | -2·ln(L) + 2k | Lower = better; balances fit and complexity |
| BIC (Bayesian) | -2·ln(L) + k·ln(n) | Lower = better; penalizes complexity more than AIC |
| Adjusted R² | 1 - (1-R²)·(n-1)/(n-k-1) | Higher = better; penalizes added predictors |

Where L = likelihood, k = number of parameters, n = sample size.

**AIC** tends to select slightly larger models (better prediction). **BIC** tends to select smaller models (better interpretation). When they disagree, consider your goal.

<div style="text-align: center; margin: 2em 0;">
<svg width="660" height="320" viewBox="0 0 660 320" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="330" y="24" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">Model Selection: AIC/BIC Tradeoff</text>
  <text x="330" y="44" text-anchor="middle" font-size="12" fill="#6b7280">Balancing model fit against complexity</text>
  <!-- Axes -->
  <line x1="80" y1="260" x2="600" y2="260" stroke="#6b7280" stroke-width="1.5"/>
  <line x1="80" y1="260" x2="80" y2="60" stroke="#6b7280" stroke-width="1.5"/>
  <text x="340" y="295" text-anchor="middle" font-size="13" fill="#6b7280">Number of Predictors (model complexity)</text>
  <text x="30" y="160" text-anchor="middle" font-size="13" fill="#6b7280" transform="rotate(-90,30,160)">Information Criterion</text>
  <!-- X-axis ticks -->
  <text x="130" y="278" text-anchor="middle" font-size="11" fill="#6b7280">1</text>
  <text x="200" y="278" text-anchor="middle" font-size="11" fill="#6b7280">2</text>
  <text x="270" y="278" text-anchor="middle" font-size="11" fill="#6b7280">3</text>
  <text x="340" y="278" text-anchor="middle" font-size="11" fill="#6b7280">4</text>
  <text x="410" y="278" text-anchor="middle" font-size="11" fill="#6b7280">5</text>
  <text x="480" y="278" text-anchor="middle" font-size="11" fill="#6b7280">6</text>
  <text x="550" y="278" text-anchor="middle" font-size="11" fill="#6b7280">7</text>
  <!-- AIC curve (U-shaped, minimum at 4) -->
  <path d="M 130,200 Q 180,140 270,105 Q 320,92 340,90 Q 400,95 480,115 Q 540,140 550,150" fill="none" stroke="#2563eb" stroke-width="2.5"/>
  <circle cx="130" cy="200" r="4" fill="#2563eb"/><circle cx="200" cy="140" r="4" fill="#2563eb"/>
  <circle cx="270" cy="105" r="4" fill="#2563eb"/><circle cx="340" cy="90" r="4" fill="#2563eb"/>
  <circle cx="410" cy="98" r="4" fill="#2563eb"/><circle cx="480" cy="115" r="4" fill="#2563eb"/>
  <circle cx="550" cy="150" r="4" fill="#2563eb"/>
  <!-- BIC curve (U-shaped, minimum at 3 - penalizes more) -->
  <path d="M 130,210 Q 180,155 270,108 Q 300,102 270,100 Q 350,112 410,135 Q 480,165 550,200" fill="none" stroke="#dc2626" stroke-width="2.5"/>
  <circle cx="130" cy="210" r="4" fill="#dc2626"/><circle cx="200" cy="148" r="4" fill="#dc2626"/>
  <circle cx="270" cy="100" r="4" fill="#dc2626"/><circle cx="340" cy="112" r="4" fill="#dc2626"/>
  <circle cx="410" cy="135" r="4" fill="#dc2626"/><circle cx="480" cy="165" r="4" fill="#dc2626"/>
  <circle cx="550" cy="200" r="4" fill="#dc2626"/>
  <!-- Optimal points -->
  <circle cx="340" cy="90" r="8" fill="none" stroke="#2563eb" stroke-width="2" stroke-dasharray="3,2"/>
  <line x1="340" y1="82" x2="340" y2="68" stroke="#2563eb" stroke-width="1"/>
  <text x="340" y="65" text-anchor="middle" font-size="11" font-weight="bold" fill="#2563eb">AIC min</text>
  <circle cx="270" cy="100" r="8" fill="none" stroke="#dc2626" stroke-width="2" stroke-dasharray="3,2"/>
  <line x1="270" y1="92" x2="255" y2="75" stroke="#dc2626" stroke-width="1"/>
  <text x="235" y="72" text-anchor="middle" font-size="11" font-weight="bold" fill="#dc2626">BIC min</text>
  <!-- Zone annotations -->
  <rect x="90" y="230" width="120" height="22" rx="4" fill="#f97316" fill-opacity="0.1"/>
  <text x="150" y="245" text-anchor="middle" font-size="10" fill="#f97316">Underfitting</text>
  <rect x="450" y="230" width="130" height="22" rx="4" fill="#f97316" fill-opacity="0.1"/>
  <text x="515" y="245" text-anchor="middle" font-size="10" fill="#f97316">Overfitting</text>
  <!-- Legend -->
  <rect x="160" y="300" width="340" height="18" rx="4" fill="#f1f5f9"/>
  <line x1="180" y1="309" x2="210" y2="309" stroke="#2563eb" stroke-width="2.5"/>
  <text x="220" y="313" font-size="11" fill="#2563eb">AIC (favors prediction)</text>
  <line x1="360" y1="309" x2="390" y2="309" stroke="#dc2626" stroke-width="2.5"/>
  <text x="400" y="313" font-size="11" fill="#dc2626">BIC (favors parsimony)</text>
</svg>
</div>

### Stepwise Regression

Automated search through predictor combinations:

| Direction | Strategy | Risk |
|-----------|----------|------|
| **Forward** | Start empty, add best predictor one at a time | May miss suppressor effects |
| **Backward** | Start full, remove worst predictor one at a time | May keep redundant predictors |
| **Both** | Add and remove at each step | Best coverage, slower |

> **Common pitfall:** Stepwise regression is exploratory, not confirmatory. The selected model may not replicate. Always validate on held-out data or use cross-validation.

## Regularized Regression: Handling Many Predictors

When you have many predictors (especially in genomics where p >> n), ordinary least squares fails. Regularization adds a penalty term:

### Ridge Regression (L2)

$$\min \sum(y_i - \hat{y}_i)^2 + \lambda \sum \beta_j^2$$

- Shrinks coefficients toward zero but never to exactly zero
- Handles multicollinearity gracefully
- Good when all predictors might be relevant

### Lasso Regression (L1)

$$\min \sum(y_i - \hat{y}_i)^2 + \lambda \sum |\beta_j|$$

- Can shrink coefficients to exactly zero (automatic variable selection)
- Produces sparse models (easy to interpret)
- Preferred when you suspect only a few predictors matter

### Elastic Net

$$\min \sum(y_i - \hat{y}_i)^2 + \lambda_1 \sum |\beta_j| + \lambda_2 \sum \beta_j^2$$

- Combines L1 and L2 penalties
- Good when predictors are correlated groups (keeps all or drops all from a group)
- Controlled by mixing parameter α (0 = ridge, 1 = lasso)

| Method | Feature Selection | Correlated Predictors | Best For |
|--------|------------------|-----------------------|----------|
| Ridge | No (shrinks all) | Handles well | Many weak effects |
| Lasso | Yes (zeros out) | Picks one arbitrarily | Few strong effects |
| Elastic Net | Yes (grouped) | Handles well | Correlated groups |

<div style="text-align: center; margin: 2em 0;">
<svg width="660" height="280" viewBox="0 0 660 280" xmlns="http://www.w3.org/2000/svg" style="background: #fafafa; border: 1px solid #e5e7eb; border-radius: 8px;">
  <text x="330" y="24" text-anchor="middle" font-size="15" font-weight="bold" fill="#1e293b">VIF: Variance Inflation from Redundant Predictors</text>
  <text x="330" y="44" text-anchor="middle" font-size="12" fill="#6b7280">VIF measures how much each predictor is explained by the others</text>
  <!-- Response variable (Y) at top -->
  <rect x="260" y="60" width="140" height="36" rx="18" fill="#1e293b" fill-opacity="0.08" stroke="#1e293b" stroke-width="2"/>
  <text x="330" y="83" text-anchor="middle" font-size="13" font-weight="bold" fill="#1e293b">Tumor Stage (Y)</text>
  <!-- Three predictor circles, overlapping -->
  <!-- X1: Tumor Size -->
  <circle cx="190" cy="185" r="55" fill="#2563eb" fill-opacity="0.15" stroke="#2563eb" stroke-width="2"/>
  <text x="155" y="180" text-anchor="middle" font-size="11" font-weight="bold" fill="#2563eb">Tumor</text>
  <text x="155" y="195" text-anchor="middle" font-size="11" font-weight="bold" fill="#2563eb">Size</text>
  <text x="155" y="212" text-anchor="middle" font-size="10" fill="#2563eb">VIF = 1.2</text>
  <!-- X2: CA19-9 -->
  <circle cx="310" cy="185" r="55" fill="#7c3aed" fill-opacity="0.15" stroke="#7c3aed" stroke-width="2"/>
  <text x="310" y="180" text-anchor="middle" font-size="11" font-weight="bold" fill="#7c3aed">CA19-9</text>
  <text x="310" y="212" text-anchor="middle" font-size="10" fill="#7c3aed">VIF = 8.5</text>
  <!-- X3: CEA -->
  <circle cx="410" cy="185" r="55" fill="#dc2626" fill-opacity="0.15" stroke="#dc2626" stroke-width="2"/>
  <text x="445" y="180" text-anchor="middle" font-size="11" font-weight="bold" fill="#dc2626">CEA</text>
  <text x="445" y="212" text-anchor="middle" font-size="10" fill="#dc2626">VIF = 9.2</text>
  <!-- Heavy overlap between CA19-9 and CEA -->
  <text x="360" y="188" text-anchor="middle" font-size="16" font-weight="bold" fill="#7c3aed" fill-opacity="0.6">||</text>
  <!-- Arrows to Y -->
  <line x1="190" y1="130" x2="285" y2="96" stroke="#2563eb" stroke-width="1.5" marker-end="url(#arrowBlk)"/>
  <line x1="310" y1="130" x2="315" y2="96" stroke="#7c3aed" stroke-width="1.5" marker-end="url(#arrowBlk)"/>
  <line x1="410" y1="130" x2="375" y2="96" stroke="#dc2626" stroke-width="1.5" marker-end="url(#arrowBlk)"/>
  <defs>
    <marker id="arrowBlk" markerWidth="8" markerHeight="6" refX="7" refY="3" orient="auto">
      <polygon points="0 0, 8 3, 0 6" fill="#6b7280"/>
    </marker>
  </defs>
  <!-- Annotation -->
  <rect x="480" y="155" width="165" height="60" rx="6" fill="#fef2f2" stroke="#fca5a5" stroke-width="1"/>
  <text x="562" y="173" text-anchor="middle" font-size="11" fill="#dc2626" font-weight="bold">CA19-9 and CEA</text>
  <text x="562" y="188" text-anchor="middle" font-size="11" fill="#dc2626">measure overlapping</text>
  <text x="562" y="203" text-anchor="middle" font-size="11" fill="#dc2626">biology (VIF ~ 9)</text>
  <!-- VIF scale at bottom -->
  <rect x="100" y="252" width="460" height="18" rx="3" fill="url(#vifGrad)"/>
  <defs>
    <linearGradient id="vifGrad" x1="0%" y1="0%" x2="100%" y2="0%">
      <stop offset="0%" stop-color="#22c55e"/>
      <stop offset="40%" stop-color="#f59e0b"/>
      <stop offset="100%" stop-color="#dc2626"/>
    </linearGradient>
  </defs>
  <text x="100" y="248" font-size="10" fill="#16a34a">VIF = 1</text>
  <text x="284" y="248" text-anchor="middle" font-size="10" fill="#f59e0b">VIF = 5</text>
  <text x="560" y="248" text-anchor="end" font-size="10" fill="#dc2626">VIF > 10</text>
  <text x="100" y="280" font-size="10" fill="#16a34a">No collinearity</text>
  <text x="284" y="280" text-anchor="middle" font-size="10" fill="#f59e0b">Investigate</text>
  <text x="560" y="280" text-anchor="end" font-size="10" fill="#dc2626">Remove or combine</text>
</svg>
</div>

## Polynomial Regression

When the relationship is curved but you want to stay in the regression framework:

$$Y = \beta_0 + \beta_1 X + \beta_2 X^2 + \varepsilon$$

Useful for dose-response curves, growth curves, and non-linear biomarker relationships. But beware overfitting with high-degree polynomials.

> **Clinical relevance:** The relationship between BMI and mortality is U-shaped — both low and high BMI increase risk. A linear model misses this entirely; a quadratic model captures it.

## Multiple Regression in BioLang

### Building a Multiple Regression Model

```bio
set_seed(42)
# Pancreatic cancer: predict tumor stage from biomarkers
let n = 120

# Simulate correlated biomarkers
let age = rnorm(n, 65, 10)
let tumor_size = rnorm(n, 3.5, 1.2)
let ca19_9 = tumor_size * 50 + rnorm(n, 100, 40)
let cea = ca19_9 * 0.3 + rnorm(n, 5, 3)  # correlated with CA19-9
let mki67 = rnorm(n, 30, 15)
let albumin = rnorm(n, 3.5, 0.5)
let crp = rnorm(n, 15, 10)
let nlr = rnorm(n, 4, 2)

# True model: stage depends on tumor_size, ca19_9, mki67, age
let stage = 1.0 + 0.4 * tumor_size + 0.003 * ca19_9 + 0.01 * mki67
    + 0.01 * age - 0.3 * albumin
    + rnorm(n, 0, 0.3)

# Fit multiple regression with all predictors
let data = table({
    "stage": stage, "age": age, "tumor_size": tumor_size,
    "ca19_9": ca19_9, "cea": cea, "mki67": mki67,
    "albumin": albumin, "crp": crp, "nlr": nlr
})
let model_full = lm("stage ~ age + tumor_size + ca19_9 + cea + mki67 + albumin + crp + nlr", data)

print("=== Full Model ===")
print("R²: {model_full.r_squared |> round(3)}")
print("Adjusted R²: {model_full.adj_r_squared |> round(3)}")
```

### Checking Multicollinearity

```bio
# Check multicollinearity via pairwise correlations
let pred_names = ["age", "tumor_size", "ca19_9", "cea", "mki67",
                  "albumin", "crp", "nlr"]
let predictors = [age, tumor_size, ca19_9, cea, mki67, albumin, crp, nlr]

print("=== Pairwise Correlations (VIF proxy) ===")
for i in 0..8 {
    for j in (i+1)..8 {
        let r = cor(predictors[i], predictors[j])
        if abs(r) > 0.7 {
            print("  {pred_names[i]} vs {pred_names[j]}: r = {r |> round(3)} *** HIGH")
        }
    }
}

# CA19-9 and CEA likely show high correlation
```

### Stepwise Model Selection

```bio
# Manual model comparison: fit reduced models and compare R²
# Drop CEA (collinear with CA19-9) and noise variables (CRP, NLR)
let data_reduced = table({
    "stage": stage, "age": age, "tumor_size": tumor_size,
    "ca19_9": ca19_9, "mki67": mki67, "albumin": albumin
})
let model_reduced = lm("stage ~ age + tumor_size + ca19_9 + mki67 + albumin", data_reduced)

print("=== Model Comparison ===")
print("Full model R²:    {model_full.r_squared |> round(3)}")
print("Full model Adj R²:    {model_full.adj_r_squared |> round(3)}")
print("Reduced model R²: {model_reduced.r_squared |> round(3)}")
print("Reduced model Adj R²: {model_reduced.adj_r_squared |> round(3)}")
print("If Adj R² is similar, the simpler model is preferred")
```

### Regularized Regression

```bio
# Regularized regression concepts
# Note: Ridge, Lasso, and Elastic Net are advanced methods
# typically used when p >> n (many predictors, few samples).
# BioLang provides lm() for standard regression; for regularization,
# use Python (scikit-learn) or R (glmnet) as shown below.

# Demonstrate the concept: compare full vs sparse models
# A "lasso-like" approach: fit models dropping one predictor at a time
# and see which predictors contribute the least
let predictors_list = ["age", "tumor_size", "ca19_9", "cea", "mki67", "albumin", "crp", "nlr"]

print("=== Variable Importance (drop-one analysis) ===")
let full_r2 = model_full.r_squared
print("Full model R²: {full_r2 |> round(4)}")

# Compare by dropping noise predictors
let model_no_crp = lm("stage ~ age + tumor_size + ca19_9 + cea + mki67 + albumin + nlr", data)
let model_no_nlr = lm("stage ~ age + tumor_size + ca19_9 + cea + mki67 + albumin + crp", data)
print("Without CRP: R² = {model_no_crp.r_squared |> round(4)}")
print("Without NLR: R² = {model_no_nlr.r_squared |> round(4)}")
print("Predictors with minimal R² drop are candidates for removal")
```

### Polynomial Regression

```bio
set_seed(42)
# Non-linear biomarker relationship
let bmi = rnorm(100, 29, 5)
let risk = 0.5 + 0.1 * (bmi - 25) ** 2 + rnorm(100, 0, 2)

# Linear fit — misses the U-shape
let model_linear = lm(risk, bmi)
print("Linear R²: {model_linear.r_squared |> round(3)}")

# Polynomial fit — add bmi² term
let bmi_sq = bmi |> map(|x| x ** 2)
let poly_data = table({"risk": risk, "bmi": bmi, "bmi_sq": bmi_sq})
let model_poly = lm("risk ~ bmi + bmi_sq", poly_data)
print("Quadratic R²: {model_poly.r_squared |> round(3)}")

# Visualize the improvement
let plot_data = table({"BMI": bmi, "Risk": risk})
plot(plot_data, {type: "scatter", x: "BMI", y: "Risk",
    title: "BMI vs Risk: Linear vs Quadratic Fit"})
```

### Predicted vs. Actual Plot

```bio
# The ultimate model validation plot
# Compute predicted values from the reduced model
let predicted_stage = model_reduced.fitted

let pred_actual = table({"Actual": stage, "Predicted": predicted_stage})
plot(pred_actual, {type: "scatter", x: "Actual", y: "Predicted",
    title: "Predicted vs Actual (Reduced Model)"})

# Points along the diagonal = good predictions
# Systematic deviations = model problems
let r_pred = cor(stage, predicted_stage)
print("Correlation (actual vs predicted): {r_pred |> round(3)}")
```

**Python:**

```python
import statsmodels.api as sm
from sklearn.linear_model import Lasso, Ridge, ElasticNet
from sklearn.preprocessing import PolynomialFeatures
from statsmodels.stats.outliers_influence import variance_inflation_factor

# Multiple regression
X = sm.add_constant(df[['age', 'tumor_size', 'ca19_9', 'cea', 'mki67']])
model = sm.OLS(stage, X).fit()
print(model.summary())

# VIF
vif = [variance_inflation_factor(X.values, i) for i in range(X.shape[1])]

# Stepwise (manual — no built-in in statsmodels)
# Use mlxtend: from mlxtend.feature_selection import SequentialFeatureSelector

# Lasso
from sklearn.linear_model import LassoCV
lasso = LassoCV(cv=5).fit(X_scaled, y)
print(f"Non-zero: {(lasso.coef_ != 0).sum()}")
```

**R:**

```r
# Multiple regression
model <- lm(stage ~ age + tumor_size + ca19_9 + cea + mki67 +
            albumin + crp + nlr)
summary(model)

# VIF
library(car)
vif(model)

# Stepwise
step_model <- step(model, direction = "both")

# Lasso
library(glmnet)
cv_lasso <- cv.glmnet(X_matrix, stage, alpha = 1)
coef(cv_lasso, s = "lambda.min")

# Ridge
cv_ridge <- cv.glmnet(X_matrix, stage, alpha = 0)

# Elastic net
cv_enet <- cv.glmnet(X_matrix, stage, alpha = 0.5)
```

## Exercises

### Exercise 1: Build and Compare Models

Given 150 samples with 6 predictors, build a full model, then use stepwise selection to find a reduced model. Compare using AIC, BIC, and adjusted R².

```bio
set_seed(42)
let n = 150
let x1 = rnorm(n, 10, 3)
let x2 = rnorm(n, 5, 2)
let x3 = x1 * 0.5 + rnorm(n, 0, 1)  # correlated with x1
let x4 = rnorm(n, 20, 5)
let x5 = rnorm(n, 0, 1)  # noise
let x6 = rnorm(n, 0, 1)  # noise

let y = 5 + 2 * x1 + 1.5 * x2 + 0.5 * x4 + rnorm(n, 0, 3)

# 1. Fit full model with all 6 predictors using lm()
# 2. Check pairwise cor() — which predictors are collinear?
# 3. Compare full vs reduced models by adj_r_squared
# 4. Which predictors matter? Do they match the true model?
```

### Exercise 2: Ridge vs. Lasso

Compare ridge and lasso on a dataset where only 3 of 8 predictors truly matter. Which method correctly identifies the true predictors?

```bio
set_seed(42)
let n = 100

# 8 predictors, only 3 are real
# Fit lm() with all 8, then with only the true 3
# Compare R² — does the reduced model perform similarly?
# Note: for true ridge/lasso, use Python (scikit-learn) or R (glmnet)
```

### Exercise 3: Polynomial vs. Linear

Fit linear, quadratic, and cubic models to a dose-response curve. Use AIC to select the best model, and show that adding unnecessary complexity (cubic) hurts.

```bio
set_seed(42)
let dose = rnorm(80, 25, 12) |> map(|d| max(d, 1))
let response = 10 + 5 * log2(dose) + rnorm(80, 0, 3)

# 1. Fit lm() with dose, dose + dose², dose + dose² + dose³
# 2. Compare R² and adj_r_squared for each
# 3. Which degree gives the best balance of fit and simplicity?
```

### Exercise 4: Predicted vs. Actual

Build a multiple regression model for gene expression prediction. Create a predicted vs. actual scatter plot. Assess where the model succeeds and fails.

```bio
set_seed(42)
let n = 200

# Simulate: predict gene expression from 4 TF binding signals
# Build model using lm(), generate predicted vs actual plot
# Calculate mean absolute error
# Identify the 5 worst predictions — what makes them outliers?
```

## Key Takeaways

- Multiple regression estimates the effect of each predictor **controlling for all others** — fundamentally different from separate simple regressions
- **Multicollinearity** (VIF > 10) inflates standard errors and makes coefficients uninterpretable — detect it before interpreting
- **Model selection** balances fit and complexity: AIC favors prediction, BIC favors parsimony, adjusted R² penalizes added terms
- **Stepwise regression** is useful for exploration but should be validated on held-out data
- **Lasso** performs automatic variable selection (zeros out irrelevant predictors); **Ridge** shrinks all coefficients but keeps them; **Elastic net** combines both
- **Polynomial regression** captures non-linear relationships within the regression framework but risks overfitting
- Always check residuals, predicted vs. actual plots, and VIF before trusting your model
- With genomics data (p >> n), regularization is not optional — it's essential

## What's Next

What if your outcome isn't continuous but **binary** — responder vs. non-responder, alive vs. dead, mutant vs. wild-type? Day 16 introduces **logistic regression**, where we predict probabilities of categorical outcomes using ROC curves, odds ratios, and the powerful GLM framework.
