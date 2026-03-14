# Day 11: Categorical Data

snp <- read.csv("snp_case_control.csv")
hwe <- read.csv("hwe_test.csv")
rare <- read.csv("rare_variant.csv")

# Chi-square: SNP case-control
cat("=== Chi-Square (SNP Case-Control) ===\n")
ct <- table(snp$status, snp$genotype)
print(ct)
print(chisq.test(ct))

# Hardy-Weinberg test
cat("\n=== HWE Test ===\n")
counts <- table(factor(hwe$genotype_count, levels = 0:2))
n <- sum(counts)
p_hat <- (2 * counts[3] + counts[2]) / (2 * n)
q_hat <- 1 - p_hat
exp_hwe <- c(q_hat^2, 2*p_hat*q_hat, p_hat^2) * n
chi2 <- sum((counts - exp_hwe)^2 / exp_hwe)
cat(sprintf("p_hat=%.3f, Chi2=%.3f, p=%.4f\n", p_hat, chi2, pchisq(chi2, 1, lower.tail=FALSE)))

# Fisher's exact: rare variant
cat("\n=== Fisher's Exact (Rare Variant) ===\n")
ct_rare <- table(rare$status, rare$carrier)
print(ct_rare)
print(fisher.test(ct_rare))
