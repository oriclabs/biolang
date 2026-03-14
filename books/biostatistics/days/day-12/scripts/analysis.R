# Day 12: Multiple Testing Correction

de <- read.csv("de_results.csv")

# Corrections
de$p_bonferroni <- p.adjust(de$pvalue, method = "bonferroni")
de$p_bh <- p.adjust(de$pvalue, method = "BH")

cat("=== Multiple Testing Correction ===\n")
cat(sprintf("Total genes: %d\n", nrow(de)))
cat(sprintf("True DE:     %d\n", sum(de$true_de)))
cat(sprintf("\nRaw p < 0.05:      %d\n", sum(de$pvalue < 0.05)))
cat(sprintf("Bonferroni < 0.05: %d\n", sum(de$p_bonferroni < 0.05)))
cat(sprintf("BH FDR < 0.05:     %d\n", sum(de$p_bh < 0.05)))

# Confusion matrix
tp <- sum(de$p_bh < 0.05 & de$true_de == 1)
fp <- sum(de$p_bh < 0.05 & de$true_de == 0)
fn <- sum(de$p_bh >= 0.05 & de$true_de == 1)
cat(sprintf("\nBH: TP=%d, FP=%d, FN=%d\n", tp, fp, fn))
cat(sprintf("Observed FDR: %.3f\n", fp / (tp + fp)))
cat(sprintf("Power:        %.3f\n", tp / (tp + fn)))
