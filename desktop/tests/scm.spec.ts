import { expect, test } from "@playwright/test";

test("Source Control is reachable and reports when the folder is not a repository", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("main").getByRole("button", { name: "Open Browser Workspace" }).click();

  await page.locator(".activity-bar").getByLabel("Source Control").click();
  await expect(page.locator(".sidebar-title")).toContainText("Source Control");
  // The browser workspace has no Git repository behind it.
  await expect(page.locator(".sidebar-empty")).toContainText("not a Git repository");

  // The shortcut reaches the same view.
  await page.locator(".activity-bar").getByLabel("Explorer").click();
  await page.keyboard.press("Control+Shift+G");
  await expect(page.locator(".sidebar-title")).toContainText("Source Control");
});
