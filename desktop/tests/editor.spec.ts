import { expect, test } from "@playwright/test";

/** Monaco renders leading indentation as non-breaking spaces. */
async function editorText(page: import("@playwright/test").Page): Promise<string> {
  const lines = await page.locator(".editor-host .view-lines .view-line").allInnerTexts();
  return lines.join("\n").replace(/ /g, " ");
}

async function openScratchFile(page: import("@playwright/test").Page) {
  await page.goto("/");
  await page.getByRole("main").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.locator('.tree-row[data-path="analysis.bl"]').click();
  const editor = page.locator(".monaco-editor").first();
  await expect(editor).toBeVisible({ timeout: 20_000 });
  await editor.click();
  await page.keyboard.press("Control+A");
  return editor;
}

test("accepting a function completion fills in its arguments", async ({ page }) => {
  await openScratchFile(page);
  await page.keyboard.insertText("reverse_comp");
  await page.keyboard.press("Control+Space");
  const suggestions = page.locator(".suggest-widget.visible");
  await expect(suggestions).toBeVisible();
  await expect(suggestions).toContainText("reverse_complement");
  await page.keyboard.press("Enter");

  // The old provider inserted the bare name and left the caller to type the
  // parentheses; a snippet completion lands with the argument selected.
  expect(await editorText(page)).toMatch(/^reverse_complement\(\w+\)$/);
});

test("hovering a sequence literal reports GC, reverse complement, and translation", async ({ page }) => {
  await openScratchFile(page);
  await page.keyboard.insertText('let cds = dna"ATGGCGTAA"');

  const literal = page.locator(".editor-host .view-line").getByText("ATGGCGTAA");
  await expect(literal).toBeVisible();
  // Monaco opens the hover from a mousemove, so the pointer has to arrive from
  // somewhere else; a single hover() at a position it already occupies is a
  // no-op.
  const box = (await literal.boundingBox())!;
  await page.mouse.move(box.x - 60, box.y + box.height / 2);
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2, { steps: 8 });

  // The glyph-margin hover widget is always in the DOM but carries `hidden`.
  const hover = page.locator(".monaco-hover:not(.hidden)");
  await expect(hover).toContainText("DNA literal — 9 bases", { timeout: 10_000 });
  // Cross-checked against bl: gc_content 0.4444, reverse_complement TTACGCCAT.
  await expect(hover).toContainText("GC content: 44.4%");
  await expect(hover).toContainText("TTACGCCAT");
  await expect(hover).toContainText("MA*");
});

test("block scaffolds are offered as snippets", async ({ page }) => {
  await openScratchFile(page);
  await page.keyboard.insertText("pipel");
  await page.keyboard.press("Control+Space");
  const suggestions = page.locator(".suggest-widget.visible");
  await expect(suggestions).toBeVisible();
  await expect(suggestions).toContainText("pipeline");
  await page.keyboard.press("Enter");

  const text = await editorText(page);
  expect(text).toContain("pipeline");
  expect(text).toContain('stage "');
  expect(text).toContain("->");
});
