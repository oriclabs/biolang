# Day 21: PCA

data <- read.csv("single_cell.csv")
gene_cols <- grep("^gene_", names(data), value = TRUE)
X <- as.matrix(data[, gene_cols])
labels <- data$true_cluster

# PCA
pca <- prcomp(X, scale. = TRUE)
var_exp <- summary(pca)$importance[2, ]

cat("=== PCA on Single-Cell Data ===\n")
cat(sprintf("Cells: %d, Genes: %d\n", nrow(X), ncol(X)))
for (i in 1:5) {
  cat(sprintf("  PC%d: %.1f%%\n", i, var_exp[i] * 100))
}

# Cluster separation
cat("\n=== Cluster Separation ===\n")
for (pc in 1:3) {
  p <- summary(aov(pca$x[, pc] ~ factor(labels)))[[1]][["Pr(>F)"]][1]
  cat(sprintf("  PC%d ~ cluster: p=%.4e\n", pc, p))
}

# Top loading genes
cat("\nTop 10 genes for PC1:\n")
loadings <- abs(pca$rotation[, 1])
top <- head(sort(loadings, decreasing = TRUE), 10)
for (i in seq_along(top)) {
  cat(sprintf("  %s: %.4f\n", names(top)[i], top[i]))
}

# Biplot
plot(pca$x[, 1], pca$x[, 2], col = labels, pch = 16, cex = 0.5,
     xlab = sprintf("PC1 (%.1f%%)", var_exp[1]*100),
     ylab = sprintf("PC2 (%.1f%%)", var_exp[2]*100),
     main = "Single-Cell PCA")
