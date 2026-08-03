import { expect, test } from "@playwright/test";

test("a run offers a methods paragraph and citation built from its provenance", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("main").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.locator('.tree-row[data-path="analysis.bl"]').click();
  await expect(page.locator(".monaco-editor")).toBeVisible({ timeout: 20_000 });
  await page.getByLabel("Run active BioLang file").click();
  await expect(page.locator(".output-view")).toContainText("Process completed", { timeout: 20_000 });

  await page.getByRole("button", { name: "provenance", exact: true }).click();
  await page.getByRole("button", { name: "Methods and citation" }).click();

  const body = page.locator(".methods-body");
  await expect(body).toContainText("Analyses were performed using BioLang");
  await expect(body).toContainText("@software{biolang");
  // The caveat has to sit next to the text people are about to paste.
  await expect(page.locator(".methods-note")).toContainText("Check it against your manuscript");
});

test("About tells researchers how to cite the tool", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("main").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.getByRole("button", { name: "Help", exact: true }).click();
  await page.getByRole("menuitem", { name: /About/ }).click();

  const about = page.locator(".about-citation");
  await expect(about).toContainText("Citing BioLang");
  await expect(about).toContainText("Bandi, Raj");
  await expect(about.getByRole("button", { name: "Copy BibTeX" })).toBeVisible();
});
