# R packages for "Practical Bioinformatics in 30 Days"
# Run this script once before starting the exercises:
#   Rscript install.R

# CRAN packages
install.packages(c(
  "dplyr",
  "jsonlite",
  "httr2",
  "digest",
  "logging",
  "futile.logger",
  "ggplot2",
  "pheatmap"
), repos = "https://cran.r-project.org")

# Bioconductor packages (optional — used in later chapters)
if (!requireNamespace("BiocManager", quietly = TRUE)) {
  install.packages("BiocManager", repos = "https://cran.r-project.org")
}
BiocManager::install(c("Biostrings", "GenomicRanges"), ask = FALSE)
