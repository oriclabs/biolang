# BioTally — Requirements

**Tagline:** Lab math, one click away.

**What it does:** Chrome sidebar extension with biology-specific calculators, unit converters, and reference tables. Always accessible while reading papers or working at the bench. No server, no account, works offline.

**Positioning:** The scientific calculator every biology researcher installs and never uninstalls.

---

## Why This Matters

Researchers do these calculations **10+ times per day**:
- "How much stock solution do I add for a 50µM working concentration?"
- "What's the Tm of this primer?"
- "How many copies of my plasmid are in 100ng?"
- "Convert OD600 to cells/mL"

Currently they: Google formulas → open a random website → enter values → close tab → repeat tomorrow. BioTally makes it one click in the sidebar, every time.

---

## Tech Stack

| Layer | Technology |
|---|---|
| UI | Vanilla JS + inline CSS (same pattern as BioGist/BioKhoj) |
| Storage | `chrome.storage.local` for recent calculations and favorites |
| Math | Pure JavaScript — no APIs, no network, fully offline |
| Framework | None — zero dependencies, instant load |

**No permissions needed except:**
- `sidePanel` — display sidebar
- `storage` — save recent calculations and preferences

**No `activeTab`, no `scripting`, no `tabs`, no `host_permissions`.** Fastest possible Chrome Web Store approval.

---

## Architecture

```
website/extension/biotally/
├── REQUIREMENTS.md
├── STORE_LISTING.md
├── chrome/
│   ├── manifest.json
│   ├── sidebar.html      # All UI + CSS
│   ├── sidebar.js         # All calculator logic
│   ├── help.html          # Self-contained help
│   └── icons/
│       ├── icon16.png
│       ├── icon32.png
│       ├── icon48.png
│       └── icon128.png
└── screenshots/
```

---

## App Layout

### Header
- 🧮 BioTally logo + brand
- Settings gear (theme toggle, decimal precision)
- History button (recent calculations)

### Calculator Selector
- Grid of calculator category buttons (like a phone calculator app selector)
- Categories: Solutions, DNA/RNA, Protein, Cells, PCR, Conversions, Reference

### Calculator Area
- Input fields with labels and units
- Calculate button (saffron/green)
- Result displayed prominently
- "Copy result" button
- "Save to history" auto-saves

### Bottom
- Recent calculations strip (last 5, tap to reload)

---

## Calculators

### Solutions & Dilutions

| Calculator | Inputs | Output | Formula |
|---|---|---|---|
| **Dilution (C1V1=C2V2)** | Stock conc, desired conc, desired volume | Volume of stock to add | V1 = C2×V2/C1 |
| **Serial dilution** | Stock conc, dilution factor, number of steps | Concentrations at each step | C×(1/factor)^n |
| **Percent solution** | Solute mass, total volume | w/v %, molarity | mass/volume×100 |
| **Molarity** | Mass (g), molecular weight (Da), volume (L) | Concentration (M, mM, µM, nM) | M = mass/(MW×vol) |
| **Mass from molarity** | Desired molarity, MW, volume | Mass to weigh | mass = M×MW×vol |
| **Buffer preparation** | Buffer type (dropdown), desired volume, concentration | Recipe (component masses/volumes) | Built-in recipes |

**Buffer presets:**
- PBS (1×, 10×)
- TBS (1×, 10×)
- TAE (1×, 50×)
- TBE (1×, 5×)
- RIPA
- Laemmli (2×, 4×)
- TE buffer
- Tris-HCl (various pH)
- HEPES
- MOPS

### DNA / RNA

| Calculator | Inputs | Output | Formula |
|---|---|---|---|
| **DNA/RNA concentration** | OD260, dilution factor, type (dsDNA/ssDNA/RNA/oligo) | ng/µL | OD×factor×50/40/33 |
| **DNA copy number** | Mass (ng), plasmid size (bp) | Number of copies | (mass×6.022e23)/(length×660×1e9) |
| **Moles of DNA ends** | Mass (ng), fragment size (bp) | pmol of ends | mass/(660×size)×1e6 |
| **DNA molecular weight** | Sequence or length (bp), type | MW in Daltons | bp×660 (dsDNA) or bp×330 (ssDNA) |
| **µg to pmol** | Mass (µg), size (bp) | pmol | µg×1e6/(660×bp) |
| **Resuspension** | Mass (µg), desired conc (ng/µL) | Volume to add (µL) | mass×1000/conc |

### PCR & Primers

| Calculator | Inputs | Output | Formula |
|---|---|---|---|
| **Primer Tm** | Sequence (up to 60 nt) | Tm (°C) — basic, salt-adjusted, nearest-neighbor | Multiple methods |
| **Primer Tm (basic)** | Sequence | 2(A+T) + 4(G+C) | Wallace rule |
| **Primer Tm (salt-adjusted)** | Sequence, [Na+] | Adjusted Tm | 81.5 + 16.6×log[Na+] + 41×(G+C)/N - 675/N |
| **Annealing temperature** | Tm of primer 1, Tm of primer 2 | Ta | (Tm1 + Tm2)/2 - 3 |
| **GC content** | Sequence | % GC | (G+C)/total × 100 |
| **Primer mass** | Sequence, nmol delivered | µg | nmol × MW / 1000 |
| **PCR product size** | Forward primer pos, reverse primer pos | Size (bp) | reverse - forward |

### Protein

| Calculator | Inputs | Output | Formula |
|---|---|---|---|
| **Protein MW** | Amino acid sequence or count | kDa | Sum of AA weights - (n-1)×18 |
| **Extinction coefficient** | Amino acid sequence | ε at 280nm (M⁻¹cm⁻¹) | Pace method: nW×5500 + nY×1490 + nC×125 |
| **Protein concentration** | A280, extinction coefficient, dilution | mg/mL | A280 / (ε × path × dilution) |
| **Isoelectric point** | Amino acid sequence | Estimated pI | Bisection method on Henderson-Hasselbalch |
| **Protein molarity** | mg/mL, MW (kDa) | µM, nM | (mg/mL × 1000) / MW |
| **SDS-PAGE migration** | Log MW vs Rf (standard curve input) | Estimated MW of unknown band | Linear regression |

### Cell Biology

| Calculator | Inputs | Output | Formula |
|---|---|---|---|
| **Cell density** | OD600, organism (E. coli / yeast / mammalian) | Cells/mL | OD × conversion factor |
| **Cell dilution** | Current density, target density, final volume | Volume of culture + media | C1V1=C2V2 |
| **Doubling time** | Initial count, final count, time elapsed | Doubling time | t × ln2 / ln(Nf/Ni) |
| **Generation number** | Initial count, final count | Number of generations | log2(Nf/Ni) |
| **Viability** | Live cells, dead cells | % viability | live/(live+dead)×100 |
| **Hemocytometer** | Cell count, squares counted, dilution | Cells/mL | (count/squares) × dilution × 10⁴ |
| **Seeding density** | Target cells, well plate type, volume | Cells per well, cells/mL | Lookup table by plate type |

**Well plate types:** 6-well, 12-well, 24-well, 48-well, 96-well, 384-well (with growth area and recommended volumes)

### Ligation & Cloning

| Calculator | Inputs | Output | Formula |
|---|---|---|---|
| **Ligation ratio** | Vector size (bp), insert size (bp), vector mass (ng), desired molar ratio | Insert mass needed (ng) | (insert bp / vector bp) × vector ng × ratio |
| **Transformation efficiency** | Colony count, DNA mass (ng), dilution factor | CFU/µg | (colonies × dilution) / (DNA ng / 1000) |
| **Colony screening** | Expected positive rate, desired confidence | Number of colonies to screen | Based on binomial probability |

### Unit Conversions

| Category | Conversions |
|---|---|
| **Length** | bp ↔ kb ↔ Mb ↔ Gb, nm ↔ µm ↔ mm ↔ cm ↔ m |
| **Mass** | pg ↔ ng ↔ µg ↔ mg ↔ g ↔ kg, Da ↔ kDa ↔ MDa |
| **Volume** | nL ↔ µL ↔ mL ↔ L |
| **Concentration** | pM ↔ nM ↔ µM ↔ mM ↔ M, ng/µL ↔ µg/mL ↔ mg/mL |
| **Temperature** | °C ↔ °F ↔ K |
| **Centrifuge** | RPM ↔ g-force (with rotor radius input) |
| **Pressure** | atm ↔ bar ↔ psi ↔ Pa ↔ mmHg |
| **Time** | seconds ↔ minutes ↔ hours (for protocol steps) |

### Quick Reference Tables

| Table | Content |
|---|---|
| **Codon table** | 64 codons → amino acids, interactive, color-coded by property |
| **Amino acids** | 20 AAs: 1-letter, 3-letter, MW, pI, hydrophobicity, charge, side chain |
| **Restriction enzymes** | Top 50: name, recognition site, cut position, overhang type, commercial source |
| **Antibiotic concentrations** | Working concentrations for E. coli / mammalian selection markers |
| **Fluorophore guide** | Excitation/emission wavelengths for common fluorophores (GFP, RFP, DAPI, etc.) |
| **Gel recipes** | Agarose % for different DNA size ranges, SDS-PAGE % for protein ranges |
| **Media recipes** | LB, SOC, YPD, M9, DMEM, RPMI — component lists |
| **Biosafety levels** | BSL-1 through BSL-4 summary |

---

## UI Features

### Calculator UX
- **Auto-detect units:** type "50" in a concentration field → dropdown shows M/mM/µM/nM
- **Unit toggle:** click the unit label to cycle through related units
- **Swap button:** on dilution calculators, swap "what you know" and "what you need"
- **Real-time calculation:** result updates as you type (no "Calculate" button needed for simple calcs)
- **Copy result:** one-click copy with units
- **Favorite calculators:** star your most-used ones → they appear first

### History
- Last 20 calculations saved automatically
- Each entry: calculator name, inputs, result, timestamp
- Tap to reload (pre-fills inputs)
- Clear history button
- Export history as CSV

### Search
- Type to search across all calculators and reference tables
- "Tm" → jumps to Primer Tm calculator
- "codon" → jumps to Codon table
- "ampicillin" → shows antibiotic concentration

### Keyboard Shortcuts
| Shortcut | Action |
|---|---|
| `Ctrl+Shift+T` | Toggle BioTally sidebar |
| `Tab` | Next input field |
| `Enter` | Calculate / copy result |
| `/` | Focus search |
| `Escape` | Close overlay / clear |

---

## Design

### Color Scheme
- **Primary:** Emerald green (#10b981) — fresh, scientific, distinct from BioGist (purple) and BioKhoj (saffron)
- **Background:** Dark slate (#020617 / #0f172a)
- **Input fields:** Slightly lighter (#1e293b) with green focus ring
- **Results:** Large, bold, white text on dark background
- **Calculator cards:** Subtle border, hover highlight
- **Light theme:** White background, dark text, same green accent

### Typography
- Numbers: tabular-nums, monospace for alignment
- Labels: system font, small caps for units
- Results: large (18px+), bold

---

## Offline Capability

**100% offline.** No network requests ever. Everything is bundled:
- All calculator formulas
- All reference tables
- All buffer recipes
- All unit conversion factors

---

## Privacy

- **No network requests** — everything runs locally
- **No tracking** — no analytics, no telemetry
- **No data collection** — only stores calculation history in chrome.storage.local
- **App Store privacy label:** "Data Not Collected"
- **Minimal permissions:** sidePanel + storage only

---

## Chrome Web Store

### Permissions (minimal)
```json
{
  "permissions": ["sidePanel", "storage"],
  "action": { ... },
  "commands": { "_execute_action": { "suggested_key": "Ctrl+Shift+T" } }
}
```

**No `activeTab`, no `scripting`, no `tabs`, no `host_permissions`, no `content_scripts`.** Should pass Chrome review in 1-2 days.

### Store Listing

**Name:** BioTally — Lab Calculator

**Short description (132 chars):**
Lab math, one click away. Dilutions, molarity, Tm, copy number, cell density, unit conversions & reference tables. Works offline.

**Category:** Productivity

---

## Relationship to Other Products

| Product | Purpose | BioTally connection |
|---|---|---|
| **BioGist** | Scan papers for entities | Paper mentions "50µM olaparib" → open BioTally for dilution calc |
| **BioKhoj** | Monitor literature | Paper methods say "Tm of 62°C" → verify with BioTally primer Tm |
| **BioGist Studio** | Code notebooks | BioLang scripts handle complex analysis; BioTally for quick bench math |

**No integration needed.** BioTally is standalone — that's its strength. Zero setup, zero learning curve.

---

## Development Plan

### Phase 1 — Core Calculators (Week 1)
- Project setup (manifest, sidebar shell, CSS)
- Solutions: dilution, serial dilution, molarity, mass from molarity
- DNA: concentration, copy number, MW
- Calculator selector grid
- Result display with copy
- History (last 20)

### Phase 2 — PCR & Protein (Week 2)
- PCR: primer Tm (3 methods), annealing temp, GC content
- Protein: MW, extinction coefficient, concentration, pI
- Cells: density, doubling time, hemocytometer, seeding density
- Ligation: ratio, transformation efficiency

### Phase 3 — Converters & Reference (Week 3)
- All unit converters
- Codon table (interactive)
- Amino acid table
- Restriction enzymes
- Antibiotic concentrations
- Fluorophore guide
- Buffer/media recipes

### Phase 4 — Polish (Week 4)
- Search across all calculators
- Favorite calculators
- Real-time calculation (as you type)
- Keyboard shortcuts
- Light theme
- Help page
- Icons and screenshots
- Chrome Web Store submission

---

## Success Metrics

- **Install → first calculation:** under 10 seconds
- **Daily active users:** high (calculators are used daily)
- **Uninstall rate:** very low (always useful)
- **Store rating:** 4.5+ (simple tools get good ratings)
- **Review approval:** 1-2 days (minimal permissions)
