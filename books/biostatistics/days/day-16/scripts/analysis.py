# Day 16: Logistic Regression
import pandas as pd
import numpy as np
from sklearn.linear_model import LogisticRegression
from sklearn.metrics import roc_auc_score, classification_report
from sklearn.model_selection import cross_val_score
from sklearn.preprocessing import StandardScaler

data = pd.read_csv("immunotherapy.csv")
features = ["tmb", "pdl1_score", "msi_status"]
X = data[features].values
y = data["response"].values

scaler = StandardScaler()
X_scaled = scaler.fit_transform(X)

# Logistic regression
lr = LogisticRegression(random_state=42)
lr.fit(X_scaled, y)

y_pred = lr.predict(X_scaled)
y_prob = lr.predict_proba(X_scaled)[:, 1]

print("=== Logistic Regression (Immunotherapy Response) ===")
print(f"Coefficients:")
for name, coef in zip(features, lr.coef_[0]):
    print(f"  {name:12s}: {coef:.3f}")
print(f"Intercept: {lr.intercept_[0]:.3f}")

print(f"\nAUC-ROC: {roc_auc_score(y, y_prob):.3f}")
print(classification_report(y, y_pred))

# Cross-validation
cv_auc = cross_val_score(lr, X_scaled, y, cv=5, scoring="roc_auc")
print(f"5-fold CV AUC: {cv_auc.mean():.3f} +/- {cv_auc.std():.3f}")
