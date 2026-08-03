# BioLang Desktop — UX Backlog

Actionable follow-ups from the desktop UI review. Ordered by release intent, not implementation difficulty.

**Product loop to protect:** Folder → Edit → Run → Results  
Learner surfaces should reinforce that loop; Expert may add power around it.

Related: [REQUIREMENTS.md](REQUIREMENTS.md), [REQUIREMENTS_STATUS.md](REQUIREMENTS_STATUS.md).

---

## Priority legend

| Tag | Meaning |
|-----|---------|
| P0 | Blocks first success or causes frequent confusion |
| P1 | Daily loop quality |
| P2 | Power-user completeness |
| P3 | Polish, a11y, platform fit |

| Effort | Rough guide |
|--------|-------------|
| S | One focused change |
| M | Multi-file UI + behavior |
| L | Layout/architecture or platform work |

---

## Teaching release

Goal: a new user completes **open → trust → run → see a table/plot** without reading docs.

### P0 — First 10 minutes

| ID | Item | Effort | Status | Notes |
|----|------|--------|--------|-------|
| T-01 | **Trust + Run on welcome examples** | M | **Done** | Welcome/comparison examples trust the folder and queue a run. Play button also trusts-and-runs when restricted. |
| T-02 | **Explain disabled Run** | S | **Done** | Run control label/tooltip states trust / busy / no-file reasons; untrusted stays clickable. |
| T-03 | **Raise minimum chrome type size** | M | **Done** | Trust banner, welcome, recent, menus, tabs, panel tabs, status bar, breadcrumbs, toasts, command buttons floored near 11–12px. |
| T-04 | **Sample / tutorial workspace** | M | **Done** | Welcome “Open tutorial project” / “Start tutorial”: browser opens demo + `analysis.bl` in Learner mode; Desktop trusts-and-runs Sequence QC starter. Packaged on-disk sample tree still optional later. |

### P1 — Teaching daily loop

| ID | Item | Effort | Status | Notes |
|----|------|--------|--------|-------|
| T-05 | **Auto-reveal Output for learners** | S | **Done** | `showOutput` forces bottom dock in learner mode; runs already call `showOutput`. |
| T-06 | **Simplify Output in Learner mode** | M | **Done** | `simplified` OutputPane + force bottom dock; hides pin/compare/detach/drag/more menu. |
| T-07 | **Richer empty editor (workspace open, no file)** | S | **Done** | New file, Import data, Go to file, and two starter analyses. |
| T-08 | **Jobs only in Expert (Learner)** | S | **Done** | Jobs activity and bottom Jobs tab hidden in learner mode. |
| T-09 | **Learner guide: trust step** | S | **Done** | Desktop untrusted workspaces insert a Trust step. |

### Teaching release exit criteria

- [x] From cold start, user can produce a plot or table in under ~2 minutes with minimal reading
- [x] Disabled Run always has an on-control reason
- [x] Learner mode never requires discovering Output docking or Jobs sidebar
- [x] Welcome examples complete without a dead-end after Open Folder
- [x] Sample/tutorial workspace entry point (T-04)

---

## Researcher release

Goal: daily analysis is fast, targets are obvious, results stay first-class, remote mistakes are hard.

### P1 — Daily analysis loop

| ID | Item | Effort | Status | Notes |
|----|------|--------|--------|-------|
| R-01 | **Execution target chip next to Run** | M | **Done** | Chip beside Play; first remote run per session asks for confirmation. |
| R-02 | **Persistent failure feedback** | S | **Done** | Error toasts last longer; sticky dismissible error stack for failures. |
| R-03 | **Settings sections (or tabs)** | M | **Done** | Editor · Trust · Credentials · Remote · References tabs; API “Add key” opens Credentials. |
| R-04 | **Native path pickers for reference builds** | S | **Done** | Browse buttons for FASTA/GTF and SSH identity via `pick_path` (Desktop). |
| R-05 | **Tab overflow** | M | **Done** | Overflow menu when more than 5 editors are open (list + close). |

### P2 — Power-user completeness

| ID | Item | Effort | Notes |
|----|------|--------|-------|
| R-06 | **Split editor groups** | L | **Done** | Horizontal split with secondary tabs, resizer, Ctrl+\\ / View menu / tab “Split Right”; focused group receives Explorer opens. |
| R-07 | **Explorer drag-and-drop** | L | **Done** | In-tree move via `move_entry`; OS files dropped onto folders/`data/` via `write_new_file`. |
| R-08 | **External file change affordance** | M | **Done** | Poll open files every 4s; banner with Reload / Keep editing when disk diverges from saved buffer. |
| R-09 | **Configurable keybindings + conflicts** | L | **Done** | Settings → Keyboard: capture chords, reset, conflict list; App handler + menus/shortcut dialog use resolved map. |
| R-10 | **Virtualize long lists** | M | **Done** | FileTree flattens + virtualizes after 80 rows; Jobs sidebar virtualizes after 60. |
| R-11 | **Title bar density** | M | **Done** | ≤1100 hides Learner/Expert + edition label; ≤960 collapses menus into a single Menu control. |
| R-12 | **Output progressive disclosure (Expert)** | S | **Done** | Primary Rerun/Export/dock; pin/compare/detach/clear/delete under More. |

### Researcher release exit criteria

- [x] Active execution target is visible at the moment of Run
- [x] Failed remote/local jobs remain inspectable without re-running (Output + sticky error stack)
- [x] Settings for credentials, SOMER, and references are findable without scrolling a single blob
- [x] Large projects remain navigable (tabs overflow menu; tree/jobs still later for virtualization)

---

## Shared polish (either release)

| ID | Item | Priority | Effort | Notes |
|----|------|----------|--------|-------|
| S-01 | **Cmd vs Ctrl shortcut labels** | P3 | S | Platform-correct glyphs on macOS. |
| S-02 | **Contrast pass on muted text** | P3 | M | Audit `--text-faint` / soft borders against WCAG for body chrome. |
| S-03 | **Dialog focus trap + Esc consistency** | P3 | M | Settings, About, Shortcuts, prompts, palette. |
| S-04 | **Product naming consistency** | P3 | S | **Done** — startup progress uses `BioLang ${productEdition}` instead of hard-coded Studio. |
| S-05 | **WCAG / scaling matrix** | P3 | L | Already on later requirements list; formalize after T-03. |

---

## Suggested sequencing

```text
Teaching (ship first)
  T-02 → T-01 → T-05 → T-06 → T-07 → T-03 → T-08 → T-09 → T-04

Researcher (after teaching loop is solid)
  R-01 → R-02 → R-03 → R-04 → R-12 → R-05 → R-11
  then R-07 / R-08 / R-10, then R-06 / R-09

Polish (continuous)
  S-01, S-04 early; S-02 / S-03 with T-03; S-05 when stabilizing a release
```

---

## Explicitly out of this backlog

Tracked elsewhere or capability-blocked (see REQUIREMENTS_STATUS):

- Full LSP rename/references when `bl lsp` exposes them
- Registry package search / lockfile UI
- Multi-root workspaces
- Collaborative editing
- Galaxy-class hosted workflow product

---

## Tracking

- Mark items done in this file or move IDs into issues with the same `T-` / `R-` / `S-` prefixes.
- Prefer shipping Teaching P0 before new Expert-only chrome.
- When in doubt: protect **Folder → Edit → Run → Results** over additional panels.
