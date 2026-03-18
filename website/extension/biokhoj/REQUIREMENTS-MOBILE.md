# BioKhoj Mobile — Requirements

**Tagline:** Your biology companion, in your pocket.

**What it does:** On-the-go biological lookups, paper feed from your watchlist, and quick sequence tools — all offline-capable. Part of the BioKhoj platform (PWA + Extension + Mobile).

**Platform:** Progressive Web App (installable on iOS/Android via "Add to Home Screen"). Same codebase as desktop PWA with mobile-first UI.

---

## Why Mobile

Researchers use their phones to:
- Check "is this variant pathogenic?" during lab meetings
- Look up gene function while reading a paper on the bus
- Reverse complement a sequence while at the bench
- Check their paper feed during coffee break
- Quick-reference codon tables, restriction enzymes, amino acid properties

No existing mobile app covers all of this in one place.

---

## Architecture

| Component | Technology |
|---|---|
| Framework | PWA (same codebase as desktop, responsive) |
| Storage | IndexedDB via Dexie.js (same schema as desktop) |
| Offline | Service Worker cache (reference tables, watchlist, cached papers) |
| Sequence tools | BioLang WASM (already compiled, ~4MB) |
| APIs | PubMed, bioRxiv, OpenAlex, NCBI, UniProt, ClinVar (same as desktop) |
| Install | "Add to Home Screen" on iOS/Android — no app store needed |

**Shared with desktop PWA:**
- `biokhoj-core.js` — API wrappers, Signal Score, concept expansion
- Dexie.js database schema — watchlist, papers, trends
- Service worker — caching, background sync

**Mobile-specific:**
- Touch-optimized UI (swipe gestures, bottom sheet modals, large tap targets)
- Mobile-first layout (single column, bottom tab bar)
- Quick tools section (not on desktop)
- Offline reference tables (bundled, no API needed)
- Camera integration for barcode/QR scanning (future)

---

## Mobile Features

### Tab 1: Feed (Home)

Paper feed from your watchlist — same as desktop but mobile-optimized:
- Paper cards: title (2 lines), journal, date, Signal Score badge
- Swipe right → mark read
- Swipe left → save to reading list
- Tap → expand: full abstract, matched entities, action buttons
- Pull-to-refresh triggers background check
- Co-mention alerts pinned at top
- "Last checked: 2 hours ago" timestamp

### Tab 2: Lookup (Quick Reference)

**The killer mobile feature — instant biological lookups:**

#### Gene Lookup
- Search by symbol or name
- Returns: function, chromosome, pathways, associated diseases, drugs
- Sources: NCBI Gene + UniProt + OpenAlex
- Cached for offline access after first lookup
- "Add to watchlist" button

#### Variant Lookup
- Search by rsID, HGVS, or ClinVar ID
- Returns: pathogenicity, allele frequency (gnomAD), clinical significance, associated conditions
- Sources: myvariant.info + ClinVar
- Color-coded: pathogenic (red), benign (green), VUS (yellow)

#### Drug Lookup
- Search by drug name
- Returns: class, target, mechanism, indications, FDA status, pharmacogenomics
- Sources: OpenAlex + PubMed
- "Watch this drug" button

#### Species Lookup
- Search by common or scientific name
- Returns: taxonomy, genome assembly, gene count, chromosomes
- Source: built-in database (from BioGist's SPECIES_DATA, offline)

### Tab 3: Tools (Quick Sequence Tools)

**Offline-capable sequence analysis — runs on BioLang WASM:**

#### Sequence Input
- Paste or type DNA/RNA/protein sequence
- Auto-detect sequence type
- Max 10,000 characters (mobile memory constraint)

#### DNA Tools
- **Reverse complement** — instant, tap to copy result
- **Transcribe** (DNA → RNA) — T → U conversion
- **Translate** (DNA → Protein) — all 6 reading frames option
- **GC content** — percentage with visual bar
- **Base composition** — A/T/G/C/N counts and percentages
- **Sequence length** — bp count
- **Find motif** — search for pattern (regex supported)
- **K-mer analysis** — generate k-mers of specified length
- **Restriction sites** — find common enzyme cut sites

#### Protein Tools
- **Molecular weight** — calculated from amino acid composition
- **Amino acid composition** — counts and percentages
- **Isoelectric point** — estimated pI
- **Hydrophobicity** — Kyte-Doolittle plot (simple text visualization)

#### Converters
- **DNA ↔ RNA** — quick conversion
- **One-letter ↔ Three-letter** amino acid codes
- **Sequence formatter** — FASTA format, fixed-width lines

### Tab 4: Reference (Offline Tables)

**Bundled reference data — works completely offline:**

#### Codon Table
- Standard genetic code (64 codons → 20 amino acids + stop)
- Tap codon → shows amino acid, abbreviation, properties
- Color-coded by amino acid property (hydrophobic, polar, charged, etc.)
- Search by codon or amino acid

#### Amino Acid Properties
- All 20 standard amino acids
- Properties: MW, pI, charge at pH 7, hydrophobicity, side chain
- Sort by any property
- One-letter and three-letter codes

#### Restriction Enzymes
- Common enzymes (~50 most used)
- Recognition sequence, cut position, commercial source
- Search by name or recognition site
- "Find in my sequence" button (links to Tools tab)

#### Nucleotide Bases
- A, T, G, C, U — structure, pairing rules, modifications
- Wobble base pairing rules
- Common modifications (5mC, m6A, etc.)

#### Abbreviations
- Common bioinformatics abbreviations (PCR, qPCR, WGS, WES, RNA-seq, ChIP-seq, etc.)
- Expandable with definitions

#### Unit Conversions
- bp / kb / Mb / Gb
- ng / µg / mg
- nM / µM / mM / M
- Daltons / kDa
- OD600 → cell count estimation

### Tab 5: Settings

- **Watchlist management** — same as desktop (add/remove/priority)
- **NCBI API key** — optional, increases rate limit
- **Notifications** — push notification toggle, frequency
- **Theme** — dark / light / system
- **Offline data** — manage cached lookups, clear cache, storage usage
- **Export/Import** — watchlist JSON, sync with desktop
- **About** — version, help link, report issues

---

## Mobile-Specific UX

### Gestures
| Gesture | Action |
|---|---|
| Swipe right on paper | Mark as read |
| Swipe left on paper | Save to reading list |
| Long press entity | Quick actions menu (watch, lookup, copy) |
| Pull down | Refresh feed |
| Tap Signal Score badge | Show breakdown popover |
| Shake device | Quick lookup mode (opens search) |

### Bottom Sheet Modals
- Entity detail (replaces full-page navigation)
- Signal Score breakdown
- Add to watchlist confirmation
- Tool results (reverse complement, translate, etc.)

### Haptic Feedback
- On watchlist add/remove
- On paper save
- On copy to clipboard

### Responsive Breakpoints
- **< 480px** — single column, bottom tab bar, cards full-width
- **480-768px** — tablet: sidebar + main area
- **> 768px** — desktop PWA layout (handled by existing code)

---

## Offline Capability

### Always offline (bundled):
- Codon table
- Amino acid properties
- Restriction enzymes
- Nucleotide bases
- Abbreviations
- Unit conversions
- Species database (43 organisms from BioGist)
- Sequence tools (WASM)

### Cached after first use (auto):
- Gene lookups (24h cache)
- Variant lookups (24h cache)
- Drug lookups (24h cache)
- Paper abstracts (until cleared)
- Watchlist (always synced to IndexedDB)

### Requires network:
- Paper feed refresh
- New entity lookups (first time)
- PubMed search
- Concept expansion suggestions
- Trend data

---

## Data Sync (Desktop ↔ Mobile)

No server — sync via export/import:

### Manual sync
- Export watchlist as JSON from desktop → import on mobile (and vice versa)
- Export reading list as JSON

### Automatic sync (future v2)
- Option 1: Shared IndexedDB via same origin (lang.bio) — works if both use the PWA
- Option 2: Chrome storage sync (`chrome.storage.sync`) — extension only, 100KB limit
- Option 3: URL-encoded watchlist share link

**v1: manual JSON export/import is sufficient.** Same origin PWA naturally shares IndexedDB.

---

## Performance Constraints

| Constraint | Limit | Mitigation |
|---|---|---|
| WASM download | ~4MB first load | Cache in service worker, show progress |
| IndexedDB size | ~50MB practical limit | Cap papers at 500, auto-prune old entries |
| Service worker | 30s execution limit | Max 5 API calls per wake cycle |
| Memory | ~100MB practical limit | Sequence tools cap at 10,000 chars |
| Battery | Background sync drains battery | Adaptive polling (rare entities weekly) |

---

## Privacy

Same model as desktop:
- **No server, no account** — all data in IndexedDB
- **No tracking** — no analytics, no telemetry
- **API calls only for lookups** — PubMed, NCBI, UniProt, ClinVar (all public)
- **Offline-first** — reference tables and cached data work without network

---

## Development Plan

Mobile is not a separate build — it's the same PWA with responsive layout:

### Phase 1 — Responsive layout
- Add mobile breakpoints to existing `biokhoj/index.html`
- Bottom tab bar for mobile (Feed / Lookup / Tools / Reference / Settings)
- Touch gestures (swipe, long press)
- Bottom sheet modals for detail views

### Phase 2 — Quick Lookup tab
- Gene/Variant/Drug/Species lookup forms
- API integration (reuse BioKhojCore)
- Result caching for offline

### Phase 3 — Sequence Tools tab
- Load BioLang WASM
- Input → reverse complement, translate, GC%, etc.
- Results with copy buttons

### Phase 4 — Reference Tables tab
- Bundled JSON data files
- Codon table, amino acids, restriction enzymes
- Search and sort within tables
- Fully offline

### Phase 5 — Polish
- Haptic feedback
- Smooth animations
- PWA install prompt
- Offline indicator
- Storage management

---

## File Structure

No new files needed — mobile features integrate into existing PWA:

```
website/biokhoj/
├── index.html          # Add mobile breakpoints, new tabs
├── app.js              # Add lookup, tools, reference logic
├── sw.js               # Already handles offline caching
├── manifest.json       # Already PWA-ready
├── data/
│   ├── codons.json     # Codon table (bundled)
│   ├── amino-acids.json # Amino acid properties (bundled)
│   ├── enzymes.json    # Restriction enzymes (bundled)
│   └── abbreviations.json # Bio abbreviations (bundled)
└── help.html           # Add mobile-specific sections
```

WASM loaded from `/wasm/bl_wasm.js` (already deployed on lang.bio).
