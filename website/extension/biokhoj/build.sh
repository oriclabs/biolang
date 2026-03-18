#!/bin/bash
# BioKhoj — Build distributable ZIPs for Chrome, Edge, and Firefox
# Run from: website/extension/biokhoj/

set -e
VERSION="1.0.0"
DIST_DIR="dist"

rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"

echo "Building BioKhoj v$VERSION..."

# ── Chrome / Edge (same package, MV3) ──
echo "==> Chrome / Edge"
cd chrome
zip -r "../$DIST_DIR/biokhoj-chrome-v$VERSION.zip" \
  manifest.json background.js sidebar.html sidebar.js help.html \
  icons/ \
  -x "*.DS_Store" -x "__MACOSX/*"
cd ..
echo "    Created $DIST_DIR/biokhoj-chrome-v$VERSION.zip"

# Edge uses the same Chrome MV3 package
cp "$DIST_DIR/biokhoj-chrome-v$VERSION.zip" "$DIST_DIR/biokhoj-edge-v$VERSION.zip"
echo "    Created $DIST_DIR/biokhoj-edge-v$VERSION.zip (same as Chrome)"

# ── Firefox (MV3 with gecko settings) ──
echo "==> Firefox"
cd firefox
zip -r "../$DIST_DIR/biokhoj-firefox-v$VERSION.zip" \
  manifest.json background.js sidebar.html sidebar.js help.html \
  icons/ \
  -x "*.DS_Store" -x "__MACOSX/*"
cd ..
echo "    Created $DIST_DIR/biokhoj-firefox-v$VERSION.zip"

echo ""
echo "Done. Packages in $DIST_DIR/:"
ls -lh "$DIST_DIR/"
echo ""
echo "Submission targets:"
echo "  Chrome:  https://chrome.google.com/webstore/devconsole"
echo "  Edge:    https://partner.microsoft.com/en-us/dashboard/microsoftedge"
echo "  Firefox: https://addons.mozilla.org/en-US/developers/"
