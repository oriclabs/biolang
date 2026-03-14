$ErrorActionPreference = "Stop"

$BookDir = Join-Path $PSScriptRoot "book"
$OutDir = Join-Path (Split-Path $PSScriptRoot -Parent | Split-Path -Parent) "website\practical-bioinformatics"

Write-Host "=== Building Practical Bioinformatics in 30 Days ===" -ForegroundColor Cyan

# Check tools
if (-not (Get-Command mdbook -ErrorAction SilentlyContinue)) {
    Write-Host "Error: mdbook not found. Install: cargo install mdbook" -ForegroundColor Red
    exit 1
}

# Install preprocessors if missing
if (-not (Get-Command mdbook-mermaid -ErrorAction SilentlyContinue)) {
    Write-Host "Installing mdbook-mermaid..."
    cargo install mdbook-mermaid
}

# Install mermaid assets
Push-Location $BookDir
mdbook-mermaid install .
Pop-Location

# Build HTML
Write-Host "Building HTML..." -ForegroundColor Yellow
mdbook build $BookDir
Write-Host "HTML output: $OutDir" -ForegroundColor Green

# Build PDF if mdbook-pdf is available
if (Get-Command mdbook-pdf -ErrorAction SilentlyContinue) {
    Write-Host "Building PDF..." -ForegroundColor Yellow
    $PdfFile = Join-Path $OutDir "pdf\practical-bioinformatics-in-30-days.pdf"
    if (Test-Path $PdfFile) {
        Copy-Item $PdfFile (Join-Path $OutDir "practical-bioinformatics-in-30-days.pdf")
        Write-Host "PDF output: $OutDir\practical-bioinformatics-in-30-days.pdf" -ForegroundColor Green
    }
} else {
    Write-Host "Skipping PDF (install: cargo install mdbook-pdf)" -ForegroundColor Yellow
}

Write-Host "=== Build complete ===" -ForegroundColor Cyan
