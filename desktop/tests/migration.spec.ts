import { expect, test } from "@playwright/test";

test("dplyr and pandas names complete to their BioLang equivalents", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("main").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.locator('.tree-row[data-path="analysis.bl"]').click();
  const editor = page.locator(".editor-host .monaco-editor").first();
  await expect(editor).toBeVisible({ timeout: 20_000 });
  await editor.click();
  await page.keyboard.press("Control+A");

  await page.keyboard.insertText("summari");
  await page.keyboard.press("Control+Space");
  const suggestions = page.locator(".suggest-widget.visible");
  await expect(suggestions).toBeVisible();
  await expect(suggestions).toContainText("summarise");
  await expect(suggestions).toContainText("dplyr");
  await page.keyboard.press("Enter");

  const text = await page.locator(".editor-host .view-lines .view-line").allInnerTexts();
  expect(text.join("")).toContain("summarize");
});

test("the Help Center carries a Coming from R or Python chapter", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("main").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.getByLabel("Help").click();
  await page.getByPlaceholder("Search all BioLang help").fill("coming from");
  await page.getByRole("button", { name: /Coming from R or Python/ }).first().click();

  const help = page.locator(".help-markdown");
  await expect(help).toBeVisible({ timeout: 20_000 });
  await expect(help).toContainText("pivot_longer");
  await expect(help).toContainText("%>%");
});

test("conversion markers survive the import and become a porting work list", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("main").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.locator('.tree-row[data-path="analysis.bl"]').click();
  const editor = page.locator(".editor-host .monaco-editor").first();
  await expect(editor).toBeVisible({ timeout: 20_000 });
  await editor.click();
  await page.keyboard.press("Control+A");
  await page.keyboard.insertText(
    'let counts = read_csv("counts.csv")\n# TODO: edgeR not yet in BioLang\nlet size = 1  # approximated from library size\nlet label = "TODO: this is data, not a marker"\n',
  );

  await page.keyboard.press("Control+j");
  await page.locator(".panel-tabs").getByRole("button", { name: "problems" }).click();
  const rows = page.locator(".problem-row");

  // The comment markers count; the string literal does not.
  await expect(rows).toHaveCount(2, { timeout: 20_000 });
  await expect(rows.first()).toContainText("Needs manual porting");
  await expect(rows.first()).toContainText("edgeR not yet in BioLang");
  await expect(rows.nth(1)).toContainText("approximation");
});
