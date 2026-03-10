# BioLang Benchmark — R Setup (Windows)
# Usage: powershell -ExecutionPolicy Bypass -File .\setup_r.ps1
#
# R benchmarks are optional — the runner skips them if Rscript is not found.

$ErrorActionPreference = "Stop"

Write-Host "=== BioLang Benchmark — R Setup ===" -ForegroundColor Cyan

# Check Rscript
$rCmd = Get-Command Rscript -ErrorAction SilentlyContinue
if (-not $rCmd) {
    Write-Host "ERROR: Rscript not found." -ForegroundColor Red
    Write-Host "  Install R from https://cran.r-project.org/bin/windows/base/"
    Write-Host "  Make sure R\bin is on your PATH."
    Write-Host ""
    Write-Host "  R is optional — benchmarks will run without it (BioLang vs Python only)."
    exit 1
}

$rVersion = & Rscript --version 2>&1
Write-Host "Found R: $rVersion"
Write-Host ""

# Install R packages
Write-Host "Installing R packages (this may take several minutes on first run) ..."

& Rscript -e @'
  # CRAN packages
  if (!requireNamespace("dplyr", quietly = TRUE)) {
    cat("Installing dplyr ...\n")
    install.packages("dplyr", repos = "https://cloud.r-project.org", quiet = TRUE)
  } else {
    cat("dplyr: already installed\n")
  }

  # Bioconductor
  if (!requireNamespace("BiocManager", quietly = TRUE)) {
    cat("Installing BiocManager ...\n")
    install.packages("BiocManager", repos = "https://cloud.r-project.org", quiet = TRUE)
  }

  bioc_pkgs <- c("Biostrings", "ShortRead", "VariantAnnotation", "GenomicRanges")
  for (pkg in bioc_pkgs) {
    if (!requireNamespace(pkg, quietly = TRUE)) {
      cat(sprintf("Installing %s ...\n", pkg))
      BiocManager::install(pkg, ask = FALSE, update = FALSE, quiet = TRUE)
    } else {
      cat(sprintf("%s: already installed\n", pkg))
    }
  }

  cat("\n=== R setup complete ===\n")
  cat("Installed Bioconductor packages:\n")
  for (pkg in bioc_pkgs) {
    v <- tryCatch(as.character(packageVersion(pkg)), error = function(e) "NOT INSTALLED")
    cat(sprintf("  %s: %s\n", pkg, v))
  }
  cat(sprintf("  dplyr: %s\n", as.character(packageVersion("dplyr"))))
'@
