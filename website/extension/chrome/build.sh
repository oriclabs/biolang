#!/bin/bash
# BLViewer Chrome Extension — Build & Package Script
# Usage:
#   ./build.sh          Build only (sync files, load unpacked)
#   ./build.sh --zip    Build + create .zip for Chrome Web Store upload

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
WEBSITE_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
VERSION=$(grep '"version"' "$SCRIPT_DIR/manifest.json" | head -1 | sed 's/.*: *"\(.*\)".*/\1/')

echo ""
echo "  BLViewer Chrome Extension v$VERSION"
echo "  ================================="
echo ""

# ── Step 1: Sync viewer.js ─────────────────────────────────────────
echo "[1/4] Syncing viewer.js..."
cp "$WEBSITE_DIR/js/viewer.js" "$SCRIPT_DIR/viewer.js"
echo "  OK  viewer.js"

# ── Step 2: Sync WASM files ────────────────────────────────────────
echo "[2/4] Syncing WASM files..."
if [ -d "$WEBSITE_DIR/wasm" ]; then
  mkdir -p "$SCRIPT_DIR/wasm"
  cp "$WEBSITE_DIR/wasm/br_wasm.js" "$SCRIPT_DIR/wasm/" 2>/dev/null || true
  cp "$WEBSITE_DIR/wasm/br_wasm_bg.wasm" "$SCRIPT_DIR/wasm/" 2>/dev/null || true
  cp "$WEBSITE_DIR/wasm/br_wasm_bg.wasm.d.ts" "$SCRIPT_DIR/wasm/" 2>/dev/null || true
  cp "$WEBSITE_DIR/wasm/br_wasm.d.ts" "$SCRIPT_DIR/wasm/" 2>/dev/null || true
  echo "  OK  wasm/"
else
  echo "  SKIP  wasm/ (not found)"
fi

# ── Step 3: Sync screenshots ──────────────────────────────────────
echo "[3/4] Syncing screenshots..."
SCREENSHOT_SRC="$WEBSITE_DIR/docs/tools/screenshots"
if [ -d "$SCREENSHOT_SRC" ]; then
  mkdir -p "$SCRIPT_DIR/screenshots"
  cp "$SCREENSHOT_SRC"/*.png "$SCRIPT_DIR/screenshots/" 2>/dev/null || true
  COUNT=$(ls "$SCRIPT_DIR/screenshots/"*.png 2>/dev/null | wc -l)
  echo "  OK  screenshots/ ($COUNT images)"
else
  echo "  SKIP  screenshots/ (not found)"
fi

# ── Step 4: Validate ──────────────────────────────────────────────
echo "[4/4] Validating..."

REQUIRED=(
  "manifest.json" "background.js" "popup.html" "popup.js"
  "viewer.html" "viewer.js" "help.html"
  "theme-init.js" "ext-loader.js" "wasm-loader.js"
  "icons/icon16.png" "icons/icon48.png" "icons/icon128.png"
)
MISSING=0
for f in "${REQUIRED[@]}"; do
  if [ ! -f "$SCRIPT_DIR/$f" ]; then
    echo "  WARN  Missing: $f"
    MISSING=1
  fi
done
[ $MISSING -eq 0 ] && echo "  OK  All required files present"

# Check for inline scripts
if grep -qP '<script>[^<]' "$SCRIPT_DIR"/*.html 2>/dev/null; then
  echo "  WARN  Inline scripts found (CSP will block these):"
  grep -n '<script>[^<]' "$SCRIPT_DIR"/*.html 2>/dev/null | sed 's/^/        /'
else
  echo "  OK  No inline scripts (CSP clean)"
fi

echo ""

# ── Zip for Chrome Web Store ──────────────────────────────────────
if [ "$1" = "--zip" ]; then
  ZIP_NAME="blviewer-chrome-v$VERSION.zip"
  ZIP_PATH="$SCRIPT_DIR/$ZIP_NAME"

  rm -f "$ZIP_PATH"

  echo "Packaging $ZIP_NAME..."

  # Create temp staging directory
  STAGING=$(mktemp -d)
  trap "rm -rf $STAGING" EXIT

  # Copy files
  for f in manifest.json background.js popup.html popup.js viewer.html viewer.js help.html theme-init.js ext-loader.js wasm-loader.js; do
    cp "$SCRIPT_DIR/$f" "$STAGING/$f"
  done

  # Copy directories
  if [ -d "$SCRIPT_DIR/icons" ]; then
    mkdir -p "$STAGING/icons"
    cp "$SCRIPT_DIR/icons/"*.png "$STAGING/icons/"
  fi
  if [ -d "$SCRIPT_DIR/wasm" ]; then
    mkdir -p "$STAGING/wasm"
    cp "$SCRIPT_DIR/wasm/"* "$STAGING/wasm/"
  fi
  if [ -d "$SCRIPT_DIR/screenshots" ]; then
    mkdir -p "$STAGING/screenshots"
    cp "$SCRIPT_DIR/screenshots/"*.png "$STAGING/screenshots/"
  fi

  # Create zip
  (cd "$STAGING" && zip -r "$ZIP_PATH" . -q)

  SIZE=$(du -h "$ZIP_PATH" | cut -f1)
  echo ""
  echo "  Created: $ZIP_NAME ($SIZE)"
  echo "  Path:    $ZIP_PATH"
  echo ""
  echo "  Upload to Chrome Web Store:"
  echo "  https://chrome.google.com/webstore/devconsole"
  echo ""
else
  echo "Build complete! Load unpacked in Chrome:"
  echo "  1. Open chrome://extensions"
  echo "  2. Enable 'Developer mode'"
  echo "  3. Click 'Load unpacked'"
  echo "  4. Select: $SCRIPT_DIR"
  echo ""
  echo "To create a .zip for Chrome Web Store:"
  echo "  ./build.sh --zip"
  echo ""
fi
