import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

/**
 * Click Run on every runnable block in the docs and check what comes back.
 *
 * Everything else stops short of this. `bl check` parses a block without
 * running it, so anything that only fails at runtime passes. The pack checks
 * run pack examples but not the prose pages. And running a block through the
 * WASM module directly does not exercise the button, the replay of earlier
 * blocks on the page, or the CLI-only classification — all of which are page
 * behaviour, and all of which have been wrong before.
 *
 * So this drives the real page. For each block the playground marked runnable:
 * click it if it offers Run, and fail on `Runtime error:` or the ✖ the
 * playground prints for a returned error. Blocks the page marks CLI Only are
 * written out instead, for check-doc-examples-cli.mjs to run through `bl`.
 *
 * Blocks that are meant to fail — the error-handling pages demonstrate failures
 * — are listed in EXPECTED_TO_FAIL below, keyed by page and heading.
 */

const websiteRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const docsRoot = path.join(websiteRoot, "docs");
const cliOutputPath = path.join(websiteRoot, "tests", "cli-only-blocks.json");

/**
 * Calls that reach the network. The playground runs these in the browser — NCBI
 * allows cross-origin requests — but clicking them makes a real request, which
 * is slow, flaky, and not this test's business. One of them hung a page for the
 * full five-minute timeout. They are collected and compile-checked instead.
 */
const REACHES_THE_NETWORK =
  /(ncbi_\w+|ensembl_\w+|uniprot_\w+|kegg_\w+|pdb_entry|string_network|go_\w+|reactome_\w+|ucsc_\w+|datasets_\w+|cosmic_\w+|fetch|http_get|http_post|llm_\w+|chat)\s*\(/;

/** Pages whose blocks demonstrate a failure on purpose. */
const EXPECTED_TO_FAIL = new Set([
  // "/docs/some-page.html#0",
]);

/** Every docs page, as a site-root URL. */
function docPages() {
  const out = [];
  const walk = (dir) => {
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
      const full = path.join(dir, entry.name);
      if (entry.isDirectory()) walk(full);
      else if (entry.name.endsWith(".html")) {
        out.push("/" + path.relative(websiteRoot, full).split(path.sep).join("/"));
      }
    }
  };
  walk(docsRoot);
  return out.sort();
}

const PAGES = docPages();
const collected = [];
const totals = { pages: 0, clicked: 0 };

// Not serial: a page that fails must not stop the rest of the sweep. One worker
// keeps the collected CLI-only list in a single process (see playwright config
// or run with --workers=1).
test.describe.configure({ mode: "default" });

for (const url of PAGES) {
  test(`doc examples run: ${url}`, async ({ page }) => {
    test.setTimeout(120_000);
    const consoleErrors = [];
    page.on("pageerror", (error) => consoleErrors.push(String(error)));

    await page.goto(url, { waitUntil: "domcontentloaded" });

    // The playground attaches buttons after fetching the builtin catalog. A page
    // with no runnable block never gets one, which is not a failure.
    const anyButton = page.locator(".bl-run-btn").first();
    const hasButtons = await anyButton
      .waitFor({ state: "attached", timeout: 20_000 })
      .then(() => true)
      .catch(() => false);
    if (!hasButtons) {
      expect(consoleErrors, `page errors on ${url}`).toEqual([]);
      return;
    }

    // The button and the output panel are appended to the wrapper the
    // playground puts around the <pre>, not inside it, so everything is located
    // from the button outwards.
    const buttons = page.locator(".bl-run-btn");
    const count = await buttons.count();
    const failures = [];
    let clicked = 0;

    for (let index = 0; index < count; index += 1) {
      const button = buttons.nth(index);
      const wrapper = button.locator("xpath=..");
      const source = ((await wrapper.locator("pre code").first().textContent()) ?? "").trim();
      const label = ((await button.textContent()) ?? "").trim();

      // CLI Only blocks are collected and run through `bl` afterwards; clicking
      // them here would prove nothing, since the button is deliberately inert.
      if (/CLI Only/i.test(label) || (await button.isDisabled())) {
        collected.push({ url, index, source, reason: "cli-only" });
        continue;
      }
      if (REACHES_THE_NETWORK.test(source)) {
        collected.push({ url, index, source, reason: "network" });
        continue;
      }

      // Dispatched rather than clicked through hit-testing. The buttons are
      // absolutely positioned inside their wrappers and can overlap each other,
      // so Playwright reported one as intercepting another's pointer events and
      // waited out the timeout. What is under test is the handler, not where the
      // button lands on screen — and this is far quicker besides.
      await button.evaluate((el) => el.click());
      clicked += 1;

      // The button reads "Running..." until the result lands.
      await expect(button).toHaveText(/Run$/i, { timeout: 30_000 });

      const output = ((await wrapper.locator(".bl-output").first().textContent()) ?? "").trim();
      const key = `${url}#${index}`;
      const failed = output.includes("Runtime error:") || output.includes("✖");

      if (failed && !EXPECTED_TO_FAIL.has(key)) {
        const lines = output.split("\n");
        const reason = lines.find((l) => l.includes("Runtime error:") || l.includes("✖")) ?? "";
        const firstLine = source.split("\n")[0];
        failures.push(`${key} :: ${reason.slice(0, 200)} :: source: ${firstLine.slice(0, 110)}`);
      }
      if (!failed && EXPECTED_TO_FAIL.has(key)) {
        failures.push(`${key} was expected to fail and did not — drop it from EXPECTED_TO_FAIL`);
      }
    }

    totals.pages += 1;
    totals.clicked += clicked;
    console.log(`  ${url}: ${clicked} run, ${count - clicked} CLI-only`);

    expect(failures, `blocks that errored on ${url}:\n  ${failures.join("\n  ")}`).toEqual([]);
    expect(consoleErrors, `page errors on ${url}`).toEqual([]);
  });
}

test.afterAll(() => {
  fs.writeFileSync(cliOutputPath, `${JSON.stringify(collected, null, 2)}\n`);
  console.log(`\n${collected.length} CLI-only blocks written to ${path.relative(websiteRoot, cliOutputPath)}`);
});
