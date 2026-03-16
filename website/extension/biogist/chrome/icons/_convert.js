const { chromium } = require('@playwright/test');
const fs = require('fs');
const path = require('path');

(async () => {
  const browser = await chromium.launch();
  // Resolve icons dir relative to this script's location
  const ICONS_DIR = path.resolve(__dirname);

  for (const size of [16, 48, 128]) {
    const svgPath = path.join(ICONS_DIR, `icon${size}.svg`);
    if (!fs.existsSync(svgPath)) {
      console.error(`Missing: ${svgPath}`);
      continue;
    }
    const svgContent = fs.readFileSync(svgPath, 'utf8');
    const html = `<!DOCTYPE html><html><head><style>*{margin:0;padding:0;}body{width:${size}px;height:${size}px;overflow:hidden;}</style></head><body>${svgContent}</body></html>`;
    const page = await browser.newPage();
    await page.setViewportSize({ width: size, height: size });
    await page.setContent(html, { waitUntil: 'networkidle' });
    await page.waitForTimeout(500);
    await page.screenshot({ path: path.join(ICONS_DIR, `icon${size}.png`), clip: { x: 0, y: 0, width: size, height: size } });
    console.log(`OK icon${size}.png`);
    await page.close();
  }
  await browser.close();
})();
