import { expect, test } from "@playwright/test";

test("Learner mode guides the first run and tracks real progress", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("main").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.locator(".activity-bar").getByLabel("Settings").click();
  await page.locator(".settings-dialog").getByRole("button", { name: "Learner", exact: true }).click();
  await page.locator(".settings-dialog").getByRole("button", { name: "Close", exact: true }).click();

  const guide = page.getByRole("complementary", { name: "Getting started" });
  await expect(guide).toBeVisible();
  // The workspace step is already done, so it starts at the next one.
  await expect(guide).toContainText("Open a BioLang file");
  await expect(guide.locator("li.done")).toHaveCount(1);

  await page.locator('.tree-row[data-path="analysis.bl"]').click();
  await expect(page.locator(".monaco-editor")).toBeVisible({ timeout: 20_000 });
  await expect(guide).toContainText("Run it");
  await expect(guide.locator("li.done")).toHaveCount(2);

  await page.getByLabel("Run active BioLang file").click();
  await expect(guide).toHaveClass(/complete/, { timeout: 20_000 });
  await expect(guide).toContainText("You have the basics");
  await expect(guide.locator("li")).toHaveCount(0);

  await guide.getByLabel("Dismiss the guide").click();
  await expect(guide).toBeHidden();
});

test("Learner mode hides the panels a newcomer has no use for", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("main").getByRole("button", { name: "Open Browser Workspace" }).click();
  const bar = page.locator(".activity-bar");
  await expect(bar.getByLabel("Bio APIs")).toBeVisible();

  await bar.getByLabel("Settings").click();
  await page.locator(".settings-dialog").getByRole("button", { name: "Learner", exact: true }).click();
  await page.locator(".settings-dialog").getByRole("button", { name: "Close", exact: true }).click();

  await expect(bar.getByLabel("Bio APIs")).toHaveCount(0);
  await expect(bar.getByLabel("Source Control")).toHaveCount(0);
  await expect(bar.getByLabel("Jobs")).toHaveCount(0);
  await expect(bar.getByLabel("Explorer")).toBeVisible();
});

test("empty workspace offers concrete next actions", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("main").getByRole("button", { name: "Open Browser Workspace" }).click();
  const empty = page.locator(".workspace-welcome.compact");
  await expect(empty.getByRole("button", { name: "New BioLang file" })).toBeVisible();
  await expect(empty.getByRole("button", { name: "Import data" })).toBeVisible();
  await expect(empty.getByRole("button", { name: "Go to file" })).toBeVisible();
});

test("welcome example opens and runs a starter analysis", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: /Sequence QC/i }).click();
  await expect(page.locator(".monaco-editor")).toBeVisible({ timeout: 20_000 });
  await expect(page.locator(".output-pane")).toBeVisible({ timeout: 20_000 });
  await expect(page.locator(".output-pane")).toContainText(/Length|GC|running|succeeded|failed/i, { timeout: 20_000 });
});

test("tutorial project opens the demo workspace on analysis.bl", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: "Open tutorial project" }).click();
  await expect(page.locator(".monaco-editor")).toBeVisible({ timeout: 20_000 });
  await expect(page.locator(".editor-tab.active")).toContainText("analysis.bl");
  await expect(page.getByRole("complementary", { name: "Getting started" })).toBeVisible();
  await expect(page.getByLabel("Execution target for Run")).toBeVisible();
});

test("settings uses section tabs", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("main").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.locator(".activity-bar").getByLabel("Settings").click();
  await expect(page.getByRole("tab", { name: "Editor" })).toHaveAttribute("aria-selected", "true");
  await page.getByRole("tab", { name: "Remote" }).click();
  await expect(page.getByText("Add connection")).toBeVisible();
  await page.getByRole("tab", { name: "Trust" }).click();
  await expect(page.getByText("Workspace trust")).toBeVisible();
});

test("Output keeps primary actions and hides power tools under More", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("main").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.locator(".activity-bar").getByLabel("Settings").click();
  await page.locator(".settings-dialog").getByRole("button", { name: "Expert", exact: true }).click();
  await page.locator(".settings-dialog").getByRole("button", { name: "Close", exact: true }).click();
  await page.locator('.tree-row[data-path="analysis.bl"]').click();
  await expect(page.locator(".monaco-editor")).toBeVisible({ timeout: 20_000 });
  await page.getByLabel("Run active BioLang file").click();
  await expect(page.locator(".output-pane")).toBeVisible({ timeout: 20_000 });
  await expect(page.locator(".output-pane").getByRole("button", { name: "Rerun" })).toBeVisible();
  await expect(page.locator(".output-pane").getByRole("button", { name: "Export" })).toBeVisible();
  await expect(page.locator(".output-pane").getByLabel("More Output actions")).toBeVisible();
  await expect(page.locator(".output-pane").getByLabel("Compare runs")).toHaveCount(0);
});
