import { expect, test } from "@playwright/test";

test("Settings lists bio API credentials and explains the browser limitation", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("main").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.locator(".activity-bar").getByLabel("Settings").click();
  await page.getByRole("tab", { name: "Credentials" }).click();

  await expect(page.getByText("Bio API credentials")).toBeVisible();
  await expect(page.getByText("Credentials need BioLang Desktop", { exact: false })).toBeVisible();

  // COSMIC refuses every request without a key, so it must be marked required.
  const cosmic = page.locator(".credential-row").filter({ hasText: "COSMIC" });
  await expect(cosmic.locator(".credential-required")).toBeVisible();
  await expect(cosmic.locator(".credential-state.missing")).toBeVisible();
  await expect(cosmic.getByRole("button", { name: "Save" })).toBeDisabled();

  // NCBI is a rate-limit improvement, not a hard requirement.
  const ncbi = page.locator(".credential-row").filter({ hasText: "NCBI" });
  await expect(ncbi.locator(".credential-required")).toHaveCount(0);
});

test("the API browser warns when a selected API needs a key that is not set", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("main").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.getByLabel("Bio APIs").click();
  await page.getByPlaceholder("Search external databases").fill("ncbi_");
  await page.locator(".api-group button").first().click();

  const notice = page.locator(".api-credential");
  await expect(notice).toBeVisible();
  await expect(notice).toContainText("NCBI key recommended");
  await notice.getByRole("button", { name: "Add key" }).click();
  await expect(page.locator(".settings-dialog")).toBeVisible();
  await expect(page.getByRole("tab", { name: "Credentials" })).toHaveAttribute("aria-selected", "true");
  await expect(page.getByText("Bio API credentials")).toBeVisible();
});
