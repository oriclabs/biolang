// @ts-check
const { test, expect } = require('@playwright/test');
const {
  openViewer,
  loadFixture,
} = require('./helpers');

test.describe('Export tests', () => {

  test.beforeEach(async ({ page }) => {
    await openViewer(page);
  });

  test('export VCF as CSV triggers download', async ({ page }) => {
    await loadFixture(page, 'sample.vcf');

    // Click export button
    await page.locator('#vw-export-btn').click();
    await page.waitForSelector('#vw-export-menu', { timeout: 3000 });

    // Set up download listener before clicking
    const downloadPromise = page.waitForEvent('download', { timeout: 10000 });

    // Click "Export as CSV"
    const csvOption = page.locator('#vw-export-menu div').filter({ hasText: 'Export as CSV' }).first();
    await csvOption.click();

    const download = await downloadPromise;
    expect(download.suggestedFilename()).toContain('.csv');

    // Read download content
    const content = await download.path().then(p => {
      const fs = require('fs');
      return fs.readFileSync(p, 'utf-8');
    });

    // Should contain header and data rows
    expect(content).toContain('CHROM');
    expect(content).toContain('chr1');
    expect(content).toContain('chr2');

    // Should have correct number of lines (header + 5 data rows)
    const lines = content.trim().split('\n');
    expect(lines.length).toBe(6);
  });

  test('export VCF as TSV triggers download', async ({ page }) => {
    await loadFixture(page, 'sample.vcf');

    await page.locator('#vw-export-btn').click();
    await page.waitForSelector('#vw-export-menu', { timeout: 3000 });

    const downloadPromise = page.waitForEvent('download', { timeout: 10000 });

    const tsvOption = page.locator('#vw-export-menu div').filter({ hasText: 'Export as TSV' }).first();
    await tsvOption.click();

    const download = await downloadPromise;
    expect(download.suggestedFilename()).toContain('.tsv');

    const content = await download.path().then(p => {
      const fs = require('fs');
      return fs.readFileSync(p, 'utf-8');
    });

    // TSV should use tab separators
    const firstLine = content.split('\n')[0];
    expect(firstLine).toContain('\t');
    expect(content).toContain('chr1');
  });

  test('export BED as BED preserves format', async ({ page }) => {
    await loadFixture(page, 'sample.bed');

    await page.locator('#vw-export-btn').click();
    await page.waitForSelector('#vw-export-menu', { timeout: 3000 });

    // Check that BED export option exists for BED files
    const menuText = await page.locator('#vw-export-menu').textContent();
    expect(menuText).toContain('BED');
  });

  test('export menu shows subset options for large files', async ({ page }) => {
    await loadFixture(page, 'sample_large.csv');

    await page.locator('#vw-export-btn').click();
    await page.waitForSelector('#vw-export-menu', { timeout: 3000 });

    const menuText = await page.locator('#vw-export-menu').textContent();
    // Should have "Export current page" option
    expect(menuText).toContain('current page');
  });

});
