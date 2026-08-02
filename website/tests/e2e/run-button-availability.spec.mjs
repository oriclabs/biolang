import { expect, test } from "@playwright/test";

/**
 * Run buttons must be enabled for every pack problem that runs in the browser.
 *
 * The playground decides this by scanning a block for calls to builtins the
 * WASM build lacks. It used to scan comments and string literals too, so prose
 * like `renders as "DNA(ACGT)"` or `appears at least twice (counting ...)` read
 * as calls to missing functions, and the block was disabled with a tooltip
 * claiming it needed file I/O or the network. Nothing about it was true, and it
 * silently took away the ability to run correct examples.
 */

const PAGES = [
  "/docs/examples/rosalind-stronghold/sequence.html",
  "/docs/examples/rosalind-stronghold/combinatorics.html",
  "/docs/examples/rosalind-stronghold/alignment.html",
];

for (const url of PAGES) {
  test(`every block on ${url.split("/").pop()} offers Run`, async ({ page }) => {
    await page.goto(url);
    // Buttons are attached after the builtin catalog is fetched.
    await expect(page.locator(".bl-run-btn").first()).toBeVisible({ timeout: 30_000 });

    const disabled = await page.evaluate(() =>
      [...document.querySelectorAll("section[id]")]
        .filter((s) => {
          const button = s.querySelector(".bl-run-btn");
          return button && button.disabled;
        })
        .map((s) => s.id),
    );

    expect(disabled, `disabled Run buttons: ${disabled.join(", ")}`).toEqual([]);
  });
}

test("a block whose comment mentions a call still runs", async ({ page }) => {
  await page.goto("/docs/examples/rosalind-stronghold/sequence.html#rna");

  const section = page.locator("section#rna");
  const button = section.locator(".bl-run-btn").first();
  await button.scrollIntoViewIfNeeded();
  await expect(button).toBeEnabled();

  await button.click();
  // The transcription, not the "DNA(" that appears in the comment above it.
  await expect(section).toContainText("GAUGGAACUUGACUACGUAAAUU", { timeout: 120_000 });
});
