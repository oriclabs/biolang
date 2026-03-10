#!/usr/bin/env bash
# Install R dependencies for BioLang benchmarks
# Usage: ./setup_r.sh
#
# Requires: R (4.0+) and Rscript
# R benchmarks are optional — the runner skips them if Rscript is not found.

set -euo pipefail

echo "=== BioLang Benchmark — R Setup ==="

# Check Rscript
if ! command -v Rscript &>/dev/null; then
  echo "ERROR: Rscript not found. Install R first."
  echo "  Ubuntu/Debian: sudo apt install r-base r-base-dev"
  echo "  macOS:         brew install r"
  echo ""
  echo "R is optional — benchmarks will run without it (BioLang vs Python only)."
  exit 1
fi

R_VERSION=$(Rscript --version 2>&1 | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1 || echo "unknown")
echo "Found R $R_VERSION"
echo ""

# Check for required system libraries (Ubuntu/Debian)
if command -v dpkg &>/dev/null; then
  MISSING_DEPS=""
  for pkg in r-base-dev libcurl4-openssl-dev libssl-dev libxml2-dev zlib1g-dev; do
    if ! dpkg -s "$pkg" &>/dev/null; then
      MISSING_DEPS="$MISSING_DEPS $pkg"
    fi
  done
  if [[ -n "$MISSING_DEPS" ]]; then
    echo "WARNING: Missing system packages needed to compile R packages:"
    echo "  $MISSING_DEPS"
    echo ""
    echo "Install them with:"
    echo "  sudo apt install$MISSING_DEPS"
    echo ""
    read -rp "Continue anyway? [y/N] " yn
    case $yn in
      [Yy]*) ;;
      *) echo "Aborted."; exit 1 ;;
    esac
  fi
fi

# Install BiocManager + Bioconductor packages
echo "Installing R packages (this may take several minutes on first run) ..."
echo ""

# Step 1: Install BiocManager first (required for Bioconductor packages)
echo "Step 1/3: Installing BiocManager ..."
Rscript -e '
  if (!requireNamespace("BiocManager", quietly = TRUE)) {
    cat("Installing BiocManager from CRAN ...\n")
    install.packages("BiocManager", repos = "https://cloud.r-project.org")
  } else {
    cat("BiocManager: already installed\n")
  }
  # Verify it loaded
  if (!requireNamespace("BiocManager", quietly = TRUE)) {
    stop("FATAL: BiocManager failed to install. Check R output above for errors.")
  }
  cat(sprintf("BiocManager %s ready\n", as.character(packageVersion("BiocManager"))))
'

# Step 2: Install CRAN packages
echo ""
echo "Step 2/3: Installing CRAN packages ..."
Rscript -e '
  if (!requireNamespace("dplyr", quietly = TRUE)) {
    cat("Installing dplyr ...\n")
    install.packages("dplyr", repos = "https://cloud.r-project.org")
  } else {
    cat("dplyr: already installed\n")
  }
'

# Step 3: Install Bioconductor packages (one at a time for better error reporting)
echo ""
echo "Step 3/3: Installing Bioconductor packages ..."
Rscript -e '
  bioc_pkgs <- c("Biostrings", "ShortRead", "VariantAnnotation", "GenomicRanges")
  failed <- character(0)
  for (pkg in bioc_pkgs) {
    if (!requireNamespace(pkg, quietly = TRUE)) {
      cat(sprintf("Installing %s (this may take a few minutes) ...\n", pkg))
      tryCatch({
        BiocManager::install(pkg, ask = FALSE, update = FALSE)
      }, error = function(e) {
        cat(sprintf("  ERROR installing %s: %s\n", pkg, conditionMessage(e)))
      })
      if (!requireNamespace(pkg, quietly = TRUE)) {
        cat(sprintf("  FAILED: %s did not install successfully\n", pkg))
        failed <- c(failed, pkg)
      }
    } else {
      cat(sprintf("%s: already installed\n", pkg))
    }
  }

  cat("\n=== R setup complete ===\n")
  cat("Package status:\n")
  for (pkg in bioc_pkgs) {
    v <- tryCatch(as.character(packageVersion(pkg)), error = function(e) "NOT INSTALLED")
    cat(sprintf("  %s: %s\n", pkg, v))
  }
  v <- tryCatch(as.character(packageVersion("dplyr")), error = function(e) "NOT INSTALLED")
  cat(sprintf("  dplyr: %s\n", v))

  if (length(failed) > 0) {
    cat(sprintf("\nWARNING: %d package(s) failed to install: %s\n", length(failed), paste(failed, collapse=", ")))
    cat("Common fixes:\n")
    cat("  Ubuntu/Debian: sudo apt install r-base-dev libcurl4-openssl-dev libssl-dev libxml2-dev zlib1g-dev\n")
    cat("  Then re-run this script.\n")
    quit(status = 1)
  }
'
