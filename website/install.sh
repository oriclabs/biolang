#!/bin/sh
# BioLang installer — https://lang.bio
# Usage: curl -fsSL https://lang.bio/install.sh | sh
#    or: curl -fsSL https://raw.githubusercontent.com/oriclabs/biolang/main/website/install.sh | sh
set -e

REPO="oriclabs/biolang"
INSTALL_DIR="${BIOLANG_INSTALL_DIR:-/usr/local/bin}"

main() {
    need_cmd curl
    need_cmd tar

    # Detect OS and architecture
    OS="$(uname -s)"
    ARCH="$(uname -m)"

    case "$OS" in
        Linux)  PLATFORM="linux" ;;
        Darwin) PLATFORM="macos" ;;
        *)      err "Unsupported OS: $OS. BioLang supports Linux and macOS. For Windows, download from GitHub releases." ;;
    esac

    case "$ARCH" in
        x86_64|amd64)   ARCH_NAME="x86_64" ;;
        aarch64|arm64)  ARCH_NAME="aarch64" ;;
        *)              err "Unsupported architecture: $ARCH" ;;
    esac

    # Linux aarch64 not yet in release matrix
    if [ "$PLATFORM" = "linux" ] && [ "$ARCH_NAME" = "aarch64" ]; then
        err "Linux aarch64 builds are not yet available. Please build from source: https://github.com/$REPO"
    fi

    ARCHIVE="biolang-${PLATFORM}-${ARCH_NAME}.tar.gz"

    # Get latest release tag
    say "Detecting latest BioLang release..."
    LATEST=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": *"//;s/".*//')
    if [ -z "$LATEST" ]; then
        err "Could not determine latest release. Check https://github.com/$REPO/releases"
    fi
    say "Latest release: $LATEST"

    # Download
    URL="https://github.com/$REPO/releases/download/${LATEST}/${ARCHIVE}"
    TMPDIR=$(mktemp -d)
    trap 'rm -rf "$TMPDIR"' EXIT

    say "Downloading $ARCHIVE..."
    curl -fSL --progress-bar "$URL" -o "$TMPDIR/$ARCHIVE" || err "Download failed. URL: $URL"

    # Extract
    say "Extracting..."
    tar xzf "$TMPDIR/$ARCHIVE" -C "$TMPDIR"

    # Install
    if [ -w "$INSTALL_DIR" ]; then
        cp "$TMPDIR/bl" "$INSTALL_DIR/bl"
        chmod +x "$INSTALL_DIR/bl"
        if [ -f "$TMPDIR/bl-lsp" ]; then
            cp "$TMPDIR/bl-lsp" "$INSTALL_DIR/bl-lsp"
            chmod +x "$INSTALL_DIR/bl-lsp"
        fi
    else
        say "Installing to $INSTALL_DIR (requires sudo)..."
        sudo cp "$TMPDIR/bl" "$INSTALL_DIR/bl"
        sudo chmod +x "$INSTALL_DIR/bl"
        if [ -f "$TMPDIR/bl-lsp" ]; then
            sudo cp "$TMPDIR/bl-lsp" "$INSTALL_DIR/bl-lsp"
            sudo chmod +x "$INSTALL_DIR/bl-lsp"
        fi
    fi

    # Verify
    if command -v bl >/dev/null 2>&1; then
        say ""
        say "BioLang installed successfully!"
        say "  bl:     $(bl version 2>&1 | head -1)"
        if command -v bl-lsp >/dev/null 2>&1; then
            say "  bl-lsp: installed"
        fi
        say ""
        say "Get started:"
        say "  bl repl          # interactive REPL"
        say "  bl run script.bl # run a script"
        say "  bl --help        # all commands"
        say ""
        say "Documentation: https://lang.bio"
    else
        say ""
        say "BioLang binaries installed to $INSTALL_DIR"
        say "Make sure $INSTALL_DIR is in your PATH."
        say ""
        say "  export PATH=\"$INSTALL_DIR:\$PATH\""
        say ""
    fi
}

say() {
    printf '%s\n' "$1"
}

err() {
    say "error: $1" >&2
    exit 1
}

need_cmd() {
    if ! command -v "$1" >/dev/null 2>&1; then
        err "need '$1' (not found in PATH)"
    fi
}

main "$@"
