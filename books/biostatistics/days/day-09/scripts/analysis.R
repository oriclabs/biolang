# Day 9: Non-Parametric Tests

micro <- read.csv("microbiome.csv")
sites <- read.csv("body_sites.csv")

# Mann-Whitney U (Wilcoxon rank-sum)
cat("=== Wilcoxon Rank-Sum (IBD vs Healthy) ===\n")
ibd <- micro$otu_count[micro$group == "IBD"]
healthy <- micro$otu_count[micro$group == "Healthy"]
wt <- wilcox.test(ibd, healthy)
cat(sprintf("IBD median:     %.0f\n", median(ibd)))
cat(sprintf("Healthy median: %.0f\n", median(healthy)))
cat(sprintf("W-statistic:    %.1f\n", wt$statistic))
cat(sprintf("P-value:        %.4f\n", wt$p.value))

# Kruskal-Wallis
cat("\n=== Kruskal-Wallis (Body Sites) ===\n")
kt <- kruskal.test(shannon_diversity ~ body_site, data = sites)
cat(sprintf("H-statistic: %.2f\n", kt$statistic))
cat(sprintf("P-value:     %.4e\n", kt$p.value))

# Post-hoc pairwise Wilcoxon
cat("\nPairwise Wilcoxon (Holm adjustment):\n")
pw <- pairwise.wilcox.test(sites$shannon_diversity, sites$body_site, p.adjust.method = "holm")
print(pw$p.value)
