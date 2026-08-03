# BioLang Desktop

BioLang Desktop is the local-first Tauri workbench for BioLang projects. It is
being kept in this repository while the integration contracts stabilize, but
is designed to move to an independent `biolang-desktop` repository.

## Prerequisites

- Node.js 20 or newer
- Rust stable with Cargo
- Platform prerequisites for Tauri 2
- A built or installed BioLang CLI for native execution and language services

## Run

Install dependencies once:

```powershell
npm install
```

Run the browser workbench:

```powershell
npm run dev
```

Open <http://127.0.0.1:1420>. Browser mode stores its workspace in IndexedDB,
runs supported BioLang code through WebAssembly, and can connect to SOMER.
Native terminals, Git, LSP, package installation, and unrestricted filesystem
access remain Desktop capabilities.

Browser API calls use the same BioLang network functions through the WASM fetch
bridge. Providers must permit browser CORS; APIs that do not can be executed
through Desktop or a configured SOMER target.

Build and test the installable PWA:

```powershell
npm run build
npm run preview -- --host 127.0.0.1
```

Open the preview URL in Chrome or Edge and use the install action in the title
bar or browser address bar. The production build precaches the application,
offline help, editor bundles, and BioLang WASM runtime.

Run the native desktop application:

```powershell
npm run tauri dev
```

The native application uses Tauri IPC and does not require a localhost service
after it is packaged. Open a folder, then explicitly trust it before running
BioLang, packages, terminals, or language services.

## BioLang Discovery

Desktop looks for `bl` and `bl-lsp` in nearby Cargo target directories and on
`PATH`. Set `BIOLANG_BIN` to select a specific BioLang executable:

```powershell
$env:BIOLANG_BIN = "C:\path\to\bl.exe"
npm run tauri dev
```

Help generation uses the current repository layout. An extracted standalone
checkout can point to a BioLang source tree with `BIOLANG_SOURCE_ROOT`.
`bl metadata --format json` is the authoritative builtin inventory. The generator
updates `src/generated/builtin-metadata.json`; a standalone frontend build can
reuse that checked-in snapshot when the CLI is unavailable.

## Workspace Formats

- `.bl` opens in Monaco and can switch to a read-only pipeline graph.
- `.bln` and `.bl.md` open as executable literate notebooks.
- `.blflow` opens in the typed workflow composer and generates BioLang for local
  or SOMER execution.
- Common biological, structural, tabular, image, SVG, and PDF files open in
  bounded viewers instead of the text editor.

Imported datasets are copied into `data/`. Import provenance and SHA-256 hashes
are recorded in `.biolang/imports.json`.

## Results and Reproducibility

Output is stored per run rather than as one shared terminal buffer. Named typed
results drive tables, plots, semantic run comparison, and artifact views. Large
tables and matrices keep an inline preview and stream complete JSONL rows to a
paged result artifact. SOMER and native Desktop fetch only the requested page.

Desktop persists run history in SQLite and automatically refreshes configured
SOMER histories. Reproducibility bundles include source, logs, typed results,
artifacts, input and environment checksums, runtime metadata, and replay
instructions. Artifact previews use byte ranges for large binary scientific
formats instead of downloading the complete file.

## Verify

```powershell
npm run build
npm test
Set-Location src-tauri
cargo test
```

See [REQUIREMENTS_STATUS.md](REQUIREMENTS_STATUS.md) for implemented and pending
scope, [STANDALONE.md](STANDALONE.md) for repository boundaries, and
[REQUIREMENTS.md](REQUIREMENTS.md) for the full product specification.
