# BioPeek — Changelog

## v1.3.0 (unreleased)

### New Features
- **Sequence color toggle** — toolbar button to turn nucleotide/amino acid coloring on/off. Persisted across sessions. Affects DNA (A/T/C/G), protein (Zappo scheme), codon, and motif coloring.
- **Protein FASTA support** — auto-detects protein sequences (.faa or by content analysis). Amino acids colored by physicochemical property (hydrophobic=amber, aromatic=purple, polar=green, positive=blue, negative=red, special=cyan). Protein-specific stats (top residues, composition). GC% offer suppressed for protein files.
- **Chromosome tag toggle highlight** — clicking a chromosome/region tag in the stats strip now shows a visible active state (accent border, tinted background, bold text). Multi-select supported. Click again to deselect. Clear visual feedback for which filter is active.
- **NCBI sequence fetch** — fetch FASTA sequences by accession (GenBank, RefSeq, UniProt) from the drop zone or toolbar. Batch fetch supports multiple accessions comma/space separated. 15s timeout with AbortController.
- **NCBI fetch modal** — styled CSS modal replaces browser prompt(). Shows helper text, supports Enter/Escape, click-outside-to-close.
- **IUPAC motif expansion** — motif search now supports IUPAC ambiguity codes (R=[AG], Y=[CT], S=[GC], W=[AT], K=[GT], M=[AC], B=[CGT], D=[AGT], H=[ACT], V=[ACG], N=[ACGT]). Pure DNA patterns auto-expanded; regex patterns passed through unchanged.
- **GC% sliding window plot** — FASTA stats view shows a line chart of GC% across a 100bp sliding window for the first sequence. Y-axis 0-100%, 50% reference line, ~200 sample points.
- **K-mer frequency table** — FASTA stats view shows top 20 4-mers with count, frequency %, and bar chart. Sampled from first 5 sequences.

- **Responsive Display dropdown** — toolbar toggle buttons (Color, Heatmap, Detail, Bookmarks, Pin, Transpose, Highlight, Split) collapse into a "Display" dropdown on narrow screens. Auto-expands to inline buttons on wide screens (>1200px). Active toggles show amber border and checkmarks.
- **Heatmap toggle** — global on/off from Display menu. Auto-detects numeric columns including CSV string values. Amber highlight when active. Resets per tab.
- **Streaming mode** — files >10MB: counts records via chunk scan, parses first 5MB as preview. "Load All Records" for full parse. Memory-efficient.
- **Sticky summary row** — mean/unique counts row sticks below column headers while scrolling.
- **First/Last pagination** — ⏮ First and Last ⏭ buttons added to pagination bar.
- **Version update banner** — returning users see one-time dismissable banner after updates.

### Fixes
- **Stack overflow on large files** — Math.min/max.apply replaced with loop-based safeMin/safeMax (crashes at >30K elements)
- **Heatmap not applying on tab switch** — cache invalidation + parseFloat on string columns
- **HTML export page-only** — now exports all filtered rows (capped at 10K for HTML, unlimited for CSV)
- **Screenshot quality bars** — renders heatmap bars matching table view, not raw ASCII chars
- **Screenshot color toggle** — respects on/off state for sequence coloring
- **Screenshot heatmap** — includes heatmap gradient on numeric cells
- **Race condition guards** — FileReader callbacks check activeTab hasn't changed
- **Light theme nucleotide colors** — darkened for readability on white background
- **Highlight dialog** — shows active rule, pre-populates column/operator/value
- **Toolbar chips after Load All Records** — updates record count and drops streaming label
- **Filter cache key** — includes Set values to prevent stale multi-select results
- **GC% offer** — suppressed when gc_pct column already exists from parser
- **Partial load message** — shows actual bytes loaded, not hardcoded "50 MB"
- **GZ files via extension popup** — binary data preserved through session storage
- **Sequence overflow in detail panel** — scrolls inside 200px container
- **Overlay auto-cleanup** — dismissed on tab switch/close/file load
- **Sort/filter icons** — SVG funnel + visible resize handles on column headers
- **innerHTML XSS prevention** — replaced innerHTML += with createElement/appendChild

### Improvements
- **Early development banner removed** — replaced with clean feedback link
- 91 automated tests (was 0)
- Help file updated with all v1.3.0 features and bug fixes

## v1.2.0 (published)

- Initial Chrome Web Store release
- FASTA, FASTQ, VCF, BED, GFF, SAM, CSV/TSV support
- Quality heatmaps, variant tables, sequence coloring
- Motif search with regex patterns
- Genomic coordinate navigation
- Multi-tab with file diff
- BioLang WASM console
- Dark/light themes
- Context menus: "Open in BioPeek", "Analyze selection"
- GZ transparent decompression via DecompressionStream
- Large file streaming with chunked preview
- Session restore via IndexedDB
- Export: CSV, TSV, BED, VCF, HTML, screenshot
