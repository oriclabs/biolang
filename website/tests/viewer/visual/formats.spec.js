// @ts-check
const { test, expect } = require('@playwright/test');
const {
  openViewer,
  loadFixture,
} = require('../e2e/helpers');

test.describe('Visual regression — format rendering', () => {

  const formats = [
    { file: 'sample.fasta', name: 'fasta' },
    { file: 'sample.fastq', name: 'fastq' },
    { file: 'sample.vcf', name: 'vcf' },
    { file: 'sample.bed', name: 'bed' },
    { file: 'sample.gff', name: 'gff' },
    { file: 'sample.csv', name: 'csv' },
    { file: 'sample.tsv', name: 'tsv' },
  ];

  for (const fmt of formats) {
    test(`visual baseline — ${fmt.name} table view`, async ({ page }) => {
      await openViewer(page);
      await loadFixture(page, fmt.file);

      // Wait for table to be fully rendered
      await page.waitForSelector('.vw-table', { timeout: 10000 });
      await page.waitForTimeout(500);

      // Dismiss the experimental banner if visible to keep screenshots consistent
      const banner = page.locator('#vw-experimental-banner');
      if (await banner.isVisible()) {
        await page.locator('#vw-experimental-dismiss').click();
        await page.waitForTimeout(200);
      }

      // Take screenshot of the workspace area (excluding variable system UI)
      const workspace = page.locator('#vw-workspace');
      await expect(workspace).toHaveScreenshot(`${fmt.name}-table.png`, {
        maxDiffPixelRatio: 0.05,
        threshold: 0.3,
      });
    });
  }

  test('visual baseline — stats view', async ({ page }) => {
    await openViewer(page);
    await loadFixture(page, 'sample.csv');

    // Dismiss experimental banner
    const banner = page.locator('#vw-experimental-banner');
    if (await banner.isVisible()) {
      await page.locator('#vw-experimental-dismiss').click();
      await page.waitForTimeout(200);
    }

    // Switch to stats view
    await page.locator('#vw-view-stats').click();
    await page.waitForTimeout(500);
    await page.waitForSelector('.vw-stat-card', { timeout: 5000 });

    const workspace = page.locator('#vw-workspace');
    await expect(workspace).toHaveScreenshot('csv-stats.png', {
      maxDiffPixelRatio: 0.05,
      threshold: 0.3,
    });
  });

  test('visual baseline — light theme', async ({ page }) => {
    await openViewer(page);
    await loadFixture(page, 'sample.vcf');

    // Dismiss experimental banner
    const banner = page.locator('#vw-experimental-banner');
    if (await banner.isVisible()) {
      await page.locator('#vw-experimental-dismiss').click();
      await page.waitForTimeout(200);
    }

    // Switch to light theme
    await page.locator('#vw-theme-toggle').click();
    await page.waitForTimeout(300);

    const workspace = page.locator('#vw-workspace');
    await expect(workspace).toHaveScreenshot('vcf-light-theme.png', {
      maxDiffPixelRatio: 0.05,
      threshold: 0.3,
    });
  });

});
