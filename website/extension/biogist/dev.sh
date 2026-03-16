#!/bin/bash
# BioGist — Dev mode: copy shared files into chrome/ for "Load unpacked" testing
# Usage: ./dev.sh

set -e
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "Syncing shared → chrome/ for dev testing..."
cp "$SCRIPT_DIR/shared/biogist-core.js" "$SCRIPT_DIR/chrome/"
cp "$SCRIPT_DIR/shared/hgnc-symbols.js" "$SCRIPT_DIR/chrome/"
cp "$SCRIPT_DIR/shared/content.js" "$SCRIPT_DIR/chrome/"
cp "$SCRIPT_DIR/shared/sidebar.html" "$SCRIPT_DIR/chrome/"
cp "$SCRIPT_DIR/shared/sidebar.js" "$SCRIPT_DIR/chrome/"
cp "$SCRIPT_DIR/shared/help.html" "$SCRIPT_DIR/chrome/"
cp -r "$SCRIPT_DIR/shared/icons" "$SCRIPT_DIR/chrome/"
cp -r "$SCRIPT_DIR/shared/screenshots" "$SCRIPT_DIR/chrome/"

echo "Done! Load unpacked from: $SCRIPT_DIR/chrome/"
echo "  chrome://extensions → Load unpacked → select chrome/"
