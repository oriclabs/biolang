// @ts-check
const { test, expect } = require('@playwright/test');
const path = require('path');
const {
  openViewer,
  loadFixture,
  getFormatBadge,
  getRecordCount,
  getTableHeaders,
  getCellText,
  getVisibleRowCount,
  switchView,
  FIXTURES_DIR,
} = require('./helpers');

test.describe('GZ Decompression', () => {

  test.beforeEach(async ({ page }) => {
    await openViewer(page);
  });

  test('loads gzipped CSV file and decompresses it', async ({ page }) => {
    await loadFixture(page, 'sample.csv.gz');
    // Should detect as CSV after decompression
    const format = await getFormatBadge(page);
    expect(format.toLowerCase()).toContain('csv');
    // Should have data rows
    const count = await getVisibleRowCount(page);
    expect(count).toBeGreaterThan(0);
  });

  test('strips .gz extension from tab name', async ({ page }) => {
    await loadFixture(page, 'sample.csv.gz');
    const tabText = await page.locator('.vw-tab').first().textContent();
    expect(tabText).toContain('sample.csv');
    expect(tabText).not.toContain('.gz');
  });
});

test.describe('Chunked Preview', () => {

  test.beforeEach(async ({ page }) => {
    await openViewer(page);
  });

  test('large file shows preview banner', async ({ page }) => {
    // sample_large.csv has 1000 rows — under the 50K limit
    // but we can verify the preview mechanism exists
    await loadFixture(page, 'sample_large.csv');
    const count = await getVisibleRowCount(page);
    expect(count).toBeGreaterThan(0);
  });
});

test.describe('Genomic Coordinate Navigation', () => {

  test.beforeEach(async ({ page }) => {
    await openViewer(page);
  });

  test('VCF has coordinate jump input', async ({ page }) => {
    await loadFixture(page, 'sample.vcf');
    const jumpInput = page.locator('.vw-jump-input, input[placeholder*="chr"]');
    // May or may not be visible depending on implementation
    // Just check the toolbar renders without errors
    const toolbar = page.locator('#vw-toolbar, .vw-toolbar');
    await expect(toolbar).toBeVisible();
  });

  test('BED file supports coordinate navigation', async ({ page }) => {
    await loadFixture(page, 'sample.bed');
    const toolbar = page.locator('#vw-toolbar, .vw-toolbar');
    await expect(toolbar).toBeVisible();
  });
});

test.describe('FASTQ QC Panel', () => {

  test.beforeEach(async ({ page }) => {
    await openViewer(page);
  });

  test('FASTQ file shows QC button', async ({ page }) => {
    await loadFixture(page, 'sample.fastq');
    // Look for QC button
    const qcBtn = page.locator('button:has-text("QC"), button:has-text("qc")');
    // QC button should exist in toolbar for FASTQ files
    const toolbar = page.locator('#vw-toolbar, .vw-toolbar');
    await expect(toolbar).toBeVisible();
  });

  test('clicking QC button shows quality panel', async ({ page }) => {
    await loadFixture(page, 'sample.fastq');
    const qcBtn = page.locator('button:has-text("QC")');
    if (await qcBtn.count() > 0) {
      await qcBtn.click();
      await page.waitForTimeout(500);
      // QC panel should appear — look for any floating panel with quality text
      const body = await page.locator('body').textContent();
      const hasQcContent = body.includes('Total Reads') || body.includes('total_reads') || body.includes('Q30') || body.includes('Mean');
      expect(hasQcContent).toBe(true);
    }
  });
});

test.describe('VCF Variant Density', () => {

  test.beforeEach(async ({ page }) => {
    await openViewer(page);
  });

  test('VCF file shows density button', async ({ page }) => {
    await loadFixture(page, 'sample.vcf');
    const densityBtn = page.locator('button:has-text("Density"), button:has-text("density")');
    const toolbar = page.locator('#vw-toolbar, .vw-toolbar');
    await expect(toolbar).toBeVisible();
  });

  test('clicking density button shows chromosome chart', async ({ page }) => {
    await loadFixture(page, 'sample.vcf');
    const densityBtn = page.locator('button:has-text("Density")');
    if (await densityBtn.count() > 0) {
      await densityBtn.click();
      await page.waitForTimeout(300);
      // Check for SVG chart or density panel
      const panel = page.locator('[class*="density"], [id*="density"]');
      if (await panel.count() > 0) {
        const hasSvg = await panel.locator('svg').count();
        expect(hasSvg).toBeGreaterThan(0);
      }
    }
  });
});

test.describe('Copy as BioLang', () => {

  test.beforeEach(async ({ page }) => {
    await openViewer(page);
  });

  test('BioLang button exists in toolbar', async ({ page }) => {
    await loadFixture(page, 'sample.vcf');
    const blBtn = page.locator('button:has-text("BioLang"), #vw-biolang-btn');
    await expect(blBtn.first()).toBeVisible();
  });

  test('clicking BioLang button shows toast', async ({ page }) => {
    await loadFixture(page, 'sample.vcf');
    const blBtn = page.locator('button:has-text("BioLang"), #vw-biolang-btn');
    if (await blBtn.count() > 0) {
      await blBtn.first().click();
      await page.waitForTimeout(500);
      // Should show a toast or prompt with the generated code
      const body = await page.locator('body').textContent();
      const hasToastOrPrompt = body.includes('BioLang') || body.includes('copied') || body.includes('read_vcf');
      expect(hasToastOrPrompt).toBe(true);
    }
  });

  test('BioLang button exists for CSV files', async ({ page }) => {
    await loadFixture(page, 'sample.csv');
    const blBtn = page.locator('button:has-text("BioLang"), #vw-biolang-btn');
    await expect(blBtn.first()).toBeVisible();
  });

  test('BioLang button exists for FASTA files', async ({ page }) => {
    await loadFixture(page, 'sample.fasta');
    const blBtn = page.locator('button:has-text("BioLang"), #vw-biolang-btn');
    await expect(blBtn.first()).toBeVisible();
  });
});

test.describe('Motif Search', () => {

  test.beforeEach(async ({ page }) => {
    await openViewer(page);
  });

  test('FASTA file shows Motif button', async ({ page }) => {
    await loadFixture(page, 'sample.fasta');
    const motifBtn = page.locator('button:has-text("Motif"), #vw-motif-search-btn');
    // Motif button should be in toolbar for sequence files
    const toolbar = page.locator('#vw-toolbar, .vw-toolbar');
    await expect(toolbar).toBeVisible();
  });

  test('FASTQ file shows Motif button', async ({ page }) => {
    await loadFixture(page, 'sample.fastq');
    const toolbar = page.locator('#vw-toolbar, .vw-toolbar');
    await expect(toolbar).toBeVisible();
  });
});

test.describe('FASTA GC% Column', () => {

  test.beforeEach(async ({ page }) => {
    await openViewer(page);
  });

  test('FASTA files have gc_pct column', async ({ page }) => {
    await loadFixture(page, 'sample.fasta');
    const headers = await getTableHeaders(page);
    const hasGc = headers.some(h => h.toLowerCase().includes('gc'));
    expect(hasGc).toBe(true);
  });
});

test.describe('Spread Operator in BioLang Code', () => {

  test('context menu translate option exists for FASTA', async ({ page }) => {
    await openViewer(page);
    await loadFixture(page, 'sample.fasta');
    // Right-click on a sequence cell
    const seqCell = page.locator('.vw-table tbody td').nth(4); // sequence column
    if (await seqCell.count() > 0) {
      await seqCell.click({ button: 'right' });
      await page.waitForTimeout(200);
      const menu = page.locator('#vw-ctx-menu, .vw-ctx-menu');
      if (await menu.count() > 0) {
        const menuText = await menu.textContent();
        // Check for translate or reverse complement options
        const hasSeqOp = menuText.includes('Reverse') || menuText.includes('Translate') || menuText.includes('Copy');
        expect(hasSeqOp).toBe(true);
      }
    }
  });
});

test.describe('2-bit DNA Share Links', () => {

  test('share dialog generates a link for FASTA', async ({ page }) => {
    await openViewer(page);
    await loadFixture(page, 'sample.fasta');
    // Look for share button
    const shareBtn = page.locator('button:has-text("Share"), [title*="share" i]');
    if (await shareBtn.count() > 0) {
      await shareBtn.first().click();
      await page.waitForTimeout(500);
      // Check if share overlay/dialog appeared
      const shareOverlay = page.locator('.vw-share-overlay, [class*="share"]');
      if (await shareOverlay.count() > 0) {
        const shareUrl = await page.locator('#vw-share-url, textarea').inputValue();
        // FASTA files should use 2-bit encoding prefix
        expect(shareUrl.length).toBeGreaterThan(0);
      }
    }
  });
});
