#!/usr/bin/env bash
# Install Python dependencies for BioLang benchmarks
# Usage: ./setup_python.sh
#
# Requires: python3 (3.10+) and pip

set -euo pipefail

echo "=== BioLang Benchmark — Python Setup ==="

# Check python3
if ! command -v python3 &>/dev/null; then
  echo "ERROR: python3 not found. Install Python 3.10+ first."
  echo "  Ubuntu/Debian: sudo apt install python3 python3-pip python3-venv"
  echo "  macOS:         brew install python3"
  exit 1
fi

PY_VERSION=$(python3 -c 'import sys; print(f"{sys.version_info.major}.{sys.version_info.minor}")')
echo "Found Python $PY_VERSION"

# Create venv if not active
VENV_DIR="$(dirname "$0")/.venv"
if [[ -z "${VIRTUAL_ENV:-}" ]]; then
  if [[ ! -d "$VENV_DIR" ]]; then
    echo "Creating virtual environment at $VENV_DIR ..."
    python3 -m venv "$VENV_DIR"
  fi
  echo "Activating virtual environment ..."
  source "$VENV_DIR/bin/activate"
else
  echo "Using active virtual environment: $VIRTUAL_ENV"
fi

# Install packages
echo ""
echo "Installing Python packages ..."
pip install --upgrade pip -q

# BioPython — FASTA/FASTQ/sequence parsing (used by most benchmarks)
pip install biopython -q

echo ""
echo "=== Python setup complete ==="
echo ""
echo "Installed packages:"
pip list 2>/dev/null | grep -iE 'biopython'
echo ""
echo "To activate the venv in future sessions:"
echo "  source $VENV_DIR/bin/activate"
