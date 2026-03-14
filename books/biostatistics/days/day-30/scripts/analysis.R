# Day 30: Capstone — GWAS Analysis

gwas <- read.csv("gwas_summary.csv")
cat(sprintf("Total SNPs: %s\n", format(nrow(gwas), big.mark = ",")))

# 1. Genome-wide significance
gws <- gwas[gwas$pvalue < 5e-8, ]
cat(sprintf("\n=== Genome-Wide Significant (p < 5e-8) ===\n"))
cat(sprintf("N hits: %d\n", nrow(gws)))
cat(sprintf("True positives:  %d\n", sum(gws$true_assoc)))
cat(sprintf("False positives: %d\n", sum(gws$true_assoc == 0)))

# 2. Genomic inflation
chisq_vals <- qnorm(1 - gwas$pvalue / 2)^2
lambda_gc <- median(chisq_vals, na.rm = TRUE) / qchisq(0.5, 1)
cat(sprintf("\nGenomic inflation (lambda): %.3f\n", lambda_gc))

# 3. FDR
gwas$padj <- p.adjust(gwas$pvalue, method = "BH")
cat(sprintf("FDR < 0.05: %d SNPs\n", sum(gwas$padj < 0.05)))

# 4. Top hits
cat("\n=== Top 10 Hits ===\n")
top <- head(gwas[order(gwas$pvalue), ], 10)
for (i in 1:nrow(top)) {
  row <- top[i, ]
  cat(sprintf("  %s chr%d:%d beta=%.4f p=%.2e %s\n",
              row$snp_id, row$chr, row$pos, row$beta, row$pvalue,
              ifelse(row$true_assoc, "*", "")))
}

# 5. Power
true_snps <- gwas[gwas$true_assoc == 1, ]
detected <- sum(true_snps$pvalue < 5e-8)
cat(sprintf("\n=== Power ===\n"))
cat(sprintf("True associations: %d\n", nrow(true_snps)))
cat(sprintf("Detected: %d (%.1f%%)\n", detected, detected/nrow(true_snps)*100))

# 6. Manhattan plot
plot(-log10(gwas$pvalue) ~ gwas$chr, pch = 16, cex = 0.3,
     col = ifelse(gwas$chr %% 2 == 0, "steelblue", "gray40"),
     xlab = "Chromosome", ylab = "-log10(p-value)",
     main = "Manhattan Plot")
abline(h = -log10(5e-8), col = "red", lty = 2)

# 7. QQ plot
expected <- -log10(ppoints(nrow(gwas)))
observed <- sort(-log10(gwas$pvalue))
plot(expected, observed, pch = 16, cex = 0.3,
     xlab = "Expected -log10(p)", ylab = "Observed -log10(p)",
     main = sprintf("QQ Plot (lambda=%.3f)", lambda_gc))
abline(0, 1, col = "red")
