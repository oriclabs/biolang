# BioKhoj Native App — Requirements

**Tagline:** The complete biology toolkit in your pocket.

**Positioning:** Bloomberg terminal for biology signals — research awareness infrastructure + lab companion.

**What it does:** Native mobile app combining literature monitoring with Signal Score ranking, rule-based entity extraction, study context detection, clinical actionability indicators, sequence analysis, file inspection, visualization, and lab tools — all offline-capable, no AI/LLM required.

**Constraint:** All intelligence is dictionary-based, regex-based, or heuristic-based. No LLM or AI API calls. Minimal battery usage.

---

## Why Native Over PWA

| Capability | PWA Limitation | Native Advantage |
|---|---|---|
| Visualization | Canvas/SVG, janky on large data | GPU-accelerated, smooth 60fps |
| File handling | No filesystem access | Open BAM/VCF/FASTA from Files app, iCloud, Google Drive |
| Camera | Basic Web API | Full camera control, ML-based gel/plate analysis |
| Performance | WASM interpreter, ~10x slower | Rust compiled to ARM, native speed |
| Offline | Service worker (fragile on iOS) | Everything bundled, rock solid |
| Notifications | Web Push (broken on iOS < 16.4) | Native push, always reliable |
| Background | Killed after 30s | Proper background fetch, scheduled tasks |
| Storage | IndexedDB (~50MB practical) | SQLite, unlimited local storage |
| App Store | Not listed, hard to discover | Searchable, reviewable, credible |

---

## Tech Stack

| Layer | Technology | Why |
|---|---|---|
| **Core logic** | Rust (reuse BioLang crates) | Already written, cross-platform, fast |
| **Cross-platform** | Tauri Mobile | Rust backend + web frontend, ships iOS + Android |
| **UI** | HTML/CSS/JS (Tailwind + saffron theme) | Reuse existing PWA UI, same design language |
| **Database** | SQLite via `rusqlite` | Fast, offline, unlimited size |
| **Visualization** | Canvas + WebGL (via Tauri webview) | GPU access through native layer |
| **Bio I/O** | `noodles` (Rust crate) | Already used in BioLang for BAM/SAM/VCF/FASTQ |
| **Sequence ops** | BioLang runtime (Rust) | Translate, complement, GC%, k-mers — native speed |
| **Camera** | Native API via Tauri plugin | Full camera access for gel/plate imaging |
| **Notifications** | Native push via Tauri plugin | Reliable on both platforms |
| **Entity dictionaries** | Bundled HGNC, DrugBank subset, MeSH terms | Offline extraction, no API needed |

### Rust Crate Reuse from BioLang

| Crate | What it provides |
|---|---|
| `bl-core` | AST, Value types, Table, BioSequence |
| `bl-lexer` + `bl-parser` | Parse BioLang scripts (for power users) |
| `bl-runtime` | Builtins: translate, complement, gc_content, kmers, etc. |
| `bl-bio` | FASTA/FASTQ/BED/GFF/VCF I/O via noodles |
| `bl-apis` | NCBI, UniProt, Ensembl, KEGG API clients |
| `bio-core` | BioSequence, GenomicInterval, Strand types |

### Build Targets

| Platform | Target | Min Version |
|---|---|---|
| iOS | aarch64-apple-ios | iOS 15+ |
| Android | aarch64-linux-android, armv7-linux-androideabi | Android 8+ (API 26) |

---

## App Structure

### Bottom Tab Bar (6 tabs)

```
┌─────────────────────────────────────┐
│            BioKhoj                  │
│                                     │
│        [Active Tab Content]         │
│                                     │
├──────┬──────┬─────┬─────┬────┬─────┤
│ Feed │Lookup│Tools│Files│ Lab│  ⚙  │
│  📰  │  🔍  │ 🧬  │ 📂  │ 🔬 │     │
└──────┴──────┴─────┴─────┴────┴─────┘
```

---

## Tab 1: Feed (Literature Monitoring & Intelligence)

### 1.1 Literature Monitoring

- Monitor **PubMed**, **bioRxiv**, **OpenAlex**, **CrossRef** for new papers
- Watch biological entities:
  - Genes (HGNC symbols)
  - Drugs (generic/brand names)
  - Variants (rsIDs, HGVS, ClinVar)
  - Diseases (MeSH / ICD vocabulary)
  - Pathways (KEGG, Reactome, GO)
  - Authors (by name or ORCID)
- Entity-grouped paper feed with unread counts per entity
- Priority levels: high / normal / low with drag-to-reorder
- Background refresh polling (native scheduler, adaptive frequency)
- Offline cache of recent papers

### 1.2 Paper Feed

- Paper cards: title (2 lines), authors, journal, date, Signal Score badge
- Matched watched entities shown as colored chips on each card
- Watched entities highlighted in abstract preview
- Swipe right → mark read (with haptic)
- Swipe left → save to reading list
- Tap → expand: full abstract, structured entity summary, context snapshot, action buttons
- Long press → share, cite, watch entities
- Co-mention alerts pinned at top
- Infinite scroll with lazy loading
- Pull-to-refresh triggers background check

### 1.3 Rule-Based Entity Extraction (Abstract Parsing)

Extract entities from paper abstracts using dictionaries and regex — no AI:

| Entity Type | Source Dictionary | Method |
|---|---|---|
| Genes | HGNC (44,959 symbols) | Dictionary lookup + word boundary regex |
| Drugs | DrugBank subset (~2,000 names) | Case-insensitive dictionary match |
| Variants | — | Regex: rsIDs, HGVS, ClinVar VCV, COSMIC |
| Diseases | MeSH / ICD vocabulary | Dictionary match + common name patterns |
| Organisms | Built-in list (43 species) | Scientific + common name regex |
| Accessions | — | Regex: GEO, SRA, BioProject, DOI, PMID |
| Clinical trials | — | Regex: NCT numbers |
| Funding | — | Regex: NIH grants, ERC, NSF |
| Repositories | — | URL pattern: GitHub, Zenodo, PyPI |
| P-values | — | Regex: p < 0.05, FDR, q-value |

Display structured **entity summary card** per paper showing all extracted entities grouped by type.

### 1.4 Study Context Detection (Heuristic)

Detect study characteristics from title/abstract using keyword matching:

**Study type badges:**

| Badge | Keywords detected |
|---|---|
| Randomized Clinical Trial | "randomized", "RCT", "double-blind", "placebo-controlled" |
| Cohort Study | "cohort", "prospective", "retrospective", "longitudinal" |
| Meta-analysis | "meta-analysis", "systematic review", "pooled analysis" |
| In Vitro | "in vitro", "cell culture", "cell line", "transfected" |
| Animal Model | "mouse model", "murine", "xenograft", "in vivo", "knockout mice" |
| Case Report | "case report", "case series" |
| Computational | "in silico", "computational", "bioinformatics", "machine learning" |

**Methodology keywords detected:**

| Category | Keywords |
|---|---|
| Sequencing | RNA-seq, WGS, WES, scRNA-seq, ChIP-seq, ATAC-seq, long-read |
| Editing | CRISPR, Cas9, Cas12, base editing, prime editing, guide RNA |
| Genetics | GWAS, linkage, association study, QTL, eQTL, fine-mapping |
| Proteomics | mass spectrometry, LC-MS, proteomics, phosphoproteomics |
| Imaging | microscopy, cryo-EM, X-ray crystallography, fluorescence |

**Additional detection:**
- Sample size mentions (n=X, N patients, cohort of X)
- Validation cohort indicators ("validation cohort", "independent replication")

Display as **compact badges** on each paper card.

### 1.5 Research Context Snapshot

Extracted per paper and displayed as a **context card**:

| Field | Detection method |
|---|---|
| Species studied | Dictionary match (Human, Mouse, Rat, etc.) |
| Tissue / cell type | Dictionary match (blood, liver, tumor, HeLa, HEK293, etc.) |
| Disease focus | MeSH term match (cancer type, cardiovascular, neurological, etc.) |
| Pathway involvement | KEGG/Reactome/GO term detection |
| Technology used | Methodology keywords (see above) |
| Genome build | Regex: GRCh38, hg19, mm10, etc. |

Shown as a **compact card** below the abstract — one glance tells you if the paper matches your model system.

### 1.6 Paper Signal Scoring (Lightweight Heuristics)

Compute paper importance score (0-100) using **no AI**:

```
signal = recency_weight        (0-25)  # newer = higher, 30-day decay
       + citation_velocity     (0-20)  # citations/month (OpenAlex)
       + journal_tier          (0-15)  # built-in tier list (~100 journals)
       + co_mention_novelty    (0-20)  # first time entity A + B together
       + entity_match_count    (0-10)  # how many watched entities match
       + author_reputation     (0-10)  # h-index via OpenAlex
```

**Signal badges:**

| Score | Badge | Color | Meaning |
|---|---|---|---|
| >= 70 | ★ High Signal | Purple | Must-read paper |
| 40-69 | Exploratory | Saffron | Worth a look |
| < 40 | Preliminary | Grey | Low relevance, collapsed by default |

**Score breakdown popover:** tap badge → animated breakdown showing each component's contribution. Builds trust and transparency.

**Journal tier list (built-in, editable):**
- Tier 1 (15pts): Nature, Science, Cell, NEJM, Lancet, JAMA
- Tier 2 (10pts): Nature Genetics, Nature Medicine, PNAS, Genome Research
- Tier 3 (5pts): PLOS, BMC, Frontiers, MDPI
- Unranked (0pts): preprints, unknown journals

### 1.7 Clinical / Actionability Indicators

Detect clinical relevance from abstract keywords:

| Indicator | Keywords | Badge |
|---|---|---|
| Biomarker | "biomarker", "diagnostic marker", "prognostic marker" | 🎯 Biomarker |
| Therapeutic target | "therapeutic target", "druggable", "drug target" | 💊 Target |
| Drug resistance | "resistance", "resistant", "refractory", "non-responder" | ⚠️ Resistance |
| Prognosis | "prognosis", "survival", "outcome", "prognostic" | 📊 Prognosis |
| FDA/EMA | "FDA approved", "EMA approved", "breakthrough therapy" | ✅ Approved |

Show as **clinical relevance hint badges** on paper cards. Pharma analysts and clinicians filter by these.

### 1.8 Co-Mention Intelligence

- Detect novel entity pairs across papers
- Maintain co-mention frequency history in SQLite
- Highlight **emerging biological associations** ("First paper linking BRCA1 + sotorasib")
- Trend sparkline per entity pair (publication frequency over time)
- Alert: "Novel co-mention detected" pinned to top of feed
- Co-mention network view: simple node graph of frequently co-mentioned entities

### 1.9 Concept Expansion Engine

When a user watches an entity, suggest related concepts:

- Query **OpenAlex concepts API** for related terms
- Query **MeSH tree** for parent/child/sibling terms
- Use **co-mention data** from cached papers
- Show suggestion card: "Also watch MDM2? apoptosis? Li-Fraumeni?" with one-click add
- Not AI — just graph lookups from public ontologies

### 1.10 Trend Analytics

- Publication frequency trends per entity (line chart, GPU-rendered)
- Identify **rapidly rising genes or drugs** ("TP53 mentions up 40% this month")
- Weekly research pulse summary (auto-generated Markdown)
- Compare multiple entities on same chart
- Time ranges: 1 month, 3 months, 1 year, 5 years
- Sparkline preview in watchlist sidebar

---

## Tab 2: Lookup (Cross-Database Smart Linking)

Instant biological lookups with offline caching:

### Gene Lookup
- Search by symbol, name, or Ensembl ID
- Result card:
  - Full name, chromosome location, strand
  - **UniProt function summary**
  - **KEGG / Reactome pathways**
  - **ClinVar variant links** for this gene
  - Associated diseases (OMIM)
  - Expression (GTEx tissue bar chart — native rendered)
  - Orthologs (key model organisms)
  - Drug interactions
  - **Recent literature** (top 3 PubMed results)
  - Links: NCBI, Ensembl, UniProt, GeneCards, OMIM, PubMed, Scholar
- "Watch in BioKhoj" button
- Cached after first lookup (offline available)

### Variant Lookup
- Search by rsID, HGVS, ClinVar ID, or chromosomal position
- Result card:
  - Clinical significance badge (pathogenic/benign/VUS) — color coded
  - **Pathogenicity** details (ClinVar)
  - **Allele frequency** (gnomAD) — population bar chart (native rendered)
  - Population breakdown: AFR, AMR, EAS, EUR, SAS
  - Consequence (missense, nonsense, synonymous, etc.)
  - Associated conditions
  - Pharmacogenomics relevance (CPIC guidelines if available)
  - Links: dbSNP, ClinVar, gnomAD
- Sources: myvariant.info, ClinVar API

### Drug Lookup
- Search by generic name, brand name, or compound ID
- Result card:
  - Drug class, **mechanism of action**
  - **Target genes**
  - FDA approval status
  - Indications
  - Pharmacogenomics (which genes affect metabolism)
  - **Active clinical trial count** (ClinicalTrials.gov)
  - Links: DrugBank, RxList, ClinicalTrials.gov, PubMed

### Disease Lookup
- Search by name or OMIM ID
- Result card:
  - Description, inheritance pattern
  - Associated genes
  - Prevalence
  - Links: OMIM, OrphaNet, MedlinePlus

### Pathway Lookup
- Search by name or KEGG/Reactome/GO ID
- Result card:
  - Pathway description
  - Gene members
  - Interactive pathway diagram (simple, native rendered)
  - Links: KEGG, Reactome, QuickGO

---

## Tab 3: Tools (Sequence Analysis)

All tools run **natively via Rust** — instant results, no WASM overhead:

### Sequence Input
- Paste, type, or **open from file** (FASTA from phone storage)
- Auto-detect: DNA / RNA / Protein
- Support up to 1 million characters (native speed)
- Sequence viewer with base coloring

### DNA Tools
| Tool | Output |
|---|---|
| Reverse complement | Reversed + complemented sequence |
| Transcribe | RNA (T→U) |
| Translate | Protein (all 6 frames option) |
| GC content | Percentage + visual bar |
| Base composition | A/T/G/C/N counts + pie chart |
| Sequence length | Length in bp |
| Find motif | Positions, count, highlighted |
| K-mer analysis | Frequency table + chart |
| Restriction sites | Enzyme, position, cut pattern |
| ORF finder | Open reading frames with start/stop |
| Tm calculator | Melting temperature (nearest-neighbor, <60bp) |
| Primer design | Forward/reverse primers with Tm matching |

### RNA Tools
| Tool | Output |
|---|---|
| Reverse transcribe | cDNA |
| Translate | Protein |
| Codon usage | Codon frequency table |

### Protein Tools
| Tool | Output |
|---|---|
| Molecular weight | MW in Daltons/kDa |
| Amino acid composition | Counts + percentages + chart |
| Isoelectric point | Estimated pI |
| Hydrophobicity plot | Kyte-Doolittle graph (native rendered) |
| Extinction coefficient | ε at 280nm |
| One-letter ↔ Three-letter | Converted amino acid codes |
| Charge at pH | Net charge at specified pH |

### Alignment (Basic)
- Pairwise: Needleman-Wunsch (global) / Smith-Waterman (local)
- Score matrix: BLOSUM62 (protein), simple match/mismatch (DNA)
- Identity percentage

### Converters
| From | To |
|---|---|
| FASTA → raw sequence | Strip headers |
| Raw → FASTA | Add header, wrap at 80 chars |
| GenBank → FASTA | Extract sequence |
| Multi-FASTA → table | Sequence stats per entry |

---

## Tab 4: Files (Bio File Inspector)

**Native file system access — the feature PWA can't do:**

### Supported Formats
| Format | Parse | Preview | Stats |
|---|---|---|---|
| FASTA / FA | Full | Sequence viewer with coloring | Count, lengths, GC% |
| FASTQ / FQ | Full | Quality score heatmap | Read count, quality distribution |
| VCF | Full | Variant table with filters | Variant count, type breakdown |
| BED | Full | Interval table, coordinate nav | Region count, total coverage |
| GFF / GTF | Full | Feature table with type filter | Feature count by type |
| SAM | Full | Alignment viewer | Mapped/unmapped, MAPQ |
| BAM | Header + limited | Read count, reference info | Index stats if BAI present |
| CSV / TSV | Full | Table viewer with sort/filter | Row/column count |

### File Sources
- Phone storage (Files app)
- iCloud Drive / Google Drive / Dropbox (via system file picker)
- Shared from other apps
- URL download (paste link → download → inspect)
- Recently opened files list

### File Viewer Features
- Sequence viewer: base coloring (A=green, T=red, G=yellow, C=blue), line numbers
- Quality heatmap: FASTQ Phred scores as color gradient
- Variant table: filterable by chromosome, type, quality
- Coordinate navigation: jump to chr:start-end
- Search within file
- Copy selection to clipboard or to Tools tab
- Stats summary auto-computed on file open
- Share file or stats via system share sheet

### Performance
- Stream large files — noodles Rust crate handles parsing natively
- Preview first 1000 records, "Load more" button
- File size warning for >100MB

---

## Tab 5: Lab (Lab Tools)

### Protocol Timer
- Multiple concurrent timers
- Presets: PCR cycles, incubation, gel electrophoresis
- Custom timer with label
- Background notification when complete
- Timer history log

### Lab Calculator
| Calculator | Formula |
|---|---|
| Dilution | C1V1 = C2V2 |
| DNA concentration | OD260 × dilution factor × 50 |
| Molar concentration | mass / (MW × volume) |
| Copy number | (mass × 6.022e23) / (length × 660) |
| Ligation ratio | (insert size / vector size) × ratio × vector mass |
| Transformation efficiency | colonies / (DNA µg × dilution) |
| Cell density | OD600 × conversion factor |

### Quick Reference Cards (Offline, Bundled)

| Card | Content |
|---|---|
| Codon table | 64 codons, interactive, color-coded by AA property |
| Amino acids | 20 AAs, MW, pI, hydrophobicity, structure |
| Restriction enzymes | Top 100, recognition site, cut position |
| Nucleotide bases | A/T/G/C/U, pairing rules, modifications |
| Buffer recipes | TAE, TBE, PBS, RIPA, Laemmli |
| Media recipes | LB, SOC, YPD, M9, DMEM |
| Antibiotics | Working concentrations for selection markers |
| Centrifuge conversion | RPM ↔ g-force |
| Fluorophore guide | Excitation/emission wavelengths |
| Unit conversions | bp↔kb↔Mb, ng↔µg↔mg, nM↔µM↔mM |
| Biosafety levels | BSL-1 through BSL-4 summary |

### Gel Image Analysis (Camera)
- Photograph agarose/PAGE gel
- Auto-detect lanes and bands
- Estimate band size using MW ladder
- Annotate bands with labels
- Export annotated image
- Save to experiment log

### Sample Logger
- Quick-log: ID, type, date, location, notes
- Barcode/QR scanner (camera)
- Photo attachment
- Search and filter
- Export as CSV

---

## Visualization Features (GPU-Accelerated)

### Genome Region Browser
- Pan and zoom across chromosome coordinates
- Tracks: genes, variants, intervals, custom BED/GFF
- Smooth 60fps native rendering
- Coordinate input: jump to chr1:1000000-2000000

### Expression Heatmap
- Gene × sample heatmap from CSV/TSV
- Color scale: blue-white-red
- Dendogram clustering
- Zoom, pan, tap cell for value

### Variant Frequency Plot
- Manhattan-style plot for VCF data
- Chromosome ideogram with variant density
- Zoom into regions, tap variant → detail

### Phylogenetic Tree Viewer
- Load Newick format
- Radial or rectangular layout
- Collapse clades, color by metadata
- Export SVG/PNG

### Protein 3D Viewer (Future v2)
- Load PDB file or fetch by ID
- Rotate, zoom, pan with touch
- Cartoon, surface, ball-and-stick rendering
- Color by chain, secondary structure, hydrophobicity

---

## Mobile Interaction Workflow

### Gestures
| Gesture | Action |
|---|---|
| Swipe right on paper | Mark as read |
| Swipe left on paper | Save to reading list |
| Long press abstract text | Quick entity watch |
| Tap entity chip | Open quick lookup overlay |
| Pull down | Refresh feed |
| Tap Signal Score badge | Show breakdown popover |
| Long press entity in watchlist | Priority / pause / delete menu |

### Bottom Sheet Modals
- Entity detail overlay (not full-page navigation)
- Signal Score breakdown
- Study context snapshot
- Clinical actionability details
- Add to watchlist confirmation
- Tool results

### Haptic Feedback
- On watchlist add/remove
- On paper save / mark read
- On copy to clipboard
- On timer complete

---

## Offline Capability

### Always offline (bundled):
- HGNC gene dictionary (44,959 symbols)
- DrugBank subset (~2,000 drug names)
- MeSH disease vocabulary
- Species database (43 organisms)
- Study type keywords
- Clinical actionability keywords
- Methodology keywords
- Codon table, amino acids, restriction enzymes
- Buffer/media recipes, unit conversions
- All sequence tools (Rust native)

### Cached after first use:
- Gene/variant/drug lookups (24h cache)
- Paper abstracts (until cleared)
- Trend data (weekly refresh)
- Watchlist (always in SQLite)
- Co-mention history

### Requires network:
- Paper feed refresh
- New entity lookups (first time)
- PubMed/bioRxiv search
- Concept expansion suggestions
- OpenAlex citation data

---

## Data & Storage

### SQLite Schema

```sql
CREATE TABLE watchlist (
  id TEXT PRIMARY KEY,
  name TEXT,
  type TEXT,
  priority TEXT DEFAULT 'normal',
  tags TEXT,           -- JSON array
  created TEXT,
  last_checked TEXT,
  papers_last_week INTEGER DEFAULT 0,
  papers_last_month INTEGER DEFAULT 0,
  muted INTEGER DEFAULT 0,
  notes TEXT DEFAULT ''
);

CREATE TABLE papers (
  pmid TEXT,
  doi TEXT,
  title TEXT,
  authors TEXT,        -- JSON array
  journal TEXT,
  date TEXT,
  abstract TEXT,
  matched_entities TEXT, -- JSON array
  source TEXT,
  signal_score INTEGER DEFAULT 0,
  study_type TEXT,
  context TEXT,        -- JSON: species, tissue, disease, pathway, technology
  clinical_flags TEXT, -- JSON: biomarker, target, resistance, prognosis
  read INTEGER DEFAULT 0,
  starred INTEGER DEFAULT 0,
  notes TEXT DEFAULT '',
  saved_at TEXT,
  PRIMARY KEY (pmid, source)
);

CREATE TABLE co_mentions (
  entity_a TEXT,
  entity_b TEXT,
  first_seen TEXT,
  paper_count INTEGER,
  papers TEXT,          -- JSON array of PMIDs
  PRIMARY KEY (entity_a, entity_b)
);

CREATE TABLE trends (
  entity TEXT,
  month TEXT,
  count INTEGER,
  PRIMARY KEY (entity, month)
);

CREATE TABLE lookups (
  query TEXT PRIMARY KEY,
  type TEXT,
  result TEXT,          -- JSON
  cached_at TEXT
);

CREATE TABLE samples (
  id TEXT PRIMARY KEY,
  type TEXT,
  date TEXT,
  location TEXT,
  notes TEXT,
  photo_path TEXT,
  barcode TEXT,
  created TEXT
);

CREATE TABLE timers (
  id TEXT PRIMARY KEY,
  label TEXT,
  duration_seconds INTEGER,
  started_at TEXT,
  completed INTEGER DEFAULT 0
);
```

### Storage Estimates
| Data | Size |
|---|---|
| HGNC dictionary | ~1.5 MB |
| DrugBank subset | ~200 KB |
| MeSH vocabulary | ~500 KB |
| Reference tables | ~500 KB |
| Watchlist (200 entities) | ~50 KB |
| Papers (500 cached) | ~5 MB |
| Lookup cache (100 entities) | ~2 MB |
| Gel images (10) | ~20 MB |
| Total typical | ~30 MB |

---

## Sync Strategy

### v1: Manual
- Export/import watchlist as JSON
- Same format as PWA and extension
- Share via system share sheet

### v2: Same origin
- PWA at lang.bio/biokhoj shares IndexedDB with mobile browser
- Deep link intercept: lang.bio/biokhoj → open native app

### v3: Cloud sync (optional, opt-in)
- Optional encrypted backend for cross-device sync
- Free tier: 1 device, Pro: unlimited
- Never required — app works fully offline

---

## Performance Constraints

| Constraint | Limit | Mitigation |
|---|---|---|
| Entity extraction | Must be <100ms per abstract | Dictionary hash lookup, compiled regex |
| Signal Score | Must be <10ms per paper | Simple arithmetic, cached values |
| Study type detection | Must be <5ms per abstract | Keyword set membership check |
| File parsing | Stream, never load full file | noodles Rust streaming API |
| SQLite queries | <50ms for any query | Proper indexes on all lookup columns |
| Background refresh | <30s per cycle | Max 5 API calls, 350ms rate limiting |
| Battery | Minimal drain | Adaptive polling: rare weekly, hot daily |
| Memory | <100MB | Sequence tools cap 1M chars, file preview 1000 records |

---

## Privacy

- **No server required** — all data in local SQLite
- **No AI/LLM calls** — all intelligence is rule-based
- **No account for core features** — only cloud sync (v3) needs account
- **No tracking** — no analytics, no telemetry, no ads
- **API calls for lookups only** — PubMed, NCBI, UniProt, ClinVar (all public)
- **Camera data stays local** — gel images stored on device only
- **Bundled dictionaries** — HGNC, DrugBank, MeSH processed locally
- **App Store privacy label:** "Data Not Collected"

---

## Monetization

| Tier | Price | Features |
|---|---|---|
| **Free** | $0 | Feed (10 entities), Lookup, Basic tools, Reference tables |
| **Pro** | $4.99/month or $39.99/year | Unlimited entities, Files tab, Visualizations, Lab tools, Gel analysis, Priority support |
| **Team** (future) | $9.99/user/month | Cloud sync, shared watchlists, team dashboard |

---

## Development Phases

### Phase 1 — Foundation (Week 1-2)
- Tauri Mobile project setup
- Rust core integration (BioLang crates)
- SQLite schema with indexes
- Bottom tab navigation (6 tabs)
- Dark/light theme with saffron primary
- Bundled dictionaries (HGNC, DrugBank, MeSH, species)

### Phase 2 — Feed & Intelligence (Week 3-5)
- Paper feed with Signal Score
- Rule-based entity extraction from abstracts
- Study type detection badges
- Clinical actionability indicators
- Research context snapshot cards
- Co-mention detection and alerts
- Concept expansion suggestions
- Background monitoring via native scheduler
- Push notifications for high-signal papers
- Adaptive polling engine

### Phase 3 — Lookup (Week 6-7)
- Gene/Variant/Drug/Disease/Pathway lookup forms
- API integration via Rust bl-apis
- Result caching in SQLite
- Cross-database smart linking
- Offline lookup for cached entries

### Phase 4 — Sequence Tools (Week 8-9)
- All DNA/RNA/Protein tools via Rust runtime
- Sequence viewer with base coloring
- Pairwise alignment
- ORF finder, Tm calculator, primer design
- Converters

### Phase 5 — Files (Week 10-11)
- File picker integration
- FASTA/FASTQ/VCF/BED/GFF/SAM parsing via noodles
- File viewer with stats
- Coordinate navigation
- Quality heatmap for FASTQ

### Phase 6 — Lab Tools (Week 12-13)
- Protocol timers with background notifications
- Lab calculators (dilution, concentration, copy number, etc.)
- Reference cards (bundled JSON)
- Sample logger with camera/barcode

### Phase 7 — Visualization (Week 14-15)
- Genome region browser (Canvas, GPU)
- Expression heatmap
- Variant frequency plot
- Phylogenetic tree viewer

### Phase 8 — Polish & Launch (Week 16-18)
- Gel image analysis (camera + band detection)
- App Store screenshots and listing
- Performance optimization
- Offline testing across all features
- Beta testing
- Submit to App Store + Google Play

---

## App Store Listing

### Name
BioKhoj — Biology Toolkit

### Subtitle (30 chars)
Research radar & lab companion

### Description
BioKhoj is the complete biology toolkit for researchers on the go. Monitor literature with intelligent scoring, look up genes and variants, analyze sequences, inspect data files, and use lab tools — all without AI, all on your device.

INTELLIGENT LITERATURE MONITORING
- Watch genes, drugs, variants, diseases across PubMed and bioRxiv
- Signal Score ranks papers by importance using journal tier, citation velocity, and co-mention novelty
- Study type badges: RCT, cohort, meta-analysis, in vitro, animal model
- Clinical actionability indicators: biomarker, therapeutic target, drug resistance
- Research context snapshot: species, tissue, disease, technology at a glance
- Concept expansion suggests related entities from OpenAlex and MeSH

INSTANT LOOKUPS
- Gene: function, pathways, diseases, drugs, expression, literature
- Variant: pathogenicity, allele frequency, pharmacogenomics
- Drug: mechanism, targets, trials, interactions

SEQUENCE TOOLS (NATIVE SPEED)
- Reverse complement, translate, GC content, k-mers, ORFs
- Primer design, Tm calculator, restriction sites
- Pairwise alignment, codon usage
- Handles up to 1 million characters

FILE INSPECTOR
- Open FASTA, FASTQ, VCF, BED, GFF, SAM from phone storage
- Quality heatmap, variant tables, coordinate navigation

LAB COMPANION
- Protocol timers with background alerts
- Dilution, concentration, copy number calculators
- Codon table, amino acid reference, buffer recipes, fluorophore guide
- Gel image analysis with camera

PRIVACY FIRST
All data stays on your device. All intelligence is rule-based — no AI, no cloud processing. No account required. No tracking.

### Category
Medical (primary), Education (secondary)

### Keywords
bioinformatics, genomics, gene, variant, sequence, FASTA, VCF, PubMed, research, biology, lab, DNA, RNA, protein

### Price
Free with in-app purchase (Pro: $4.99/month)

---

## Out of Scope (v1)

| Feature | Reason | Revisit |
|---|---|---|
| LLM/AI summarization | Contradicts "no AI" constraint, privacy risk | v2 if users request (opt-in, local model only) |
| Cloud sync | Contradicts privacy-first model | v3 (opt-in, encrypted) |
| Protein 3D viewer | Complex WebGL/Metal rendering | v2 |
| Shared watchlists | No users at launch | v2 |
| Full reference manager | Competes with Zotero/Mendeley | Never |
| Social features | Not aligned with privacy model | Never |
