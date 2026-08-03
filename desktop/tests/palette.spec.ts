import { expect, test } from "@playwright/test";

test("the command palette is fully keyboard driven and matches fuzzily", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("main").getByRole("button", { name: "Open Browser Workspace" }).click();

  await page.keyboard.press("Control+Shift+P");
  const input = page.getByRole("combobox", { name: "Run a command" });
  await expect(input).toBeVisible();
  await expect(input).toHaveValue(">");

  // "raf" is not a substring of "BioLang: Run Active File" — the old palette
  // filtered with includes() and would have shown nothing.
  await page.keyboard.type("raf");
  const options = page.locator("#palette-results [role=option]");
  await expect(options.first()).toContainText("Run Active File");
  await expect(options.first()).toHaveAttribute("aria-selected", "true");

  await page.keyboard.press("ArrowDown");
  await expect(options.nth(1)).toHaveAttribute("aria-selected", "true");
  await expect(options.first()).toHaveAttribute("aria-selected", "false");
  await page.keyboard.press("ArrowUp");
  await expect(options.first()).toHaveAttribute("aria-selected", "true");

  await page.keyboard.press("Escape");
  await expect(input).toBeHidden();
});

test("Ctrl+P opens files that are not already open, and Enter runs the selection", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("main").getByRole("button", { name: "Open Browser Workspace" }).click();

  await page.keyboard.press("Control+p");
  const input = page.getByRole("combobox", { name: "Go to file" });
  await expect(input).toBeVisible();
  await page.keyboard.type("analysis");
  await expect(page.locator("#palette-results [role=option]").first()).toContainText("analysis.bl");

  await page.keyboard.press("Enter");
  await expect(input).toBeHidden();
  await expect(page.locator(".editor-tab.active")).toContainText("analysis.bl");
  await expect(page.locator(".monaco-editor")).toBeVisible({ timeout: 20_000 });
});

test("Ctrl+Shift+O lists symbols from the active file", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("main").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.locator('.tree-row[data-path="analysis.bl"]').click();
  await expect(page.locator(".monaco-editor")).toBeVisible({ timeout: 20_000 });

  await page.keyboard.press("Control+Shift+O");
  const input = page.getByRole("combobox", { name: "Go to symbol" });
  await expect(input).toBeVisible();
  await expect(input).toHaveValue("@");
  await expect(page.locator("#palette-results [role=option]").first()).toBeVisible();
});
