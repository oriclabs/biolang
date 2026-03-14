# Day 26: Meta-Analysis

data <- read.csv("gwas_meta.csv")

# Manual inverse-variance fixed-effect
weights <- 1 / data$se^2
beta_fe <- sum(weights * data$beta) / sum(weights)
se_fe <- 1 / sqrt(sum(weights))
z_fe <- beta_fe / se_fe
p_fe <- 2 * pnorm(-abs(z_fe))

cat("=== Fixed-Effect Meta-Analysis ===\n")
cat(sprintf("Combined beta: %.5f (SE=%.5f)\n", beta_fe, se_fe))
cat(sprintf("Z=%.3f, p=%.4e\n", z_fe, p_fe))
cat(sprintf("95%% CI: (%.5f, %.5f)\n", beta_fe - 1.96*se_fe, beta_fe + 1.96*se_fe))

# Heterogeneity
Q <- sum(weights * (data$beta - beta_fe)^2)
df <- nrow(data) - 1
p_het <- pchisq(Q, df, lower.tail = FALSE)
I2 <- max(0, (Q - df) / Q * 100)
cat(sprintf("\nQ=%.2f, df=%d, p=%.4f, I2=%.1f%%\n", Q, df, p_het, I2))

# Using meta package if available
if (requireNamespace("meta", quietly = TRUE)) {
  cat("\n=== meta::metagen ===\n")
  m <- meta::metagen(TE = data$beta, seTE = data$se, studlab = data$study)
  print(summary(m))
}
