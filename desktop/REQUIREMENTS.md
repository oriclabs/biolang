# BioLang Desktop IDE Requirements

**Working name:** BioLang Desktop

**Purpose:** A lightweight, cross-platform BioLang development environment that gives bioinformaticians one integrated workspace for writing code, running analyses, managing files and packages, using terminals and APIs, inspecting biological data, and optionally submitting work to remote compute.

**Supported desktop platforms:** Windows, macOS, and Linux.

**Product principle:** Local-first and offline-capable. A remote server may extend execution capacity, but must not be required for normal editing, terminal use, package management, or local BioLang execution.

---

## Product Scope

BioLang Desktop is a dedicated IDE distributed as a native desktop application. Its interface is implemented with web technologies inside a Tauri shell, while privileged operating-system integration is provided by a Rust host process.

It must let a user complete the common BioLang workflow without leaving the application:

- Open or create a project.
- Browse and edit project files.
- Receive BioLang diagnostics, completion, hover help, navigation, and formatting.
- Run, stop, and inspect BioLang programs.
- Use a real interactive terminal.
- Install, update, inspect, and remove BioLang packages.
- Browse BioLang functions and external biological APIs.
- Preview common bioinformatics file formats and analysis results.
- Run locally by default and optionally submit jobs to remote compute.

### Non-goals for the Initial Release

- Reimplementing the BioLang compiler, runtime, formatter, LSP, or package manager in the GUI.
- Requiring accounts, cloud storage, or a permanent network connection.
- Becoming a hosted workflow platform equivalent to Galaxy.
- Replacing the browser-based BioLang Workbench notebook.
- Providing a full general-purpose web browser with unrestricted desktop privileges.
- Implementing collaborative editing in the first release.

---

## Relationship to BioLang Workbench

| Product | Primary use | Runtime | Storage |
|---|---|---|---|
| BioLang Desktop | Projects, scripts, packages, terminal, large files, local and remote jobs | Native BioLang CLI and LSP | Local filesystem |
| BioLang Workbench | Lightweight notebooks, teaching, sharing, browser experiments | BioLang WASM | Browser storage |

The products may share syntax metadata, documentation, themes, and notebook formats, but they have separate requirements and deployment models.

---

## Architecture

### Required Technology Direction

| Layer | Requirement |
|---|---|
| Desktop shell | Tauri 2 |
| UI | TypeScript with a component-based web framework |
| Code editor | Monaco Editor by default; CodeMirror 6 remains an acceptable lightweight alternative after a measured prototype |
| Terminal UI | xterm.js |
| Native host | Rust commands and services exposed through Tauri IPC |
| Language intelligence | Existing `bl lsp` process over standard LSP transport |
| Local execution | Existing `bl` executable, supervised as a child process |
| Package operations | Existing BioLang package commands and project manifest |
| Remote execution | Optional backend implementing the execution contract in this document |
| Secrets | Operating-system credential store or keychain |

### Process Model

```text
Desktop webview
Editor | Explorer | Packages | APIs | Results | Terminal
                         |
                      Tauri IPC
                         |
Rust desktop host
Files | PTY | Processes | Jobs | Secrets | Native dialogs
                  /              \
       Local execution       Remote execution
       bl run / bl lsp       BioLang service / HPC / Galaxy
```

The application must not require an HTTP server on `localhost`. The trusted application UI communicates with the Rust host through Tauri IPC. A local HTTP service may be introduced only for a capability that cannot be implemented safely or reliably through IPC.

BioLang programs must execute outside the UI process. The Rust host supervises each process, captures output, supports cancellation, and reports state to the UI.

### Shared Core Interfaces

The implementation must define stable internal interfaces for:

- `ExecutionBackend`: submit, start, stop, status, stream logs, collect outputs, and report resource use.
- `LanguageService`: start, restart, send LSP messages, and report health.
- `PackageService`: list, search, inspect, install, update, and remove packages using BioLang commands.
- `FileService`: permission-checked filesystem access, watching, streaming reads, and atomic writes.
- `SecretService`: store, retrieve, and delete API credentials without exposing raw values to untrusted content.

The first release implements `LocalExecutionBackend`. Remote, Galaxy, Slurm, and cloud adapters are later implementations of the same execution behavior.

### SOMER Remote Backend

SOMER (Scalable Omics Management and Execution Runtime) is the separate remote
execution project selected for the first backend implementation.

- Desktop keeps direct Tauri-supervised `bl` execution as the default.
- A status-bar selector switches between Local and saved SOMER connection profiles.
- Desktop submits the active source through the shared `@somer/client` v1 API.
- SOMER derives user ownership from authentication and returns only that user's jobs.
- Profile URLs and resource choices may persist; bearer tokens remain session-only
  until an operating-system keychain service is implemented.
- SOMER server and agent lifecycles are independent of Desktop, so remote jobs
  continue after the application closes.
- Galaxy, Slurm, containers, and workflow engines integrate behind SOMER rather
  than adding separate UI protocols.

---

## User Interface

### Primary Layout

- **Activity bar:** Explorer, Search, Packages, APIs, Jobs, and Source Control when supported.
- **Left sidebar:** Context for the selected activity.
- **Editor area:** Tabbed code editors, documentation, data previews, result views, and diff views.
- **Optional right sidebar:** Symbols, variables, outline, or result details.
- **Bottom panel:** Terminal, Problems, Output, Jobs, and Debug information when available.
- **Status bar:** Workspace, BioLang version, LSP state, execution target, active environment, cursor position, and job status.

Panels must be resizable, hideable, and restorable. Layout state is saved per workspace. The interface must remain usable on laptop-sized displays without overlapping controls or requiring every panel to be visible.

### Commands and Navigation

- Provide a searchable command palette.
- Provide keyboard navigation for all core workflows.
- Use native menus where platform conventions require them.
- Expose commands through menus, toolbar icons, contextual menus, and shortcuts where appropriate.
- Provide tooltips for icon-only commands.
- Allow users to configure shortcuts and detect conflicts.

---

## Project and File Management

### Workspace Requirements

- Open a folder as a workspace.
- Open recent workspaces.
- Create a project from a BioLang template.
- Support multiple root folders in a later release.
- Detect and display the BioLang project manifest and package state.
- Persist workspace-specific IDE settings separately from user settings.
- Detect external file changes and offer safe reload or comparison behavior.

### File Explorer

- Create, rename, move, duplicate, and delete files and folders with confirmation for destructive operations.
- Support drag and drop within the workspace.
- Exclude generated or large directories through configurable patterns.
- Show file type, modification state, and diagnostics.
- Support reveal in the operating-system file manager.
- Never silently modify files outside the open workspace.

### Large Biological Files

The UI must not load a complete large file into JavaScript memory. The Rust host must provide bounded, streaming, indexed, or paginated access.

Initial preview targets:

- FASTA and FASTQ sequence records.
- VCF variants.
- BED intervals.
- CSV and TSV tables.
- JSON and BioLang result values.

Later preview targets may include BAM, CRAM, GFF/GTF, Newick, PDB/mmCIF, and common image formats. Binary genomic formats should use established libraries and indexes rather than custom parsers.

---

## Code Editing and Language Support

### Editor Features

- BioLang syntax highlighting, including biological literals.
- Line numbers, bracket matching, automatic closing, indentation, and comment toggling.
- Multiple cursors and selections.
- Find and replace in a file and across a workspace.
- Symbol outline and breadcrumb navigation.
- Go to definition, find references, rename, and document symbols when supported by `bl lsp`.
- Inline diagnostics with a consolidated Problems panel.
- Completion with signatures, descriptions, and package origin.
- Hover documentation with examples and links to full reference material.
- Signature help and parameter information.
- Code formatting through the current BioLang formatter implementation.
- Diff editor for unsaved changes, file history, and package changes.
- Configurable font, size, tab width, wrapping, minimap, and theme.

The IDE must consume language metadata from BioLang's current implementation or generated metadata. It must not maintain an independent handwritten list of builtins, aliases, signatures, or documentation.

### LSP Lifecycle

- Start `bl lsp` when a BioLang workspace or file opens.
- Display server startup, failure, restart, and version mismatch states.
- Restart automatically after recoverable failures with bounded retries.
- Preserve LSP logs for troubleshooting.
- Allow the user to restart the language server manually.
- Detect incompatible BioLang executable versions and provide a clear resolution path.

---

## Execution and Job Management

### Local Runs

- Run the active file, selected code, or a configured project command.
- Stop a running process and its owned child-process tree.
- Stream standard output and standard error without blocking the interface.
- Preserve the exact executable, arguments, working directory, environment profile, start time, exit status, and duration.
- Allow rerunning a previous job with the same configuration.
- Support multiple concurrent jobs with configurable limits.
- Provide run configurations that can be stored in workspace settings.
- Never execute code merely because a project was opened.

### Job States

At minimum, jobs use these states:

```text
queued -> starting -> running -> succeeded
                            \-> failed
                            \-> cancelling -> cancelled
```

Lost remote connections must produce an explicit `unknown` or `disconnected` condition rather than incorrectly marking a job failed.

### Remote Runs

Remote execution is optional and must use an authenticated adapter. The adapter must support:

- Submit a command or packaged project snapshot.
- Reference remote datasets without uploading them again when possible.
- Report queue and execution state.
- Stream or incrementally retrieve logs.
- Cancel jobs.
- List and download declared output artifacts.
- Report backend identity, BioLang version, and resource profile.
- Resume monitoring after the desktop application restarts.

Uploading local data requires an explicit user action, visible destination, size estimate, and cancellation support.

---

## Integrated Terminal

- Render the terminal with xterm.js backed by a real native pseudoterminal.
- Use the user's selected shell and normal shell initialization behavior.
- Start terminals in the active workspace by default.
- Support multiple named terminal sessions, split panes, resize, copy/paste, search, clear, and kill.
- Preserve terminal dimensions correctly during panel resizing.
- Detect links and file paths without automatically executing them.
- Allow tasks and package operations to open in a dedicated terminal when requested.
- Do not implement a simulated command console as a substitute for a PTY.

---

## Package Management

The package interface is a frontend for BioLang's package model. The CLI remains the source of truth.

Required capabilities:

- Show the packages declared by the current project.
- Show installed, missing, outdated, incompatible, and failed packages.
- Search configured package registries or sources.
- Display package version, source, license, documentation, exports, and trust information when available.
- Install, update, and remove packages through BioLang commands.
- Preview manifest and lockfile changes before confirmation where feasible.
- Stream operation logs and surface actionable errors.
- Refresh language intelligence after dependency changes.
- Support local path packages for development.
- Never execute package lifecycle scripts without workspace trust and visible consent.

Package resolution rules must not be duplicated in the desktop application.

---

## API and Documentation Browser

### Offline Help Center

- Provide an integrated, searchable Help activity that works without a workspace or network connection.
- Cover every ordered language-guide chapter, runtime builtin, practical tutorial chapter, and repository BioLang example.
- Generate the help index from BioLang's checked-in runtime metadata, books, and examples before development, testing, and production builds.
- Show source collection, category, signature, return type, full documentation, and runnable example where available.
- Allow repository sources to open in the editor when they are inside the active workspace.
- Allow builtin and example code to be added to the active BioLang file without executing it automatically.
- Keep external documentation links inert inside the privileged application webview unless opened through an isolated browser capability.
- Expose Help through the activity bar, application menu, command palette, and `F1`.

### BioLang API Browser

- Browse functions, types, modules, package exports, aliases, and examples.
- Search by name, category, parameter, return type, or biological domain.
- Insert a function call or example into the active editor.
- Show the BioLang version associated with the displayed metadata.
- Generate content from structured runtime or documentation metadata rather than maintaining separate manual definitions.

### External Biological APIs

- Provide structured request forms for supported services such as NCBI and UniProt.
- Show endpoint, parameters, rate-limit information, request status, and response metadata.
- Preview JSON, text, sequence, and table responses.
- Save a response to the workspace or generate equivalent BioLang code.
- Require explicit requests; the IDE must not send biological data to third parties automatically.
- Store API keys in the operating-system credential store.
- Redact credentials from logs, command history, exported settings, and error reports.

### Web Content

Documentation and external web pages may open in a restricted webview or the system browser. Arbitrary web content must not receive Tauri filesystem, process, terminal, package, job, or secret capabilities.

---

## Results and Data Inspection

- Render text, numbers, booleans, lists, records, tables, biological sequences, errors, and plots appropriately.
- Provide sortable, filterable, virtualized tables with CSV or TSV export.
- Provide sequence search, coordinates, base coloring, reverse complement, translation, and copy/export where relevant.
- Link diagnostics and stack traces to source locations.
- Allow declared job artifacts to be previewed without moving them into editor memory in full.
- Clearly distinguish a preview, truncated result, sampled result, and complete dataset.
- Preserve provenance: source job, command, inputs, timestamp, BioLang version, and execution backend.

---

## Security and Privacy

### Workspace Trust

An untrusted workspace may be browsed and edited, but cannot automatically:

- Run BioLang programs or project tasks.
- Start workspace-defined terminals or commands.
- Execute package scripts or plugins.
- Read credentials.
- Enable workspace-provided native integrations.

Trust is granted explicitly and can be revoked.

### Capability Boundaries

- Apply least-privilege Tauri capabilities per window and webview.
- Keep external web content isolated from privileged application IPC.
- Validate all IPC inputs in Rust, including paths and command parameters.
- Canonicalize and verify filesystem paths before access.
- Restrict normal project operations to approved workspace roots.
- Treat BioLang code, terminals, packages, and plugins as code execution.
- Use the operating-system keychain for secrets.
- Redact secrets and sensitive query parameters from logs.
- Provide telemetry only as explicit opt-in, with documented payloads.

### Data Privacy

- Local projects and results remain local by default.
- Remote upload and API requests require clear user actions.
- The application must show the destination service before transmitting data.
- Closing or deleting a remote connection profile must not silently delete remote data.

---

## Reliability and Performance

### Performance Targets

Targets must be measured on representative Windows, macOS, and Linux machines:

- Show the application window within 2 seconds on a typical development machine after installation warm-up.
- Keep an idle workspace responsive while background language services are running.
- Open ordinary BioLang source files without perceptible blocking.
- Virtualize long lists, large tables, logs, and search results.
- Stream large files and process output using bounded buffers.
- Keep terminal typing latency interactive during background jobs.
- Avoid retaining completed job output indefinitely in UI memory.

Exact memory and startup budgets must be established through prototypes of Monaco and CodeMirror before locking the editor choice.

### Recovery

- Restore open workspaces, editor tabs, unsaved buffers, panel layout, and reconnectable jobs after an application restart.
- Write settings and project metadata atomically.
- Preserve recovery copies for unsaved files after a crash.
- Do not treat a disconnected remote job as cancelled or failed.
- Provide diagnostic logs with user-controlled export and redaction.

---

## Accessibility and Cross-platform Behavior

- Meet WCAG 2.2 AA for application UI where applicable.
- Support keyboard-only operation for all core workflows.
- Preserve visible focus indicators and meaningful accessible labels.
- Respect operating-system reduced-motion, contrast, and scaling preferences.
- Test at 100%, 125%, 150%, and 200% display scaling.
- Use platform-correct path handling, shells, menus, shortcuts, and line endings.
- Avoid platform-specific behavior unless guarded and documented.

---

## Updates and Compatibility

- Provide signed application releases for each supported platform.
- Verify update signatures before installation.
- Show release notes and allow users to defer non-critical updates.
- Detect the available `bl` executable and report its path and version.
- Support either a bundled compatible BioLang runtime or an explicitly configured system installation; the distribution decision must be made before beta.
- Define and enforce compatibility between the IDE, CLI, LSP, package metadata, and remote protocol versions.

---

## Delivery Phases

### Phase 0: Technical Prototype

- Tauri shell on Windows, macOS, and Linux.
- Compare Monaco and CodeMirror using realistic BioLang files.
- Connect to `bl lsp` and validate diagnostics, completion, hover, and formatting.
- Run `bl` as a cancellable child process.
- Connect xterm.js to a real PTY.
- Stream and preview a large FASTA or VCF without loading the full file into the webview.
- Establish startup, memory, terminal latency, and large-file baselines.

### Phase 1: Local IDE MVP

- Workspace and file explorer.
- Tabbed BioLang editor.
- LSP integration and formatting.
- Run, stop, output, Problems, and local job history.
- Integrated terminal.
- Settings, themes, shortcuts, recovery, and workspace trust.
- Windows, macOS, and Linux packaging.

### Phase 2: Bioinformatics Workspace

- Package manager UI.
- BioLang API and documentation browser.
- External biological API browser with secure credentials.
- FASTA, FASTQ, VCF, BED, CSV, and TSV viewers.
- Table, sequence, plot, and artifact inspectors.
- Project templates and reproducible run configurations.

### Phase 3: Remote Compute

- Versioned remote execution protocol.
- Connection profiles and authentication.
- Upload, remote dataset reference, queue status, logs, cancellation, and artifacts.
- Job reconnection after application restart.
- One production adapter selected from a BioLang server, Slurm, or Galaxy integration.

### Phase 4: Extended Workflows

- Source-control integration.
- Notebook interoperability with `.bln` files.
- Workflow and pipeline visualization.
- Additional indexed genomic formats.
- Plugin model only after capability isolation, signing, and trust requirements are defined.

---

## MVP Acceptance Criteria

The local IDE MVP is complete when a user can:

1. Install and launch the signed application on Windows, macOS, and Linux.
2. Open a BioLang project and edit files with syntax highlighting and LSP diagnostics.
3. Format a document using BioLang's current formatter.
4. Run and cancel a BioLang program while viewing streamed output.
5. Open multiple real terminal sessions in the project directory.
6. Recover unsaved editor content after a forced application termination.
7. Work without an account or internet connection, except for explicitly requested APIs.
8. Open an untrusted project without automatically executing project code.
9. Preview a large supported biological file without exhausting webview memory.
10. Complete the workflow without starting or configuring a local HTTP server.

---

## Decisions Required Before Implementation

- Product name and relationship to the existing BioLang and BioGist brands.
- Frontend framework.
- Monaco versus CodeMirror after prototype measurements.
- Bundled BioLang runtime versus system installation, or support for both.
- Package registry and trust metadata available to the GUI.
- Remote execution protocol and first production adapter.
- Supported source-control scope for the first post-MVP release.
- Application signing, release channels, and update infrastructure.

---

## Reference Technologies

- [Tauri architecture](https://v2.tauri.app/concept/architecture/)
- [Tauri capabilities and permissions](https://v2.tauri.app/security/capabilities/)
- [Monaco Editor](https://microsoft.github.io/monaco-editor/)
- [xterm.js](https://xtermjs.org/)
- [Galaxy API](https://docs.galaxyproject.org/en/release_20.01/api/api.html)
