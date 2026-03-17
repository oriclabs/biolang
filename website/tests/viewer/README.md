# BioPeek Test Suite

Automated test suite for BioPeek (`viewer.html`), using [Playwright](https://playwright.dev/) for end-to-end and visual regression testing.

## Prerequisites

- Node.js 18+

## Installation

```bash
cd website/tests/viewer
npm install
npx playwright install chromium
```

## Running Tests

### All tests

```bash
npm test
```

### Specific suites

```bash
# E2E tests only (file loading, table interactions, features, stats, export)
npm run test:e2e

# Visual regression tests only
npm run test:visual

# Everything
npm run test:all
```

### Individual test files

```bash
npx playwright test e2e/load.spec.js
npx playwright test e2e/table.spec.js
npx playwright test e2e/features.spec.js
npx playwright test e2e/stats.spec.js
npx playwright test e2e/export.spec.js
npx playwright test visual/formats.spec.js
```

## Updating Visual Baselines

Visual regression tests compare screenshots against stored baselines. On the first run, baselines are created automatically. To update them after intentional UI changes:

```bash
npx playwright test visual/ --update-snapshots
```

## Viewing Test Reports

After running tests, an HTML report is generated:

```bash
npx playwright show-report
```

## Test Structure

```
tests/viewer/
  package.json                 # Dependencies and scripts
  playwright.config.js         # Playwright configuration
  README.md                    # This file
  fixtures/                    # Test data files
    sample.fasta               # 3 FASTA sequences
    sample.fastq               # 3 FASTQ reads
    sample.vcf                 # 5 VCF variants
    sample.bed                 # 5 BED regions
    sample.gff                 # 5 GFF features
    sample.csv                 # 5-row gene expression CSV
    sample.tsv                 # 5-row variant TSV
    sample_large.csv           # 1000-row CSV (for pagination/performance)
    generate.js                # Script to regenerate sample_large.csv
  e2e/
    helpers.js                 # Shared test utilities (openViewer, loadFixture, etc.)
    load.spec.js               # Format detection and file loading (7 formats)
    table.spec.js              # Sorting, searching, pagination, row detail
    features.spec.js           # Theme toggle, bookmarks, keyboard shortcuts, tabs
    stats.spec.js              # Statistics view rendering and navigation
    export.spec.js             # CSV/TSV/BED export, download verification
  visual/
    formats.spec.js            # Screenshot baselines for each format + themes
```

## How It Works

- Tests open `viewer.html` via `file://` protocol (no server required)
- Files are loaded by setting the hidden `#vw-file-input` element via `page.setInputFiles()`
- Tests wait for `#vw-workspace.active` and `.vw-tab` to confirm parsing is complete
- Format is verified via the `#vw-format-chip` toolbar badge
- Row counts are verified via `#vw-count-chip` and table row counts
- Keyboard shortcuts are tested via `page.keyboard.press()`
- Downloads are captured via `page.waitForEvent('download')`

## Documentation Screenshots

Generate high-quality screenshots of every BioPeek feature for use in documentation and help files:

```bash
npm run screenshots
# Screenshots saved to screenshots/output/
```

This captures 22 screenshots covering the landing page, file loading for each format, view modes (table/stats/raw), sorting, search/filter, column tooltips, row detail, themes, bookmarks, split view, keyboard help, multi-tab, and context menus.

## Configuration

The Playwright config (`playwright.config.js`) is set up with:
- **Browser**: Chromium only
- **Timeout**: 30s per test
- **Screenshots**: On failure
- **Traces**: Retained on failure
- **Reporter**: List (console) + HTML report
