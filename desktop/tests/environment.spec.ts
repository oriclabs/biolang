import { expect, test } from "@playwright/test";

test("clicking an environment object evaluates it and fields can be drilled into", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("main").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.getByRole("button", { name: "Run", exact: true }).click();
  await page.getByRole("menuitem", { name: "BioLang Console Ctrl+Shift+`", exact: true }).click();

  const consolePane = page.locator(".console-pane");
  await expect(consolePane).toBeVisible({ timeout: 20_000 });
  await consolePane.locator(".console-editor").click();
  await page.keyboard.insertText("let stats = {mean: 4, total: 12}");
  await page.keyboard.press("Control+Enter");

  const row = consolePane.locator(".console-variable-open", { hasText: "stats" });
  await expect(row).toBeVisible({ timeout: 20_000 });

  // The row was an inert div before; clicking it now evaluates the name.
  await row.click();
  await expect(consolePane.locator(".console-prompt").last()).toContainText("stats", { timeout: 20_000 });

  await consolePane.getByLabel("Show fields of stats").click();
  const members = consolePane.locator(".console-variable-members button");
  await expect(members.filter({ hasText: "mean" })).toBeVisible();
  await members.filter({ hasText: "mean" }).click();
  await expect(consolePane.locator(".console-prompt").last()).toContainText("stats.mean", { timeout: 20_000 });
});
