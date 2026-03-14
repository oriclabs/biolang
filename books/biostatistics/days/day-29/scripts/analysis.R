# Day 29: Capstone — Differential Expression Analysis

counts <- read.csv("counts.csv", row.names = 1)
meta <- read.csv("sample_metadata.csv")
gene_info <- read.csv("gene_info.csv")

ctrl <- meta$sample[meta$condition == "Control"]
trt <- meta$sample[meta$condition == "Treated"]

# Filter low-count genes
keep <- rowMeans(counts) >= 10
counts_filt <- counts[keep, ]
cat(sprintf("Genes after filtering: %d / %d\n", nrow(counts_filt), nrow(counts)))

# Log2 fold change + t-test
log_counts <- log2(counts_filt + 1)
results <- data.frame(
  gene = rownames(counts_filt),
  log2fc = rowMeans(log_counts[, trt]) - rowMeans(log_counts[, ctrl]),
  pvalue = apply(counts_filt, 1, function(row) {
    t.test(log2(row[trt] + 1), log2(row[ctrl] + 1))$p.value
  })
)
results$padj <- p.adjust(results$pvalue, method = "BH")

# Summary
sig <- results[results$padj < 0.05 & abs(results$log2fc) > 1, ]
cat(sprintf("\n=== DE Results ===\n"))
cat(sprintf("Significant (FDR<0.05, |LFC|>1): %d\n", nrow(sig)))
cat(sprintf("  Up:   %d\n", sum(sig$log2fc > 0)))
cat(sprintf("  Down: %d\n", sum(sig$log2fc < 0)))

# Compare with truth
merged <- merge(results, gene_info, by = "gene")
tp <- sum(merged$padj < 0.05 & merged$true_de == 1)
fp <- sum(merged$padj < 0.05 & merged$true_de == 0)
fn <- sum(merged$padj >= 0.05 & merged$true_de == 1)
cat(sprintf("\nTP=%d, FP=%d, FN=%d\n", tp, fp, fn))
cat(sprintf("FDR: %.3f, Sensitivity: %.3f\n", fp/(tp+fp), tp/(tp+fn)))

# Volcano plot
plot(results$log2fc, -log10(results$pvalue), pch = 16, cex = 0.3,
     xlab = "Log2 Fold Change", ylab = "-log10(p-value)",
     main = "Volcano Plot", col = ifelse(results$padj < 0.05, "red", "gray"))
abline(v = c(-1, 1), lty = 2)
