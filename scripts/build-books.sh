#!/usr/bin/env bash
# Build all mdBook books (HTML + PDF) and copy PDFs to website for download.
# Prerequisites: mdbook, mdbook-pdf (cargo install mdbook-pdf)
# Chrome/Chromium must be available for PDF generation.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(dirname "$SCRIPT_DIR")"

# Helper: build a single book
# Usage: build_book <src_dir> <website_subdir> <pdf_filename> <title>
build_book() {
    local src_dir="$1"
    local web_dir="$2"
    local pdf_name="$3"
    local title="$4"

    echo "=== Building $title ==="
    cd "$ROOT/books/$src_dir"
    mdbook build

    # Flatten HTML subdirectory if mdbook created it (multi-backend mode)
    if [ -d "$ROOT/website/$web_dir/html" ]; then
        cp -r "$ROOT/website/$web_dir/html/"* "$ROOT/website/$web_dir/"
        rm -rf "$ROOT/website/$web_dir/html"
    fi
    # Move PDF to a clean filename
    if [ -f "$ROOT/website/$web_dir/pdf/output.pdf" ]; then
        mv "$ROOT/website/$web_dir/pdf/output.pdf" "$ROOT/website/$web_dir/$pdf_name"
        rm -rf "$ROOT/website/$web_dir/pdf"
        echo "  PDF: website/$web_dir/$pdf_name"
    else
        echo "  WARNING: PDF not generated (is Chrome/Chromium installed?)"
    fi
    echo ""
}

# Build all books (source dirs are under books/)
build_book "language"               "book"                   "the-biolang-book.pdf"                        "The BioLang Programming Language"
build_book "practice/book"          "practice"               "bioinformatics-in-practice.pdf"               "Bioinformatics in Practice"
build_book "biostatistics/book"     "biostatistics"          "biostatistics-in-practice.pdf"                "Biostatistics in Practice"
build_book "funcgenomics/book"      "funcgenomics"           "functional-genomics-in-practice.pdf"          "Functional Genomics in Practice"
build_book "clinical-genomics/book" "clinical-genomics"      "clinical-genomics-in-practice.pdf"            "Clinical Genomics in Practice"
build_book "sequence-analysis/book" "sequence-analysis"      "sequence-analysis-in-practice.pdf"            "Sequence Analysis in Practice"
build_book "data-wrangling/book"    "data-wrangling"         "data-wrangling-for-biologists.pdf"            "Data Wrangling for Biologists"
build_book "genomic-visualization/book" "genomic-visualization" "genomic-visualization-in-practice.pdf"     "Genomic Visualization in Practice"
build_book "proteomics-structural/book" "proteomics-structural" "proteomics-structural-biology-in-practice.pdf" "Proteomics & Structural Biology in Practice"
build_book "reproducible-pipelines/book" "reproducible-pipelines" "reproducible-pipelines-in-practice.pdf"  "Reproducible Pipelines in Practice"

echo "=== Done ==="
echo "All books built. Check website/ for HTML and PDF outputs."
