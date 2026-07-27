# Desktop Requirements Status

This ledger maps `REQUIREMENTS.md` to the current implementation.

## Implemented

- Tauri 2 native shell with React, Monaco, xterm.js, and Rust IPC.
- Workspace selection and bounded file tree.
- Create, rename, delete, read, and save within canonicalized workspace paths.
- Tabbed editing, syntax highlighting, Monaco editing commands, and breadcrumbs.
- BioLang LSP startup, completion, hover, diagnostics, and Problems panel.
- Local BioLang process execution, streaming output, cancellation, and job history.
- SOMER connection profiles, resource selection, remote execution, cancellation,
  completed-job history synchronization, and reopenable logs.
- SOMER bearer tokens stored in the operating-system credential store, plus
  direct, HTTP-gateway, and supervised SSH-tunnel connection modes.
- Native PTY terminal.
- Project dependency listing and installation through the BioLang CLI.
- Generated offline help covering language, builtins, tutorials, and examples.
- Versioned `bl metadata --format json` export covering all 744 registered
  builtins, including authoritative runtime arity and curated details where available.
- Help generation and the function browser consume the generated metadata snapshot
  instead of maintaining a separate handwritten builtin list.
- Navigable internal Help links, correctly labeled copyable code blocks, and
  generated-index integrity validation.
- Bio API reference surface and insertion into source.
- Command palette, menus, shortcuts, settings, and compact-layout validation.
- Workspace trust enforced in both the UI and native Rust command boundary.
- Recent workspaces and bounded unsaved-buffer session recovery.
- Native, bounded workspace content search with source navigation.
- Bounded FASTA, FASTQ, VCF, BED, GFF/GTF, SAM, Newick, PDB/mmCIF, image,
  SVG, PDF, CSV, TSV, JSON, and text previews.
- Sortable/filterable table previews, numeric heatmaps, structure projections,
  multi-record FASTA navigation, motif search, sequence copy and
  reverse-complement inspection, and preview export.
- Native data import into the workspace with a SHA-256 provenance ledger.
- Typed, bounded JSON Lines execution events from the BioLang CLI.
- Literate `.bln` and `.bl.md` notebook editing, persistent-state run-all
  execution, single-cell execution, and inline text/TSV cell results.
- Read-only callable pipeline graphs for ordinary `.bl` source.
- Versioned `.blflow` DAG authoring with validated multi-input nodes,
  topological generation, branching/merging, scatter/gather modes, named
  parameter editing, local execution, cancellation, and remote source conversion.
- Duplicate and reveal-in-file-manager Explorer actions.
- Keyboard-accessible, viewport-safe context menus for workspace, Explorer,
  and editor-tab actions.
- Multiple persistent terminal tabs.
- Rerun completed jobs on their recorded target and preserve disconnected history.
- Configurable tab width and editor theme.
- Standalone source-root and runtime integration boundaries.
- Dedicated React managers for jobs/SOMER polling, LSP document lifecycle,
  workspace state/trust, and bounded session recovery.
- Git branch and modified/staged/untracked decorations in the Explorer, refreshed
  after workspace activation, manual refresh, and saves.
- Extracted Explorer tree and settings dialog components to reduce the main
  workbench component's ownership surface.

## Verification

- TypeScript production build passes.
- Playwright workbench workflows pass in Chromium.
- Rust host compilation and native preview/workflow-generator tests pass.

## Blocked by BioLang Capabilities

- Document formatting: the current `bl 0.3.0` CLI has no formatter command.
- Rename, references, symbols, and signature help require corresponding `bl lsp`
  methods before Desktop can expose reliable behavior.
- Registry package search, versions, licenses, trust metadata, updates, and
  lockfile previews require structured CLI/package-registry APIs.

## Later Desktop Work

- Split the remaining `App.tsx` rendering, menu construction, Explorer actions,
  and help/package UI coordination into focused components.
- Reopen remote artifacts and resume monitoring active SOMER jobs after restart.
- Add a persistent BioLang notebook kernel/REPL protocol. Current run-all cells
  share one interpreter; an individually run cell is intentionally isolated.
- Render structured notebook values beyond text and TSV, including BioLang
  tables, plots, images, and export to HTML/PDF.
- Virtualize and index large FASTA record sets beyond the bounded native preview.
- Add a focused Source Control activity with staging, commit, and diff views.
- Add split editor groups, a BioLang REPL terminal profile, NCBI/BLAST request
  panels, visual sequence inspection, and deep-path Explorer breadcrumbs.
- Multi-root workspaces, drag/drop explorer moves, file watching, and diff views.
- Selected-code and named run configurations.
- Concurrent local jobs and process-tree/job-object cancellation.
- Remote project snapshots, dataset references, artifacts, and restart reconnection.
- Terminal split panes, search, link detection, and task terminals.
- Structured external API request forms and operating-system keychain integration.
- Virtualized full-dataset viewers, richer plot inspector, and sequence translation.
- Jupyter import/export controls in Desktop.
- Crash-safe atomic recovery files rather than browser storage.
- Native keychain storage for biological API credentials.
- Configurable shortcuts and conflict detection.
- WCAG audit, scaling matrix, startup/memory benchmarks, and cross-platform CI.
- Signed installers, runtime bundling decision, updater service, release channels,
  SBOM, and application signing.
- Curated descriptions and examples for the 385 builtins that currently expose
  runtime-derived arity/signatures but no authored reference text.
