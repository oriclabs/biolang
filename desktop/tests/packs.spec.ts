import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "@playwright/test";

/**
 * Example-pack deep links.
 *
 * The catalog and bundles are served from the real build output rather than a
 * fixture, so these fail if `scripts/build-packs.mjs` changes shape — the whole
 * point of the pack format is that the published artifact is what clients get.
 */

const here = path.dirname(fileURLToPath(import.meta.url));
const packsRoot = path.resolve(here, "..", "..", "website", "packs");

async function servePacks(page: import("@playwright/test").Page) {
  await page.route("**/packs/*.json", async (route) => {
    const name = new URL(route.request().url()).pathname.split("/").pop() ?? "";
    try {
      const body = await readFile(path.join(packsRoot, name), "utf8");
      await route.fulfill({ status: 200, contentType: "application/json", body });
    } catch {
      await route.fulfill({ status: 404, body: "pack artifacts not built" });
    }
  });
}

test("a pack deep link downloads the pack and opens the problem", async ({ page }) => {
  await servePacks(page);
  await page.goto("/?pack=rosalind-armory&problem=INI");

  await expect(page.locator(".monaco-editor").first()).toBeVisible({ timeout: 30_000 });
  await expect(page.locator(".view-line").first()).toContainText("Rosalind: INI");

  // The whole pack lands in the workspace, not only the linked problem.
  await expect(page.locator('.tree-row[data-path="rosalind-armory/examples/subo.bl"]')).toBeVisible();
});

test("the problem id is matched case-insensitively", async ({ page }) => {
  await servePacks(page);
  // Rosalind writes ids upper-case in its URLs and people retype them by hand.
  await page.goto("/?pack=rosalind-armory&problem=rvco");
  await expect(page.locator(".monaco-editor").first()).toBeVisible({ timeout: 30_000 });
  await expect(page.locator(".view-line").first()).toContainText("RVCO");
});

test("an unknown pack reports the name instead of failing silently", async ({ page }) => {
  await servePacks(page);
  await page.goto("/?pack=not-a-pack");
  // Errors surface twice by design — a toast and the sticky "Recent errors"
  // list — so scope to the toast rather than matching both.
  await expect(page.getByRole("status")).toContainText('No example pack named "not-a-pack"', {
    timeout: 30_000,
  });
});

test("an unknown problem still installs the pack and says what was missing", async ({ page }) => {
  await servePacks(page);
  await page.goto("/?pack=rosalind-armory&problem=ZZZZ");
  await expect(page.getByRole("status")).toContainText('has no problem "ZZZZ"', { timeout: 30_000 });
  await expect(page.locator('.tree-row[data-path="rosalind-armory/examples/ini.bl"]')).toBeVisible();
});

test("an unreachable catalog says so rather than showing an empty pack list", async ({ page }) => {
  await page.route("**/packs/*.json", (route) => route.fulfill({ status: 503, body: "down" }));
  await page.goto("/?pack=rosalind-armory");
  await expect(page.getByRole("status")).toContainText("Pack catalog unavailable", {
    timeout: 30_000,
  });
});

test("following the same link twice does not duplicate the pack", async ({ page }) => {
  await servePacks(page);
  await page.goto("/?pack=rosalind-armory&problem=INI");
  await expect(page.locator(".monaco-editor").first()).toBeVisible({ timeout: 30_000 });

  await page.goto("/?pack=rosalind-armory&problem=INI");
  await expect(page.locator(".monaco-editor").first()).toBeVisible({ timeout: 30_000 });

  // One folder, one copy of each file — installing is a replace, not an append.
  await expect(page.locator('.tree-row[data-path="rosalind-armory"]')).toHaveCount(1);
  await expect(page.locator('.tree-row[data-path="rosalind-armory/examples/ini.bl"]')).toHaveCount(1);
});

test("a downloaded pack runs in the browser and reports the right answer", async ({ page }) => {
  await servePacks(page);
  await page.goto("/?pack=rosalind-armory&problem=INI");
  const editor = page.locator(".monaco-editor").first();
  await expect(editor).toBeVisible({ timeout: 30_000 });

  // The point of shipping packs to the browser: pressing Run has to work, not
  // just the download. Driven from the menu rather than the shortcut so the
  // assertion does not depend on where focus happens to be.
  await page.getByRole("button", { name: "Run", exact: true }).click();
  await page.getByRole("menuitem", { name: /Run Active File/ }).click();
  await expect(page.locator(".output-pane-body")).toContainText("Match:", { timeout: 25_000 });
  await expect(page.locator(".output-pane-body")).not.toContainText("false");
});
