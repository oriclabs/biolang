#!/usr/bin/env Rscript
# Day 14: Statistics for Bioinformatics — R equivalent

# Step 1: Descriptive statistics
expression <- c(5.2, 8.1, 3.4, 6.7, 4.1, 9.3, 7.5, 2.8)
cat(sprintf("Mean:     %.2f\n", mean(expression)))
cat(sprintf("Median:   %.2f\n", median(expression)))
cat(sprintf("Stdev:    %.2f\n", sd(expression)))
cat(sprintf("Variance: %.2f\n", var(expression)))
cat(sprintf("Min:      %.1f\n", min(expression)))
cat(sprintf("Max:      %.1f\n", max(expression)))
cat(sprintf("Range:    %.1f\n", max(expression) - min(expression)))
cat(sprintf("Q25:      %.2f\n", quantile(expression, 0.25)))
cat(sprintf("Q75:      %.2f\n", quantile(expression, 0.75)))
cat("\n")

data <- read.csv("data/experiment.csv")
print(summary(data[, -1]))
cat("\n")

# Step 2: t-tests
normal <- c(5.2, 4.8, 5.1, 4.9, 5.3)
tumor <- c(8.1, 7.9, 8.5, 7.6, 8.3)
result <- t.test(normal, tumor)
cat(sprintf("Two-sample t-test: t=%.3f, p=%g\n", result$statistic, result$p.value))

before <- c(10.2, 8.5, 12.1, 9.8, 11.3)
after <- c(7.1, 6.2, 8.5, 7.0, 8.8)
result <- t.test(before, after, paired = TRUE)
cat(sprintf("Paired t-test: p=%.4f\n", result$p.value))

observed <- c(2.1, 1.9, 2.3, 2.0, 2.2)
result <- t.test(observed, mu = 2.0)
cat(sprintf("One-sample t-test: p=%.4f\n", result$p.value))
cat("\n")

# Step 3: Wilcoxon
control <- c(1.2, 3.5, 2.1, 4.8, 1.5)
treated <- c(5.2, 8.1, 6.3, 9.5, 7.2)
result <- wilcox.test(control, treated)
cat(sprintf("Wilcoxon p=%.4f\n", result$p.value))
cat("\n")

# Step 4: ANOVA
ctrl <- c(5.0, 4.8, 5.2, 4.9)
low_dose <- c(6.5, 7.1, 6.8, 6.3)
high_dose <- c(9.2, 8.8, 9.5, 9.0)
groups <- factor(rep(c("ctrl", "low", "high"), each = 4))
values <- c(ctrl, low_dose, high_dose)
result <- summary(aov(values ~ groups))
cat(sprintf("ANOVA: F=%.2f, p=%g\n", result[[1]]$`F value`[1], result[[1]]$`Pr(>F)`[1]))
cat("\n")

# Step 5: Correlation
gene_a <- c(2.1, 3.5, 4.2, 5.8, 6.1, 7.3)
gene_b <- c(1.8, 3.2, 3.9, 5.5, 6.4, 7.0)
cat(sprintf("Pearson r: %.3f\n", cor(gene_a, gene_b)))
cat(sprintf("Spearman rho: %.3f\n", cor(gene_a, gene_b, method = "spearman")))
cat(sprintf("Kendall tau: %.3f\n", cor(gene_a, gene_b, method = "kendall")))
cat("\n")

# Step 6: Linear regression
x <- c(1.0, 2.0, 3.0, 4.0, 5.0, 6.0)
y <- c(2.1, 3.9, 6.2, 7.8, 10.1, 12.3)
model <- lm(y ~ x)
s <- summary(model)
cat(sprintf("Slope: %.3f\n", coef(model)[2]))
cat(sprintf("Intercept: %.3f\n", coef(model)[1]))
cat(sprintf("R-squared: %.3f\n", s$r.squared))
cat(sprintf("p-value: %g\n", s$coefficients[2, 4]))
cat("\n")

# Step 7: Multiple testing
raw_pvals <- c(0.001, 0.005, 0.01, 0.03, 0.04, 0.049, 0.06, 0.1, 0.5, 0.9)
bh <- p.adjust(raw_pvals, method = "BH")
bonf <- p.adjust(raw_pvals, method = "bonferroni")
cat("Raw       | BH        | Bonferroni\n")
for (i in seq_along(raw_pvals)) {
  cat(sprintf("%.3f    | %.4f   | %.4f\n", raw_pvals[i], bh[i], bonf[i]))
}
cat("\n")

# Step 8: Categorical tests
m <- matrix(c(30, 15, 10, 25), nrow = 2)
result <- chisq.test(m, correct = FALSE)
cat(sprintf("Chi-square: stat=%.2f, p=%.4f\n", result$statistic, result$p.value))
m2 <- matrix(c(8, 1, 2, 9), nrow = 2)
result <- fisher.test(m2)
cat(sprintf("Fisher's exact: p=%.4f\n", result$p.value))
cat("\n")

# Step 9: Full experiment analysis
control_cols <- c("control_1", "control_2", "control_3")
treated_cols <- c("treated_1", "treated_2", "treated_3")

genes <- data$gene
pvals <- numeric(nrow(data))
log2fcs <- numeric(nrow(data))

for (i in seq_len(nrow(data))) {
  ctrl_vals <- as.numeric(data[i, control_cols])
  trt_vals <- as.numeric(data[i, treated_cols])
  ctrl_m <- mean(ctrl_vals)
  trt_m <- mean(trt_vals)
  log2fcs[i] <- log2(trt_m / ctrl_m)
  pvals[i] <- t.test(ctrl_vals, trt_vals)$p.value
}

adj_pvals <- p.adjust(pvals, method = "BH")
result_df <- data.frame(
  gene = genes,
  log2fc = round(log2fcs, 2),
  pvalue = round(pvals, 4),
  adj_pvalue = round(adj_pvals, 4)
)
print(result_df)

sig <- result_df[result_df$adj_pvalue < 0.05, ]
cat(sprintf("\nSignificant: %d (%d up, %d down)\n",
            nrow(sig), sum(sig$log2fc > 0), sum(sig$log2fc < 0)))
