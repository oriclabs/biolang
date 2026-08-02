import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

async function openSearch(page: Page) {
  await page.goto("/");
  await page.getByRole("main").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.locator(".activity-bar").getByLabel("Search").click();
  return page.getByPlaceholder("Search file contents");
}

test("search groups hits by file and honours the match toggles", async ({ page }) => {
  const field = await openSearch(page);
  await field.fill("gc");
  await expect(page.locator(".result-count")).toContainText("results in", { timeout: 20_000 });
  await expect(page.locator(".search-file-heading").first()).toBeVisible();

  // Whole word rules out `gc_content`, so the count must drop.
  const before = Number((await page.locator(".result-count").innerText()).split(" ")[0]);
  await page.getByLabel("Match whole word").click();
  await expect
    .poll(async () => Number((await page.locator(".result-count").innerText()).split(" ")[0]), { timeout: 20_000 })
    .toBeLessThan(before);
});

test("an incomplete regular expression is reported, not thrown", async ({ page }) => {
  const field = await openSearch(page);
  await page.getByLabel("Use regular expression").click();
  await field.fill("gc(");
  await expect(page.locator(".result-count")).toContainText("Incomplete regular expression", { timeout: 20_000 });
});

test("Replace All rewrites matches across the workspace after confirmation", async ({ page }) => {
  const field = await openSearch(page);
  await page.getByLabel("Show replace").click();
  await field.fill("Reverse complement");
  await expect(page.locator(".result-count")).toContainText("results in", { timeout: 20_000 });

  await page.getByLabel("Replace with").fill("Revcomp");
  await page.getByRole("button", { name: "Replace All" }).click();
  await page.getByRole("button", { name: "Replace All", exact: true }).last().click();

  await expect(page.locator(".toast")).toContainText("Replaced in", { timeout: 20_000 });
  await field.fill("Revcomp");
  await expect(page.locator(".result-count")).toContainText("results in", { timeout: 20_000 });
});
