#!/usr/bin/env Rscript
# Day 10: Table Analysis Pipeline (R/dplyr equivalent)
#
# Run init.bl first to generate data files, then:
#   Rscript r/install.R
#   Rscript scripts/analysis.R

library(dplyr)
library(tidyr)

# ── Step 1: Load data ───────────────────────────────────────────────

expr <- read.csv("data/expression.csv", stringsAsFactors = FALSE)
gene_info <- read.csv("data/gene_info.csv", stringsAsFactors = FALSE)

cat(sprintf("Expression data: %d genes x %d columns\n", nrow(expr), ncol(expr)))
cat(sprintf("Columns: %s\n", paste(colnames(expr), collapse = ", ")))
cat(sprintf("Gene info: %d annotations\n\n", nrow(gene_info)))

# ── Step 2: Explore ─────────────────────────────────────────────────

cat("=== First 5 rows ===\n")
print(head(expr, 5))
cat("\n")

cat("=== Summary statistics ===\n")
print(summary(expr))
cat("\n")

# ── Step 3: Add derived columns ─────────────────────────────────────

analyzed <- expr %>%
  mutate(
    significant = padj < 0.05,
    direction = ifelse(log2fc > 0, "up", "down"),
    neg_log_p = -log10(pval)
  )

cat("=== With derived columns (first 5) ===\n")
print(head(analyzed, 5))
cat("\n")

# ── Step 4: Filter significant genes ────────────────────────────────

sig_genes <- analyzed %>% filter(significant)
cat(sprintf("Significant genes: %d of %d\n", nrow(sig_genes), nrow(analyzed)))

sig_up <- analyzed %>% filter(significant, direction == "up")
sig_down <- analyzed %>% filter(significant, direction == "down")
cat(sprintf("  Upregulated: %d\n", nrow(sig_up)))
cat(sprintf("  Downregulated: %d\n\n", nrow(sig_down)))

# ── Step 5: Count by category ──────────────────────────────────────

cat("=== Genes per chromosome ===\n")
chr_counts <- analyzed %>% count(chr, name = "count")
print(chr_counts)
cat("\n")

cat("=== Genes by biotype ===\n")
biotype_counts <- analyzed %>% count(biotype, name = "count")
print(biotype_counts)
cat("\n")

# ── Step 6: Group and summarize ─────────────────────────────────────

cat("=== Significant genes by direction ===\n")
direction_summary <- sig_genes %>%
  group_by(direction) %>%
  summarise(
    count = n(),
    mean_fc = mean(log2fc),
    min_padj = min(padj),
    .groups = "drop"
  )
print(direction_summary)
cat("\n")

# ── Step 7: Join with annotations ───────────────────────────────────

annotated <- analyzed %>% left_join(gene_info, by = "gene")
cat(sprintf("Annotated table: %d rows x %d columns\n", nrow(annotated), ncol(annotated)))

missing <- analyzed %>% anti_join(gene_info, by = "gene")
cat(sprintf("Genes without annotations: %d\n\n", nrow(missing)))

# ── Step 8: Top 10 most significant ─────────────────────────────────

cat("=== Top 10 most significant genes ===\n")
top10 <- annotated %>%
  filter(significant) %>%
  arrange(padj) %>%
  head(10) %>%
  select(gene, log2fc, padj, direction, pathway)
print(top10)
cat("\n")

# ── Step 9: Pathway summary ─────────────────────────────────────────

cat("=== Pathway summary (significant genes only) ===\n")
pathway_summary <- annotated %>%
  filter(significant) %>%
  group_by(pathway) %>%
  summarise(
    n_genes = n(),
    mean_fc = mean(log2fc),
    .groups = "drop"
  )
print(pathway_summary)
cat("\n")

# ── Step 10: Pivot demonstration ────────────────────────────────────

cat("=== Pivot: long to wide ===\n")
long_data <- tibble(
  gene = c("BRCA1", "BRCA1", "TP53", "TP53", "EGFR", "EGFR"),
  sample = c("Control", "Treated", "Control", "Treated", "Control", "Treated"),
  expression = c(5.2, 8.1, 3.4, 7.6, 2.1, 9.3)
)

cat("Long format:\n")
print(long_data)

wide_data <- long_data %>%
  pivot_wider(names_from = sample, values_from = expression)
cat("Wide format:\n")
print(wide_data)

back_to_long <- wide_data %>%
  pivot_longer(cols = c(Control, Treated), names_to = "sample", values_to = "expression")
cat("Back to long:\n")
print(back_to_long)
cat("\n")

# ── Step 11: Window functions ───────────────────────────────────────

cat("=== Ranked by significance ===\n")
ranked <- sig_genes %>%
  arrange(padj) %>%
  mutate(row_number = row_number()) %>%
  select(gene, log2fc, padj, row_number) %>%
  head(5)
print(ranked)
cat("\n")

# ── Step 12: Export results ─────────────────────────────────────────

dir.create("results", showWarnings = FALSE)
write.csv(annotated, "results/annotated_results.csv", row.names = FALSE)
cat("Saved: results/annotated_results.csv\n")
cat("Done!\n")
