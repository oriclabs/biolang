# Day 8: t-Tests

expr <- read.csv("expression.csv")
paired <- read.csv("paired_treatment.csv")

# Independent t-tests per gene
cat("=== Independent Two-Sample t-Tests ===\n")
for (gene in unique(expr$gene)) {
  gdata <- expr[expr$gene == gene, ]
  tumor <- gdata$expression[gdata$group == "Tumor"]
  normal <- gdata$expression[gdata$group == "Normal"]
  tt <- t.test(tumor, normal)
  fc <- mean(tumor) / mean(normal)
  cat(sprintf("%8s: t=%.2f, p=%.4f, FC=%.2f\n", gene, tt$statistic, tt$p.value, fc))
}

# Paired t-test
cat("\n=== Paired t-Test ===\n")
tt_paired <- t.test(paired$before, paired$after, paired = TRUE)
cat(sprintf("Mean before: %.1f\n", mean(paired$before)))
cat(sprintf("Mean after:  %.1f\n", mean(paired$after)))
cat(sprintf("t-statistic: %.3f\n", tt_paired$statistic))
cat(sprintf("p-value:     %.4f\n", tt_paired$p.value))
cat(sprintf("95%% CI of diff: (%.2f, %.2f)\n",
            tt_paired$conf.int[1], tt_paired$conf.int[2]))
