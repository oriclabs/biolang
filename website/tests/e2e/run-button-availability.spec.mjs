import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

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
 *
 * The pages are discovered rather than listed. This used to name three
 * Stronghold pages by hand, which covered them well and the other thirty not at
 * all — a whole section could ship with every button dead and nothing would
 * notice. Sections are generated from the packs, so the list has to be too.
 *
 * Note what this deliberately does *not* do: click Run on every block. Whether
 * the answers are right is settled by tests/run_pack_wasm.mjs, which puts all
 * of them through the same WASM module in about twenty seconds. Repeating that
 * through a browser would take roughly forty minutes to re-test the identical
 * code path, and a suite that slow is one people switch off. What only a
 * browser can show is that the button exists, is enabled, and is wired to
 * something that produces output — and that is a property of the page, not of
 * each block on it.
 */

const websiteRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const examplesRoot = path.join(websiteRoot, "docs", "examples");
const packsRoot = path.resolve(websiteRoot, "..", "packs");

/**
 * Problems the manifests declare as needing the network. Their Run buttons are
 * *supposed* to be disabled — the browser cannot reach UniProt or NCBI without
 * CORS — so they are the one legitimate exception, read from the manifests
 * rather than listed here so a new one does not have to be remembered.
 */
function networkProblemIds() {
  const ids = new Set();
  for (const pack of fs.readdirSync(packsRoot, { withFileTypes: true })) {
    if (!pack.isDirectory()) continue;
    const manifest = path.join(packsRoot, pack.name, "pack.toml");
    if (!fs.existsSync(manifest)) continue;
    for (const block of fs.readFileSync(manifest, "utf8").split("[[problem]]").slice(1)) {
      const id = block.match(/id = "([^"]+)"/);
      if (id && /^network = true/m.test(block)) ids.add(id[1].toLowerCase());
    }
  }
  return ids;
}

const NETWORK = networkProblemIds();

/** Every generated pack section page, as a served URL. */
function sectionPages() {
  const pages = [];
  for (const pack of fs.readdirSync(examplesRoot, { withFileTypes: true })) {
    if (!pack.isDirectory()) continue;
    for (const file of fs.readdirSync(path.join(examplesRoot, pack.name))) {
      // `all.html` repeats every section on one page; skipping it keeps the
      // suite from checking the same blocks twice.
      if (!file.endsWith(".html") || file === "all.html") continue;
      pages.push(`/docs/examples/${pack.name}/${file}`);
    }
  }
  return pages.sort();
}

const PAGES = sectionPages();

test("the section pages were found at all", () => {
  // A generation failure would otherwise show up as a suite that passes by
  // testing nothing.
  expect(PAGES.length).toBeGreaterThan(20);
});

for (const url of PAGES) {
  const name = url.split("/").slice(-2).join("/");
  test(`every block on ${name} offers Run`, async ({ page }) => {
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

    const unexpected = disabled.filter((id) => !NETWORK.has(id));
    expect(unexpected, `disabled Run buttons: ${unexpected.join(", ")}`).toEqual([]);
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
