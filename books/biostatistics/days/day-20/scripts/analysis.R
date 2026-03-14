# Day 20: Batch Effects

data <- read.csv("batch_expression.csv")
gene_cols <- grep("^gene_", names(data), value = TRUE)
X <- as.matrix(data[, gene_cols])

# PCA before correction
pca <- prcomp(X, scale. = TRUE)
cat("=== PCA Before Batch Correction ===\n")
var_exp <- summary(pca)$importance[2, 1:3]
cat(sprintf("PC1: %.1f%%, PC2: %.1f%%, PC3: %.1f%%\n",
            var_exp[1]*100, var_exp[2]*100, var_exp[3]*100))

# ANOVA: PC1 vs batch and condition
cat(sprintf("PC1 ~ Batch:     p=%.4e\n",
            summary(aov(pca$x[,1] ~ data$batch))[[1]][["Pr(>F)"]][1]))
cat(sprintf("PC1 ~ Condition: p=%.4e\n",
            summary(aov(pca$x[,1] ~ data$condition))[[1]][["Pr(>F)"]][1]))

# Batch correction: subtract batch means
X_corr <- X
for (batch in unique(data$batch)) {
  mask <- data$batch == batch
  batch_mean <- colMeans(X[mask, ])
  grand_mean <- colMeans(X)
  X_corr[mask, ] <- sweep(X[mask, , drop=FALSE], 2, batch_mean - grand_mean)
}

# PCA after
pca2 <- prcomp(X_corr, scale. = TRUE)
cat("\n=== PCA After Batch Correction ===\n")
var_exp2 <- summary(pca2)$importance[2, 1:3]
cat(sprintf("PC1: %.1f%%, PC2: %.1f%%\n", var_exp2[1]*100, var_exp2[2]*100))

plot(pca$x[,1], pca$x[,2], col = as.factor(data$batch), pch = 16,
     main = "PCA Before Correction", xlab = "PC1", ylab = "PC2")
