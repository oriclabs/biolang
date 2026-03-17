#!/bin/bash
# BioPeek — Dev mode: copy shared files into chrome/ for "Load unpacked" testing
# Usage: ./dev.sh

set -e
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
WEBSITE_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "Syncing website viewer.js → shared..."
cp "$WEBSITE_DIR/js/viewer.js" "$SCRIPT_DIR/shared/viewer.js"

echo "Syncing shared → chrome/ for dev testing..."
cp "$SCRIPT_DIR/shared/"*.html "$SCRIPT_DIR/chrome/"
cp "$SCRIPT_DIR/shared/"*.js "$SCRIPT_DIR/chrome/"
cp -r "$SCRIPT_DIR/shared/icons" "$SCRIPT_DIR/chrome/"
[ -d "$SCRIPT_DIR/shared/screenshots" ] && cp -r "$SCRIPT_DIR/shared/screenshots" "$SCRIPT_DIR/chrome/"
[ -d "$SCRIPT_DIR/shared/wasm" ] && cp -r "$SCRIPT_DIR/shared/wasm" "$SCRIPT_DIR/chrome/"

echo "Done! Load unpacked from: $SCRIPT_DIR/chrome/"
