# Day 2: Descriptive Statistics

quality <- read.csv("quality_scores.csv")
expression <- read.csv("expression.csv")

# Quality score descriptives
qs <- quality$quality_score
cat("=== FASTQ Quality Scores ===\n")
cat("Summary:\n")
print(summary(qs))
cat(sprintf("SD:   %.2f\n", sd(qs)))
cat(sprintf("IQR:  %.2f\n", IQR(qs)))
cat(sprintf("Skew: %.3f\n", moments::skewness(qs)))

# Expression descriptives
expr <- expression$expression
cat("\n=== Gene Expression ===\n")
print(summary(expr))
cat(sprintf("SD: %.2f\n", sd(expr)))

# Log-transformed
log_expr <- log2(expr + 1)
cat("\nLog2-transformed:\n")
print(summary(log_expr))
cat(sprintf("Log2 SD: %.2f\n", sd(log_expr)))
