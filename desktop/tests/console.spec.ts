import { expect, test } from "@playwright/test";

test("Shift+Enter sends the current line to the console and advances", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("main").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.locator('.tree-row[data-path="analysis.bl"]').click();
  const editor = page.locator(".editor-host .monaco-editor").first();
  await expect(editor).toBeVisible({ timeout: 20_000 });
  await editor.click();
  await page.keyboard.press("Control+A");
  await page.keyboard.insertText('println("first line")\nprintln("second line")');

  // Put the caret back on line one, then step through it.
  await page.keyboard.press("Control+Home");
  await page.keyboard.press("Shift+Enter");

  const consolePane = page.locator(".console-pane");
  await expect(consolePane).toBeVisible({ timeout: 20_000 });
  await expect(consolePane.locator(".console-prompt")).toContainText('println("first line")');
  await expect(consolePane.locator(".console-transcript")).toContainText("first line", { timeout: 20_000 });

  // Focus stayed in the file editor and the caret moved on, so pressing it
  // again steps to the next statement without touching the mouse.
  await page.keyboard.press("Shift+Enter");
  await expect(consolePane.locator(".console-prompt").nth(1)).toContainText('println("second line")');
  await expect(consolePane.locator(".console-transcript")).toContainText("second line", { timeout: 20_000 });
});

test("Shift+Enter sends a multi-line selection as one submission", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("main").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.locator('.tree-row[data-path="analysis.bl"]').click();
  const editor = page.locator(".editor-host .monaco-editor").first();
  await expect(editor).toBeVisible({ timeout: 20_000 });
  await editor.click();
  await page.keyboard.press("Control+A");
  await page.keyboard.insertText("let a = 2\nlet b = 3\nprintln(a * b)");
  await page.keyboard.press("Control+A");
  await page.keyboard.press("Shift+Enter");

  const consolePane = page.locator(".console-pane");
  await expect(consolePane).toBeVisible({ timeout: 20_000 });
  await expect(consolePane.locator(".console-prompt")).toContainText("let a = 2");
  await expect(consolePane.locator(".console-transcript")).toContainText("6", { timeout: 20_000 });
});
