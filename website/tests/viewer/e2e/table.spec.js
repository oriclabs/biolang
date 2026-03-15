// @ts-check
const { test, expect } = require('@playwright/test');
const {
  openViewer,
  loadFixture,
  getVisibleRowCount,
  getCellText,
} = require('./helpers');

test.describe('Table interaction tests', () => {

  test.beforeEach(async ({ page }) => {
    await openViewer(page);
  });

  test('sort by clicking column header', async ({ page }) => {
    await loadFixture(page, 'sample.vcf');

    // Use FILTER column for easier assertion — it has only 2 text values: PASS and LowQual
    const filterHeader = page.locator('.vw-table th', { hasText: 'FILTER' }).first();
    await expect(filterHeader).toBeVisible();

    // Get the column index by counting preceding siblings
    const filterIdx = await filterHeader.evaluate(el => {
      let idx = 0;
      let sib = el.previousElementSibling;
      while (sib) { idx++; sib = sib.previousElementSibling; }
      return idx;
    });

    // Record initial order of FILTER column
    const initialValues = await page.locator(`.vw-table tbody tr td:nth-child(${filterIdx + 1})`).allTextContents();
    const initialClean = initialValues.map(v => v.trim());

    // Click FILTER header to sort (click the text area, not the filter icon)
    await filterHeader.click({ position: { x: 20, y: 10 } });
    await page.waitForTimeout(500);

    // Values should be sorted (ascending: LowQual before PASS)
    const sortedValues = await page.locator(`.vw-table tbody tr td:nth-child(${filterIdx + 1})`).allTextContents();
    const sortedClean = sortedValues.map(v => v.trim());
    expect(sortedClean).not.toEqual(initialClean); // Order changed
    expect(sortedClean[0]).toBe('LowQual'); // L < P in ascending

    // Click again for descending sort
    await filterHeader.click({ position: { x: 20, y: 10 } });
    await page.waitForTimeout(500);

    const descValues = await page.locator(`.vw-table tbody tr td:nth-child(${filterIdx + 1})`).allTextContents();
    const descClean = descValues.map(v => v.trim());
    expect(descClean[0]).toBe('PASS'); // P > L in descending
  });

  test('search filters rows', async ({ page }) => {
    await loadFixture(page, 'sample.vcf');

    // Initially should have 5 rows
    let rows = await getVisibleRowCount(page);
    expect(rows).toBe(5);

    // Type "PASS" in search box
    await page.locator('#vw-search').fill('PASS');
    // Wait for debounce (250ms in the viewer) + rendering
    await page.waitForTimeout(500);

    // Should show only rows with PASS in FILTER column (4 rows)
    rows = await getVisibleRowCount(page);
    expect(rows).toBe(4);

    // Clear search
    await page.locator('#vw-search').fill('');
    await page.waitForTimeout(500);

    // All 5 rows should be back
    rows = await getVisibleRowCount(page);
    expect(rows).toBe(5);
  });

  test('search for specific chromosome filters correctly', async ({ page }) => {
    await loadFixture(page, 'sample.vcf');

    await page.locator('#vw-search').fill('chr2');
    await page.waitForTimeout(500);

    const rows = await getVisibleRowCount(page);
    expect(rows).toBe(2);
  });

  test('pagination works with large files', async ({ page }) => {
    await loadFixture(page, 'sample_large.csv');

    // Should show first page (up to 100 rows)
    let rows = await getVisibleRowCount(page);
    expect(rows).toBeGreaterThan(0);
    expect(rows).toBeLessThanOrEqual(100);

    // Check if pagination controls exist
    const pagination = page.locator('.vw-pagination, [class*="pagination"], button:has-text("Next"), button:has-text("›")');
    const paginationCount = await pagination.count();

    if (paginationCount > 0) {
      // Click next page
      const nextBtn = page.locator('button:has-text("›"), button:has-text("Next")').first();
      if (await nextBtn.isVisible()) {
        await nextBtn.click();
        await page.waitForTimeout(300);

        // Should still show rows
        rows = await getVisibleRowCount(page);
        expect(rows).toBeGreaterThan(0);
      }
    }
  });

  test('row detail panel appears when enabled and row clicked', async ({ page }) => {
    await loadFixture(page, 'sample.vcf');

    // Enable detail mode
    await page.locator('#vw-detail-toggle').click();
    await page.waitForTimeout(200);

    // Click on first data row
    await page.locator('.vw-table tbody tr:first-child').click();
    await page.waitForTimeout(300);

    // Detail panel should appear
    const detailPanel = page.locator('.vw-detail-panel');
    await expect(detailPanel).toBeVisible({ timeout: 5000 });

    // Detail panel should have content
    const detailRows = page.locator('.vw-detail-row');
    const count = await detailRows.count();
    expect(count).toBeGreaterThan(0);

    // Close detail panel
    await page.locator('.vw-detail-close').click();
    await page.waitForTimeout(200);
    await expect(detailPanel).not.toBeVisible();
  });

});
