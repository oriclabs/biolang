#!/usr/bin/env bash
set -euo pipefail

BOOK_DIR="$(cd "$(dirname "$0")/book" && pwd)"
OUT_DIR="$(cd "$(dirname "$0")/../../website" && pwd)/practical-bioinformatics"

echo "=== Building Practical Bioinformatics in 30 Days ==="

# Check tools
command -v mdbook >/dev/null 2>&1 || { echo "Error: mdbook not found. Install: cargo install mdbook"; exit 1; }

# Install preprocessors if missing
command -v mdbook-mermaid >/dev/null 2>&1 || { echo "Installing mdbook-mermaid..."; cargo install mdbook-mermaid; }

# Install mermaid assets
cd "$BOOK_DIR"
mdbook-mermaid install .

# Build HTML
echo "Building HTML..."
mdbook build "$BOOK_DIR"
echo "HTML output: $OUT_DIR"

# Build PDF if mdbook-pdf is available
if command -v mdbook-pdf >/dev/null 2>&1; then
    echo "Building PDF..."
    # mdbook-pdf runs as a backend during mdbook build, so PDF is generated alongside HTML
    PDF_FILE="$OUT_DIR/pdf/practical-bioinformatics-in-30-days.pdf"
    if [ -f "$PDF_FILE" ]; then
        cp "$PDF_FILE" "$OUT_DIR/practical-bioinformatics-in-30-days.pdf"
        echo "PDF output: $OUT_DIR/practical-bioinformatics-in-30-days.pdf"
    fi
else
    echo "Skipping PDF (install: cargo install mdbook-pdf)"
fi

echo "=== Build complete ==="
