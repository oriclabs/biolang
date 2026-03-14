# Day 15: Multiple Regression
import pandas as pd
import numpy as np
from sklearn.linear_model import LinearRegression, Ridge, Lasso
from sklearn.preprocessing import StandardScaler
from sklearn.model_selection import cross_val_score

data = pd.read_csv("biomarkers.csv")
features = [f"bm{i}" for i in range(1, 11)]
X = data[features].values
y = data["tumor_stage"].values

# Standardize
scaler = StandardScaler()
X_scaled = scaler.fit_transform(X)

# OLS
ols = LinearRegression().fit(X_scaled, y)
print("=== OLS Multiple Regression ===")
print(f"R-squared: {ols.score(X_scaled, y):.3f}")
for name, coef in zip(features, ols.coef_):
    print(f"  {name}: {coef:7.3f}")

# Ridge
ridge = Ridge(alpha=1.0).fit(X_scaled, y)
print(f"\n=== Ridge (alpha=1.0) R2={ridge.score(X_scaled, y):.3f} ===")

# Lasso (feature selection)
lasso = Lasso(alpha=0.1).fit(X_scaled, y)
print(f"\n=== Lasso (alpha=0.1) R2={lasso.score(X_scaled, y):.3f} ===")
print("Selected features:")
for name, coef in zip(features, lasso.coef_):
    if abs(coef) > 0.01:
        print(f"  {name}: {coef:.3f}")

# Cross-validation
cv_ols = cross_val_score(LinearRegression(), X_scaled, y, cv=5)
print(f"\n5-fold CV R2 (OLS): {cv_ols.mean():.3f} +/- {cv_ols.std():.3f}")
