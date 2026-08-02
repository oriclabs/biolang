import { expect, test } from "@playwright/test";

test("the Tests panel explains the convention and is Desktop-gated in the browser", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("main").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.keyboard.press("Control+j");
  await page.getByRole("button", { name: "View", exact: true }).click();
  await page.getByRole("menuitem", { name: "Tests Tab", exact: true }).click();
  await page.locator(".panel-tabs").getByRole("button", { name: "tests" }).click();

  const pane = page.locator(".test-pane");
  await expect(pane).toBeVisible();
  await expect(pane).toContainText("No tests run yet");
  await expect(pane).toContainText("test_something");
  // The browser build has no `bl` binary to run a suite with.
  await expect(pane.getByRole("button", { name: "Run all tests" })).toBeDisabled();
});
