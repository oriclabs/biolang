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

    # A missing archive is the ordinary failure here, not a network fault: a
    # release may predate a platform being added to the build matrix. Say which
    # platform is missing rather than printing a bare URL. Linux aarch64 was in
    # exactly this state until it joined the matrix, and the hard-coded refusal
    # that used to live above stayed long after it needed to.
    say "Downloading $ARCHIVE..."
    if ! curl -fSL --progress-bar "$URL" -o "$TMPDIR/$ARCHIVE"; then
        err "No ${PLATFORM}-${ARCH_NAME} build in release ${LATEST}.
  Looked for: $URL
  Available builds are listed at https://github.com/$REPO/releases/tag/${LATEST}
  To build from source instead: cargo install --git https://github.com/$REPO bl-cli"
    fi

    # Verify against the checksums published with the release. This script is
    # piped from the internet straight into a shell, so confirming the archive
    # is the one that was published costs one request and is worth making.
    # The Windows installer did this from the start; parity here means the docs
    # can say both verify without qualifying which.
    SUMS_URL="https://github.com/$REPO/releases/download/${LATEST}/checksums.sha256"
    if curl -fsSL "$SUMS_URL" -o "$TMPDIR/checksums.sha256" 2>/dev/null; then
        # Releases up to v1.1.0 recorded artifacts/<name>/<name> rather than a
        # bare filename, so match on the trailing component either way.
        EXPECTED=$(awk -v a="$ARCHIVE" '$2 ~ ("(^|/)" a "$") { print $1; exit }' "$TMPDIR/checksums.sha256")
        if [ -z "$EXPECTED" ]; then
            say "note: $ARCHIVE is not listed in checksums.sha256; skipping verification."
        else
            if command -v sha256sum >/dev/null 2>&1; then
                ACTUAL=$(sha256sum "$TMPDIR/$ARCHIVE" | cut -d' ' -f1)
            elif command -v shasum >/dev/null 2>&1; then
                ACTUAL=$(shasum -a 256 "$TMPDIR/$ARCHIVE" | cut -d' ' -f1)
            else
                ACTUAL=""
                say "note: no sha256sum or shasum available; skipping verification."
            fi
            if [ -n "$ACTUAL" ]; then
                if [ "$EXPECTED" != "$ACTUAL" ]; then
                    err "checksum mismatch for $ARCHIVE
  expected $EXPECTED
  actual   $ACTUAL"
                fi
                say "Checksum verified."
            fi
        fi
    else
        say "note: checksums.sha256 not published for ${LATEST}; skipping verification."
    fi

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
