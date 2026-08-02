import { expect, test } from "@playwright/test";
import {
  chordFromEvent,
  chordsEqual,
  commandForEvent,
  findKeybindingConflicts,
  formatChord,
  matchEvent,
  resolveBindings,
  type KeyChord,
} from "../src/keybindings";

function fakeEvent(partial: Partial<KeyboardEvent> & { key: string }): KeyboardEvent {
  const target = {
    closest: () => null,
  };
  return {
    key: partial.key,
    code: partial.code ?? "",
    ctrlKey: partial.ctrlKey ?? false,
    metaKey: partial.metaKey ?? false,
    shiftKey: partial.shiftKey ?? false,
    altKey: partial.altKey ?? false,
    target: partial.target ?? target,
  } as unknown as KeyboardEvent;
}

test("default bindings resolve without conflicts", () => {
  const bindings = resolveBindings({});
  expect(findKeybindingConflicts(bindings)).toEqual([]);
  expect(formatChord(bindings.save)).toMatch(/Ctrl\+S|⌘S/);
});

test("matchEvent treats Ctrl and Meta as the same modifier", () => {
  const chord: KeyChord = { key: "s", ctrl: true };
  expect(matchEvent(fakeEvent({ key: "s", ctrlKey: true }), chord)).toBe(true);
  expect(matchEvent(fakeEvent({ key: "s", metaKey: true }), chord)).toBe(true);
  expect(matchEvent(fakeEvent({ key: "s", shiftKey: true, ctrlKey: true }), chord)).toBe(false);
});

test("commandForEvent picks the configured chord after override", () => {
  const bindings = resolveBindings({
    run: { key: "r", ctrl: true },
  });
  const event = fakeEvent({ key: "r", ctrlKey: true });
  expect(commandForEvent(event, bindings)?.id).toBe("run");
  expect(commandForEvent(fakeEvent({ key: "Enter", ctrlKey: true }), bindings)?.id).not.toBe("run");
});

test("chordsEqual normalizes casing and optional flags", () => {
  expect(chordsEqual({ key: "P", ctrl: true }, { key: "p", ctrl: true, shift: false })).toBe(true);
  expect(chordsEqual(chordFromEvent(fakeEvent({ key: "\\", code: "Backslash", ctrlKey: true })), { key: "backslash", ctrl: true })).toBe(true);
});

test("Settings Keyboard tab records and resets a shortcut", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("main").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.locator(".activity-bar").getByLabel("Settings").click();
  await page.getByRole("tab", { name: "Keyboard" }).click();

  await expect(page.getByText("Keyboard shortcuts")).toBeVisible();
  const runRow = page.locator(".keybinding-row").filter({ hasText: "Run Active File" });
  await runRow.getByRole("button").filter({ hasText: /Ctrl\+Enter|⌘Enter/ }).click();
  await page.keyboard.press("Control+R");
  await expect(runRow.getByRole("button").first()).toContainText(/Ctrl\+R|⌘R/);

  await runRow.getByRole("button", { name: "Reset" }).click();
  await expect(runRow.getByRole("button").first()).toContainText(/Ctrl\+Enter|⌘Enter/);
});
