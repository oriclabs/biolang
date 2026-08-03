import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

async function openScratchFile(page: Page) {
  await page.goto("/");
  await page.getByRole("main").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.locator('.tree-row[data-path="analysis.bl"]').click();
  const editor = page.locator(".editor-host .monaco-editor").first();
  await expect(editor).toBeVisible({ timeout: 20_000 });
  await editor.click();
  await page.keyboard.press("Control+A");
  return editor;
}

/** Put the caret inside a word and let Monaco settle before acting on it. */
async function clickWord(page: Page, word: string) {
  await page.waitForTimeout(500);
  const token = page.locator(".editor-host .view-line").getByText(word, { exact: true }).first();
  await expect(token).toBeVisible();
  const box = (await token.boundingBox())!;
  await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
  await page.waitForTimeout(300);
}

/** Monaco renders leading indentation as non-breaking spaces. */
async function editorText(page: Page): Promise<string> {
  const lines = await page.locator(".editor-host .view-lines .view-line").allInnerTexts();
  return lines.join("\n").replace(/ /g, " ");
}

test("Format Document applies the canonical BioLang layout", async ({ page }) => {
  await openScratchFile(page);
  // Auto-closing brackets supply the closing brace, so the fixture must not.
  await page.keyboard.insertText("fn go(x) {\nlet y = x+1\nprintln(y ,x)");
  await page.waitForTimeout(400);
  await page.keyboard.press("Shift+Alt+F");

  // Indentation follows the editor's own tab size, and the formatter always
  // leaves a final newline — hence the trailing empty view line. Operator
  // spacing is deliberately untouched, so `x+1` stays as written.
  await expect
    .poll(() => editorText(page), { timeout: 20_000 })
    .toBe("fn go(x) {\n  let y = x+1\n  println(y, x)\n}\n");
});

test("Find All References lists uses and skips strings and comments", async ({ page }) => {
  await openScratchFile(page);
  await page.keyboard.insertText(
    'let count = 1\nprintln(count)\n# count here does not count\nlet label = "count"\n',
  );

  await clickWord(page, "count");
  await page.keyboard.press("Shift+F12");

  const panel = page.locator(".reference-results");
  await expect(panel).toBeVisible({ timeout: 20_000 });
  await expect(panel.locator("header")).toContainText("2 references to");
  // Two real uses; the comment and the string literal must not be counted.
  await expect(panel.locator(".search-result")).toHaveCount(2);
});

test("Rename Symbol rewrites every use but leaves text alone", async ({ page }) => {
  await openScratchFile(page);
  await page.keyboard.insertText('let count = 1\nprintln(count)\nlet label = "count"\n');

  await clickWord(page, "count");
  await page.keyboard.press("F2");
  const renameInput = page.locator("input.rename-input");
  await expect(renameInput).toBeVisible({ timeout: 20_000 });
  // Visible is not enough: until it takes focus, the keystrokes below land in
  // the editor and replace the document instead of naming the symbol.
  await expect(renameInput).toBeFocused();
  await page.keyboard.press("Control+A");
  await page.keyboard.type("total");
  await page.keyboard.press("Enter");

  await expect
    .poll(() => editorText(page), { timeout: 20_000 })
    .toBe('let total = 1\nprintln(total)\nlet label = "count"\n');
});

test("Quick Fix offers a spelling correction for an unknown name", async ({ page }) => {
  await openScratchFile(page);
  await page.keyboard.insertText('let seq = dna"ACGT"\nlet r = reverse_complemnt(seq)\n');

  await clickWord(page, "reverse_complemnt");
  await page.keyboard.press("Control+.");

  const actions = page.locator(".action-widget, .monaco-action-bar .actionList, .context-view.monaco-menu-container");
  await expect(actions.first()).toBeVisible({ timeout: 20_000 });
  await expect(actions.first()).toContainText("reverse_complement");
});

test("pipeline stages carry run lenses that evaluate in the console", async ({ page }) => {
  await openScratchFile(page);
  await page.keyboard.insertText(
    'pipeline qc(path) {\nstage "load" -> [1, 2, 3]\nstage "count" -> len([1, 2, 3])',
  );
  await page.waitForTimeout(600);

  const lenses = page.locator(".codelens-decoration a");
  await expect(lenses.filter({ hasText: "Run all 2 stages" })).toBeVisible({ timeout: 20_000 });
  await expect(lenses.filter({ hasText: "Run stage" })).toHaveCount(2);
  // Only stages after the first offer a cumulative run.
  await expect(lenses.filter({ hasText: "Run to here" })).toHaveCount(1);

  await lenses.filter({ hasText: "Run stage" }).nth(1).click();
  const consolePane = page.locator(".console-pane");
  await expect(consolePane).toBeVisible({ timeout: 20_000 });
  await expect(consolePane.locator(".console-prompt")).toContainText("len([1, 2, 3])");
  await expect(consolePane.locator(".console-transcript")).toContainText("3", { timeout: 20_000 });
});

test("printed values are annotated beside the line that produced them", async ({ page }) => {
  await openScratchFile(page);
  await page.keyboard.insertText('let gc = 42\nprintln("first")\nprintln(gc)\n');
  await page.getByLabel("Run active BioLang file").click();

  const annotations = page.locator(".editor-host .inline-run-result");
  await expect(annotations.first()).toBeVisible({ timeout: 20_000 });
  const texts = await annotations.allInnerTexts();
  expect(texts.join("|")).toContain("first");
  expect(texts.join("|")).toContain("42");
});
