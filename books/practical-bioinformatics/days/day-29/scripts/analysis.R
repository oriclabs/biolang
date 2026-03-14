library(dplyr)

counts <- read.delim("data/counts.tsv", sep = "\t", stringsAsFactors = FALSE)
samples <- read.delim("data/samples.tsv", sep = "\t", stringsAsFactors = FALSE)
gene_info <- read.delim("data/gene_info.tsv", sep = "\t", stringsAsFactors = FALSE)

sample_ids <- samples$sample_id
ctrl_ids <- samples$sample_id[samples$condition == "control"]
treat_ids <- samples$sample_id[samples$condition == "treated"]

min_total <- 10
row_totals <- rowSums(counts[, sample_ids])
filtered <- counts[row_totals >= min_total, ]

lib_sizes <- colSums(counts[, sample_ids])

cpm <- filtered
for (sid in sample_ids) {
  cpm[[sid]] <- round(filtered[[sid]] / lib_sizes[sid] * 1e6, 2)
}

de_results <- data.frame(
  gene = cpm$gene,
  stringsAsFactors = FALSE
)

ctrl_mean <- rowMeans(cpm[, ctrl_ids])
treat_mean <- rowMeans(cpm[, treat_ids])

pseudocount <- 0.01
log2fc <- log2((treat_mean + pseudocount) / (ctrl_mean + pseudocount))

pvalues <- sapply(seq_len(nrow(cpm)), function(i) {
  ctrl_vals <- as.numeric(cpm[i, ctrl_ids])
  treat_vals <- as.numeric(cpm[i, treat_ids])
  tryCatch(
    t.test(ctrl_vals, treat_vals)$p.value,
    error = function(e) 1.0
  )
})
pvalues[is.nan(pvalues)] <- 1.0

de_results$ctrl_mean <- round(ctrl_mean, 2)
de_results$treat_mean <- round(treat_mean, 2)
de_results$log2fc <- round(log2fc, 4)
de_results$pvalue <- pvalues
de_results$direction <- ifelse(log2fc > 0, "up", "down")

de_results <- de_results[order(de_results$pvalue), ]

m <- nrow(de_results)
padj_raw <- pmin(de_results$pvalue * m / seq_len(m), 1.0)

monotonic <- numeric(m)
running_min <- 1.0
for (i in m:1) {
  if (padj_raw[i] < running_min) {
    running_min <- padj_raw[i]
  }
  monotonic[i] <- running_min
}

de_results$padj <- round(monotonic, 6)

fc_threshold <- 1.0
fdr_threshold <- 0.05

significant <- de_results %>%
  filter(abs(log2fc) > fc_threshold, padj < fdr_threshold)

up_genes <- significant %>% filter(direction == "up")
down_genes <- significant %>% filter(direction == "down")

dir.create("data/output", showWarnings = FALSE, recursive = TRUE)

write.table(significant, "data/output/de_genes.tsv",
            sep = "\t", row.names = FALSE, quote = FALSE)

fc_values <- abs(significant$log2fc)
summary_lines <- c(
  "=== RNA-seq Differential Expression Summary ===",
  "",
  sprintf("Total genes in count matrix: %d", nrow(counts)),
  sprintf("Genes after low-count filter: %d", nrow(filtered)),
  sprintf("Significant DE genes (|log2FC| > %.1f, FDR < %.2f): %d",
          fc_threshold, fdr_threshold, nrow(significant)),
  sprintf("  Up-regulated: %d", nrow(up_genes)),
  sprintf("  Down-regulated: %d", nrow(down_genes)),
  "",
  sprintf("Mean |log2FC| of DE genes: %.2f", mean(fc_values)),
  sprintf("Median |log2FC| of DE genes: %.2f", median(fc_values)),
  sprintf("Max |log2FC|: %.2f", max(fc_values)),
  "",
  "Output files:",
  "  data/output/de_genes.tsv       - Significant DE gene table",
  "  data/output/summary.txt        - This summary"
)

writeLines(summary_lines, "data/output/summary.txt")
cat(paste(summary_lines, collapse = "\n"), "\n")
