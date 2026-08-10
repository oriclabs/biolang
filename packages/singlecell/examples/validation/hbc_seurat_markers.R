# Validation-only Seurat marker oracle for the measured HBC cluster labels.
# Reconstructing the RNA assay from the hashed raw inputs is equivalent to
# retaining the large integrated R object: FindAllMarkers uses normalized RNA
# counts plus the already-exported cluster identities, not CCA coordinates.

validation_library <- Sys.getenv("BIOLANG_VALIDATION_R_LIB", unset = "")
if (!nzchar(validation_library) && dir.exists(".validation-r-library")) {
  validation_library <- normalizePath(".validation-r-library")
}
if (nzchar(validation_library)) {
  .libPaths(c(normalizePath(validation_library, mustWork = TRUE), .libPaths()))
}

suppressPackageStartupMessages({
  library(Matrix)
  library(Seurat)
})

args <- commandArgs(trailingOnly = TRUE)
if (length(args) != 4L) {
  stop(paste(
    "usage: Rscript hbc_seurat_markers.R",
    "CTRL_10X_DIR STIM_10X_DIR SEURAT_CELLS_CSV_GZ OUTPUT_DIR"
  ))
}

ctrl_dir <- normalizePath(args[[1L]], mustWork = TRUE)
stim_dir <- normalizePath(args[[2L]], mustWork = TRUE)
cells_path <- normalizePath(args[[3L]], mustWork = TRUE)
output_dir <- normalizePath(args[[4L]], mustWork = TRUE)
log_path <- file.path(output_dir, "markers-run.log")
timing_path <- file.path(output_dir, "markers-timings.tsv")
run_started <- Sys.time()
writeLines("timestamp\telapsed_seconds\tstage", timing_path)

stamp <- function(label) {
  timestamp <- format(Sys.time(), "%Y-%m-%dT%H:%M:%S%z")
  elapsed <- as.numeric(difftime(Sys.time(), run_started, units = "secs"))
  line <- sprintf("[%s] %s", timestamp, label)
  message(line)
  cat(
    sprintf("%s\t%.3f\t%s\n", timestamp, elapsed, label),
    file = timing_path,
    append = TRUE
  )
}

log_connection <- file(log_path, open = "wt")
sink(log_connection, split = TRUE)
sink(log_connection, type = "message")
on.exit({
  sink(type = "message")
  sink()
  close(log_connection)
}, add = TRUE)

stamp("read and filter hashed HBC inputs")
ctrl <- CreateSeuratObject(
  Read10X(ctrl_dir, gene.column = 2L),
  min.features = 100L,
  project = "ctrl"
)
stim <- CreateSeuratObject(
  Read10X(stim_dir, gene.column = 2L),
  min.features = 100L,
  project = "stim"
)
merged <- JoinLayers(merge(ctrl, stim, add.cell.ids = c("ctrl", "stim")))
rm(ctrl, stim)
merged$nUMI <- merged$nCount_RNA
merged$nGene <- merged$nFeature_RNA
merged$log10GenesPerUMI <- log10(merged$nGene) / log10(merged$nUMI)
merged$mitoRatio <- PercentageFeatureSet(merged, pattern = "^MT-") / 100
filtered <- subset(
  merged,
  subset = nUMI >= 500 & nGene >= 250 &
    log10GenesPerUMI > 0.80 & mitoRatio < 0.20
)
counts <- GetAssayData(filtered, layer = "counts")
keep_genes <- Matrix::rowSums(counts > 0) >= 10L
filtered <- CreateSeuratObject(
  counts[keep_genes, , drop = FALSE],
  meta.data = filtered[[]],
  project = "HBC"
)
if (nrow(filtered) != 14065L || ncol(filtered) != 29629L) {
  stop("marker reconstruction failed the HBC gene/cell checkpoint")
}

stamp("join measured Seurat 0.8 cluster labels")
cells <- read.csv(gzfile(cells_path), stringsAsFactors = FALSE)
if (anyDuplicated(cells$cell_id) || nrow(cells) != ncol(filtered)) {
  stop("Seurat cell manifest is not a unique 29,629-cell oracle")
}
labels <- setNames(as.character(cells$cluster), cells$cell_id)
labels <- labels[colnames(filtered)]
if (anyNA(labels)) {
  stop("Seurat cell manifest does not cover every reconstructed cell")
}
Idents(filtered) <- labels

stamp("LogNormalize RNA scale.factor=10000")
DefaultAssay(filtered) <- "RNA"
filtered <- NormalizeData(
  filtered,
  normalization.method = "LogNormalize",
  scale.factor = 10000,
  verbose = FALSE
)

stamp("FindAllMarkers only.pos=TRUE min.pct=0.1 logfc.threshold=0.25")
markers <- FindAllMarkers(
  filtered,
  only.pos = TRUE,
  min.pct = 0.1,
  logfc.threshold = 0.25,
  test.use = "wilcox",
  verbose = TRUE
)
markers$gene <- rownames(markers)
write.csv(
  markers,
  file.path(output_dir, "markers.csv"),
  row.names = FALSE,
  quote = TRUE
)

write.csv(
  data.frame(
    cells = ncol(filtered),
    genes = nrow(filtered),
    clusters = length(unique(Idents(filtered))),
    positive_marker_rows = nrow(markers),
    stringsAsFactors = FALSE
  ),
  file.path(output_dir, "markers-summary.csv"),
  row.names = FALSE,
  quote = TRUE
)
writeLines(
  capture.output(sessionInfo()),
  file.path(output_dir, "markers-session-info.txt")
)
stamp(sprintf("SEURAT_HBC_MARKERS_OK rows=%d", nrow(markers)))
