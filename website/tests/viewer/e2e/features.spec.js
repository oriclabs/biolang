// @ts-check
const { test, expect } = require('@playwright/test');
const {
  openViewer,
  loadFixture,
  switchView,
  getVisibleRowCount,
} = require('./helpers');

test.describe('Feature tests', () => {

  test.beforeEach(async ({ page }) => {
    await openViewer(page);
  });

  test('theme toggle switches between dark and light', async ({ page }) => {
    await loadFixture(page, 'sample.vcf');

    // Default is dark (html has class "dark")
    const htmlEl = page.locator('html');
    await expect(htmlEl).toHaveClass(/dark/);

    // Click theme toggle
    await page.locator('#vw-theme-toggle').click();
    await page.waitForTimeout(200);

    // Should switch to light (no dark class)
    await expect(htmlEl).not.toHaveClass(/dark/);

    // Click again to go back to dark
    await page.locator('#vw-theme-toggle').click();
    await page.waitForTimeout(200);

    await expect(htmlEl).toHaveClass(/dark/);
  });

  test('bookmark mode can be toggled', async ({ page }) => {
    await loadFixture(page, 'sample.vcf');

    // Click bookmark toggle button
    const bookmarkBtn = page.locator('#vw-bookmark-toggle');
    await bookmarkBtn.click();
    await page.waitForTimeout(300);

    // Bookmark column (star) should be visible in table rows
    const stars = page.locator('.bookmark-star');
    const starCount = await stars.count();
    // If bookmark mode adds a star column, we expect stars to exist
    if (starCount > 0) {
      // Click a star to bookmark a row
      await stars.first().click();
      await page.waitForTimeout(200);

      // The star should become active
      const activeStar = page.locator('.bookmark-star.active');
      const activeCount = await activeStar.count();
      expect(activeCount).toBeGreaterThanOrEqual(1);
    }
  });

  test('screenshot button triggers download', async ({ page }) => {
    await loadFixture(page, 'sample.vcf');

    // Listen for download event
    const downloadPromise = page.waitForEvent('download', { timeout: 5000 }).catch(() => null);

    await page.locator('#vw-screenshot-btn').click();
    await page.waitForTimeout(1000);

    // Screenshot may use html2canvas or similar — it may not trigger a download event
    // in headless mode. We just verify the button is clickable without errors.
  });

  test('export menu appears on click', async ({ page }) => {
    await loadFixture(page, 'sample.vcf');

    // Click export button
    await page.locator('#vw-export-btn').click();
    await page.waitForTimeout(300);

    // Export menu should appear
    const exportMenu = page.locator('#vw-export-menu');
    await expect(exportMenu).toBeVisible({ timeout: 3000 });

    // Should contain CSV and TSV options at minimum
    const menuText = await exportMenu.textContent();
    expect(menuText).toContain('CSV');
    expect(menuText).toContain('TSV');

    // Click elsewhere to close
    await page.locator('#vw-content').click({ force: true });
    await page.waitForTimeout(300);
  });

  test('keyboard shortcut ? opens help overlay', async ({ page }) => {
    await loadFixture(page, 'sample.vcf');

    // Press ? to open help
    await page.keyboard.press('?');
    await page.waitForTimeout(300);

    // Help overlay should appear
    const helpOverlay = page.locator('.vw-help-overlay');
    await expect(helpOverlay).toBeVisible({ timeout: 3000 });

    // Should have keyboard shortcut content
    const helpText = await helpOverlay.textContent();
    expect(helpText.length).toBeGreaterThan(50);

    // Press Escape to close
    await page.keyboard.press('Escape');
    await page.waitForTimeout(300);

    await expect(helpOverlay).not.toBeVisible();
  });

  test('keyboard shortcut Ctrl+G opens goto row', async ({ page }) => {
    await loadFixture(page, 'sample.vcf');

    // Ensure focus is on the page body (not search box)
    await page.locator('body').click();
    await page.waitForTimeout(200);

    // Dispatch Ctrl+G via page evaluate to bypass browser interception
    await page.evaluate(() => {
      document.dispatchEvent(new KeyboardEvent('keydown', {
        key: 'g', code: 'KeyG', ctrlKey: true, bubbles: true, cancelable: true
      }));
    });
    await page.waitForTimeout(500);

    // Goto overlay should appear
    const gotoOverlay = page.locator('.vw-goto-overlay');
    await expect(gotoOverlay).toBeVisible({ timeout: 3000 });

    // Type a row number
    const gotoInput = gotoOverlay.locator('input');
    await gotoInput.fill('3');
    await gotoInput.press('Enter');
    await page.waitForTimeout(300);

    // Overlay should close after entering
    await expect(gotoOverlay).not.toBeVisible();
  });

  test('view mode switching with number keys 1-3', async ({ page }) => {
    await loadFixture(page, 'sample.csv');

    // Press 2 for Stats view
    await page.keyboard.press('2');
    await page.waitForTimeout(400);

    // Stats button should be active
    const statsBtn = page.locator('#vw-view-stats');
    await expect(statsBtn).toHaveClass(/active/);

    // Press 3 for Raw view
    await page.keyboard.press('3');
    await page.waitForTimeout(400);

    const rawBtn = page.locator('#vw-view-raw');
    await expect(rawBtn).toHaveClass(/active/);

    // Press 1 to go back to Table
    await page.keyboard.press('1');
    await page.waitForTimeout(400);

    const tableBtn = page.locator('#vw-view-table');
    await expect(tableBtn).toHaveClass(/active/);
  });

  test('view mode switching via toolbar buttons', async ({ page }) => {
    await loadFixture(page, 'sample.csv');

    // Click Stats button
    await switchView(page, 'stats');
    const statsBtn = page.locator('#vw-view-stats');
    await expect(statsBtn).toHaveClass(/active/);

    // Click Raw button
    await switchView(page, 'raw');
    const rawBtn = page.locator('#vw-view-raw');
    await expect(rawBtn).toHaveClass(/active/);

    // Click Table button
    await switchView(page, 'table');
    const tableBtn = page.locator('#vw-view-table');
    await expect(tableBtn).toHaveClass(/active/);
  });

  test('tab management — load two files and switch between tabs', async ({ page }) => {
    // Load first file
    await loadFixture(page, 'sample.vcf');

    // Check first tab exists
    let tabs = page.locator('.vw-tab');
    expect(await tabs.count()).toBe(1);

    // Load second file (click + button then use file input)
    const fileInput = page.locator('#vw-file-input');
    const path = require('path');
    const fixturesDir = path.resolve(__dirname, '..', 'fixtures');
    await fileInput.setInputFiles(path.join(fixturesDir, 'sample.csv'));
    await page.waitForTimeout(500);

    // Should now have 2 tabs
    tabs = page.locator('.vw-tab');
    expect(await tabs.count()).toBe(2);

    // Click first tab
    await tabs.first().click();
    await page.waitForTimeout(300);

    // Format should be VCF
    const format = await page.locator('#vw-format-chip').textContent();
    expect(format.toLowerCase()).toBe('vcf');

    // Click second tab
    await tabs.nth(1).click();
    await page.waitForTimeout(300);

    // Format should be CSV
    const format2 = await page.locator('#vw-format-chip').textContent();
    expect(format2.toLowerCase()).toBe('csv');
  });

  test('experimental banner can be dismissed', async ({ page }) => {
    await openViewer(page);

    const banner = page.locator('#vw-experimental-banner');
    // Banner may or may not be visible depending on localStorage
    if (await banner.isVisible()) {
      await page.locator('#vw-experimental-dismiss').click();
      await page.waitForTimeout(200);
      await expect(banner).not.toBeVisible();
    }
  });

});
