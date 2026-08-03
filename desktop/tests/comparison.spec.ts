import { expect, test } from "@playwright/test";

test("the welcome screen compares the same analysis in three languages", async ({ page }) => {
  await page.goto("/");
  const panel = page.locator(".welcome-comparison");
  await expect(panel).toBeVisible();
  await panel.locator("summary").click();
  await expect(panel).toContainText("Nothing to install");
  await expect(panel).toContainText("read_fasta");

  await panel.getByRole("tab", { name: "Python" }).click();
  await expect(panel).toContainText("biopython, pandas");
  await expect(panel).toContainText("SeqIO.parse");

  await panel.getByRole("tab", { name: "R" }).click();
  await expect(panel).toContainText("Biostrings, dplyr");
  await expect(panel).toContainText("readDNAStringSet");
});

test("the BioLang side of the comparison actually runs", async ({ page }) => {
  await page.goto("/");
  const panel = page.locator(".welcome-comparison");
  await panel.locator("summary").click();
  await panel.getByRole("tab", { name: "BioLang" }).click();
  await panel.getByRole("button", { name: "Run it" }).click();

  await expect(page.locator(".editor-tab.active")).toContainText("compare_fasta_gc.bl");
  await page.getByLabel("Run active BioLang file").click();

  // The claim on the welcome screen has to hold: this produces a real table.
  await expect(page.getByRole("button", { name: "tables", exact: true })).toBeVisible({ timeout: 20_000 });
  await page.getByRole("button", { name: "tables", exact: true }).click();
  await expect(page.locator(".output-tables-view")).toContainText("gc");
  await expect(page.locator(".output-tables-view")).toContainText("ori_candidate");
});
