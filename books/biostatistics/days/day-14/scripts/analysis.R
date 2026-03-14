# Day 14: Linear Regression

data <- read.csv("cell_lines.csv")

# Simple linear regression
fit <- lm(ic50_uM ~ expression, data = data)
cat("=== Linear Regression: Expression -> IC50 ===\n")
print(summary(fit))

# Diagnostics
cat("\nResidual diagnostics:\n")
cat(sprintf("Shapiro-Wilk p: %.4f\n", shapiro.test(residuals(fit))$p.value))

# Prediction
new_data <- data.frame(expression = 10.0)
pred <- predict(fit, new_data, interval = "prediction")
cat(sprintf("\nPredicted IC50 at expression=10: %.2f (%.2f, %.2f)\n",
            pred[1], pred[2], pred[3]))

# Plot
plot(data$expression, data$ic50_uM, xlab = "Expression", ylab = "IC50 (uM)",
     main = "Expression vs IC50", pch = 16)
abline(fit, col = "red", lwd = 2)
