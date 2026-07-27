import { test, expect } from "@playwright/test";

async function runExample(page, sourceText) {
  const block = page.locator("pre").filter({ hasText: sourceText });
  const wrapper = block.locator("xpath=..");
  const run = wrapper.locator(".bl-run-btn");

  await expect(run).toBeVisible();
  await run.click();
  await expect(run).toBeEnabled({ timeout: 150_000 });
  return wrapper.locator(".bl-output:visible");
}

test("SVG-returning bio plots render visually", async ({ page }) => {
  await page.goto("/docs/builtins/bio-plots.html", { waitUntil: "domcontentloaded" });

  const documentedOncoprint = page.locator("#oncoprint > .mt-2 svg");
  await expect(documentedOncoprint.locator("text")).toContainText([
    "TP53",
    "BRCA1",
    "EGFR",
    "OncoPrint",
  ]);
  await expect(documentedOncoprint).not.toContainText("Mutation Landscape");

  const oncoprint = await runExample(page, "oncoprint(muts");
  await expect(oncoprint.locator("svg")).toBeVisible();
  await expect(oncoprint.locator("svg text").filter({ hasText: "OncoPrint" })).toBeVisible();
  await expect(oncoprint).not.toContainText('→ "<svg');

  const sashimi = await runExample(page, "sashimi(junctions");
  await expect(sashimi.locator("svg")).toBeVisible();
  await expect(sashimi).not.toContainText('→ "<svg');
});
