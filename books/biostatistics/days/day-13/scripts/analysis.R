# Day 13: Correlation

genes <- read.csv("gene_correlation.csv")
anscombe <- read.csv("anscombe.csv")

# Pearson and Spearman
cat("=== Gene Correlations ===\n")
pairs <- list(c("brca1","bard1"), c("brca1","proliferation"), c("bard1","proliferation"))
for (pair in pairs) {
  ct <- cor.test(genes[[pair[1]]], genes[[pair[2]]])
  rho <- cor(genes[[pair[1]]], genes[[pair[2]]], method = "spearman")
  cat(sprintf("%s vs %s: r=%.3f (p=%.4f), rho=%.3f\n",
              pair[1], pair[2], ct$estimate, ct$p.value, rho))
}

# Partial correlation
cat("\nPartial r(BRCA1,BARD1 | proliferation):\n")
resid1 <- residuals(lm(brca1 ~ proliferation, data = genes))
resid2 <- residuals(lm(bard1 ~ proliferation, data = genes))
cat(sprintf("  r = %.3f\n", cor(resid1, resid2)))

# Anscombe's quartet
cat("\n=== Anscombe's Quartet ===\n")
cat(sprintf("Set 1: r=%.3f\n", cor(anscombe$x, anscombe$y1)))
cat(sprintf("Set 2: r=%.3f\n", cor(anscombe$x, anscombe$y2)))
cat(sprintf("Set 3: r=%.3f\n", cor(anscombe$x, anscombe$y3)))
cat(sprintf("Set 4: r=%.3f\n", cor(anscombe$x4, anscombe$y4)))
