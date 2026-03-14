# Day 16: Logistic Regression

data <- read.csv("immunotherapy.csv")

# Logistic regression
fit <- glm(response ~ tmb + pdl1_score + msi_status, data = data, family = binomial)
cat("=== Logistic Regression ===\n")
print(summary(fit))

# Odds ratios
cat("\nOdds Ratios:\n")
or <- exp(coef(fit))
ci <- exp(confint(fit))
for (i in seq_along(or)) {
  cat(sprintf("  %s: OR=%.3f (%.3f, %.3f)\n",
              names(or)[i], or[i], ci[i,1], ci[i,2]))
}

# ROC and AUC
probs <- predict(fit, type = "response")
if (requireNamespace("pROC", quietly = TRUE)) {
  roc_obj <- pROC::roc(data$response, probs)
  cat(sprintf("\nAUC: %.3f\n", pROC::auc(roc_obj)))
} else {
  # Manual AUC approximation
  cat("\nInstall pROC for AUC calculation\n")
}
