#!/bin/bash
# BioGist — Build extension packages for each browser
# Usage: ./build.sh [chrome|firefox|edge|all]

set -e
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
VERSION=$(grep '"version"' "$SCRIPT_DIR/chrome/manifest.json" | head -1 | grep -oP '\d+\.\d+\.\d+')
BUILD_DIR="$SCRIPT_DIR/dist"

build_browser() {
  local browser="$1"
  local out="$BUILD_DIR/$browser"
  echo "Building BioGist v$VERSION for $browser..."

  rm -rf "$out"
  mkdir -p "$out"

  # Copy shared files
  cp "$SCRIPT_DIR/shared/content.js" "$out/"
  cp "$SCRIPT_DIR/shared/sidebar.html" "$out/"
  cp "$SCRIPT_DIR/shared/sidebar.js" "$out/"
  cp "$SCRIPT_DIR/shared/help.html" "$out/"
  cp -r "$SCRIPT_DIR/shared/icons" "$out/"
  cp -r "$SCRIPT_DIR/shared/screenshots" "$out/"

  # Copy browser-specific files
  cp "$SCRIPT_DIR/$browser/manifest.json" "$out/"
  cp "$SCRIPT_DIR/$browser/background.js" "$out/"

  # Edge uses Chrome's files
  if [ "$browser" = "edge" ]; then
    cp "$SCRIPT_DIR/chrome/manifest.json" "$out/"
    cp "$SCRIPT_DIR/chrome/background.js" "$out/"
  fi

  # Create zip
  local zipname="biogist-${browser}-v${VERSION}.zip"
  (cd "$out" && zip -r "$BUILD_DIR/$zipname" . -x "*.DS_Store")
  echo "  Created: $BUILD_DIR/$zipname"
}

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
    echo "All builds complete:"
    ls -la "$BUILD_DIR"/*.zip 2>/dev/null
    ;;
  *) echo "Usage: $0 [chrome|firefox|edge|all]" ;;
esac
