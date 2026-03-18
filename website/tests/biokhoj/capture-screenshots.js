// BioKhoj — Screenshot Capture Script
// Captures screenshots of both PWA and extension sidebar for help docs + store listing
// Run with: npx playwright test capture-screenshots.js
// Requires: npm install -D @playwright/test

const { test, expect } = require('@playwright/test');
const path = require('path');

const SCREENSHOT_DIR = path.join(__dirname, '..', '..', 'biokhoj', 'screenshots');
const EXT_SCREENSHOT_DIR = path.join(__dirname, '..', '..', 'extension', 'biokhoj', 'shared', 'screenshots');

// ══════════════════════════════════════════════════════════════════════
// PWA Screenshots
// ══════════════════════════════════════════════════════════════════════

test.describe('PWA Screenshots', () => {
  test.beforeEach(async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 800 });
  });

  test('feed-tab', async ({ page }) => {
    await page.goto('http://localhost:8080/biokhoj/');
    await page.waitForTimeout(2000);
    await page.screenshot({ path: path.join(SCREENSHOT_DIR, 'pwa-feed.png'), fullPage: false });
  });

  test('feed-signal-breakdown', async ({ page }) => {
    await page.goto('http://localhost:8080/biokhoj/');
    await page.waitForTimeout(2000);
    const badge = page.locator('.signal-badge').first();
    if (await badge.isVisible()) {
      await badge.click();
      await page.waitForTimeout(500);
    }
    await page.screenshot({ path: path.join(SCREENSHOT_DIR, 'pwa-signal-breakdown.png'), fullPage: false });
  });

  test('feed-filters', async ({ page }) => {
    await page.goto('http://localhost:8080/biokhoj/');
    await page.waitForTimeout(1000);
    // Click multi-entity filter to show active state
    const multiBtn = page.locator('#comention-filter-btn');
    if (await multiBtn.isVisible()) await multiBtn.click();
    await page.waitForTimeout(500);
    await page.screenshot({ path: path.join(SCREENSHOT_DIR, 'pwa-feed-filters.png'), fullPage: false });
  });

  test('feed-insights-dropdown', async ({ page }) => {
    await page.goto('http://localhost:8080/biokhoj/');
    await page.waitForTimeout(1000);
    const insightsBtn = page.locator('#insights-dropdown-btn');
    if (await insightsBtn.isVisible()) await insightsBtn.click();
    await page.waitForTimeout(300);
    await page.screenshot({ path: path.join(SCREENSHOT_DIR, 'pwa-insights-dropdown.png'), fullPage: false });
  });

  test('discover-tab', async ({ page }) => {
    await page.goto('http://localhost:8080/biokhoj/');
    await page.waitForTimeout(1000);
    await page.click('[data-tab="discover"], .tab-btn[data-tab="discover"]');
    await page.waitForTimeout(1000);
    await page.screenshot({ path: path.join(SCREENSHOT_DIR, 'pwa-discover.png'), fullPage: false });
  });

  test('watchlist-tab', async ({ page }) => {
    await page.goto('http://localhost:8080/biokhoj/');
    await page.waitForTimeout(1000);
    await page.click('[data-tab="watchlist"], .tab-btn[data-tab="watchlist"]');
    await page.waitForTimeout(500);
    await page.screenshot({ path: path.join(SCREENSHOT_DIR, 'pwa-watchlist.png'), fullPage: false });
  });

  test('watchlist-preset-packs', async ({ page }) => {
    await page.goto('http://localhost:8080/biokhoj/');
    await page.waitForTimeout(1000);
    await page.click('[data-tab="watchlist"], .tab-btn[data-tab="watchlist"]');
    await page.waitForTimeout(500);
    await page.screenshot({ path: path.join(SCREENSHOT_DIR, 'pwa-preset-packs.png'), fullPage: false });
  });

  test('trends-tab', async ({ page }) => {
    await page.goto('http://localhost:8080/biokhoj/');
    await page.waitForTimeout(1000);
    await page.click('[data-tab="trends"], .tab-btn[data-tab="trends"]');
    await page.waitForTimeout(1000);
    await page.screenshot({ path: path.join(SCREENSHOT_DIR, 'pwa-trends.png'), fullPage: false });
  });

  test('reading-list-tab', async ({ page }) => {
    await page.goto('http://localhost:8080/biokhoj/');
    await page.waitForTimeout(1000);
    await page.click('[data-tab="reading-list"], .tab-btn[data-tab="reading-list"]');
    await page.waitForTimeout(500);
    await page.screenshot({ path: path.join(SCREENSHOT_DIR, 'pwa-reading-list.png'), fullPage: false });
  });

  test('reading-list-export', async ({ page }) => {
    await page.goto('http://localhost:8080/biokhoj/');
    await page.waitForTimeout(1000);
    await page.click('[data-tab="reading-list"], .tab-btn[data-tab="reading-list"]');
    await page.waitForTimeout(500);
    const exportBtn = page.locator('#export-btn');
    if (await exportBtn.isVisible()) await exportBtn.click();
    await page.waitForTimeout(300);
    await page.screenshot({ path: path.join(SCREENSHOT_DIR, 'pwa-export-dropdown.png'), fullPage: false });
  });

  test('settings-tab', async ({ page }) => {
    await page.goto('http://localhost:8080/biokhoj/');
    await page.waitForTimeout(1000);
    await page.click('[data-tab="settings"], .tab-btn[data-tab="settings"]');
    await page.waitForTimeout(500);
    await page.screenshot({ path: path.join(SCREENSHOT_DIR, 'pwa-settings.png'), fullPage: false });
  });

  test('settings-budget', async ({ page }) => {
    await page.goto('http://localhost:8080/biokhoj/');
    await page.waitForTimeout(1000);
    await page.click('[data-tab="settings"], .tab-btn[data-tab="settings"]');
    await page.waitForTimeout(500);
    const budgetCard = page.locator('#api-budget-card');
    if (await budgetCard.isVisible()) {
      await budgetCard.screenshot({ path: path.join(SCREENSHOT_DIR, 'pwa-budget.png') });
    }
  });

  test('dark-theme', async ({ page }) => {
    await page.goto('http://localhost:8080/biokhoj/');
    await page.waitForTimeout(2000);
    await page.screenshot({ path: path.join(SCREENSHOT_DIR, 'pwa-dark.png'), fullPage: false });
  });

  test('light-theme', async ({ page }) => {
    await page.goto('http://localhost:8080/biokhoj/');
    await page.waitForTimeout(1000);
    await page.click('[data-tab="settings"], .tab-btn[data-tab="settings"]');
    await page.waitForTimeout(300);
    const themeToggle = page.locator('#settings-theme');
    if (await themeToggle.isVisible()) await themeToggle.click();
    await page.waitForTimeout(300);
    await page.click('[data-tab="feed"], .tab-btn[data-tab="feed"]');
    await page.waitForTimeout(500);
    await page.screenshot({ path: path.join(SCREENSHOT_DIR, 'pwa-light.png'), fullPage: false });
  });

  test('mobile-view', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 812 });
    await page.goto('http://localhost:8080/biokhoj/');
    await page.waitForTimeout(2000);
    await page.screenshot({ path: path.join(SCREENSHOT_DIR, 'pwa-mobile.png'), fullPage: false });
  });
});

// ══════════════════════════════════════════════════════════════════════
// Extension Sidebar Screenshots (using sidebar.html directly)
// ══════════════════════════════════════════════════════════════════════

test.describe('Extension Sidebar Screenshots', () => {
  test.beforeEach(async ({ page }) => {
    await page.setViewportSize({ width: 350, height: 700 });
  });

  test('sidebar-recent', async ({ page }) => {
    await page.goto('file://' + path.resolve(__dirname, '..', '..', 'extension', 'biokhoj', 'chrome', 'sidebar.html'));
    await page.waitForTimeout(1000);
    await page.screenshot({ path: path.join(EXT_SCREENSHOT_DIR, 'sidebar-recent.png'), fullPage: false });
  });

  test('sidebar-watch', async ({ page }) => {
    await page.goto('file://' + path.resolve(__dirname, '..', '..', 'extension', 'biokhoj', 'chrome', 'sidebar.html'));
    await page.waitForTimeout(500);
    const watchTab = page.locator('.sidebar-tab[data-tab="watch"]');
    if (await watchTab.isVisible()) await watchTab.click();
    await page.waitForTimeout(300);
    await page.screenshot({ path: path.join(EXT_SCREENSHOT_DIR, 'sidebar-watch.png'), fullPage: false });
  });

  test('sidebar-search', async ({ page }) => {
    await page.goto('file://' + path.resolve(__dirname, '..', '..', 'extension', 'biokhoj', 'chrome', 'sidebar.html'));
    await page.waitForTimeout(500);
    const searchTab = page.locator('.sidebar-tab[data-tab="search"]');
    if (await searchTab.isVisible()) await searchTab.click();
    await page.waitForTimeout(300);
    await page.screenshot({ path: path.join(EXT_SCREENSHOT_DIR, 'sidebar-search.png'), fullPage: false });
  });

  test('sidebar-hot', async ({ page }) => {
    await page.goto('file://' + path.resolve(__dirname, '..', '..', 'extension', 'biokhoj', 'chrome', 'sidebar.html'));
    await page.waitForTimeout(500);
    const trendsTab = page.locator('.sidebar-tab[data-tab="trends"]');
    if (await trendsTab.isVisible()) await trendsTab.click();
    await page.waitForTimeout(300);
    await page.screenshot({ path: path.join(EXT_SCREENSHOT_DIR, 'sidebar-hot.png'), fullPage: false });
  });

  test('sidebar-narrow', async ({ page }) => {
    await page.setViewportSize({ width: 280, height: 700 });
    await page.goto('file://' + path.resolve(__dirname, '..', '..', 'extension', 'biokhoj', 'chrome', 'sidebar.html'));
    await page.waitForTimeout(500);
    await page.screenshot({ path: path.join(EXT_SCREENSHOT_DIR, 'sidebar-narrow.png'), fullPage: false });
  });

  test('sidebar-dark', async ({ page }) => {
    await page.goto('file://' + path.resolve(__dirname, '..', '..', 'extension', 'biokhoj', 'chrome', 'sidebar.html'));
    await page.waitForTimeout(1000);
    await page.screenshot({ path: path.join(EXT_SCREENSHOT_DIR, 'sidebar-dark.png'), fullPage: false });
  });

  test('sidebar-light', async ({ page }) => {
    await page.goto('file://' + path.resolve(__dirname, '..', '..', 'extension', 'biokhoj', 'chrome', 'sidebar.html'));
    await page.waitForTimeout(500);
    // Toggle theme
    const themeBtn = page.locator('#btn-theme');
    if (await themeBtn.isVisible()) await themeBtn.click();
    await page.waitForTimeout(300);
    await page.screenshot({ path: path.join(EXT_SCREENSHOT_DIR, 'sidebar-light.png'), fullPage: false });
  });
});
