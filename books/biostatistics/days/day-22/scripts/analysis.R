# Day 22: Clustering

data <- read.csv("tumor_expression.csv")
gene_cols <- grep("^gene_", names(data), value = TRUE)
X <- scale(as.matrix(data[, gene_cols]))
true_labels <- data$true_subtype

# K-means
set.seed(42)
km <- kmeans(X, centers = 4, nstart = 10)
cat("=== K-Means (k=4) ===\n")
cat(sprintf("Total WSS: %.0f\n", km$tot.withinss))

# Hierarchical
hc <- hclust(dist(X), method = "ward.D2")
hc_labels <- cutree(hc, k = 4)

# Compare with true labels
cat("\nConfusion Matrix (K-Means vs True):\n")
print(table(KMeans = km$cluster, True = true_labels))

# Elbow and silhouette
cat("\n=== Elbow Method ===\n")
for (k in 2:6) {
  km_k <- kmeans(X, centers = k, nstart = 10)
  cat(sprintf("  k=%d: WSS=%.0f\n", k, km_k$tot.withinss))
}

# Dendrogram (truncated)
plot(hc, labels = FALSE, main = "Hierarchical Clustering", hang = -1)
rect.hclust(hc, k = 4, border = 2:5)
