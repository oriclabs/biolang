import { chromium } from "@playwright/test";
import assert from "node:assert/strict";

const browser = await chromium.connectOverCDP("http://127.0.0.1:9333");
const pages = browser.contexts().flatMap((context) => context.pages());
const page = pages.find((candidate) => candidate.url().includes("127.0.0.1:1420")) ?? pages[0];

if (!page) throw new Error("No BioLang Desktop webview was found");
const emptyMode = process.argv.includes("--empty");
await page.waitForSelector(".app-shell", { state: "visible", timeout: 10_000 });

if (emptyMode) {
  await page.waitForSelector(".workspace-welcome", { state: "visible", timeout: 10_000 });
  assert.equal(await page.locator(".workspace-heading").count(), 0);
  assert.equal(await page.getByText("No folder open", { exact: true }).count(), 1);
  console.log(JSON.stringify({
    title: await page.title(),
    workspaceSelected: false,
    welcomeVisible: true,
  }, null, 2));
  await browser.close();
  process.exit(0);
}

const helloFile = page.locator('.tree-row[data-path="examples/hello.bl"]');
await helloFile.waitFor({ state: "visible", timeout: 10_000 });
await helloFile.click();
await page.waitForSelector(".monaco-editor", { state: "visible", timeout: 20_000 });

await page.keyboard.press("Control+`");
await page.waitForSelector(".xterm", { state: "visible", timeout: 10_000 });
await page.waitForFunction(() => document.querySelector(".terminal-wrap")?.getAttribute("data-state") !== "starting");
const terminalState = await page.locator(".terminal-wrap").getAttribute("data-state");
const terminalSession = await page.locator(".terminal-wrap").getAttribute("data-session");
assert.equal(terminalState, "ready", `Terminal state was ${terminalState}`);
assert.match(terminalSession ?? "", /^\d+$/, "Terminal session was not assigned");
await page.evaluate(async ({ sessionId }) => {
  const internals = window.__TAURI_INTERNALS__;
  await internals.invoke("terminal_write", {
    sessionId: Number(sessionId),
    data: "echo BIOLANG_PTY_OK\r",
  });
}, { sessionId: terminalSession });
await page.waitForFunction(() => document.querySelector(".terminal-host")?.getAttribute("data-output-tail")?.includes("BIOLANG_PTY_OK"));
const terminalText = await page.locator(".terminal-host").getAttribute("data-output-tail");
assert.match(terminalText ?? "", /BIOLANG_PTY_OK/, `Terminal output was: ${terminalText}`);

const jobId = await page.evaluate(async () => {
  const internals = window.__TAURI_INTERNALS__;
  return internals.invoke("run_file", { path: "examples/hello.bl" });
});
assert.equal(typeof jobId, "number");
await page.locator(".panel-tabs button", { hasText: "output" }).click();
await page.waitForFunction(() => document.querySelector(".output-view")?.textContent?.includes("Hello from BioLang!"));

const result = {
  title: await page.title(),
  url: page.url(),
  tabs: await page.locator(".editor-tab").count(),
  workspace: await page.locator(".workspace-heading span").textContent(),
  status: await page.locator(".statusbar").innerText(),
  lspReady: await page.locator(".status-health.ready").count() === 1,
  ptyRoundTrip: terminalText?.includes("BIOLANG_PTY_OK"),
  nativeJobId: jobId,
  runOutput: (await page.locator(".output-view").textContent())?.includes("Hello from BioLang!"),
};

console.log(JSON.stringify(result, null, 2));
await browser.close();
