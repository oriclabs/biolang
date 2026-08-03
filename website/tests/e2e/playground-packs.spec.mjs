import { expect, test } from "@playwright/test";

/**
 * The playground's example menu is driven by the published pack catalog rather
 * than a hardcoded list, so a new pack appears without editing wasm-playground.js.
 *
 * These run against the real /packs/index.json produced by build-packs.mjs.
 * `website/packs/` is generated and gitignored, so run
 * `node scripts/build-packs.mjs --out website/packs` before this suite —
 * deliberately not skipped when absent, since a silent skip would hide the
 * catalog disappearing from a deploy.
 */

test("pack problems appear in the examples menu and run", async ({ page }) => {
  await page.goto("/playground.html");

  await page.locator("#pg-examples-toggle").click();
  const menu = page.locator("#pg-examples-menu");

  // The catalog is fetched after load, so the pack heading arrives late.
  await expect(menu.locator("button", { hasText: "Rosalind — Bioinformatics Armory" })).toBeVisible({
    timeout: 30_000,
  });

  const ini = menu.locator('[data-pack-ex="rosalind-armory:INI"]');
  await expect(ini).toContainText("Introduction to the Bioinformatics Armory");
  await ini.click();

  // The real pack source lands in the editor…
  await expect(page.locator("#pg-editor")).toHaveValue(/Rosalind: INI/, { timeout: 30_000 });
  // …and running it produces the answer the pack asserts.
  await expect(page.locator("#pg-output")).toContainText("20 12 17 21", { timeout: 120_000 });
});

test("network problems are labelled before they are chosen", async ({ page }) => {
  await page.goto("/playground.html");
  await page.locator("#pg-examples-toggle").click();
  const menu = page.locator("#pg-examples-menu");

  await expect(menu.locator('[data-pack-ex="rosalind-armory:GBK"]')).toContainText("needs NCBI", {
    timeout: 30_000,
  });
  // MEME does not reproduce the official answer, and says so up front.
  await expect(menu.locator('[data-pack-ex="rosalind-armory:MEME"]')).toContainText("partial");
});

test("the built-in examples still work when no catalog is published", async ({ page }) => {
  // A site built before packs existed, or an offline copy, must not lose its
  // own examples because a fetch failed.
  await page.route("**/packs/index.json", (route) => route.fulfill({ status: 404, body: "" }));
  await page.goto("/playground.html");

  await page.locator("#pg-examples-toggle").click();
  const hello = page.locator("#pg-examples-menu button", { hasText: "Hello DNA" });
  await expect(hello).toBeVisible();
  await hello.click();
  await expect(page.locator("#pg-output")).toContainText("GC", { timeout: 120_000 });
});
