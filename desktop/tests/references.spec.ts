import { expect, test } from "@playwright/test";

test("Settings explains reference builds and gates them to Desktop", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("main").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.locator(".activity-bar").getByLabel("Settings").click();
  await page.getByRole("tab", { name: "References" }).click();

  await expect(page.getByText("Reference builds", { exact: true })).toBeVisible();
  await expect(page.getByText("references.toml", { exact: false })).toBeVisible();
  // The registry lives in the home directory, which the browser cannot reach.
  await expect(page.getByLabel("Reference build name")).toHaveCount(0);
});
