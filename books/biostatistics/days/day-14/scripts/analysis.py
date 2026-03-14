# Day 14: Linear Regression
import pandas as pd
import numpy as np
from scipy import stats

data = pd.read_csv("cell_lines.csv")
x = data["expression"].values
y = data["ic50_uM"].values

# Simple linear regression
slope, intercept, r_value, p_value, std_err = stats.linregress(x, y)
print("=== Linear Regression: Expression -> IC50 ===")
print(f"Intercept: {intercept:.3f}")
print(f"Slope:     {slope:.3f}")
print(f"R-squared: {r_value**2:.3f}")
print(f"P-value:   {p_value:.4e}")
print(f"Std Error: {std_err:.3f}")

# Residual analysis
y_pred = intercept + slope * x
residuals = y - y_pred
print(f"\nResidual SD: {residuals.std():.3f}")

# Shapiro-Wilk on residuals
_, p_shapiro = stats.shapiro(residuals)
print(f"Shapiro-Wilk (residuals): p={p_shapiro:.4f}")

# Prediction for new expression value
x_new = 10.0
y_pred_new = intercept + slope * x_new
print(f"\nPredicted IC50 at expression={x_new}: {y_pred_new:.2f} uM")
