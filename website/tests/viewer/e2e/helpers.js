// Shared helpers for BioPeek E2E tests
const path = require('path');

const VIEWER_URL = 'file:///' + path.resolve(__dirname, '..', '..', '..', 'viewer.html').replace(/\\/g, '/');
const FIXTURES_DIR = path.resolve(__dirname, '..', 'fixtures');

/**
 * Navigate to viewer and wait for it to be ready.
 */
async function openViewer(page) {
  await page.goto(VIEWER_URL, { waitUntil: 'domcontentloaded' });
  // Wait for the drop zone to be visible (app initialized)
  await page.waitForSelector('#vw-drop-zone', { state: 'visible', timeout: 10000 });
}

/**
 * Load a fixture file into the viewer by setting the file input.
 * Returns after the workspace and table are rendered.
 */
async function loadFixture(page, fileName) {
  const filePath = path.join(FIXTURES_DIR, fileName);
  const fileInput = page.locator('#vw-file-input');
  await fileInput.setInputFiles(filePath);

  // Wait for workspace to become active
  await page.waitForSelector('#vw-workspace.active', { timeout: 15000 });
  // Wait for a tab to appear
  await page.waitForSelector('.vw-tab', { timeout: 10000 });
  // Wait for format chip to update (signals parsing is done)
  await page.waitForFunction(() => {
    const chip = document.getElementById('vw-format-chip');
    return chip && chip.textContent !== 'FASTA' || document.querySelector('.vw-tab');
  }, { timeout: 10000 });
  // Small delay for rendering
  await page.waitForTimeout(300);
}

/**
 * Get the format badge text from the toolbar.
 */
async function getFormatBadge(page) {
  return await page.locator('#vw-format-chip').textContent();
}

/**
 * Get the record count text from the toolbar.
 */
async function getRecordCount(page) {
  return await page.locator('#vw-count-chip').textContent();
}

/**
 * Get the footer row count text.
 */
async function getFooterRows(page) {
  return await page.locator('#vw-footer-rows').textContent();
}

/**
 * Get all visible table header texts (excluding the row-number column '#').
 */
async function getTableHeaders(page) {
  const headers = await page.locator('.vw-table th').allTextContents();
  // Clean up sort arrows and whitespace
  return headers.map(h => h.replace(/[▲▼↕]/g, '').trim()).filter(h => h !== '#' && h !== '');
}

/**
 * Get the text content of a specific table cell by row and column index.
 * Row 0 is the first data row, col 0 is the first column after '#'.
 */
async function getCellText(page, row, col) {
  // +1 for the row-number column (#), +1 because nth-child is 1-based
  const colSelector = col + 2;
  const rowSelector = row + 1; // tbody rows are 1-based
  const cell = page.locator(`.vw-table tbody tr:nth-child(${rowSelector}) td:nth-child(${colSelector})`);
  return (await cell.textContent()).trim();
}

/**
 * Get the number of visible table rows.
 */
async function getVisibleRowCount(page) {
  return await page.locator('.vw-table tbody tr').count();
}

/**
 * Switch to a view mode by clicking the toolbar button.
 * @param {'table'|'stats'|'raw'|'console'} viewName
 */
async function switchView(page, viewName) {
  await page.locator(`#vw-view-${viewName}`).click();
  await page.waitForTimeout(300);
}

module.exports = {
  VIEWER_URL,
  FIXTURES_DIR,
  openViewer,
  loadFixture,
  getFormatBadge,
  getRecordCount,
  getFooterRows,
  getTableHeaders,
  getCellText,
  getVisibleRowCount,
  switchView,
};
