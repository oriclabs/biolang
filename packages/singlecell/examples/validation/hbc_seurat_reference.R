# Independent HBC/Seurat oracle for BioLang validation.
#
# This script is intentionally not called by BioLang, Cargo, package tests, or
# the book build. Seurat and its GPL dependencies belong only to the external
# validation environment.

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
if (length(args) != 3L) {
  stop(paste(
    "usage: Rscript hbc_seurat_reference.R",
    "CTRL_10X_DIR STIM_10X_DIR OUTPUT_DIR"
  ))
}

ctrl_dir <- normalizePath(args[[1L]], mustWork = TRUE)
stim_dir <- normalizePath(args[[2L]], mustWork = TRUE)
output_dir <- normalizePath(args[[3L]], mustWork = FALSE)
dir.create(output_dir, recursive = TRUE, showWarnings = FALSE)

log_path <- file.path(output_dir, "run.log")
timing_path <- file.path(output_dir, "timings.tsv")
run_started <- Sys.time()
writeLines("timestamp\telapsed_seconds\tstage", timing_path)
log_connection <- file(log_path, open = "wt")
sink(log_connection, split = TRUE)
sink(log_connection, type = "message")
on.exit({
  sink(type = "message")
  sink()
  close(log_connection)
}, add = TRUE)

stamp <- function(stage, ...) {
  label <- paste0(stage, paste0(...))
  timestamp <- format(Sys.time(), "%Y-%m-%dT%H:%M:%S%z")
  cat(sprintf(
    "[%s] %s%s\n",
    timestamp,
    label,
    ""
  ))
  cat(
    sprintf(
      "%s\t%.3f\t%s\n",
      timestamp,
      as.numeric(difftime(Sys.time(), run_started, units = "secs")),
      label
    ),
    file = timing_path,
    append = TRUE
  )
  flush.console()
}

write_csv <- function(value, name) {
  write.csv(
    value,
    file.path(output_dir, name),
    row.names = FALSE,
    quote = TRUE
  )
}

write_csv_gz <- function(value, name) {
  connection <- gzfile(file.path(output_dir, name), open = "wt")
  on.exit(close(connection), add = TRUE)
  write.csv(value, connection, row.names = FALSE, quote = TRUE)
}

sha256_file <- function(path) {
  if (!requireNamespace("digest", quietly = TRUE)) {
    return(NA_character_)
  }
  digest::digest(file = path, algo = "sha256", serialize = FALSE)
}

input_files <- unlist(lapply(
  c(ctrl = ctrl_dir, stim = stim_dir),
  function(directory) file.path(
    directory,
    c("matrix.mtx.gz", "features.tsv.gz", "barcodes.tsv.gz")
  )
), use.names = FALSE)
if (!all(file.exists(input_files))) {
  stop("each input directory must contain matrix.mtx.gz, features.tsv.gz, and barcodes.tsv.gz")
}

input_manifest <- data.frame(
  sample = rep(c("ctrl", "stim"), each = 3L),
  file = basename(input_files),
  bytes = as.numeric(file.info(input_files)$size),
  sha256 = vapply(input_files, sha256_file, character(1L)),
  stringsAsFactors = FALSE
)
write_csv(input_manifest, "input-manifest.csv")

set.seed(123456)
options(future.globals.maxSize = 8 * 1024^3)

stamp("ENVIRONMENT ", R.version.string)
stamp("ENVIRONMENT Seurat=", as.character(packageVersion("Seurat")))
stamp("ENVIRONMENT SeuratObject=", as.character(packageVersion("SeuratObject")))
stamp("ENVIRONMENT sctransform=", as.character(packageVersion("sctransform")))
stamp(
  "ENVIRONMENT glmGamPoi=",
  if (requireNamespace("glmGamPoi", quietly = TRUE)) {
    as.character(packageVersion("glmGamPoi"))
  } else {
    "not-installed (SCTransform native backend)"
  }
)

stamp("READ ctrl")
ctrl_counts <- Read10X(data.dir = ctrl_dir, gene.column = 2L)
stamp("READ stim")
stim_counts <- Read10X(data.dir = stim_dir, gene.column = 2L)

# These calls mirror the HBC quality-control setup lesson.
ctrl <- CreateSeuratObject(
  counts = ctrl_counts,
  min.features = 100L,
  project = "ctrl"
)
stim <- CreateSeuratObject(
  counts = stim_counts,
  min.features = 100L,
  project = "stim"
)
rm(ctrl_counts, stim_counts)
gc()

stamp("CHECKPOINT initial ctrl cells=", ncol(ctrl), " stim cells=", ncol(stim))
if (ncol(ctrl) != 15688L || ncol(stim) != 15756L) {
  stop("initial HBC cell checkpoint failed (expected ctrl=15688, stim=15756)")
}

stamp("MERGE samples")
merged <- merge(
  x = ctrl,
  y = stim,
  add.cell.ids = c("ctrl", "stim")
)
merged <- JoinLayers(merged)
rm(ctrl, stim)
gc()

merged$nUMI <- merged$nCount_RNA
merged$nGene <- merged$nFeature_RNA
merged$log10GenesPerUMI <- log10(merged$nGene) / log10(merged$nUMI)
merged$mitoRatio <- PercentageFeatureSet(merged, pattern = "^MT-") / 100

stamp("QC filter cells")
filtered <- subset(
  merged,
  subset = nUMI >= 500 &
    nGene >= 250 &
    log10GenesPerUMI > 0.80 &
    mitoRatio < 0.20
)
rm(merged)
gc()

sample_counts <- table(filtered$orig.ident)
stamp(
  "CHECKPOINT filtered cells=", ncol(filtered),
  " ctrl=", unname(sample_counts[["ctrl"]]),
  " stim=", unname(sample_counts[["stim"]])
)
if (ncol(filtered) != 29629L ||
    unname(sample_counts[["ctrl"]]) != 14847L ||
    unname(sample_counts[["stim"]]) != 14782L) {
  stop("filtered HBC cell checkpoint failed")
}

stamp("QC filter genes")
counts <- GetAssayData(filtered, layer = "counts")
keep_genes <- Matrix::rowSums(counts > 0) >= 10L
filtered_counts <- counts[keep_genes, , drop = FALSE]
filtered_metadata <- filtered[[]]
filtered <- CreateSeuratObject(
  counts = filtered_counts,
  meta.data = filtered_metadata,
  project = "HBC"
)
rm(counts, filtered_counts, filtered_metadata, keep_genes)
gc()

stamp("CHECKPOINT retained genes=", nrow(filtered), " cells=", ncol(filtered))
if (nrow(filtered) != 14065L || ncol(filtered) != 29629L) {
  stop("filtered HBC gene/cell checkpoint failed")
}

qc_cells <- filtered[[]]
qc_cells$cell_id <- rownames(qc_cells)
qc_cells$sample <- qc_cells$orig.ident
qc_cells$barcode <- sub("^(ctrl|stim)_", "", qc_cells$cell_id)
write_csv(
  qc_cells[, c(
    "cell_id", "sample", "barcode", "nCount_RNA", "nFeature_RNA",
    "log10GenesPerUMI", "mitoRatio"
  )],
  "qc-cells.csv"
)
write_csv(
  data.frame(gene = rownames(filtered), stringsAsFactors = FALSE),
  "retained-genes.csv"
)

stamp("SCT split samples")
split_seurat <- SplitObject(filtered, split.by = "orig.ident")
rm(filtered, qc_cells)
gc()

for (sample_name in names(split_seurat)) {
  stamp("SCT start sample=", sample_name)
  split_seurat[[sample_name]] <- SCTransform(
    split_seurat[[sample_name]],
    vars.to.regress = "mitoRatio",
    vst.flavor = "v2",
    conserve.memory = TRUE,
    verbose = TRUE
  )
  stamp(
    "SCT finish sample=", sample_name,
    " features=", nrow(split_seurat[[sample_name]][["SCT"]]),
    " variable=", length(VariableFeatures(split_seurat[[sample_name]]))
  )
  gc()
}

stamp("INTEGRATION select 3000 features")
integration_features <- SelectIntegrationFeatures(
  object.list = split_seurat,
  nfeatures = 3000L
)
write_csv(
  data.frame(
    rank = seq_along(integration_features),
    gene = integration_features,
    stringsAsFactors = FALSE
  ),
  "integration-features.csv"
)

stamp("INTEGRATION PrepSCTIntegration")
split_seurat <- PrepSCTIntegration(
  object.list = split_seurat,
  anchor.features = integration_features,
  verbose = TRUE
)

# HBC leaves dims unspecified here. Seurat therefore uses the package default
# (1:30) for anchor finding; 40 PCs are used later for UMAP and neighbors.
stamp("INTEGRATION FindIntegrationAnchors course defaults")
integration_anchors <- FindIntegrationAnchors(
  object.list = split_seurat,
  normalization.method = "SCT",
  anchor.features = integration_features,
  verbose = TRUE
)

stamp("INTEGRATION IntegrateData course defaults")
integrated <- IntegrateData(
  anchorset = integration_anchors,
  normalization.method = "SCT",
  verbose = TRUE
)
integrated[["RNA"]] <- JoinLayers(integrated[["RNA"]])
rm(split_seurat, integration_anchors)
gc()

stamp("PCA RunPCA course defaults")
integrated <- RunPCA(integrated, verbose = TRUE)

stamp("UMAP dims=1:40 seed=123456")
set.seed(123456)
integrated <- RunUMAP(
  integrated,
  dims = 1:40,
  reduction = "pca",
  seed.use = 123456,
  verbose = TRUE
)

stamp("GRAPH FindNeighbors dims=1:40 k.param=20")
integrated <- FindNeighbors(
  integrated,
  dims = 1:40,
  k.param = 20L,
  verbose = TRUE
)

resolutions <- c(0.4, 0.6, 0.8, 1.0, 1.4)
stamp("CLUSTER resolutions=", paste(resolutions, collapse = ","))
integrated <- FindClusters(
  integrated,
  resolution = resolutions,
  algorithm = 1L,
  random.seed = 0L,
  verbose = TRUE
)

resolution_column <- grep(
  "_snn_res\\.0\\.8$",
  colnames(integrated[[]]),
  value = TRUE
)
if (length(resolution_column) != 1L) {
  stop("could not identify the resolution 0.8 cluster column")
}
Idents(integrated) <- resolution_column

metadata <- integrated[[]]
metadata$cell_id <- rownames(metadata)
metadata$sample <- metadata$orig.ident
metadata$barcode <- sub("^(ctrl|stim)_", "", metadata$cell_id)
metadata$cluster <- as.character(Idents(integrated))
umap <- Embeddings(integrated, reduction = "umap")
metadata$umap_1 <- umap[, 1L]
metadata$umap_2 <- umap[, 2L]

resolution_columns <- grep("_snn_res\\.", colnames(metadata), value = TRUE)
cell_columns <- c(
  "cell_id", "sample", "barcode", "cluster", "umap_1", "umap_2",
  resolution_columns
)
write_csv_gz(metadata[, cell_columns, drop = FALSE], "cells.csv.gz")

pca <- Embeddings(integrated, reduction = "pca")[, 1:40, drop = FALSE]
pca_output <- data.frame(
  cell_id = rownames(pca),
  sample = metadata[rownames(pca), "sample"],
  barcode = metadata[rownames(pca), "barcode"],
  pca,
  check.names = FALSE
)
write_csv_gz(pca_output, "pcs.csv.gz")

cluster_trajectory <- vapply(
  resolution_columns,
  function(column) length(unique(metadata[[column]])),
  integer(1L)
)
write_csv(
  data.frame(
    resolution_column = resolution_columns,
    clusters = unname(cluster_trajectory),
    stringsAsFactors = FALSE
  ),
  "cluster-trajectory.csv"
)

summary <- data.frame(
  cells = ncol(integrated),
  genes = nrow(integrated[["RNA"]]),
  integration_features = length(integration_features),
  anchor_dims = 30L,
  downstream_pcs = 40L,
  neighbors = 20L,
  resolution = 0.8,
  clusters = length(unique(Idents(integrated))),
  sct_backend = if (requireNamespace("glmGamPoi", quietly = TRUE)) {
    "glmGamPoi-available"
  } else {
    "sctransform-native"
  },
  stringsAsFactors = FALSE
)
write_csv(summary, "summary.csv")

session_output <- capture.output(sessionInfo())
writeLines(session_output, file.path(output_dir, "session-info.txt"))

artifact_files <- setdiff(
  list.files(output_dir, full.names = TRUE),
  c(log_path, file.path(output_dir, "artifact-manifest.csv"))
)
artifact_manifest <- data.frame(
  file = basename(artifact_files),
  bytes = as.numeric(file.info(artifact_files)$size),
  sha256 = vapply(artifact_files, sha256_file, character(1L)),
  stringsAsFactors = FALSE
)
write_csv(artifact_manifest, "artifact-manifest.csv")

stamp(
  "SEURAT_HBC_VALIDATION_OK cells=", summary$cells,
  " genes=", summary$genes,
  " clusters=", summary$clusters,
  " seurat=", as.character(packageVersion("Seurat"))
)
