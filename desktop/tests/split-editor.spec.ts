import { expect, test } from "@playwright/test";

test("expert mode can split the editor to the right", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("main").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.locator(".activity-bar").getByLabel("Settings").click();
  await page.locator(".settings-dialog").getByRole("button", { name: "Expert", exact: true }).click();
  await page.locator(".settings-dialog").getByRole("button", { name: "Close", exact: true }).click();

  await page.locator('.tree-row[data-path="analysis.bl"]').click();
  await expect(page.locator(".monaco-editor")).toBeVisible({ timeout: 20_000 });

  await page.getByLabel("Split editor right").click();
  await expect(page.locator(".editor-split-body")).toBeVisible();
  await expect(page.locator(".editor-group.secondary")).toBeVisible();
  await expect(page.locator(".editor-group.secondary .editor-tab.active")).toContainText("analysis.bl");

  // Open another file into the secondary (focused) group.
  await page.locator('.tree-row[data-path="sequence_basics.bl"]').click();
  await expect(page.locator(".editor-group.secondary")).toContainText("sequence_basics.bl");

  await page.getByLabel("Close editor split").first().click();
  await expect(page.locator(".editor-split-body")).toHaveCount(0);
});
