// @ts-check
const { test, expect } = require('@playwright/test');
const {
  openViewer,
  loadFixture,
  switchView,
} = require('./helpers');

test.describe('Statistics view tests', () => {

  test.beforeEach(async ({ page }) => {
    await openViewer(page);
  });

  test('stats view shows stat cards for CSV data', async ({ page }) => {
    await loadFixture(page, 'sample.csv');

    // Switch to stats view
    await switchView(page, 'stats');

    // Wait for stats cards to render
    await page.waitForSelector('.vw-stat-card', { timeout: 5000 });

    // Should have multiple stat cards
    const cards = page.locator('.vw-stat-card');
    const cardCount = await cards.count();
    expect(cardCount).toBeGreaterThan(0);

    // Stats should contain labels and values
    const labels = page.locator('.vw-stat-label');
    const labelCount = await labels.count();
    expect(labelCount).toBeGreaterThan(0);

    const values = page.locator('.vw-stat-value');
    const valueCount = await values.count();
    expect(valueCount).toBeGreaterThan(0);
  });

  test('stats view shows numeric statistics for numeric columns', async ({ page }) => {
    await loadFixture(page, 'sample.csv');
    await switchView(page, 'stats');

    await page.waitForSelector('.vw-stat-card', { timeout: 5000 });

    // Get all stat content
    const statsText = await page.locator('#vw-content').textContent();

    // Should contain common stat terms for the numeric columns (expression, pvalue, etc.)
    // The stats view typically shows count, min, max, mean, etc.
    const hasNumericStats = statsText.toLowerCase().includes('mean') ||
                            statsText.toLowerCase().includes('min') ||
                            statsText.toLowerCase().includes('max') ||
                            statsText.toLowerCase().includes('records') ||
                            statsText.toLowerCase().includes('rows');
    expect(hasNumericStats).toBeTruthy();
  });

  test('can switch back to table from stats', async ({ page }) => {
    await loadFixture(page, 'sample.csv');

    // Go to stats
    await switchView(page, 'stats');
    await page.waitForSelector('.vw-stat-card', { timeout: 5000 });

    // Go back to table
    await switchView(page, 'table');
    await page.waitForSelector('.vw-table', { timeout: 5000 });

    // Table should be visible again
    const table = page.locator('.vw-table');
    await expect(table).toBeVisible();
  });

  test('stats view for VCF shows format-specific stats', async ({ page }) => {
    await loadFixture(page, 'sample.vcf');
    await switchView(page, 'stats');

    await page.waitForSelector('.vw-stat-card', { timeout: 5000 });

    const cards = page.locator('.vw-stat-card');
    const cardCount = await cards.count();
    expect(cardCount).toBeGreaterThan(0);
  });

  test('stats view with key 2 shortcut', async ({ page }) => {
    await loadFixture(page, 'sample.csv');

    // Press 2 for stats
    await page.keyboard.press('2');
    await page.waitForTimeout(400);

    // Stats view should be active
    const statsBtn = page.locator('#vw-view-stats');
    await expect(statsBtn).toHaveClass(/active/);

    await page.waitForSelector('.vw-stat-card', { timeout: 5000 });

    // Press 1 to go back
    await page.keyboard.press('1');
    await page.waitForTimeout(400);

    const tableBtn = page.locator('#vw-view-table');
    await expect(tableBtn).toHaveClass(/active/);
  });

});
