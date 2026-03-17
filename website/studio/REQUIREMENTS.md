# BioGist Studio — Requirements

**Full name:** Biological Genomic Interactive Scripting Terminal

**Tagline:** Write, run, and share bioinformatics notebooks in your browser.

**What it does:** Browser-based BioLang notebook with code cells, markdown cells, inline plots, table viewers, and shared variables — powered by WebAssembly. No server, no install, works offline.

**URL:** `lang.bio/studio`

**What it's NOT:**
- Not Jupyter (no Python kernel, no server)
- Not Google Colab (no cloud compute)
- Not an IDE (no project management, no git)

It's a **lightweight, browser-based BioLang notebook** — like Observable or Deno notebooks, but for bioinformatics.

---

## Tech Stack

| Layer | Technology |
|---|---|
| Runtime | BioLang WASM (already compiled, ~4MB, deployed at lang.bio) |
| UI | Vanilla JS + Tailwind CSS (CDN) |
| Storage | IndexedDB via Dexie.js |
| Plots | Inline SVG (BioLang's plot builtins output SVG) |
| Tables | Custom grid renderer (sortable, filterable) |
| Markdown | Marked.js (CDN) for markdown cell rendering |
| Code editor | CodeMirror 6 (CDN) with custom BioLang mode |
| Export | html2canvas or custom for PDF, native for HTML/JSON |
| Offline | Service Worker caches WASM + app shell |
| Install | PWA — "Add to Home Screen" |

### BioLang WASM Integration

Same bridge as `lang.bio/playground`:
- `window.BioLangWasm.run(code)` → returns output string
- `__blFetch.sync(url)` → synchronous XHR for file/API access
- Variables persist in WASM memory between cell executions
- `init()` must be called once to install fetch hooks

---

## Architecture

```
lang.bio/studio/
├── index.html          # Main app page
├── app.js              # Notebook engine, cell management, execution
├── editor.js           # CodeMirror setup, BioLang syntax, autocomplete
├── renderer.js         # Output rendering: text, tables, SVG plots, errors
├── sw.js               # Service Worker (offline, WASM caching)
├── manifest.json       # PWA manifest
├── templates/          # Pre-built notebook templates (JSON)
│   ├── quickstart.json
│   ├── qc-pipeline.json
│   ├── variant-analysis.json
│   ├── rnaseq-de.json
│   └── sequence-ops.json
├── help.html           # Self-contained help page
└── data/               # Sample data files for templates
    ├── sample.fasta
    ├── sample.fastq
    └── sample.vcf
```

---

## Core Features

### 1. Notebook Structure

A notebook is an ordered list of cells. Each cell is either **code** or **markdown**.

```json
{
  "version": 1,
  "name": "My Analysis",
  "created": "2026-03-17T10:00:00Z",
  "modified": "2026-03-17T12:30:00Z",
  "cells": [
    { "type": "markdown", "content": "# QC Analysis\nThis notebook..." },
    { "type": "code", "content": "let seq = dna\"ATCGATCG\"\ngc_content(seq)", "output": null },
    { "type": "code", "content": "translate(seq)", "output": null }
  ]
}
```

File extension: `.bln` (BioLang Notebook)

### 2. Code Cells

- **Editor:** CodeMirror 6 with custom BioLang language mode
  - Syntax highlighting: keywords, strings, DNA/RNA/protein literals, comments, operators, pipes
  - Line numbers
  - Auto-indent
  - Bracket matching
  - Auto-closing quotes/brackets
- **Run button:** Play icon (▶) on each cell
- **Status indicator:** idle (grey) / running (purple pulse) / success (green) / error (red)
- **Execution:** runs via BioLang WASM, output captured and rendered below cell
- **Variables persist:** cell 1 defines `let x = 5`, cell 2 can use `x`
- **Cell toolbar:** Run, Add cell above, Add cell below, Move up, Move down, Delete, Toggle markdown/code

### 3. Markdown Cells

- Edit mode: plain text editor with markdown syntax
- View mode: rendered HTML (headers, bold, italic, links, lists, code blocks, tables)
- Toggle: double-click to edit, click away to render
- Rendered via Marked.js
- Support images (base64 inline or URL)

### 4. Output Rendering

| Output Type | How it's rendered |
|---|---|
| Text/String | Monospace text block |
| Number | Formatted number |
| Boolean | `true` / `false` badge |
| Table | Interactive grid: sortable columns, filterable, pagination for >100 rows |
| List/Array | Collapsible tree view |
| Record/Object | Key-value card |
| DNA/RNA/Protein | Sequence viewer with base coloring |
| SVG (from plot builtins) | Inline SVG rendered below cell |
| Error | Red error box with message, line number, suggestion |
| Null/Nil | Grey "no output" indicator |

### 5. Auto-complete

- Trigger: Ctrl+Space or automatic after typing 2+ characters
- Sources:
  - BioLang builtins (~200 functions)
  - User-defined variables from previous cells
  - DNA/RNA/Protein literal prefixes (`dna"`, `rna"`, `protein"`)
  - Keywords: `let`, `if`, `else`, `for`, `while`, `fn`, `return`, `match`, `import`, `pipeline`
- Each suggestion shows: name, signature, one-line description

### 6. Inline Documentation

- Hover over any builtin → tooltip with signature + description + example
- Click "docs" link → jumps to lang.bio/docs for full reference
- Built-in doc database (bundled JSON, ~50KB)

### 7. Shared Environment

- All cells share one WASM environment
- Variables defined in cell N are available in cell N+1, N+2, etc.
- Re-running a cell updates the environment
- "Reset environment" button clears all variables (re-initializes WASM)
- Environment inspector panel: shows all defined variables with types and values

---

## File & Data Features

### 8. Save / Load

- **Auto-save:** every 30 seconds to IndexedDB
- **Manual save:** Ctrl+S → save to IndexedDB with name
- **Notebook list:** sidebar showing all saved notebooks with dates
- **Download:** save as `.bln` file (JSON)
- **Upload:** drag-and-drop `.bln` file to open

### 9. Share via URL

- "Share" button → encodes notebook as compressed base64 in URL hash
- Recipient opens URL → full working notebook loads instantly
- Max URL length: ~8KB of code (covers most notebooks)
- For larger notebooks: generates a shareable `.bln` download link instead

### 10. Import

- Drag-and-drop `.bl` script → opens as single code cell
- Drag-and-drop `.bln` notebook → opens full notebook
- Drag-and-drop `.csv` / `.tsv` → available as `data` variable
- Drag-and-drop `.fasta` / `.fastq` / `.vcf` / `.bed` → available as parsed variable
- Paste code → creates new code cell

### 11. Data Files

- Upload panel: drag-and-drop or file picker
- Uploaded files stored in IndexedDB
- Available in code cells via `read_csv("uploaded/file.csv")` or similar
- File list sidebar showing uploaded data files
- Delete / rename uploaded files
- Sample data files bundled with templates

### 12. Export

| Format | What it exports |
|---|---|
| `.bln` (JSON) | Full notebook with code, markdown, outputs |
| `.bl` (script) | Code cells only, concatenated |
| `.html` | Rendered notebook as standalone HTML page |
| `.pdf` | Rendered notebook as PDF (via print-to-PDF) |
| `.md` | Markdown: code in fenced blocks, outputs as text |

---

## Table Viewer

### 13. Interactive Tables

When a cell outputs a `Table` value:
- Render as a grid with column headers
- **Sort:** click column header to sort asc/desc
- **Filter:** text filter per column
- **Pagination:** 50 rows per page for large tables
- **Column resize:** drag column borders
- **Copy:** select cells → Ctrl+C copies as TSV
- **Download:** export table as CSV
- **Freeze columns:** pin first N columns while scrolling
- **Column types:** auto-detect number/string/date, align accordingly
- **Summary row:** count, mean, min, max for numeric columns

---

## Plot Rendering

### 14. Inline SVG Plots

BioLang's plot builtins (`plot()`, `scatter()`, `bar()`, `histogram()`, `volcano_plot()`, `heatmap()`, `boxplot()`, `pca_plot()`) output SVG strings.

Studio renders them:
- Inline below the code cell
- Responsive: scale to cell width
- Click to expand: full-screen lightbox view
- Download: save as SVG or PNG
- Interactive (future): zoom, pan, tooltips on data points

---

## Templates

### 15. Pre-built Notebooks

Available from "New → From Template" menu:

| Template | Content |
|---|---|
| **Quickstart** | DNA ops, translation, GC content, k-mers — introduction to BioLang |
| **QC Pipeline** | Read FASTQ, quality stats, filter, trim, report |
| **Variant Analysis** | Read VCF, filter by quality, annotate, frequency lookup |
| **RNA-seq DE** | Load expression matrix, normalize, t-test, volcano plot |
| **Sequence Operations** | Complement, translate, motifs, restriction sites, alignment |
| **API Queries** | NCBI gene lookup, UniProt search, PubMed query |
| **Statistics** | t-test, ANOVA, correlation, PCA, clustering |
| **Data Wrangling** | Read CSV, filter, mutate, group, summarize, join |

Each template includes:
- Markdown cells explaining the analysis
- Code cells with working examples
- Sample data files (bundled)
- Expected outputs

---

## UI Layout

### Desktop (>768px)

```
┌──────────────────────────────────────────────────┐
│ BioGist Studio    [New] [Open] [Save] [Share] [⚙]│
├────────┬─────────────────────────────────────────┤
│        │                                         │
│ Files  │  [Markdown cell - rendered]             │
│        │                                         │
│ nb1.bln│  [Code cell - editor + output]          │
│ nb2.bln│                                         │
│ nb3.bln│  [Code cell - editor + output]          │
│        │                                         │
│ Data   │  [Table output - interactive grid]      │
│        │                                         │
│ f.csv  │  [Code cell - editor + SVG plot]        │
│ s.fasta│                                         │
│        │  [+ Add cell]                           │
├────────┴─────────────────────────────────────────┤
│ Environment: seq=DNA(12bp) | gc=0.5 | table=...  │
└──────────────────────────────────────────────────┘
```

### Mobile (<768px)

```
┌──────────────────────┐
│ Studio   [≡] [▶] [⚙] │
├──────────────────────┤
│                      │
│ [Markdown cell]      │
│                      │
│ [Code cell + output] │
│                      │
│ [Code cell + output] │
│                      │
│ [+ Add cell]         │
│                      │
├──────────────────────┤
│ Files | Env | Help   │
└──────────────────────┘
```

Sidebar collapses to bottom sheet on mobile. Cells are full-width.

---

## Keyboard Shortcuts

| Shortcut | Action |
|---|---|
| `Shift+Enter` | Run cell and move to next |
| `Ctrl+Enter` | Run cell and stay |
| `Ctrl+Shift+Enter` | Run all cells |
| `Ctrl+S` | Save notebook |
| `Ctrl+Shift+N` | New notebook |
| `Ctrl+Shift+O` | Open notebook |
| `Ctrl+Shift+E` | Export notebook |
| `Escape` | Exit cell edit mode |
| `Enter` (on selected cell) | Enter edit mode |
| `Ctrl+Shift+M` | Toggle cell type (code ↔ markdown) |
| `Ctrl+Shift+D` | Delete cell |
| `Ctrl+Shift+Up` | Move cell up |
| `Ctrl+Shift+Down` | Move cell down |
| `Ctrl+Space` | Trigger autocomplete |
| `Ctrl+Shift+R` | Reset environment |
| `Ctrl+/` | Toggle comment |

---

## Version History

### 16. Auto-save Snapshots

- Auto-save every 30 seconds to IndexedDB
- Keep last 50 snapshots per notebook
- "History" panel: list of snapshots with timestamps
- Click snapshot → preview, restore, or diff against current
- Diff view: side-by-side comparison of two versions
- Snapshots auto-pruned: keep all from last hour, hourly for last day, daily for last week

---

## Multi-tab

### 17. Multiple Notebooks

- Tab bar at top showing open notebooks
- Click tab to switch
- Close tab (with unsaved changes warning)
- Drag tabs to reorder
- "+" button to create new notebook
- Each notebook has its own WASM environment (isolated variables)

---

## NCBI / API Integration

### 18. API Cells

BioLang API builtins work via the fetch bridge (same as playground):
- `ncbi_gene("BRCA1")` → fetches from NCBI E-utilities
- `ncbi_search("cancer", "pubmed")` → searches PubMed
- `uniprot_entry("P38398")` → fetches UniProt data
- `read_csv("https://...")` → fetches remote CSV

Network requests go through `__blFetch.sync()` XHR bridge.

Rate limiting: respect NCBI 3 req/s (or 10 with API key).

Optional NCBI API key in settings.

---

## Settings

### 19. Configuration

| Setting | Options |
|---|---|
| Theme | Dark (default) / Light |
| Font size | 12px / 14px / 16px / 18px |
| Font family | JetBrains Mono / Fira Code / Consolas / System |
| Auto-save interval | 15s / 30s / 60s / Off |
| Tab size | 2 / 4 spaces |
| Line wrapping | On / Off |
| Auto-complete | On / Off |
| Show line numbers | On / Off |
| NCBI API key | Optional text input |
| Max output lines | 100 / 500 / 1000 / Unlimited |

Persisted in IndexedDB.

---

## Offline Capability

### Always available offline:
- WASM runtime (cached by service worker after first load)
- All saved notebooks (IndexedDB)
- All uploaded data files (IndexedDB)
- Templates (bundled)
- Autocomplete database (bundled)
- Documentation tooltips (bundled)
- All sequence operations (WASM)

### Requires network:
- NCBI/UniProt/API queries
- Remote file loading (URLs)
- Share via URL (generating link)

---

## Design

### Color Scheme
- **Primary:** Purple (#7c3aed) — matches BioGist branding
- **Background:** Dark slate (#0f172a / #020617) for dark mode
- **Code cells:** Slightly lighter background (#1e293b)
- **Output cells:** Dark background (#0f172a) with colored text
- **Success:** Green (#4ade80)
- **Error:** Red (#f87171)
- **Running:** Purple pulse animation
- **Markdown:** Rendered with soft text colors
- **Light theme:** White background, dark text, same accent colors

### Typography
- Code: JetBrains Mono (CDN) or Fira Code
- UI: System font stack (same as BioGist)
- Markdown: System serif for rendered prose (optional)

---

## Privacy

- **No server** — all execution in WASM, all storage in IndexedDB
- **No account** — no login, no cloud save
- **No tracking** — no analytics, no telemetry
- **API calls only when code explicitly requests** — `ncbi_gene()` etc.
- **Shared notebooks via URL** — no server stores the content

---

## Performance Constraints

| Constraint | Limit | Mitigation |
|---|---|---|
| WASM load | ~4MB first load | Service worker cache, progress indicator |
| Cell execution | 30s timeout | Kill long-running cells, show warning |
| Table rendering | Max 10,000 rows | Pagination at 50 rows, virtual scroll for large tables |
| Plot rendering | Max 100,000 data points | Downsample for display, full data in export |
| Notebook size | Max 100 cells | Warning at 50, hard limit at 100 |
| URL share | Max ~8KB encoded | Fall back to .bln download for larger |
| IndexedDB | ~50MB per notebook | Warn on large outputs, auto-prune old snapshots |

---

## Development Phases

### Phase 1 — Core Notebook (Week 1-2)
- PWA scaffold (index.html, app.js, sw.js, manifest.json)
- Cell model: code + markdown
- CodeMirror 6 integration with BioLang syntax
- BioLang WASM integration (run cell, capture output)
- Text output rendering
- Shared environment (variables persist across cells)
- Cell toolbar: run, add, delete, move, toggle type
- Markdown rendering via Marked.js
- Auto-save to IndexedDB
- Dark theme with purple accents

### Phase 2 — Rich Output (Week 3-4)
- Table viewer (sortable, filterable, paginated)
- SVG plot rendering (inline below cells)
- Sequence viewer (DNA/RNA/Protein with coloring)
- Error rendering with line numbers
- Array/object tree view
- Environment inspector panel

### Phase 3 — Editor Polish (Week 5-6)
- Auto-complete (builtins + variables)
- Inline documentation tooltips
- Keyboard shortcuts (all 16)
- Line wrapping, font size, tab size settings
- Find and replace within cell
- Comment toggling

### Phase 4 — File Management (Week 7-8)
- Notebook list sidebar
- Save/load multiple notebooks
- Download as .bln / .bl / .html / .md
- Import .bl scripts and .bln notebooks
- Data file upload (CSV, FASTA, FASTQ, VCF)
- Data file sidebar
- Share via URL (compressed base64)

### Phase 5 — Templates & Finishing (Week 9-10)
- 8 pre-built templates with sample data
- Version history (auto-snapshots, diff view)
- Multi-tab notebooks
- Settings page
- Help page
- Performance optimization
- Offline testing
- Mobile responsive layout

---

## Relationship to Other Products

| Product | Relationship |
|---|---|
| **BioGist** | Scan a paper → copy entity → use in Studio notebook |
| **BioKhoj** | Find a paper → read methods → reproduce in Studio |
| **lang.bio/playground** | Studio replaces playground as the primary WASM coding experience |
| **BioLang CLI (`bl`)** | Studio is the browser equivalent of `bl repl` — same language, same builtins |

**Studio does NOT replace the playground.** Playground stays as a simple single-block code runner for docs and tutorials. Studio is for multi-cell notebook workflows.
