import { expect, test } from "@playwright/test";

test("a workspace with a manifest shows its tasks", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("main").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.keyboard.press("Control+j");
  await page.locator(".panel-tabs").getByRole("button", { name: "assignment" }).click();

  const pane = page.locator(".assignment-pane");
  await expect(pane).toBeVisible();
  await expect(pane).toContainText("Week 1 — Sequence basics");
  await expect(pane).toContainText("0 of 2 complete");

  // Nothing has been checked yet, which is different from being wrong.
  await expect(pane.locator("li.pending")).toHaveCount(2);
  await expect(pane).toContainText("Not checked yet");

  // The instructor's hint is what a stuck student needs.
  await expect(pane).toContainText("not a percentage");
});

test("the assignment tab stays hidden without a manifest", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("main").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.keyboard.press("Control+j");

  // Remove the manifest and the tab must go with it.
  await page.locator('.tree-row[data-path="assignment.toml"]').click({ button: "right" });
  const remove = page.getByRole("menuitem", { name: /Delete/ });
  if (await remove.count()) {
    await remove.first().click();
    const confirm = page.getByRole("button", { name: /Delete/ }).last();
    if (await confirm.count()) await confirm.click();
    await expect(page.locator(".panel-tabs").getByRole("button", { name: "assignment" })).toHaveCount(0);
  }
});
