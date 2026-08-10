#!/usr/bin/env Rscript

# Validation-only oracle for the MIT-covered Seurat 5.5.1 anchor and weighting
# path. This script is not imported by BioLang and its outputs are not runtime
# inputs. It deliberately avoids SCTransform so this fixture isolates Seurat's
# own R/integration.R and src/integration.cpp behavior.

suppressPackageStartupMessages({
  library(Matrix)
  library(Seurat)
})

if (as.character(packageVersion("Seurat")) != "5.5.1") {
  stop("This fixture is pinned to Seurat 5.5.1")
}

out_dir <- if (length(commandArgs(trailingOnly = TRUE)) > 0) {
  commandArgs(trailingOnly = TRUE)[[1]]
} else {
  file.path("validation-results", "seurat-mit-anchor-fixture")
}
dir.create(out_dir, recursive = TRUE, showWarnings = FALSE)

left <- rbind(
  c(2.0, 0.1, -0.2), c(1.8, -0.1, 0.2), c(2.2, 0.0, 0.1),
  c(-2.0, 0.2, 0.1), c(-1.8, -0.2, -0.1), c(-2.2, 0.0, 0.2)
)
right <- rbind(
  c(2.1, 0.0, -0.1), c(1.9, 0.1, 0.1), c(2.3, -0.1, 0.0),
  c(-1.9, 0.1, 0.0), c(-1.7, -0.1, -0.2), c(-2.1, 0.1, 0.1)
)
rownames(left) <- paste0("L", seq_len(nrow(left)))
rownames(right) <- paste0("R", seq_len(nrow(right)))
colnames(left) <- colnames(right) <- paste0("G", seq_len(ncol(left)))

cca <- Seurat:::RunCCA.default(
  object1 = t(left), object2 = t(right), standardize = TRUE,
  num.cc = 2, seed.use = 42, verbose = FALSE
)
embedding <- Seurat:::L2Norm(cca$ccv)
left_embedding <- embedding[rownames(left), , drop = FALSE]
right_embedding <- embedding[rownames(right), , drop = FALSE]

knn <- function(query, reference, k) {
  t(vapply(seq_len(nrow(query)), function(i) {
    distances <- sqrt(rowSums((reference - matrix(
      query[i, ], nrow(reference), ncol(reference), byrow = TRUE
    )) ^ 2))
    order(distances, seq_along(distances))[seq_len(min(k, length(distances)))]
  }, integer(min(k, nrow(reference)))))
}

k_anchor <- 2L
k_score <- 3L
ab <- knn(left_embedding, right_embedding, k_score)
ba <- knn(right_embedding, left_embedding, k_score)
aa <- knn(left_embedding, left_embedding, k_score)
bb <- knn(right_embedding, right_embedding, k_score)

pairs <- do.call(rbind, lapply(seq_len(nrow(left)), function(i) {
  candidates <- ab[i, seq_len(k_anchor)]
  mutual <- candidates[vapply(candidates, function(j) {
    i %in% ba[j, seq_len(k_anchor)]
  }, logical(1))]
  if (length(mutual) == 0) return(NULL)
  cbind(left = i, right = mutual)
}))

# With only three features, Seurat's TopDimFeatures includes all of them. This
# therefore exercises FilterAnchors' L2-normalized high-dimensional kNN without
# needing an S4 DimReduc wrapper in the oracle fixture.
k_filter <- 3L
filter_ab <- knn(
  Seurat:::L2Norm(left), Seurat:::L2Norm(right), k_filter
)
pairs <- pairs[vapply(seq_len(nrow(pairs)), function(i) {
  pairs[i, "right"] %in% filter_ab[pairs[i, "left"], ]
}, logical(1)), , drop = FALSE]

offset <- nrow(left)
raw_score <- vapply(seq_len(nrow(pairs)), function(i) {
  left_set <- c(aa[pairs[i, "left"], ], ab[pairs[i, "left"], ] + offset)
  right_set <- c(ba[pairs[i, "right"], ], bb[pairs[i, "right"], ] + offset)
  length(intersect(left_set, right_set))
}, numeric(1))
low <- as.numeric(quantile(raw_score, 0.01, names = FALSE))
high <- as.numeric(quantile(raw_score, 0.90, names = FALSE))
score <- pmax(0, pmin(1, (raw_score - low) / (high - low)))
anchors <- data.frame(
  left = pairs[, "left"] - 1L,
  right = pairs[, "right"] - 1L,
  raw_score = raw_score,
  score = score
)

write.csv(left_embedding, file.path(out_dir, "left-embedding.csv"), quote = FALSE)
write.csv(right_embedding, file.path(out_dir, "right-embedding.csv"), quote = FALSE)
write.csv(anchors, file.path(out_dir, "anchors.csv"), row.names = FALSE, quote = FALSE)

# Isolate the compiled FindWeightsC/IntegrateDataC formulas with a fixture whose
# expected correction is also asserted in Rust.
weights <- Seurat:::FindWeightsC(
  cells2 = 0,
  distances = matrix(c(0.8, 0.8, 0.0), nrow = 1),
  anchor_cells2 = c("q0", "q1", "q2"),
  integration_matrix_rownames = c("q0", "q1", "q2"),
  cell_index = matrix(c(1, 2, 3), nrow = 1),
  anchor_score = c(0.5, 1.0, 1.0),
  min_dist = 0,
  sd = 1,
  display_progress = FALSE
)
corrected <- Seurat:::IntegrateDataC(
  integration_matrix = sparseMatrix(
    i = 1:3, j = rep(1, 3), x = c(2, 4, 5), dims = c(3, 1)
  ),
  weights = weights,
  expression_cells2 = sparseMatrix(i = 1, j = 1, x = 8, dims = c(1, 1))
)
write.csv(as.matrix(weights), file.path(out_dir, "weights.csv"), quote = FALSE)
writeLines(sprintf("%.17g", as.numeric(corrected[1, 1])),
           file.path(out_dir, "corrected.txt"))
writeLines(capture.output(sessionInfo()), file.path(out_dir, "session-info.txt"))

cat(normalizePath(out_dir, winslash = "/", mustWork = TRUE), "\n")
