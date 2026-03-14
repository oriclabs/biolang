#!/usr/bin/env Rscript
# Day 13: Gene Expression and RNA-seq — R equivalent
#
# Uses DESeq2 for differential expression and edgeR for normalization.
# Falls back to base R t-tests if Bioconductor packages are unavailable.
#
# Usage:
#   cd days/day-13
#   Rscript scripts/analysis.R

cat("=== Step 1: Loading Count Matrix ===\n")

counts <- read.csv("data/counts.csv", stringsAsFactors = FALSE)
cat(sprintf("Genes: %d\n", nrow(counts)))
cat(sprintf("Columns: %s\n", paste(colnames(counts), collapse = ", ")))
print(head(counts, 5))
cat("\n")

# ── Step 2: Library Sizes ─────────────────────────────────────────────────

cat("=== Step 2: Library Sizes ===\n")

samples <- c("normal_1", "normal_2", "normal_3", "tumor_1", "tumor_2", "tumor_3")
lib_sizes <- data.frame(
  sample = samples,
  total = colSums(counts[, samples])
)
print(lib_sizes)
cat("\n")

# ── Step 3: CPM Normalization ─────────────────────────────────────────────

cat("=== Step 3: CPM Normalization ===\n")

count_matrix <- as.matrix(counts[, samples])
rownames(count_matrix) <- counts$gene

# Manual CPM
cpm_manual <- sweep(count_matrix, 2, colSums(count_matrix), "/") * 1e6

# If edgeR is available, compare
if (requireNamespace("edgeR", quietly = TRUE)) {
  library(edgeR)
  cpm_edger <- cpm(count_matrix)
  cat("CPM (edgeR, first 5 genes):\n")
  print(round(cpm_edger[1:5, ], 1))
} else {
  cat("CPM (manual, first 5 genes):\n")
  print(round(cpm_manual[1:5, ], 1))
}
cat("\n")

# ── Step 4: TPM Normalization ─────────────────────────────────────────────

cat("=== Step 4: TPM Normalization ===\n")

gene_lengths <- read.csv("data/gene_lengths.csv", stringsAsFactors = FALSE)
# Match order
gene_order <- match(counts$gene, gene_lengths$gene)
lengths_kb <- gene_lengths$length[gene_order] / 1000

# RPK: reads per kilobase
rpk <- sweep(count_matrix, 1, lengths_kb, "/")
# TPM: normalize RPK to sum to 1M per sample
tpm_matrix <- sweep(rpk, 2, colSums(rpk), "/") * 1e6

cat("TPM normalized (first 5 genes):\n")
print(round(tpm_matrix[1:5, ], 1))
cat("\n")

# ── Step 5: Mean Expression per Condition ─────────────────────────────────

cat("=== Step 5: Mean Expression per Condition ===\n")

normals <- c("normal_1", "normal_2", "normal_3")
tumors <- c("tumor_1", "tumor_2", "tumor_3")

gene_means <- data.frame(
  gene = counts$gene,
  normal_mean = round(rowMeans(count_matrix[, normals]), 1),
  tumor_mean = round(rowMeans(count_matrix[, tumors]), 1)
)
print(gene_means)
cat("\n")

# ── Step 6: Fold Change ──────────────────────────────────────────────────

cat("=== Step 6: Fold Change ===\n")

normal_means <- rowMeans(count_matrix[, normals])
tumor_means <- rowMeans(count_matrix[, tumors])
log2fc <- log2(tumor_means / normal_means)

fc_table <- data.frame(
  gene = counts$gene,
  normal_mean = round(normal_means, 1),
  tumor_mean = round(tumor_means, 1),
  log2fc = round(log2fc, 2)
)
cat("Fold changes (first 10):\n")
print(head(fc_table, 10))
cat("\n")

# ── Step 7: Differential Expression ─────────────────────────────────────

cat("=== Step 7: Differential Expression ===\n")

# Try DESeq2 first, fall back to t-test
if (requireNamespace("DESeq2", quietly = TRUE)) {
  library(DESeq2)

  col_data <- data.frame(
    condition = factor(c(rep("normal", 3), rep("tumor", 3))),
    row.names = samples
  )

  dds <- DESeqDataSetFromMatrix(
    countData = count_matrix,
    colData = col_data,
    design = ~ condition
  )
  dds$condition <- relevel(dds$condition, ref = "normal")
  dds <- DESeq(dds, quiet = TRUE)
  res <- results(dds, contrast = c("condition", "tumor", "normal"))

  de <- data.frame(
    gene = rownames(res),
    log2fc = round(res$log2FoldChange, 2),
    pvalue = res$pvalue,
    padj = res$padj,
    mean_ctrl = round(normal_means, 1),
    mean_treat = round(tumor_means, 1)
  )
  de <- de[order(de$pvalue), ]
  cat("DE results (DESeq2):\n")
} else {
  # Fallback: per-gene t-test
  pvalues <- sapply(1:nrow(count_matrix), function(i) {
    t.test(count_matrix[i, tumors], count_matrix[i, normals])$p.value
  })
  padj <- p.adjust(pvalues, method = "BH")

  de <- data.frame(
    gene = counts$gene,
    log2fc = round(log2fc, 2),
    pvalue = pvalues,
    padj = padj,
    mean_ctrl = round(normal_means, 1),
    mean_treat = round(tumor_means, 1)
  )
  de <- de[order(de$pvalue), ]
  cat("DE results (t-test + BH correction):\n")
}

cat(sprintf("Total: %d genes\n", nrow(de)))
print(head(de, 5))
cat("\n")

# ── Step 8: Filter Significant Results ───────────────────────────────────

cat("=== Step 8: Significant DE Genes ===\n")

significant <- de[!is.na(de$padj) & de$padj < 0.05 & abs(de$log2fc) > 1.0, ]
significant <- significant[order(significant$padj), ]

cat(sprintf("Significant DE genes (|log2FC| > 1, padj < 0.05): %d\n", nrow(significant)))
print(significant)

up <- sum(significant$log2fc > 0)
down <- sum(significant$log2fc < 0)
cat(sprintf("Upregulated in tumor: %d\n", up))
cat(sprintf("Downregulated in tumor: %d\n", down))
cat("\n")

# ── Step 9: Multiple Testing Correction Demo ─────────────────────────────

cat("=== Step 9: Multiple Testing Correction ===\n")

raw_pvals <- c(0.001, 0.01, 0.03, 0.04, 0.049, 0.06, 0.1)
adj <- p.adjust(raw_pvals, method = "BH")
cat("Raw vs Adjusted p-values:\n")
for (i in seq_along(raw_pvals)) {
  cat(sprintf("  %s -> %s\n", raw_pvals[i], round(adj[i], 4)))
}
cat("\n")

# ── Step 10: Volcano Plot ───────────────────────────────────────────────

cat("=== Step 10: Volcano Plot ===\n")

if (!is.na(de$padj[1])) {
  png("results/volcano_plot.png", width = 800, height = 600)

  de_plot <- de[!is.na(de$padj), ]
  colors <- ifelse(de_plot$padj < 0.05 & de_plot$log2fc > 1, "red",
            ifelse(de_plot$padj < 0.05 & de_plot$log2fc < -1, "blue", "gray"))

  plot(de_plot$log2fc, -log10(de_plot$pvalue),
       col = colors, pch = 16, cex = 1.5,
       xlab = "log2 Fold Change",
       ylab = "-log10(p-value)",
       main = "Tumor vs Normal DE")
  abline(v = c(-1, 1), lty = 2, col = "darkgray")
  abline(h = -log10(0.05), lty = 2, col = "darkgray")

  # Label significant genes
  sig_plot <- de_plot[de_plot$padj < 0.05 & abs(de_plot$log2fc) > 1, ]
  text(sig_plot$log2fc, -log10(sig_plot$pvalue),
       labels = sig_plot$gene, pos = 3, cex = 0.7)

  dev.off()
  cat("Saved results/volcano_plot.png\n")
} else {
  cat("Skipping plot (no adjusted p-values)\n")
}
cat("\n")

# ── Step 11: Export Results ──────────────────────────────────────────────

cat("=== Step 11: Export Results ===\n")

write.csv(significant, "results/significant_genes.csv", row.names = FALSE)
cat(sprintf("Saved %d significant genes to results/significant_genes.csv\n", nrow(significant)))

write.csv(fc_table, "results/fold_changes.csv", row.names = FALSE)
cat(sprintf("Saved fold changes for %d genes to results/fold_changes.csv\n", nrow(fc_table)))

cat("\n=== Pipeline complete ===\n")
