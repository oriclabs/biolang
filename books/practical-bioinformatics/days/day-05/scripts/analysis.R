#!/usr/bin/env Rscript
# Day 5: Data Structures for Biology — R equivalent
#
# Demonstrates vectors, lists, data.frames, sets, and interval overlaps
# using base R and dplyr.

library(dplyr, warn.conflicts = FALSE)

cat(strrep("=", 60), "\n")
cat("Day 5: Data Structures for Biology (R)\n")
cat(strrep("=", 60), "\n")

# ============================================================
# 1. VECTORS (LISTS): Ordered Collections
# ============================================================
cat("\n--- 1. Vectors: Ordered Collections ---\n")

expression <- c(2.1, 5.4, 3.2, 8.7, 1.1, 6.3)
cat(sprintf("Expression values: %s\n", paste(expression, collapse = ", ")))
cat(sprintf("Mean: %.2f\n", mean(expression)))
cat(sprintf("Median: %.2f\n", median(expression)))
cat(sprintf("Stdev: %.2f\n", sd(expression)))
cat(sprintf("Min: %.1f, Max: %.1f\n", min(expression), max(expression)))

sorted_expr <- sort(expression, decreasing = TRUE)
top3 <- head(sorted_expr, 3)
cat(sprintf("Top 3 values: %s\n", paste(top3, collapse = ", ")))

# Sample filtering
samples <- c("control_1", "control_2", "treated_1", "treated_2", "treated_3")
treated <- samples[grepl("treated", samples)]
cat(sprintf("Treated samples: %s\n", paste(treated, collapse = ", ")))
cat(sprintf("Total: %d, Treated: %d\n", length(samples), length(treated)))

# Matrix (nested structure equivalent)
data_matrix <- matrix(
  c(2.1, 3.4, 5.6,
    1.8, 4.2, 6.1,
    3.0, 2.9, 4.8),
  nrow = 3, byrow = TRUE
)
cat(sprintf("Sample 2, Gene 3: %.1f\n", data_matrix[2, 3]))

# ============================================================
# 2. LISTS (RECORDS): Structured Metadata
# ============================================================
cat("\n--- 2. Lists: Structured Metadata ---\n")

gene <- list(
  symbol = "BRCA1",
  name = "BRCA1 DNA repair associated",
  chromosome = "17",
  start = 43044295,
  end = 43125483,
  strand = "+",
  biotype = "protein_coding"
)

cat(sprintf("%s on chr%s\n", gene$symbol, gene$chromosome))
cat(sprintf("Length: %d bp\n", gene$end - gene$start))
cat(sprintf("Keys: %s\n", paste(names(gene), collapse = ", ")))
cat(sprintf("Has strand: %s\n", "strand" %in% names(gene)))
cat(sprintf("Has expression: %s\n", "expression" %in% names(gene)))

# Data frame from list of records
variants <- data.frame(
  chrom = c("chr17", "chr17", "chr7"),
  pos = c(43091434, 7674220, 55249071),
  ref_allele = c("A", "C", "C"),
  alt_allele = c("G", "T", "T"),
  gene = c("BRCA1", "TP53", "EGFR"),
  stringsAsFactors = FALSE
)

chr17_vars <- variants %>% filter(chrom == "chr17")
cat(sprintf("Chr17 variants: %d\n", nrow(chr17_vars)))

genes <- variants$gene
cat(sprintf("Affected genes: %s\n", paste(genes, collapse = ", ")))

# ============================================================
# 3. DATA FRAMES: The Workhorse
# ============================================================
cat("\n--- 3. Data Frames: Analysis Results ---\n")

results <- data.frame(
  gene = c("BRCA1", "TP53", "EGFR", "MYC", "KRAS"),
  log2fc = c(2.4, -1.1, 3.8, 1.9, -0.3),
  pval = c(0.001, 0.23, 0.000001, 0.04, 0.67),
  stringsAsFactors = FALSE
)

cat(sprintf("Rows: %d, Columns: %d\n", nrow(results), ncol(results)))
cat(sprintf("Columns: %s\n", paste(colnames(results), collapse = ", ")))

# Filter and sort
significant <- results %>%
  filter(pval < 0.05) %>%
  arrange(log2fc)
cat("Significant genes (sorted by log2fc):\n")
print(significant, row.names = FALSE)

# Select columns
gene_pvals <- results %>% select(gene, pval)
cat("Gene-pval pairs:\n")
print(head(gene_pvals, 3), row.names = FALSE)

# Mutate — add column
annotated <- results %>% mutate(significant = pval < 0.05)
cat("With significance flag:\n")
print(head(annotated, 3), row.names = FALSE)

# Group by direction
direction_counts <- results %>%
  mutate(direction = ifelse(log2fc > 0, "up", "down")) %>%
  group_by(direction) %>%
  summarize(count = n(), .groups = "drop")
cat("Counts by direction:\n")
print(as.data.frame(direction_counts), row.names = FALSE)

# ============================================================
# 4. SETS: Unique Membership and Comparisons
# ============================================================
cat("\n--- 4. Sets: Venn Diagram Logic ---\n")

experiment_a <- c("BRCA1", "TP53", "EGFR", "MYC", "KRAS")
experiment_b <- c("TP53", "EGFR", "PTEN", "RB1", "MYC")

shared <- intersect(experiment_a, experiment_b)
only_a <- setdiff(experiment_a, experiment_b)
only_b <- setdiff(experiment_b, experiment_a)
all_genes <- union(experiment_a, experiment_b)

cat(sprintf("Experiment A: %s\n", paste(experiment_a, collapse = ", ")))
cat(sprintf("Experiment B: %s\n", paste(experiment_b, collapse = ", ")))
cat(sprintf("Shared genes: %s\n", paste(shared, collapse = ", ")))
cat(sprintf("Only in A: %s\n", paste(only_a, collapse = ", ")))
cat(sprintf("Only in B: %s\n", paste(only_b, collapse = ", ")))
cat(sprintf("Total unique: %d\n", length(all_genes)))

# ============================================================
# 5. GENOMIC INTERVALS: Coordinates and Overlaps
# ============================================================
cat("\n--- 5. Genomic Intervals ---\n")

regions <- data.frame(
  chrom = c("chr17", "chr17", "chr17"),
  start = c(43125283, 43124017, 43125000),
  end = c(43125483, 43124115, 43125600),
  name = c("promoter", "exon1", "enhancer"),
  stringsAsFactors = FALSE
)

cat(sprintf("Promoter: chr17:%d-%d\n", regions$start[1], regions$end[1]))
cat(sprintf("Exon 1: chr17:%d-%d\n", regions$start[2], regions$end[2]))
cat(sprintf("Enhancer: chr17:%d-%d\n", regions$start[3], regions$end[3]))

# Query overlaps manually (GenomicRanges would be used in practice)
q_chrom <- "chr17"
q_start <- 43125300
q_end <- 43125400

hits <- regions %>%
  filter(chrom == q_chrom & start < q_end & end > q_start)
cat(sprintf("Regions overlapping %s:%d-%d: %d\n", q_chrom, q_start, q_end, nrow(hits)))
for (i in seq_len(nrow(hits))) {
  cat(sprintf("  %s: %s:%d-%d\n", hits$name[i], hits$chrom[i], hits$start[i], hits$end[i]))
}

# ============================================================
# 6. COMBINING STRUCTURES: Real Analysis Pattern
# ============================================================
cat("\n--- 6. Combining Structures ---\n")

samples_list <- list(
  list(id = "S1", condition = "control", genes = c("BRCA1", "TP53", "EGFR")),
  list(id = "S2", condition = "treated", genes = c("TP53", "MYC", "KRAS", "EGFR")),
  list(id = "S3", condition = "treated", genes = c("BRCA1", "TP53", "PTEN"))
)

# Core genes across all samples
all_gene_sets <- lapply(samples_list, function(s) s$genes)
common <- Reduce(intersect, all_gene_sets)
cat(sprintf("Core genes (in all samples): %s\n", paste(common, collapse = ", ")))

# Treatment-specific genes
treated_sets <- lapply(
  Filter(function(s) s$condition == "treated", samples_list),
  function(s) s$genes
)
control_sets <- lapply(
  Filter(function(s) s$condition == "control", samples_list),
  function(s) s$genes
)
treated_genes <- Reduce(union, treated_sets)
control_genes <- Reduce(union, control_sets)
treatment_specific <- setdiff(treated_genes, control_genes)
cat(sprintf("Treatment-specific genes: %s\n", paste(treatment_specific, collapse = ", ")))

# Summary
summary_df <- data.frame(
  category = c("Core (all samples)", "Treatment-specific", "Control genes", "Treated genes"),
  count = c(length(common), length(treatment_specific), length(control_genes), length(treated_genes)),
  stringsAsFactors = FALSE
)
cat("Summary:\n")
print(summary_df, row.names = FALSE)

cat("\n", strrep("=", 60), "\n", sep = "")
cat("Day 5 complete! Week 1 foundations finished.\n")
cat(strrep("=", 60), "\n")
