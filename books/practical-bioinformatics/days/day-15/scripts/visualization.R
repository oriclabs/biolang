#!/usr/bin/env Rscript
# Day 15: Publication-Quality Visualization — R equivalent

library(ggplot2)

dir.create("figures", showWarnings = FALSE)

# ── Load data ──────────────────────────────────────────────────────

de <- read.csv("data/de_results.csv")
cat(sprintf("Loaded DE results: %d genes\n", nrow(de)))

gwas <- read.csv("data/gwas_results.csv")
cat(sprintf("Loaded GWAS results: %d variants\n", nrow(gwas)))

# ── Scatter plot ───────────────────────────────────────────────────

scatter_df <- data.frame(x = c(1, 2, 3, 4, 5), y = c(2.1, 3.9, 6.2, 7.8, 10.1))
p <- ggplot(scatter_df, aes(x = x, y = y)) +
  geom_point(color = "steelblue", size = 3) +
  labs(title = "Gene Expression Correlation") +
  theme_minimal()
ggsave("figures/scatter.svg", p, width = 6, height = 4)
cat("Saved figures/scatter.svg\n")

# ── Histogram ──────────────────────────────────────────────────────

values <- c(2.1, 3.5, 4.2, 5.8, 6.1, 7.3, 3.8, 5.5, 4.9, 6.7, 3.2, 5.1)
p <- ggplot(data.frame(x = values), aes(x = x)) +
  geom_histogram(bins = 6, fill = "steelblue", color = "white") +
  labs(title = "Expression Distribution", x = "Expression", y = "Count") +
  theme_minimal()
ggsave("figures/histogram.svg", p, width = 6, height = 4)
cat("Saved figures/histogram.svg\n")

# ── Bar chart ──────────────────────────────────────────────────────

var_df <- data.frame(
  category = c("SNP", "Insertion", "Deletion", "MNV"),
  count = c(3500, 450, 520, 30)
)
p <- ggplot(var_df, aes(x = reorder(category, -count), y = count)) +
  geom_col(fill = "steelblue") +
  labs(title = "Variant Types", x = "", y = "Count") +
  theme_minimal()
ggsave("figures/bar_chart.svg", p, width = 6, height = 4)
cat("Saved figures/bar_chart.svg\n")

# ── Boxplot ────────────────────────────────────────────────────────

box_df <- data.frame(
  group = rep(c("control", "treated", "resistant"), each = 6),
  expression = c(5.2, 4.8, 5.1, 4.9, 5.3, 5.0,
                 8.1, 7.9, 8.5, 7.6, 8.3, 8.0,
                 5.5, 5.3, 5.8, 5.1, 5.6, 5.4)
)
p <- ggplot(box_df, aes(x = group, y = expression, fill = group)) +
  geom_boxplot() +
  labs(title = "Expression by Group", y = "Expression") +
  theme_minimal() +
  theme(legend.position = "none")
ggsave("figures/boxplot.svg", p, width = 6, height = 4)
cat("Saved figures/boxplot.svg\n")

# ── Volcano plot ───────────────────────────────────────────────────

de$neglog10p <- -log10(de$padj)
de$sig <- ifelse(de$padj < 0.05 & de$log2fc > 1, "Up",
           ifelse(de$padj < 0.05 & de$log2fc < -1, "Down", "NS"))

p <- ggplot(de, aes(x = log2fc, y = neglog10p, color = sig)) +
  geom_point(alpha = 0.7) +
  scale_color_manual(values = c("Up" = "red", "Down" = "blue", "NS" = "grey60")) +
  geom_hline(yintercept = -log10(0.05), linetype = "dashed", color = "grey50") +
  geom_vline(xintercept = c(-1, 1), linetype = "dashed", color = "grey50") +
  labs(title = "A) Differential Expression",
       x = "log2 Fold Change", y = "-log10(adjusted p-value)") +
  theme_minimal()
ggsave("figures/fig1_volcano.svg", p, width = 8, height = 6)
cat("Saved figures/fig1_volcano.svg\n")

# ── MA plot ────────────────────────────────────────────────────────

de$log10mean <- log10(de$baseMean)
p <- ggplot(de, aes(x = log10mean, y = log2fc, color = sig)) +
  geom_point(alpha = 0.7) +
  scale_color_manual(values = c("Up" = "red", "Down" = "blue", "NS" = "grey60")) +
  geom_hline(yintercept = 0, linetype = "dashed", color = "grey50") +
  labs(title = "B) MA Plot",
       x = "log10(baseMean)", y = "log2 Fold Change") +
  theme_minimal()
ggsave("figures/fig2_ma.svg", p, width = 8, height = 6)
cat("Saved figures/fig2_ma.svg\n")

# ── Summary ────────────────────────────────────────────────────────

up <- sum(de$padj < 0.05 & de$log2fc > 1)
down <- sum(de$padj < 0.05 & de$log2fc < -1)
ns <- nrow(de) - up - down

cat(sprintf("\nUpregulated (padj<0.05, log2FC>1):   %d\n", up))
cat(sprintf("Downregulated (padj<0.05, log2FC<-1): %d\n", down))
cat(sprintf("Not significant:                      %d\n", ns))
cat("\nDone.\n")
