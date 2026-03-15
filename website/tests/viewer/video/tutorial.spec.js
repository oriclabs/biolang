// @ts-check
const { test } = require('@playwright/test');
const path = require('path');
const fs = require('fs');

const { openViewer, loadFixture } = require('../e2e/helpers');

// ---------------------------------------------------------------------------
// Narration helpers — inject text overlays that Playwright's video captures
// ---------------------------------------------------------------------------

async function narrate(page, title, subtitle, durationMs = 2000) {
  await page.evaluate(({ title, subtitle }) => {
    let banner = document.getElementById('tutorial-banner');
    if (!banner) {
      banner = document.createElement('div');
      banner.id = 'tutorial-banner';
      banner.style.cssText = [
        'position:fixed', 'top:0', 'left:0', 'right:0', 'z-index:99999',
        'background:linear-gradient(135deg,#1e1b4b,#312e81)',
        'color:#e0e7ff', 'padding:16px 24px',
        'font-family:system-ui,sans-serif',
        'box-shadow:0 4px 20px rgba(0,0,0,0.5)',
        'transition:opacity 0.3s',
        'display:flex', 'flex-direction:column', 'gap:4px',
        'border-bottom:2px solid #7c3aed',
      ].join(';');
      document.body.prepend(banner);
    }
    banner.style.opacity = '1';
    banner.innerHTML =
      '<div style="font-size:18px;font-weight:700;color:#c4b5fd">' + title + '</div>' +
      (subtitle
        ? '<div style="font-size:13px;color:#a5b4fc;font-weight:400">' + subtitle + '</div>'
        : '');
  }, { title, subtitle });
  await page.waitForTimeout(durationMs);
}

async function hideBanner(page) {
  await page.evaluate(() => {
    const b = document.getElementById('tutorial-banner');
    if (b) b.style.opacity = '0';
  });
  await page.waitForTimeout(300);
}

async function showStep(page, stepNum, totalSteps) {
  await page.evaluate(({ stepNum, totalSteps }) => {
    let badge = document.getElementById('tutorial-step');
    if (!badge) {
      badge = document.createElement('div');
      badge.id = 'tutorial-step';
      badge.style.cssText = [
        'position:fixed', 'top:8px', 'right:16px', 'z-index:99999',
        'background:#7c3aed', 'color:white',
        'padding:4px 12px', 'border-radius:20px',
        'font-family:system-ui', 'font-size:12px', 'font-weight:700',
        'box-shadow:0 2px 8px rgba(0,0,0,0.3)',
      ].join(';');
      document.body.appendChild(badge);
    }
    badge.textContent = stepNum + ' / ' + totalSteps;
  }, { stepNum, totalSteps });
}

async function highlight(page, selector) {
  const el = page.locator(selector).first();
  await el.evaluate(el => {
    el.style.outline = '3px solid #f59e0b';
    el.style.outlineOffset = '2px';
    el.style.transition = 'outline 0.2s';
  });
  await page.waitForTimeout(400);
  await el.click({ force: true });
  await page.waitForTimeout(300);
  await el.evaluate(el => {
    el.style.outline = '';
    el.style.outlineOffset = '';
  });
}

const TOTAL_STEPS = 16;
let currentStep = 0;

async function step(page, title, subtitle, durationMs = 1800) {
  currentStep++;
  await showStep(page, currentStep, TOTAL_STEPS);
  await narrate(page, title, subtitle, durationMs);
}

// ---------------------------------------------------------------------------
// Single continuous test — one VCF file, all key features
// ---------------------------------------------------------------------------

test('BLViewer Feature Tutorial', async ({ page }) => {
  test.setTimeout(180_000);
  await page.setViewportSize({ width: 1440, height: 900 });

  // ── INTRO ──
  await openViewer(page);
  const dismissBtn = page.locator('#vw-experimental-dismiss');
  if (await dismissBtn.isVisible({ timeout: 1000 }).catch(() => false)) {
    await dismissBtn.click();
    await page.waitForTimeout(200);
  }

  await step(page,
    'Welcome to BLViewer',
    '100% client-side bioinformatics file viewer. Your data never leaves your browser.',
    2500);

  // ── LOAD FILE ──
  await step(page,
    'Drop or Browse Files',
    'Supports FASTA, FASTQ, VCF, BED, GFF, SAM, CSV, TSV with auto-detection.',
    1500);
  await hideBanner(page);
  await loadFixture(page, 'sample.vcf');
  await page.waitForTimeout(1500);

  // ── TABLE VIEW ──
  await step(page,
    'Table View with ATCG Coloring',
    'Sortable columns, nucleotide highlighting, and format-aware parsing.',
    2000);

  // ── SORTING ──
  await step(page,
    'Click Headers to Sort',
    'Ascending, descending, or original order. Hold Shift for multi-column sort.',
    1200);
  await hideBanner(page);
  const qualHeader = page.locator('.vw-table th').filter({ hasText: 'QUAL' }).first();
  await qualHeader.click({ force: true });
  await page.waitForTimeout(800);
  await qualHeader.click({ force: true });
  await page.waitForTimeout(800);

  // ── SEARCH ──
  await step(page,
    'Instant Search & Filter',
    'Type to filter rows in real time. Supports regex patterns.',
    1200);
  await hideBanner(page);
  const searchBox = page.locator('#vw-search');
  await searchBox.fill('PASS');
  await page.waitForTimeout(1500);
  await searchBox.fill('');
  await page.waitForTimeout(500);

  // ── COLUMN TOOLTIP ──
  await step(page,
    'Column Statistics on Hover',
    'Hover numeric headers for min, max, mean, median.',
    1200);
  await hideBanner(page);
  await qualHeader.hover();
  await page.waitForTimeout(2000);

  // ── RIGHT-CLICK MENU ──
  await step(page,
    'Right-Click Menus',
    'Column headers: filter, pin, hide. Cells: copy, BLAST, dbSNP lookup.',
    1500);
  await hideBanner(page);
  const filterHeader = page.locator('.vw-table th').filter({ hasText: 'FILTER' }).first();
  await filterHeader.click({ button: 'right', force: true });
  await page.waitForTimeout(1800);
  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);

  // ── ROW DETAIL ──
  await step(page,
    'Row Detail Panel',
    'Click Detail, then click any row for a card-style field view.',
    1200);
  await hideBanner(page);
  await highlight(page, '#vw-detail-toggle');
  await page.waitForTimeout(300);
  await page.locator('.vw-table tbody tr').first().click({ force: true });
  await page.waitForTimeout(1800);
  await highlight(page, '#vw-detail-toggle');
  await page.waitForTimeout(300);

  // ── BOOKMARKS ──
  await step(page,
    'Bookmark Important Rows',
    'Star rows to mark them. Export only bookmarked data later.',
    1200);
  await hideBanner(page);
  await highlight(page, '#vw-bookmark-toggle');
  await page.waitForTimeout(300);
  const starCells = page.locator('.vw-table tbody tr td.vw-star-cell');
  if (await starCells.count() >= 2) {
    await starCells.nth(0).click({ force: true });
    await page.waitForTimeout(300);
    await starCells.nth(1).click({ force: true });
  }
  await page.waitForTimeout(1200);
  await highlight(page, '#vw-bookmark-toggle');
  await page.waitForTimeout(200);

  // ── STATS VIEW ──
  await step(page,
    'Stats View',
    'Press 2 for format-specific statistics with charts.',
    1200);
  await page.keyboard.press('2');
  await page.waitForTimeout(2000);

  // ── RAW VIEW ──
  await step(page,
    'Raw Text View',
    'Press 3 for the original file with line numbers.',
    1200);
  await page.keyboard.press('3');
  await page.waitForTimeout(1500);

  // ── THEME ──
  await step(page,
    'Dark & Light Themes',
    'Toggle with the theme button. Preference is remembered.',
    1200);
  await page.keyboard.press('1'); // back to table
  await page.waitForTimeout(500);
  await hideBanner(page);
  await highlight(page, '#vw-theme-toggle');
  await page.waitForTimeout(1500);
  await highlight(page, '#vw-theme-toggle');
  await page.waitForTimeout(800);

  // ── KEYBOARD HELP ──
  await step(page,
    'Keyboard Shortcuts',
    'Press ? to see all shortcuts. Ctrl+G to jump to row. Ctrl+H for find & replace.',
    1200);
  await page.keyboard.press('?');
  await page.waitForTimeout(2000);
  await page.keyboard.press('Escape');
  await page.waitForTimeout(300);

  // ── EXPORT ──
  await step(page,
    'Export & Convert',
    'Export as CSV, TSV, HTML. Convert between formats (VCF to BED, GFF to BED).',
    1200);
  await hideBanner(page);
  await highlight(page, '#vw-export-btn');
  await page.waitForTimeout(1800);
  await page.keyboard.press('Escape');
  await page.waitForTimeout(300);

  // ── OUTRO ──
  await step(page,
    "That's BLViewer!",
    'No uploads. No server. Works offline as a PWA or Chrome extension. Try it at lang.bio/viewer',
    3000);

  await hideBanner(page);
  await page.waitForTimeout(1500);

  // ── Save video ──
  const video = page.video();
  if (video) {
    const videoPath = await video.path();
    test.info().attach('tutorial-video-source', { body: videoPath });
  }
});

// Copy video to output/ after browser context closes
test.afterAll(async () => {
  const resultsDir = path.resolve(__dirname, '..', 'test-results');
  const outputDir = path.resolve(__dirname, 'output');
  fs.mkdirSync(outputDir, { recursive: true });

  function findWebm(dir) {
    if (!fs.existsSync(dir)) return [];
    const results = [];
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
      const full = path.join(dir, entry.name);
      if (entry.isDirectory()) results.push(...findWebm(full));
      else if (entry.name.endsWith('.webm')) results.push(full);
    }
    return results;
  }

  const videos = findWebm(resultsDir);
  if (videos.length > 0) {
    const src = videos.sort((a, b) => fs.statSync(b).size - fs.statSync(a).size)[0];
    const dest = path.resolve(outputDir, 'tutorial.webm');
    fs.copyFileSync(src, dest);
    console.log('Tutorial video saved to: ' + dest);
  }
});
