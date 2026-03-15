const { test } = require('@playwright/test');
const path = require('path');

const BIOGIST_URL = 'file:///' + path.resolve(__dirname, '..', '..', '..', 'biogist.html').replace(/\\/g, '/');
const SCREENSHOTS_DIR = path.resolve(__dirname, '..', '..', '..', 'extension', 'biogist', 'screenshots');

test.describe('BioGist Screenshots', () => {

  test('landing page', async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto(BIOGIST_URL, { waitUntil: 'domcontentloaded' });
    await page.waitForTimeout(1000);
    await page.screenshot({ path: path.join(SCREENSHOTS_DIR, 'landing.png') });
  });

  test('with sample text scanned', async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto(BIOGIST_URL, { waitUntil: 'domcontentloaded' });
    await page.waitForTimeout(500);

    // Click "Try with BRCA1 paper" button
    const tryBtn = page.locator('button:has-text("BRCA1")');
    if (await tryBtn.count() > 0) {
      await tryBtn.first().click();
      await page.waitForTimeout(500);
    }

    // Click Scan
    const scanBtn = page.locator('button:has-text("Scan")');
    if (await scanBtn.count() > 0) {
      await scanBtn.first().click();
      await page.waitForTimeout(1000);
    }

    await page.screenshot({ path: path.join(SCREENSHOTS_DIR, 'scanned.png') });
  });

  test('entity detail view', async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto(BIOGIST_URL, { waitUntil: 'domcontentloaded' });
    await page.waitForTimeout(500);

    // Click example, scan, then click an entity
    const tryBtn = page.locator('button:has-text("BRCA1")');
    if (await tryBtn.count() > 0) {
      await tryBtn.first().click();
      await page.waitForTimeout(500);
    }
    const scanBtn = page.locator('button:has-text("Scan")');
    if (await scanBtn.count() > 0) {
      await scanBtn.first().click();
      await page.waitForTimeout(1000);
    }

    // Click first detected entity
    const entity = page.locator('.entity-item, [class*="entity"]').first();
    if (await entity.count() > 0) {
      await entity.click();
      await page.waitForTimeout(2000); // Wait for API response
    }

    await page.screenshot({ path: path.join(SCREENSHOTS_DIR, 'detail.png') });
  });
});
