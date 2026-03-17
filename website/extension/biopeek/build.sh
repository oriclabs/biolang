#!/bin/bash
# BioPeek — Build extension packages for each browser
# Usage: ./build.sh [chrome|firefox|edge|all]

set -e
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
VERSION=$(grep '"version"' "$SCRIPT_DIR/chrome/manifest.json" | head -1 | grep -oP '\d+\.\d+\.\d+')
BUILD_DIR="$SCRIPT_DIR/dist"
WEBSITE_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Sync shared files from website source
sync_shared() {
  echo "Syncing shared files from website..."
  cp "$WEBSITE_DIR/js/viewer.js" "$SCRIPT_DIR/shared/viewer.js"
  if [ -d "$WEBSITE_DIR/wasm" ]; then
    mkdir -p "$SCRIPT_DIR/shared/wasm"
    cp "$WEBSITE_DIR/wasm"/bl_wasm* "$SCRIPT_DIR/shared/wasm/" 2>/dev/null || true
  fi
}

build_browser() {
  local browser="$1"
  local out="$BUILD_DIR/$browser"
  echo "Building BioPeek v$VERSION for $browser..."

  rm -rf "$out"
  mkdir -p "$out"

  # Copy shared files
  cp "$SCRIPT_DIR/shared/viewer.html" "$out/"
  cp "$SCRIPT_DIR/shared/viewer.js" "$out/"
  cp "$SCRIPT_DIR/shared/popup.html" "$out/"
  cp "$SCRIPT_DIR/shared/popup.js" "$out/"
  cp "$SCRIPT_DIR/shared/help.html" "$out/"
  cp "$SCRIPT_DIR/shared/ext-loader.js" "$out/"
  cp "$SCRIPT_DIR/shared/theme-init.js" "$out/"
  cp "$SCRIPT_DIR/shared/wasm-loader.js" "$out/"
  cp -r "$SCRIPT_DIR/shared/icons" "$out/"
  [ -d "$SCRIPT_DIR/shared/screenshots" ] && cp -r "$SCRIPT_DIR/shared/screenshots" "$out/"
  [ -d "$SCRIPT_DIR/shared/wasm" ] && cp -r "$SCRIPT_DIR/shared/wasm" "$out/"

  # Copy browser-specific files (edge uses chrome's)
  local src_browser="$browser"
  [ "$browser" = "edge" ] && src_browser="chrome"
  cp "$SCRIPT_DIR/$src_browser/manifest.json" "$out/"
  cp "$SCRIPT_DIR/$src_browser/background.js" "$out/"

  # Create zip
  local zipname="biopeek-${browser}-v${VERSION}.zip"
  (cd "$out" && zip -r "$BUILD_DIR/$zipname" . -x "*.DS_Store" -q)
  local size=$(du -h "$BUILD_DIR/$zipname" | cut -f1)
  echo "  Created: $BUILD_DIR/$zipname ($size)"
}

sync_shared
mkdir -p "$BUILD_DIR"

case "${1:-all}" in
  chrome)  build_browser chrome ;;
  firefox) build_browser firefox ;;
  edge)    build_browser edge ;;
  all)
    build_browser chrome
    build_browser firefox
    build_browser edge
    echo ""
    echo "All builds:"
    ls -la "$BUILD_DIR"/*.zip 2>/dev/null
    ;;
  *) echo "Usage: $0 [chrome|firefox|edge|all]" ;;
esac
