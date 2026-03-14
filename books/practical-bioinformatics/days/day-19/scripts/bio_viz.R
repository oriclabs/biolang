#!/usr/bin/env Rscript
# Day 19: Biological Data Visualization — R equivalent
# Requires: ggplot2, dplyr, survival, survminer, pROC, pheatmap, ggrepel
# See ../r/install.R for installation.

library(ggplot2)
library(dplyr)
library(survival)
library(survminer)
library(pROC)
library(pheatmap)

dir.create("figures", showWarnings = FALSE)

cat(strrep("=", 60), "\n")
cat("Day 19: Biological Data Visualization (R)\n")
cat(strrep("=", 60), "\n")

# ── Section 1: GWAS Visualization ────────────────────────────────────

cat("\n── GWAS Visualization ──\n\n")

gwas <- read.csv("data/gwas.csv", stringsAsFactors = FALSE)
cat(sprintf("Loaded GWAS data: %d variants\n", nrow(gwas)))

# Manhattan plot
gwas$neglog10p <- -log10(gwas$pvalue)
chrom_order <- c(paste0("chr", 1:22), "chrX")
gwas$chrom <- factor(gwas$chrom, levels = chrom_order)
gwas <- gwas[order(gwas$chrom, gwas$pos), ]

# Compute cumulative positions
offsets <- gwas %>%
  group_by(chrom) %>%
  summarize(max_pos = max(pos), .groups = "drop") %>%
  mutate(offset = cumsum(lag(max_pos + 1e6, default = 0)))

gwas <- gwas %>%
  left_join(offsets, by = "chrom") %>%
  mutate(genome_pos = pos + offset)

p <- ggplot(gwas, aes(x = genome_pos, y = neglog10p, color = chrom)) +
  geom_point(size = 1, alpha = 0.7) +
  geom_hline(yintercept = -log10(5e-8), color = "red", linetype = "dashed") +
  scale_color_manual(values = rep(c("#1f77b4", "#aec7e8"), 12), guide = "none") +
  labs(x = "Chromosome", y = "-log10(p-value)", title = "Genome-Wide Association Study") +
  theme_minimal()
ggsave("figures/manhattan.png", p, width = 14, height = 5, dpi = 150)
cat("Saved figures/manhattan.png\n")

# QQ plot
observed <- sort(-log10(gwas$pvalue))
expected <- -log10(seq(1 / length(observed), 1, length.out = length(observed)))

qq_df <- data.frame(expected = expected, observed = observed)
p <- ggplot(qq_df, aes(x = expected, y = observed)) +
  geom_point(size = 1, alpha = 0.7) +
  geom_abline(slope = 1, intercept = 0, color = "red", linetype = "dashed") +
  labs(x = "Expected -log10(p)", y = "Observed -log10(p)",
       title = "QQ Plot — Observed vs Expected") +
  theme_minimal()
ggsave("figures/qq_plot.png", p, width = 6, height = 6, dpi = 150)
cat("Saved figures/qq_plot.png\n")

# ── Section 2: Expression Visualization ──────────────────────────────

cat("\n── Expression Visualization ──\n\n")

# Violin plot
violin_df <- data.frame(
  group = rep(c("control", "low_dose", "high_dose"), each = 8),
  value = c(5.2, 4.8, 5.1, 4.9, 5.3, 5.0, 4.7, 5.4,
            6.5, 7.1, 6.8, 6.3, 7.0, 6.6, 6.9, 7.2,
            9.2, 8.8, 9.5, 9.0, 8.6, 9.3, 8.9, 9.1)
)
violin_df$group <- factor(violin_df$group, levels = c("control", "low_dose", "high_dose"))

p <- ggplot(violin_df, aes(x = group, y = value, fill = group)) +
  geom_violin() +
  geom_jitter(width = 0.1, size = 1) +
  labs(title = "Expression by Treatment Group") +
  theme_minimal() +
  theme(legend.position = "none")
ggsave("figures/violin.png", p, width = 8, height = 5, dpi = 150)
cat("Saved figures/violin.png\n")

# Density plot
density_values <- c(2.1, 3.5, 4.2, 5.8, 6.1, 7.3, 3.8, 5.5, 4.9, 6.7, 3.2, 5.1, 4.5, 6.0, 7.8)
p <- ggplot(data.frame(x = density_values), aes(x = x)) +
  geom_density(fill = "steelblue", alpha = 0.5) +
  labs(title = "Expression Density", x = "Expression", y = "Density") +
  theme_minimal()
ggsave("figures/density.png", p, width = 8, height = 5, dpi = 150)
cat("Saved figures/density.png\n")

# PCA plot
expr <- read.csv("data/expression_matrix.csv", row.names = 1)
pca_result <- prcomp(t(expr), scale. = TRUE)
pca_df <- data.frame(
  PC1 = pca_result$x[, 1],
  PC2 = pca_result$x[, 2],
  sample = colnames(expr),
  group = c(rep("control", 4), rep("treatment", 4))
)
var_explained <- summary(pca_result)$importance[2, ]

p <- ggplot(pca_df, aes(x = PC1, y = PC2, color = group, label = sample)) +
  geom_point(size = 3) +
  geom_text(vjust = -1, size = 3) +
  labs(
    x = sprintf("PC1 (%.1f%%)", var_explained[1] * 100),
    y = sprintf("PC2 (%.1f%%)", var_explained[2] * 100),
    title = "PCA — Sample Clustering"
  ) +
  theme_minimal()
ggsave("figures/pca.png", p, width = 8, height = 6, dpi = 150)
cat("Saved figures/pca.png\n")

# Clustered heatmap
png("figures/clustered_heatmap.png", width = 800, height = 500)
pheatmap(as.matrix(expr), main = "Hierarchical Clustering", scale = "row")
dev.off()
cat("Saved figures/clustered_heatmap.png\n")

# ── Section 3: Clinical Visualization ────────────────────────────────

cat("\n── Clinical Visualization ──\n\n")

# Kaplan-Meier
km_data <- data.frame(
  time = c(12, 24, 36, 8, 48, 15, 30, 20, 42, 6),
  event = c(1, 1, 0, 1, 0, 1, 0, 1, 0, 1)
)
fit <- survfit(Surv(time, event) ~ 1, data = km_data)
p <- ggsurvplot(fit, data = km_data, title = "Overall Survival",
                xlab = "Time (months)", ylab = "Survival probability")
ggsave("figures/kaplan_meier.png", p$plot, width = 8, height = 5, dpi = 150)
cat("Saved figures/kaplan_meier.png\n")

# ROC curve
pred_data <- data.frame(
  score = c(0.9, 0.8, 0.7, 0.6, 0.5, 0.4, 0.3, 0.2, 0.1),
  label = c(1, 1, 0, 1, 0, 0, 0, 1, 0)
)
roc_obj <- roc(pred_data$label, pred_data$score)
png("figures/roc_curve.png", width = 600, height = 600)
plot(roc_obj, main = sprintf("Classifier Performance (AUC = %.2f)", auc(roc_obj)))
dev.off()
cat("Saved figures/roc_curve.png\n")

cat("\n", strrep("=", 60), "\n")
cat("R visualization complete.\n")
cat(strrep("=", 60), "\n")
