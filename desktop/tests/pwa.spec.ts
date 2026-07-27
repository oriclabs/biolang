import { expect, test } from "@playwright/test";

test("Studio Web runs BioLang WASM and restores its browser workspace", async ({ page }) => {
  await page.goto("/");

  await expect(page.getByRole("heading", { name: "BioLang Studio Web" })).toBeVisible();
  await expect(page.locator('link[rel="manifest"]')).toHaveAttribute("href", "./manifest.webmanifest");
  const manifest = await page.evaluate(async () => {
    const response = await fetch("./manifest.webmanifest");
    return response.json() as Promise<{ name: string; display: string }>;
  });
  expect(manifest).toMatchObject({ name: "BioLang Studio Web", display: "standalone" });

  await page.getByRole("main").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.locator('.tree-row[data-path="analysis.bl"]').click();
  await expect(page.locator(".monaco-editor")).toBeVisible({ timeout: 20_000 });
  await page.getByLabel("Run active BioLang file").click();
  await expect(page.locator(".output-view")).toContainText("GC content:", { timeout: 20_000 });
  await expect(page.locator(".output-view")).toContainText("Reverse complement:");
  await expect(page.locator(".output-view")).toContainText("Process completed");

  await page.reload();
  await expect(page.locator('.tree-row[data-path="analysis.bl"]')).toBeVisible();
  await expect(page.locator(".editor-tab.active")).toContainText("analysis.bl");

  await page.getByLabel("Packages").click();
  await expect(page.getByRole("button", { name: "Install dependencies" }).last()).toBeDisabled();
  await expect(page.getByText("Install packages with Desktop or on the selected SOMER runtime.")).toBeVisible();

  await page.getByRole("button", { name: "Run", exact: true }).click();
  await expect(page.getByRole("menuitem", { name: "New Terminal Ctrl+`", exact: true })).toBeDisabled();
  await expect(page.getByRole("menuitem", { name: "BioLang Console Ctrl+Shift+`", exact: true })).toBeEnabled();
});
