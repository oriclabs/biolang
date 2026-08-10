#!/usr/bin/env Rscript

# Standalone sctransform conformance oracle for BioLang.
#
# This script is deliberately outside BioLang's runtime, package graph, Cargo
# workspace, and book build. It executes the separately licensed R package and
# exports only numeric observations for black-box validation. No implementation
# source from sctransform is copied or inspected here.

validation_library <- Sys.getenv("BIOLANG_VALIDATION_R_LIB", unset = "")
if (!nzchar(validation_library) && dir.exists(".validation-r-library")) {
  validation_library <- normalizePath(".validation-r-library")
}
if (nzchar(validation_library)) {
  .libPaths(c(normalizePath(validation_library, mustWork = TRUE), .libPaths()))
}

suppressPackageStartupMessages({
  library(Matrix)
})

args <- commandArgs(trailingOnly = TRUE)
if (length(args) < 3L || length(args) > 5L) {
  stop(paste(
    "usage: Rscript sctransform_oracle.R",
    "synthetic|sampling|tenx INPUT_DIR OUTPUT_DIR [MAX_PROBE_GENES] [MAX_PROBE_CELLS]"
  ))
}

mode <- args[[1L]]
input_dir <- normalizePath(args[[2L]], mustWork = FALSE)
output_dir <- normalizePath(args[[3L]], mustWork = FALSE)
max_probe_genes <- if (length(args) >= 4L) as.integer(args[[4L]]) else 3000L
max_probe_cells <- if (length(args) >= 5L) as.integer(args[[5L]]) else 64L

assert_fresh_directory <- function(path, label) {
  if (dir.exists(path) && length(list.files(path, all.files = TRUE, no.. = TRUE)) > 0L) {
    stop(label, " must be absent or empty to prevent mixed validation artifacts: ", path)
  }
  dir.create(path, recursive = TRUE, showWarnings = FALSE)
}

write_synthetic_mex <- function(path) {
  assert_fresh_directory(path, "synthetic fixture directory")
  set.seed(1448145L)
  n_cells <- 480L
  n_genes <- 120L
  depth <- exp(rnorm(n_cells, mean = 0, sd = 0.55))
  depth <- depth / mean(depth)
  condition <- rep(c(0, 1), each = n_cells / 2L)
  base_mean <- exp(seq(log(0.025), log(7.5), length.out = n_genes))
  counts <- matrix(0, nrow = n_genes, ncol = n_cells)

  for (gene in seq_len(n_genes)) {
    effect <- if (gene >= 91L && gene <= 110L) {
      ifelse(condition == 1, 2.5, 1.0)
    } else {
      1.0
    }
    mu <- base_mean[[gene]] * depth * effect
    if (gene <= 30L) {
      counts[gene, ] <- rpois(n_cells, lambda = mu)
    } else {
      theta <- 0.8 + (gene - 30L) * 0.22
      counts[gene, ] <- rnbinom(n_cells, mu = mu, size = theta)
    }
  }

  # Explicit boundary cases: genes below min_cells, one exactly at the
  # threshold, and one all-zero gene. These expose axis/drop behaviour without
  # relying on a biological dataset to happen to contain each case.
  counts[111:115, ] <- 0
  for (gene in 111:115) {
    detected <- gene - 109L # 2, 3, 4, 5, 6 detections
    cells <- seq_len(detected) * 17L
    counts[gene, cells] <- seq_len(detected)
  }
  counts[120, ] <- 0

  genes <- sprintf("G%03d", seq_len(n_genes))
  cells <- sprintf("CELL%04d", seq_len(n_cells))
  rownames(counts) <- genes
  colnames(counts) <- cells
  sparse <- as(counts, "dgCMatrix")
  Matrix::writeMM(sparse, file.path(path, "matrix.mtx"))
  write.table(
    data.frame(id = genes, name = genes, type = "Gene Expression"),
    file.path(path, "features.tsv"), sep = "\t", quote = FALSE,
    row.names = FALSE, col.names = FALSE
  )
  write.table(
    data.frame(barcode = cells), file.path(path, "barcodes.tsv"),
    sep = "\t", quote = FALSE, row.names = FALSE, col.names = FALSE
  )
  sparse
}

write_sampling_mex <- function(path) {
  assert_fresh_directory(path, "sampling fixture directory")
  set.seed(9137L)
  n_cells <- 480L
  n_genes <- 3000L
  depth <- exp(rnorm(n_cells, mean = 0, sd = 0.55))
  depth <- depth / mean(depth)
  base_mean <- exp(seq(log(0.025), log(8), length.out = n_genes))
  counts <- matrix(0, nrow = n_genes, ncol = n_cells)
  for (gene in seq_len(n_genes)) {
    theta <- 0.8 + 12 * gene / n_genes
    counts[gene, ] <- rnbinom(n_cells, mu = base_mean[[gene]] * depth, size = theta)
  }
  genes <- sprintf("S%04d", seq_len(n_genes))
  cells <- sprintf("CELL%04d", seq_len(n_cells))
  rownames(counts) <- genes
  colnames(counts) <- cells
  sparse <- as(counts, "dgCMatrix")
  Matrix::writeMM(sparse, file.path(path, "matrix.mtx"))
  write.table(
    data.frame(id = genes, name = genes, type = "Gene Expression"),
    file.path(path, "features.tsv"), sep = "\t", quote = FALSE,
    row.names = FALSE, col.names = FALSE
  )
  write.table(
    data.frame(barcode = cells), file.path(path, "barcodes.tsv"),
    sep = "\t", quote = FALSE, row.names = FALSE, col.names = FALSE
  )
  sparse
}

read_mex <- function(path) {
  matrix_path <- file.path(path, "matrix.mtx")
  if (!file.exists(matrix_path) && file.exists(paste0(matrix_path, ".gz"))) {
    matrix_path <- paste0(matrix_path, ".gz")
  }
  feature_path <- file.path(path, "features.tsv")
  if (!file.exists(feature_path) && file.exists(paste0(feature_path, ".gz"))) {
    feature_path <- paste0(feature_path, ".gz")
  }
  barcode_path <- file.path(path, "barcodes.tsv")
  if (!file.exists(barcode_path) && file.exists(paste0(barcode_path, ".gz"))) {
    barcode_path <- paste0(barcode_path, ".gz")
  }
  umi <- Matrix::readMM(matrix_path)
  features <- read.delim(feature_path, header = FALSE, stringsAsFactors = FALSE)
  barcodes <- read.delim(barcode_path, header = FALSE, stringsAsFactors = FALSE)
  rownames(umi) <- make.unique(as.character(features[[min(2L, ncol(features))]]))
  colnames(umi) <- as.character(barcodes[[1L]])
  as(umi, "dgCMatrix")
}

if (!mode %in% c("synthetic", "sampling", "tenx")) {
  stop("mode must be 'synthetic', 'sampling', or 'tenx'")
}
if (mode == "synthetic") {
  umi <- write_synthetic_mex(input_dir)
} else if (mode == "sampling") {
  umi <- write_sampling_mex(input_dir)
} else {
  umi <- read_mex(normalizePath(input_dir, mustWork = TRUE))
}
assert_fresh_directory(output_dir, "oracle output directory")

set.seed(1448145L)
clip <- sqrt(ncol(umi) / 30)
started <- proc.time()[["elapsed"]]
oracle <- sctransform::vst(
  umi,
  vst.flavor = "v2",
  n_cells = min(5000L, ncol(umi)),
  n_genes = min(2000L, nrow(umi)),
  min_cells = 5L,
  res_clip_range = c(-clip, clip),
  return_cell_attr = TRUE,
  return_gene_attr = TRUE,
  return_corrected_umi = FALSE,
  verbosity = 0
)
elapsed <- proc.time()[["elapsed"]] - started

model <- as.data.frame(oracle$model_pars_fit)
raw_model <- as.data.frame(oracle$model_pars)
attributes <- as.data.frame(oracle$gene_attr)
model_genes <- rownames(model)
intercept_name <- intersect(c("(Intercept)", "Intercept", "intercept"), colnames(model))
if (length(intercept_name) != 1L || !"theta" %in% colnames(model)) {
  stop("unexpected sctransform model_pars_fit columns: ", paste(colnames(model), collapse = ", "))
}
if (!"residual_variance" %in% colnames(attributes)) {
  stop("unexpected sctransform gene_attr columns: ", paste(colnames(attributes), collapse = ", "))
}

ranked <- rownames(attributes)[order(attributes$residual_variance, decreasing = TRUE)]
original_index <- match(model_genes, rownames(umi))
gene_rows <- data.frame(
  gene = model_genes,
  gene_index = original_index - 1L,
  detected_cells = Matrix::rowSums(umi[model_genes, , drop = FALSE] > 0),
  theta = model$theta,
  intercept = model[[intercept_name]],
  raw_theta = raw_model[model_genes, "theta"],
  raw_intercept = raw_model[model_genes, intercept_name],
  model_outlier = as.logical(oracle$model_pars_outliers[model_genes]),
  residual_variance = attributes[model_genes, "residual_variance"],
  stringsAsFactors = FALSE
)
rank_rows <- data.frame(
  rank = seq_along(ranked),
  gene = ranked,
  gene_index = match(ranked, rownames(umi)) - 1L,
  residual_variance = attributes[ranked, "residual_variance"],
  stringsAsFactors = FALSE
)

# vst returns gene x cell residuals. BioLang exposes centered scale-data, so
# center after the identical clipping boundary before exporting observations.
centered <- oracle$y - rowMeans(oracle$y)
# Validate the genes that drive variable-feature selection and PCA. A prefix of
# the input gene axis systematically misses the high-residual-variance tail.
probe_genes <- head(intersect(ranked, rownames(centered)), max_probe_genes)
probe_cells <- head(colnames(umi), max_probe_cells)
probe <- centered[probe_genes, probe_cells, drop = FALSE]
residual_rows <- data.frame(
  gene = rep(rownames(probe), times = ncol(probe)),
  cell = rep(colnames(probe), each = nrow(probe)),
  residual = as.vector(probe),
  stringsAsFactors = FALSE
)

write.csv(gene_rows, file.path(output_dir, "genes.csv"), row.names = FALSE, quote = TRUE)
write.csv(rank_rows, file.path(output_dir, "ranking.csv"), row.names = FALSE, quote = TRUE)
write.csv(residual_rows, file.path(output_dir, "residuals.csv"), row.names = FALSE, quote = TRUE)
write.csv(
  data.frame(
    gene = names(oracle$genes_log_gmean_step1),
    log_geometric_mean = as.numeric(oracle$genes_log_gmean_step1),
    stringsAsFactors = FALSE
  ),
  file.path(output_dir, "fit-genes.csv"), row.names = FALSE, quote = TRUE
)
write.csv(
  data.frame(cell = as.character(oracle$cells_step1), stringsAsFactors = FALSE),
  file.path(output_dir, "fit-cells.csv"), row.names = FALSE, quote = TRUE
)
argument <- function(name, fallback = NA) {
  value <- oracle$arguments[[name]]
  if (is.null(value) || length(value) == 0L) fallback else value[[1L]]
}
write.csv(
  data.frame(
    implementation = "sctransform::vst",
    sctransform_version = as.character(packageVersion("sctransform")),
    glmGamPoi_version = if (requireNamespace("glmGamPoi", quietly = TRUE)) {
      as.character(packageVersion("glmGamPoi"))
    } else {
      NA_character_
    },
    cells = ncol(umi), genes = nrow(umi), modelled_genes = nrow(model),
    clip = clip, seed = 1448145L, cells_for_fit = min(5000L, ncol(umi)),
    genes_for_fit = min(2000L, nrow(umi)), min_cells = 5L,
    actual_method = as.character(argument("method")),
    actual_n_cells = as.integer(argument("n_cells")),
    actual_n_genes = as.integer(argument("n_genes")),
    actual_exclude_poisson = as.logical(argument("exclude_poisson")),
    actual_theta_regularization = as.character(argument("theta_regularization")),
    actual_min_variance = as.character(argument("min_variance")),
    actual_bw_adjust = as.numeric(argument("bw_adjust")),
    residual_probe_strategy = "top_residual_variance",
    residual_probe_genes = length(probe_genes),
    residual_probe_cells = length(probe_cells),
    elapsed_seconds = elapsed
  ),
  file.path(output_dir, "manifest.csv"), row.names = FALSE, quote = TRUE
)
capture.output(sessionInfo(), file = file.path(output_dir, "session-info.txt"))
cat(sprintf(
  "SCTRANSFORM_ORACLE_OK cells=%d genes=%d modelled=%d elapsed=%.3fs\n",
  ncol(umi), nrow(umi), nrow(model), elapsed
))
