# Day 4: Probability

hw <- read.csv("hardy_weinberg.csv")
diag <- read.csv("diagnostic_test.csv")

# Hardy-Weinberg equilibrium test
cat("=== Hardy-Weinberg ===\n")
geno_counts <- table(hw$genotype)
print(geno_counts)

p <- 0.3; q <- 1 - p; n <- nrow(hw)
obs <- c(sum(hw$genotype == "AA"),
         sum(hw$genotype %in% c("Aa", "aA")),
         sum(hw$genotype == "aa"))
exp <- c(p^2 * n, 2*p*q * n, q^2 * n)
chi2 <- sum((obs - exp)^2 / exp)
pval <- pchisq(chi2, df = 1, lower.tail = FALSE)
cat(sprintf("HWE chi-square: %.3f, p=%.4f\n", chi2, pval))

# Diagnostic test — Bayes' theorem
cat("\n=== Diagnostic Test ===\n")
ct <- table(disease = diag$disease, test = diag$test_positive)
print(ct)

prev <- 0.01; sens <- 0.99; spec <- 0.95
ppv_theory <- (sens * prev) / (sens * prev + (1 - spec) * (1 - prev))
cat(sprintf("Theoretical PPV: %.4f\n", ppv_theory))

tp <- sum(diag$disease == 1 & diag$test_positive == 1)
fp <- sum(diag$disease == 0 & diag$test_positive == 1)
cat(sprintf("Observed PPV:    %.4f\n", tp / (tp + fp)))
