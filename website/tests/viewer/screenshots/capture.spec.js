// @ts-check
const { test, expect } = require('@playwright/test');
const path = require('path');
const {
  openViewer,
  loadFixture,
  switchView,
  getVisibleRowCount,
  FIXTURES_DIR,
} = require('../e2e/helpers');

const OUTPUT_DIR = path.resolve(__dirname, 'output');

test.describe('Documentation screenshots', () => {
  test.use({ viewport: { width: 1280, height: 800 } });

  test.beforeEach(async ({ page }) => {
    await openViewer(page);
  });

  // ── Landing & File Loading ──────────────────────────────────────────

  test('capture: landing page', async ({ page }) => {
    // Dismiss experimental banner if present
    const banner = page.locator('#vw-experimental-dismiss');
    if (await banner.isVisible().catch(() => false)) {
      await banner.click();
      await page.waitForTimeout(200);
    }
    await page.waitForTimeout(300);
    await page.screenshot({ path: path.join(OUTPUT_DIR, 'landing.png'), fullPage: false });
  });

  test('capture: loading VCF', async ({ page }) => {
    await loadFixture(page, 'sample.vcf');
    await page.waitForTimeout(500);
    await page.screenshot({ path: path.join(OUTPUT_DIR, 'loading-vcf.png'), fullPage: false });
  });

  test('capture: loading FASTA', async ({ page }) => {
    await loadFixture(page, 'sample.fasta');
    await page.waitForTimeout(500);
    await page.screenshot({ path: path.join(OUTPUT_DIR, 'loading-fasta.png'), fullPage: false });
  });

  test('capture: loading FASTQ', async ({ page }) => {
    await loadFixture(page, 'sample.fastq');
    await page.waitForTimeout(500);
    await page.screenshot({ path: path.join(OUTPUT_DIR, 'loading-fastq.png'), fullPage: false });
  });

  test('capture: loading BED', async ({ page }) => {
    await loadFixture(page, 'sample.bed');
    await page.waitForTimeout(500);
    await page.screenshot({ path: path.join(OUTPUT_DIR, 'loading-bed.png'), fullPage: false });
  });

  test('capture: loading CSV', async ({ page }) => {
    await loadFixture(page, 'sample.csv');
    await page.waitForTimeout(500);
    await page.screenshot({ path: path.join(OUTPUT_DIR, 'loading-csv.png'), fullPage: false });
  });

  // ── View Modes ──────────────────────────────────────────────────────

  test('capture: table view', async ({ page }) => {
    await loadFixture(page, 'sample.vcf');
    await switchView(page, 'table');
    await page.waitForTimeout(500);
    await page.screenshot({ path: path.join(OUTPUT_DIR, 'view-table.png'), fullPage: false });
  });

  test('capture: stats view', async ({ page }) => {
    await loadFixture(page, 'sample.vcf');
    await page.keyboard.press('2');
    await page.waitForTimeout(800);
    await page.screenshot({ path: path.join(OUTPUT_DIR, 'view-stats.png'), fullPage: false });
  });

  test('capture: raw view', async ({ page }) => {
    await loadFixture(page, 'sample.vcf');
    await page.keyboard.press('3');
    await page.waitForTimeout(500);
    await page.screenshot({ path: path.join(OUTPUT_DIR, 'view-raw.png'), fullPage: false });
  });

  // ── Table Features ──────────────────────────────────────────────────

  test('capture: sort ascending', async ({ page }) => {
    await loadFixture(page, 'sample.vcf');

    // Find and click the QUAL column header
    const headers = await page.locator('.vw-table th').allTextContents();
    const qualIdx = headers.findIndex(h => h.includes('QUAL'));
    expect(qualIdx).toBeGreaterThan(-1);

    await page.locator(`.vw-table th:nth-child(${qualIdx + 1})`).click();
    await page.waitForTimeout(500);
    await page.screenshot({ path: path.join(OUTPUT_DIR, 'sort-ascending.png'), fullPage: false });
  });

  test('capture: sort descending', async ({ page }) => {
    await loadFixture(page, 'sample.vcf');

    const headers = await page.locator('.vw-table th').allTextContents();
    const qualIdx = headers.findIndex(h => h.includes('QUAL'));
    expect(qualIdx).toBeGreaterThan(-1);

    // Click twice for descending
    await page.locator(`.vw-table th:nth-child(${qualIdx + 1})`).click();
    await page.waitForTimeout(300);
    await page.locator(`.vw-table th:nth-child(${qualIdx + 1})`).click();
    await page.waitForTimeout(500);
    await page.screenshot({ path: path.join(OUTPUT_DIR, 'sort-descending.png'), fullPage: false });
  });

  test('capture: search filter', async ({ page }) => {
    await loadFixture(page, 'sample.vcf');

    await page.locator('#vw-search').fill('PASS');
    await page.waitForTimeout(600);
    await page.screenshot({ path: path.join(OUTPUT_DIR, 'search-filter.png'), fullPage: false });
  });

  test('capture: column header hover tooltip', async ({ page }) => {
    await loadFixture(page, 'sample.vcf');

    // Find QUAL column header and hover over it
    const headers = await page.locator('.vw-table th').allTextContents();
    const qualIdx = headers.findIndex(h => h.includes('QUAL'));
    expect(qualIdx).toBeGreaterThan(-1);

    await page.locator(`.vw-table th:nth-child(${qualIdx + 1})`).hover();
    await page.waitForTimeout(800);
    await page.screenshot({ path: path.join(OUTPUT_DIR, 'column-header-hover.png'), fullPage: false });
  });

  test('capture: row detail panel', async ({ page }) => {
    await loadFixture(page, 'sample.vcf');

    // Enable detail mode
    await page.locator('#vw-detail-toggle').click();
    await page.waitForTimeout(300);

    // Click first data row
    await page.locator('.vw-table tbody tr:first-child').click();
    await page.waitForTimeout(500);

    // Wait for detail panel to appear
    const detailPanel = page.locator('.vw-detail-panel');
    await expect(detailPanel).toBeVisible({ timeout: 5000 });

    await page.screenshot({ path: path.join(OUTPUT_DIR, 'row-detail.png'), fullPage: false });
  });

  // ── Toolbar Features ────────────────────────────────────────────────

  test('capture: light theme', async ({ page }) => {
    await loadFixture(page, 'sample.vcf');

    // Switch to light theme
    await page.locator('#vw-theme-toggle').click();
    await page.waitForTimeout(500);

    await page.screenshot({ path: path.join(OUTPUT_DIR, 'theme-light.png'), fullPage: false });
  });

  test('capture: dark theme', async ({ page }) => {
    await loadFixture(page, 'sample.vcf');
    await page.waitForTimeout(300);

    // Dark is the default
    await page.screenshot({ path: path.join(OUTPUT_DIR, 'theme-dark.png'), fullPage: false });
  });

  test('capture: bookmarks', async ({ page }) => {
    await loadFixture(page, 'sample.vcf');

    // Enable bookmark mode
    await page.locator('#vw-bookmark-toggle').click();
    await page.waitForTimeout(300);

    // Star some rows if bookmark stars are visible
    const stars = page.locator('.bookmark-star');
    const starCount = await stars.count();
    if (starCount >= 3) {
      await stars.nth(0).click();
      await page.waitForTimeout(150);
      await stars.nth(2).click();
      await page.waitForTimeout(150);
      await stars.nth(4).click({ timeout: 2000 }).catch(() => {});
      await page.waitForTimeout(150);
    } else if (starCount > 0) {
      await stars.first().click();
      await page.waitForTimeout(150);
    }
    await page.waitForTimeout(300);
    await page.screenshot({ path: path.join(OUTPUT_DIR, 'bookmarks.png'), fullPage: false });
  });

  test('capture: split view', async ({ page }) => {
    await loadFixture(page, 'sample.vcf');

    // Click split view toggle
    await page.locator('#vw-split-toggle').click();
    await page.waitForTimeout(800);
    await page.screenshot({ path: path.join(OUTPUT_DIR, 'split-view.png'), fullPage: false });
  });

  test('capture: keyboard help overlay', async ({ page }) => {
    await loadFixture(page, 'sample.vcf');

    // Press ? to open help
    await page.keyboard.press('?');
    await page.waitForTimeout(500);

    const helpOverlay = page.locator('.vw-help-overlay');
    await expect(helpOverlay).toBeVisible({ timeout: 5000 });

    await page.screenshot({ path: path.join(OUTPUT_DIR, 'keyboard-help.png'), fullPage: false });
  });

  // ── Column Interactions ─────────────────────────────────────────────

  test('capture: multi-tab view', async ({ page }) => {
    // Load first file
    await loadFixture(page, 'sample.vcf');

    // Load second file
    const fileInput = page.locator('#vw-file-input');
    await fileInput.setInputFiles(path.join(FIXTURES_DIR, 'sample.csv'));
    await page.waitForTimeout(800);

    // Verify two tabs are present
    const tabs = page.locator('.vw-tab');
    expect(await tabs.count()).toBe(2);

    await page.screenshot({ path: path.join(OUTPUT_DIR, 'multi-tab.png'), fullPage: false });
  });

  // ── Context Menus ───────────────────────────────────────────────────

  test('capture: header context menu', async ({ page }) => {
    await loadFixture(page, 'sample.vcf');

    // Right-click a column header (skip the # row-number column)
    const secondHeader = page.locator('.vw-table th:nth-child(2)');
    await secondHeader.click({ button: 'right' });
    await page.waitForTimeout(500);

    // Check if context menu appeared
    const ctxMenu = page.locator('#vw-ctx-menu, .vw-ctx-menu');
    const menuVisible = await ctxMenu.isVisible().catch(() => false);

    if (menuVisible) {
      await page.screenshot({ path: path.join(OUTPUT_DIR, 'header-menu.png'), fullPage: false });
    } else {
      // Fallback: capture the page as-is after right-click attempt
      await page.screenshot({ path: path.join(OUTPUT_DIR, 'header-menu.png'), fullPage: false });
    }
  });

  test('capture: cell context menu', async ({ page }) => {
    await loadFixture(page, 'sample.vcf');

    // Right-click a data cell
    const firstCell = page.locator('.vw-table tbody tr:first-child td:nth-child(2)');
    await firstCell.click({ button: 'right' });
    await page.waitForTimeout(500);

    // Check if context menu appeared
    const ctxMenu = page.locator('#vw-ctx-menu, .vw-ctx-menu');
    const menuVisible = await ctxMenu.isVisible().catch(() => false);

    if (menuVisible) {
      await page.screenshot({ path: path.join(OUTPUT_DIR, 'cell-menu.png'), fullPage: false });
    } else {
      // Fallback: capture the page as-is
      await page.screenshot({ path: path.join(OUTPUT_DIR, 'cell-menu.png'), fullPage: false });
    }
  });
});
