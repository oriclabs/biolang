#!/usr/bin/env bash
# Publish BioLang crates to crates.io in dependency order.
# Run from the workspace root: ./scripts/publish.sh
#
# Prerequisites:
#   cargo login <your-crates-io-token>
#
# Use --dry-run to test without publishing:
#   ./scripts/publish.sh --dry-run

set -euo pipefail

DRY_RUN=""
if [[ "${1:-}" == "--dry-run" ]]; then
  DRY_RUN="--dry-run"
  echo "=== DRY RUN ==="
fi

DELAY=30  # crates.io needs time to index each crate

# Publish order follows the dependency chain
CRATES=(
  bio-core
  bl-core
  bl-lexer
  bl-parser
  bl-compiler
  bl-jit
  bl-bio
  bl-apis
  bl-runtime
  bl-repl
  bl-lsp
  bl-cli
)

for crate in "${CRATES[@]}"; do
  echo ""
  echo ">>> Publishing $crate..."
  cargo publish -p "$crate" $DRY_RUN

  if [[ -z "$DRY_RUN" && "$crate" != "${CRATES[-1]}" ]]; then
    echo "    Waiting ${DELAY}s for crates.io to index..."
    sleep "$DELAY"
  fi
done

echo ""
echo "=== All crates published! ==="
echo "Users can now: cargo install bl-cli"
