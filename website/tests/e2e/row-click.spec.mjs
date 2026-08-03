import { expect, test } from "@playwright/test";

/**
 * The coverage table's rows are meant to be clickable as a whole, not only on
 * the problem id. That relies on a stretched pseudo-element over a `<tr>`,
 * which is exactly the sort of CSS that can silently not apply — so this clicks
 * a cell far from the link text and checks navigation actually happens.
 */

test("clicking anywhere in a coverage row opens that problem", async ({ page }) => {
  await page.goto("/docs/examples/rosalind-armory.html");

  const row = page.locator("table tbody tr").first();
  await expect(row).toBeVisible();

  // The last cell holds the "Checked" text and contains no anchor of its own.
  const lastCell = row.locator("td").last();
  await expect(lastCell.locator("a")).toHaveCount(0);

  // Clicked by coordinate rather than by element, so this exercises the
  // delegated handler the way a reader would. The table sits below the fold,
  // so it has to be scrolled into view first — mouse coordinates are viewport
  // relative, and clicking an off-screen point hits nothing at all.
  await lastCell.scrollIntoViewIfNeeded();
  const box = await lastCell.boundingBox();
  await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);

  await expect(page).toHaveURL(/rosalind-armory\/[a-z]+\.html#[a-z]+$/);
});

test("the row link is still a real link", async ({ page }) => {
  await page.goto("/docs/examples/rosalind-armory.html");

  // Keyboard and middle-click users depend on this being an anchor with an
  // href rather than a JavaScript click handler.
  const anchor = page.locator("table tbody tr").first().locator("a").first();
  await expect(anchor).toHaveAttribute("href", /rosalind-armory\/[a-z]+\.html#[a-z]+/);
});

test("hovering a row changes its background", async ({ page }) => {
  await page.goto("/docs/examples/rosalind-armory.html");

  const row = page.locator("table tbody tr").first();
  const before = await row.evaluate((el) => getComputedStyle(el).backgroundColor);
  await row.hover();
  const after = await row.evaluate((el) => getComputedStyle(el).backgroundColor);

  expect(after).not.toBe(before);
});
