import { expect, test } from "@playwright/test";

test("a run's provenance offers a reproducibility check", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("main").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.locator('.tree-row[data-path="analysis.bl"]').click();
  await expect(page.locator(".monaco-editor")).toBeVisible({ timeout: 20_000 });
  await page.getByLabel("Run active BioLang file").click();
  await expect(page.locator(".output-view")).toContainText("Process completed", { timeout: 20_000 });

  await page.getByRole("button", { name: "provenance", exact: true }).click();
  const panel = page.locator(".restore-panel");
  await expect(panel).toBeVisible();

  // The browser build has no workspace on disk to compare against. It must say
  // so rather than reporting an empty diff as "nothing changed".
  await panel.getByRole("button", { name: "Compare with now" }).click();
  await expect(page.locator(".restore-note")).toContainText("BioLang Desktop", { timeout: 20_000 });
  await expect(page.locator(".restore-clean")).toHaveCount(0);
});
