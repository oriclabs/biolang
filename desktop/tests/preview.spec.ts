import { expect, test } from "@playwright/test";

test("a FASTQ preview reports encoding, quality and GC rather than raw lines", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("main").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.locator('.tree-row[data-path="data/reads.fastq"]').click();

  const metrics = page.locator(".preview-metrics");
  await expect(metrics).toBeVisible({ timeout: 20_000 });
  await expect(metrics).toContainText("Reads sampled");
  await expect(metrics).toContainText("Phred+33");
  await expect(metrics).toContainText("GC content");

  // The per-position profile is what tells you where to trim. Asserted by
  // point count, not visibility: uniform demo quality draws a flat polyline
  // whose bounding box has zero height, which Playwright reports as hidden.
  await expect(page.getByRole("img", { name: /Mean quality by position/ })).toBeVisible();
  const points = await page.locator(".preview-chart-line").getAttribute("points");
  expect(points?.split(" ").length).toBeGreaterThan(1);
});

test("a VCF preview classifies variants and reports Ti/Tv and samples", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("main").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.locator('.tree-row[data-path="data/cohort.vcf"]').click();

  const metrics = page.locator(".preview-metrics");
  await expect(metrics).toBeVisible({ timeout: 20_000 });
  await expect(metrics).toContainText("Variants sampled");
  await expect(metrics).toContainText("SNVs");
  await expect(metrics).toContainText("Samples");
  await expect(page.getByRole("img", { name: /Variant classes/ })).toBeVisible();
});
