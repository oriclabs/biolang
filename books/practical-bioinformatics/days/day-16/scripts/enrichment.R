#!/usr/bin/env Rscript
# Day 16: Pathway and Enrichment Analysis — R equivalent
#
# Requires: install.packages(c("clusterProfiler", "org.Hs.eg.db", "enrichplot"))
# Data: run init.bl first to generate data/ files.

library(clusterProfiler)
library(org.Hs.eg.db)

# ── Step 1: Load Gene Sets (GMT) ───────────────────────────────────────

cat("=== Step 1: Load Gene Sets ===\n\n")

gmt <- read.gmt("data/hallmark.gmt")
gene_sets <- split(gmt$gene, gmt$term)
cat(sprintf("Gene sets loaded: %d\n", length(gene_sets)))
cat(sprintf("DNA repair genes: %d\n", length(gene_sets[["HALLMARK_DNA_REPAIR"]])))

# ── Step 2: Over-Representation Analysis (ORA) ────────────────────────

cat("\n=== Step 2: Over-Representation Analysis ===\n\n")

de <- read.csv("data/de_results.csv")
cat(sprintf("Total genes in DE results: %d\n", nrow(de)))

sig <- de[de$padj < 0.05 & abs(de$log2fc) > 1.0, ]
sig_genes <- sig$gene
cat(sprintf("Significant DE genes: %d\n", length(sig_genes)))

# enricher() for custom gene sets
ora_result <- enricher(
  gene = sig_genes,
  TERM2GENE = gmt,
  pvalueCutoff = 1.0,
  qvalueCutoff = 1.0,
  universe = de$gene
)

ora_df <- as.data.frame(ora_result)
sig_ora <- ora_df[ora_df$p.adjust < 0.05, ]
cat(sprintf("Significant terms (FDR < 0.05): %d\n", nrow(sig_ora)))
print(sig_ora[, c("ID", "Count", "pvalue", "p.adjust", "geneID")])

# ── Step 3: GSEA ──────────────────────────────────────────────────────

cat("\n=== Step 3: Gene Set Enrichment Analysis ===\n\n")

ranked <- read.csv("data/ranked_genes.csv")
cat(sprintf("Ranked genes loaded: %d\n", nrow(ranked)))

# Create named vector for GSEA
gene_list <- ranked$score
names(gene_list) <- ranked$gene
gene_list <- sort(gene_list, decreasing = TRUE)

gsea_result <- GSEA(
  geneList = gene_list,
  TERM2GENE = gmt,
  pvalueCutoff = 1.0,
  seed = TRUE
)

gsea_df <- as.data.frame(gsea_result)
gsea_sig <- gsea_df[gsea_df$p.adjust < 0.25, ]
cat(sprintf("Significant terms (FDR < 0.25): %d\n", nrow(gsea_sig)))
print(gsea_sig[, c("ID", "enrichmentScore", "NES", "pvalue", "p.adjust")])

# ── Step 4: Export ─────────────────────────────────────────────────────

cat("\n=== Step 4: Export ===\n\n")

write.csv(sig_ora, "results/ora_results_r.csv", row.names = FALSE)
cat("Saved results/ora_results_r.csv\n")

cat("\n=== Analysis complete ===\n")
