# Day 15: Multiple Regression

data <- read.csv("biomarkers.csv")
features <- paste0("bm", 1:10)

# Full model
formula_full <- as.formula(paste("tumor_stage ~", paste(features, collapse = " + ")))
fit_full <- lm(formula_full, data = data)
cat("=== Full Model ===\n")
print(summary(fit_full))

# Stepwise selection
cat("\n=== Stepwise Selection ===\n")
fit_step <- step(fit_full, direction = "both", trace = 0)
print(summary(fit_step))

# Check VIF for collinearity
cat("\nVIF (selected model):\n")
if (requireNamespace("car", quietly = TRUE)) {
  print(car::vif(fit_step))
} else {
  # Manual VIF for bm1 and bm3
  r2_1 <- summary(lm(bm1 ~ bm3, data = data))$r.squared
  cat(sprintf("VIF(bm1 ~ bm3): %.2f\n", 1 / (1 - r2_1)))
}

# Cross-validated RMSE
set.seed(42)
folds <- sample(rep(1:5, length.out = nrow(data)))
cv_rmse <- sapply(1:5, function(k) {
  train <- data[folds != k, ]
  test <- data[folds == k, ]
  fit <- lm(formula(fit_step), data = train)
  sqrt(mean((predict(fit, test) - test$tumor_stage)^2))
})
cat(sprintf("\n5-fold CV RMSE: %.3f +/- %.3f\n", mean(cv_rmse), sd(cv_rmse)))
