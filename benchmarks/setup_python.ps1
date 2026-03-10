# BioLang Benchmark — Python Setup (Windows)
# Usage: powershell -ExecutionPolicy Bypass -File .\setup_python.ps1

$ErrorActionPreference = "Stop"

Write-Host "=== BioLang Benchmark — Python Setup ===" -ForegroundColor Cyan

# Check python
$pyCmd = Get-Command python -ErrorAction SilentlyContinue
if (-not $pyCmd) {
    Write-Host "ERROR: python not found." -ForegroundColor Red
    Write-Host "  Install Python 3.10+ from https://www.python.org/downloads/"
    Write-Host "  Make sure 'Add Python to PATH' is checked during install."
    exit 1
}

$pyVersion = & python --version 2>&1
Write-Host "Found $pyVersion"

# Create venv if not present
$venvDir = Join-Path $PSScriptRoot ".venv"
$activateScript = Join-Path $venvDir "Scripts\Activate.ps1"

if (-not (Test-Path $activateScript)) {
    Write-Host "Creating virtual environment at $venvDir ..."
    & python -m venv $venvDir
    if ($LASTEXITCODE -ne 0) {
        Write-Host "ERROR: Failed to create venv." -ForegroundColor Red
        exit 1
    }
}

Write-Host "Activating virtual environment ..."
& $activateScript

# Install packages
Write-Host ""
Write-Host "Installing Python packages ..."
& pip install --upgrade pip -q
& pip install biopython -q

Write-Host ""
Write-Host "=== Python setup complete ===" -ForegroundColor Green
Write-Host ""
Write-Host "Installed packages:"
& pip list 2>$null | Select-String -Pattern "biopython" -CaseSensitive:$false
Write-Host ""
Write-Host "To activate the venv in future sessions:"
Write-Host "  $activateScript"
Write-Host ""
Write-Host "Or run benchmarks directly (the runner auto-activates .venv):"
Write-Host "  powershell -ExecutionPolicy Bypass -File .\run_all.ps1"
