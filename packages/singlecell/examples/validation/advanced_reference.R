# Cross-check BioLang's paired pseudobulk example with base R.

genes <- c("IFIT1", "ACTB", "MS4A1", "CD3D", "MT-ND1")
profiles <- list()
profile_names <- c()

for (donor in 0:3) {
  for (condition in c("control", "treated")) {
    cells <- matrix(0, nrow = 8, ncol = length(genes))
    for (replicate in 0:7) {
      interferon <- if (condition == "treated") {
        10 + donor * 2 + replicate %% 3
      } else {
        0
      }
      cells[replicate + 1, ] <- c(
        5 + interferon,
        60 + replicate,
        2,
        18,
        2 + replicate %% 2
      )
    }
    counts <- colSums(cells)
    profiles[[length(profiles) + 1]] <- log2(1 + 1e6 * counts / sum(counts))
    profile_names <- c(
      profile_names,
      paste0("D", donor + 1, "@@", condition)
    )
  }
}

profile_matrix <- do.call(rbind, profiles)
control <- profile_matrix[grepl("@@control$", profile_names), , drop = FALSE]
treated <- profile_matrix[grepl("@@treated$", profile_names), , drop = FALSE]
effects <- colMeans(treated) - colMeans(control)
pvalues <- vapply(seq_along(genes), function(i) {
  t.test(control[, i], treated[, i], paired = TRUE)$p.value
}, numeric(1))

args <- commandArgs(trailingOnly = TRUE)
biolang_path <- if (length(args) > 0) {
  args[[1]]
} else {
  "singlecell-results/paired-de.csv"
}
biolang <- read.csv(biolang_path, check.names = FALSE)

for (i in seq_along(genes)) {
  observed <- biolang$log2fc[biolang$gene == genes[[i]]]
  stopifnot(isTRUE(all.equal(observed, effects[[i]], tolerance = 1e-9)))
}
observed_p <- biolang$pvalue[biolang$gene == "IFIT1"]
stopifnot(isTRUE(all.equal(observed_p, pvalues[[1]], tolerance = 1e-10)))

cat(sprintf(
  "advanced validation: BioLang and R agree (IFIT1 log2fc=%.6f, p=%.6g)\n",
  effects[[1]],
  pvalues[[1]]
))
