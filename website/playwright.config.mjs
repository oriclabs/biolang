import { defineConfig, devices } from "@playwright/test";

/**
 * Self-contained config: `npx playwright test` starts its own static server,
 * so the run-button suite needs nothing else set up first.
 *
 * Env knobs:
 *   BL_BASE_URL      point at an already-running server instead of starting one
 *   BL_DIR           restrict discovery to a subdirectory (e.g. docs/bio)
 *   BL_MAX_BLOCKS    cap Run clicks per page (execution is cumulative, so a
 *                    long page is quadratic — useful for a fast smoke run)
 */
const baseURL = process.env.BL_BASE_URL || "http://localhost:3000";

export default defineConfig({
  testDir: "./tests/e2e",
  // WASM is ~6 MB and the first click on a page pays for the download plus a
  // cumulative replay of earlier blocks, so pages need real headroom.
  timeout: 5 * 60 * 1000,
  expect: { timeout: 30 * 1000 },
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: 0,
  workers: process.env.CI ? 2 : 4,
  reporter: [
    ["list"],
    ["json", { outputFile: "tests/e2e/results.json" }],
    ["html", { outputFolder: "tests/e2e/report", open: "never" }],
  ],
  use: {
    baseURL,
    ...devices["Desktop Chrome"],
    // Every failure keeps a trace; there is no point re-running a 5-minute page.
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
    video: "off",
  },
  webServer: process.env.BL_BASE_URL
    ? undefined
    : {
        command: "npx http-server . -p 3000 -c-1 --ext html --silent",
        url: "http://localhost:3000/docs/index.html",
        reuseExistingServer: true,
        timeout: 60 * 1000,
      },
});
