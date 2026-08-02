import { expect, test } from "@playwright/test";

test("figures export at print resolution as well as screen resolution", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("main").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.locator('.tree-row[data-path="analysis.bl"]').click();
  const editor = page.locator(".editor-host .monaco-editor").first();
  await expect(editor).toBeVisible({ timeout: 20_000 });
  await editor.click();
  await page.keyboard.press("Control+A");
  await page.keyboard.insertText('phylo_tree("(TP53:1,BRCA1:1);")');
  await page.getByLabel("Run active BioLang file").click();
  await expect(page.getByRole("img", { name: "BioLang plot output" })).toBeVisible({ timeout: 20_000 });

  // 72 DPI was the only option before; journals ask for 300.
  await expect(page.getByRole("button", { name: "PNG", exact: true })).toBeVisible();
  const print = page.getByRole("button", { name: "PNG 4x" });
  await expect(print).toBeVisible();
  await expect(print).toHaveAttribute("title", /300 DPI/);
});

test("the run bundle is named for what it is", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("main").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.locator('.tree-row[data-path="analysis.bl"]').click();
  await expect(page.locator(".monaco-editor")).toBeVisible({ timeout: 20_000 });
  await page.getByLabel("Run active BioLang file").click();
  await expect(page.locator(".output-view")).toContainText("Process completed", { timeout: 20_000 });

  await page.getByLabel("More Output actions").click();
  await expect(page.getByRole("button", { name: "Export reproducibility bundle", exact: true })).toBeVisible();
});
