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

### Fixes
- **GZ files via extension popup** — opening .gz files from the popup or context menu no longer shows "Binary file" error. Binary data preserved through base64 session storage. Files >7MB open viewer directly for drag-and-drop.
- **Sequence overflow in detail panel** — long sequences no longer break out of the detail overlay. Sequences scroll inside a 200px max-height container with proper word-break.
- **Sticky table headers** — column headers now reliably stay fixed when scrolling through data rows (z-index increased to 4).
- **Overlay auto-cleanup** — warnings, error popups, GC% offers, and preview banners automatically dismissed when switching tabs, closing tabs, or loading new files.

### Improvements
- **Early development banner removed** — replaced with clean feedback link
- Help file updated with protein support, gz fixes, changelog

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
