#!/usr/bin/env Rscript

# Prepare the two HBC post-QC count matrices for standalone SCTransform
# conformance testing. This is validation-only data preparation: it uses public
# Matrix APIs, does not inspect package implementation source, and is not a
# BioLang runtime or build dependency.

suppressPackageStartupMessages(library(Matrix))

args <- commandArgs(trailingOnly = TRUE)
if (length(args) != 3L) {
  stop(paste(
    "usage: Rscript prepare_hbc_sctransform_fixture.R",
    "CTRL_RAW_DIR STIM_RAW_DIR OUTPUT_DIR"
  ))
}

ctrl_dir <- normalizePath(args[[1L]], mustWork = TRUE)
stim_dir <- normalizePath(args[[2L]], mustWork = TRUE)
output_dir <- normalizePath(args[[3L]], mustWork = FALSE)
if (dir.exists(output_dir) &&
    length(list.files(output_dir, all.files = TRUE, no.. = TRUE)) > 0L) {
  stop("output directory must be absent or empty: ", output_dir)
}
dir.create(output_dir, recursive = TRUE, showWarnings = FALSE)

first_existing <- function(directory, stem) {
  candidates <- file.path(directory, c(stem, paste0(stem, ".gz")))
  found <- candidates[file.exists(candidates)]
  if (length(found) != 1L) stop("expected exactly one ", stem, " in ", directory)
  found[[1L]]
}

read_lines <- function(path) {
  if (endsWith(path, ".gz")) readLines(gzfile(path)) else readLines(path)
}

read_mex <- function(directory) {
  matrix_path <- first_existing(directory, "matrix.mtx")
  feature_path <- first_existing(directory, "features.tsv")
  barcode_path <- first_existing(directory, "barcodes.tsv")
  connection <- if (endsWith(matrix_path, ".gz")) gzfile(matrix_path) else matrix_path
  counts <- as(Matrix::readMM(connection), "dgCMatrix")
  feature_fields <- strsplit(read_lines(feature_path), "\t", fixed = TRUE)
  feature_names <- make.unique(vapply(
    feature_fields,
    function(fields) fields[[min(2L, length(fields))]],
    character(1L)
  ))
  for (index in seq_along(feature_fields)) {
    name_column <- min(2L, length(feature_fields[[index]]))
    feature_fields[[index]][[name_column]] <- feature_names[[index]]
  }
  barcodes <- sub("\r$", "", read_lines(barcode_path))
  rownames(counts) <- feature_names
  colnames(counts) <- barcodes
  list(counts = counts, feature_fields = feature_fields)
}

filter_cells <- function(input) {
  counts <- input$counts
  umi <- Matrix::colSums(counts)
  genes <- Matrix::colSums(counts > 0)
  mitochondrial <- startsWith(rownames(counts), "MT-")
  mito_fraction <- Matrix::colSums(counts[mitochondrial, , drop = FALSE]) / umi
  novelty <- log10(genes) / log10(umi)
  keep <- umi >= 500 & genes >= 250 & novelty > 0.80 & mito_fraction < 0.20
  input$counts <- counts[, keep, drop = FALSE]
  input
}

write_mex <- function(input, keep_genes, directory) {
  dir.create(directory, recursive = TRUE, showWarnings = FALSE)
  counts <- input$counts[keep_genes, , drop = FALSE]
  Matrix::writeMM(counts, file.path(directory, "matrix.mtx"))
  fields <- input$feature_fields[keep_genes]
  writeLines(vapply(fields, function(x) paste(x, collapse = "\t"), character(1L)),
             file.path(directory, "features.tsv"))
  writeLines(colnames(counts), file.path(directory, "barcodes.tsv"))
}

ctrl <- filter_cells(read_mex(ctrl_dir))
stim <- filter_cells(read_mex(stim_dir))
if (!identical(rownames(ctrl$counts), rownames(stim$counts))) {
  stop("control and stimulated feature axes differ")
}
detected <- Matrix::rowSums(ctrl$counts > 0) + Matrix::rowSums(stim$counts > 0)
keep_genes <- detected >= 10L

if (ncol(ctrl$counts) != 14847L || ncol(stim$counts) != 14782L ||
    sum(keep_genes) != 14065L) {
  stop(sprintf(
    "HBC checkpoint mismatch: ctrl=%d stim=%d genes=%d",
    ncol(ctrl$counts), ncol(stim$counts), sum(keep_genes)
  ))
}

write_mex(ctrl, keep_genes, file.path(output_dir, "ctrl"))
write_mex(stim, keep_genes, file.path(output_dir, "stim"))
write.csv(
  data.frame(ctrl_cells = ncol(ctrl$counts), stim_cells = ncol(stim$counts),
             retained_genes = sum(keep_genes)),
  file.path(output_dir, "manifest.csv"), row.names = FALSE, quote = FALSE
)
cat(sprintf(
  "HBC_SCTRANSFORM_FIXTURE_OK ctrl=%d stim=%d genes=%d\n",
  ncol(ctrl$counts), ncol(stim$counts), sum(keep_genes)
))
