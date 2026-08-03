suppressPackageStartupMessages({
  library(Matrix)
  library(Seurat)
})

args <- commandArgs(trailingOnly = TRUE)
if (length(args) != 2) {
  stop("usage: Rscript seurat_reference.R INPUT_DIR OUTPUT_CSV")
}

set.seed(1)
counts <- Read10X(data.dir = args[[1]], gene.column = 2)
object <- CreateSeuratObject(
  counts = counts,
  min.cells = 3,
  min.features = 20,
  project = "biolang-validation"
)
object[["percent.mt"]] <- PercentageFeatureSet(object, pattern = "^MT-")
object <- subset(
  object,
  subset = nFeature_RNA <= 5000 & percent.mt <= 25
)
object <- NormalizeData(object, normalization.method = "LogNormalize",
                        scale.factor = 10000, verbose = FALSE)
object <- FindVariableFeatures(
  object,
  selection.method = "vst",
  nfeatures = min(2000, nrow(object)),
  verbose = FALSE
)
object <- ScaleData(object, features = VariableFeatures(object), verbose = FALSE)
npcs <- min(30, ncol(object) - 1, length(VariableFeatures(object)) - 1)
object <- RunPCA(object, features = VariableFeatures(object), npcs = npcs,
                 seed.use = 1, verbose = FALSE)
object <- FindNeighbors(object, dims = seq_len(npcs), k.param = 15,
                        verbose = FALSE)
object <- FindClusters(object, resolution = 0.5, algorithm = 4,
                       random.seed = 1, verbose = FALSE)

result <- data.frame(
  barcode = colnames(object),
  cluster = as.character(Idents(object)),
  check.names = FALSE
)
write.csv(result, args[[2]], row.names = FALSE, quote = FALSE)
cat(sprintf(
  "SEURAT_VALIDATION_OK cells=%d clusters=%d seurat=%s\n",
  ncol(object), length(unique(Idents(object))),
  as.character(packageVersion("Seurat"))
))
