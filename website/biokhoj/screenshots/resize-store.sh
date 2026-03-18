#!/bin/bash
# Resize store screenshots to Chrome Web Store dimensions (1280x800)
# Requires: ImageMagick (convert command)
# Run from: biokhoj/screenshots/

STORE_DIR="store"
OUTPUT_DIR="store-submit"

mkdir -p "$OUTPUT_DIR"

for f in "$STORE_DIR"/*.png; do
  name=$(basename "$f")
  echo "Resizing $name → $OUTPUT_DIR/$name"
  convert "$f" -resize 1280x800 -background '#020617' -gravity center -extent 1280x800 "$OUTPUT_DIR/$name"
done

echo "Done. Store-ready screenshots in $OUTPUT_DIR/"
echo ""
echo "Chrome Web Store requirements:"
echo "  - 1280x800 or 640x400 PNG/JPEG"
echo "  - Min 1, max 5 screenshots"
echo "  - No alpha channel in some cases (use -flatten if needed)"
