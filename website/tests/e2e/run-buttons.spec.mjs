/**
 * Click every Run button on every documentation page and assert the example
 * executes without an unexpected error.
 *
 * One test per page, so a page can be run on its own:
 *
 *   npx playwright test                              # whole site
 *   npx playwright test -g "docs/bio/kmers.html"     # one page
 *   BL_DIR=docs/bio npx playwright test              # one section
 *   BL_MAX_BLOCKS=5 npx playwright test              # fast smoke run
 *
 * Pages are discovered from the filesystem by which runner they load, so no
 * manifest needs maintaining. Pages that load neither runner have no Run
 * buttons and are skipped.
 *
 * Outcomes per block:
 *   ok            — ran, no error
 *   native-only   — "Not available in the browser: f" (correct, by design)
 *   expected      — listed in expected-errors.json, or the source carries a
 *                   marker comment like `# expect: error`
 *   FAIL          — anything else that produced an error
 */
import { test, expect } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const SITE = path.resolve(HERE, "../..");
const CFG = JSON.parse(fs.readFileSync(path.join(HERE, "expected-errors.json"), "utf8"));
const ALLOWED = CFG.alwaysAllowed.map((r) => new RegExp(r));
const MARKERS = CFG.codeMarkers;
const MAX_BLOCKS = Number(process.env.BL_MAX_BLOCKS || 0) || Infinity;

/** Which runner a page loads — determines whether it has Run buttons at all. */
function runnerOf(html) {
  if (/book-runner/.test(html)) return "book";
  if (/playground\.js/.test(html)) return "playground";
  return null;
}

function discover(dir, out = []) {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) {
      if (["node_modules", "wasm", "tests", "assets", "extension"].includes(e.name)) continue;
      discover(p, out);
    } else if (e.name.endsWith(".html") && e.name !== "print.html" && e.name !== "404.html") {
      const html = fs.readFileSync(p, "utf8");
      const runner = runnerOf(html);
      if (runner) {
        out.push({
          url: "/" + path.relative(SITE, p).split(path.sep).join("/"),
          rel: path.relative(SITE, p).split(path.sep).join("/"),
          runner,
        });
      }
    }
  }
  return out;
}

const only = process.env.BL_DIR ? path.join(SITE, process.env.BL_DIR) : null;
const roots = only ? [only] : [path.join(SITE, "docs"), path.join(SITE, "books")];
const PAGES = roots.filter(fs.existsSync).flatMap((r) => discover(r));

if (PAGES.length === 0) {
  test("pages were discovered", () => {
    throw new Error(`no runnable pages found under ${roots.join(", ")}`);
  });
}

/** Is this error output acceptable rather than a bug? */
function isAcceptable(output, code, pageRules, index) {
  if (ALLOWED.some((r) => r.test(output.trim()))) return "native-only";
  if (MARKERS.some((m) => code.toLowerCase().includes(m.toLowerCase()))) return "expected";
  if (pageRules && pageRules[String(index)]) return "expected";
  return null;
}

for (const pg of PAGES) {
  test(`${pg.rel} — Run buttons execute`, async ({ page }) => {
    const consoleErrors = [];
    page.on("pageerror", (e) => consoleErrors.push(String(e.message).split("\n")[0]));

    await page.goto(pg.url, { waitUntil: "domcontentloaded" });

    // Runners add buttons on DOMContentLoaded; a page may legitimately have none.
    const buttons = page.locator(".bl-run-btn");
    await page
      .waitForFunction(() => document.querySelectorAll(".bl-run-btn").length > 0, null, {
        timeout: 15_000,
      })
      .catch(() => {});
    const total = await buttons.count();
    test.skip(total === 0, "page loads a runner but exposes no Run buttons");

    // Disabled buttons are the runner's own "CLI Only" markers — never clicked.
    const enabled = [];
    for (let i = 0; i < total; i++) {
      if (!(await buttons.nth(i).isDisabled())) enabled.push(i);
    }
    test.skip(enabled.length === 0, "every block on this page is marked CLI-only");

    const pageRules = CFG.pages[pg.rel] || CFG.pages["/" + pg.rel] || null;
    const results = [];
    const failures = [];
    const budget = Math.min(enabled.length, MAX_BLOCKS);

    for (let n = 0; n < budget; n++) {
      const idx = enabled[n];
      const btn = buttons.nth(idx);
      await btn.scrollIntoViewIfNeeded();

      // The block's own source, for marker detection.
      const code = await page.evaluate((i) => {
        const b = document.querySelectorAll(".bl-run-btn")[i];
        if (!b) return "";
        // playground: button sits in a wrapper around <pre>; book: bar above <pre>
        const wrap = b.closest("div") || b.parentElement;
        const pre =
          (wrap && wrap.querySelector("pre")) ||
          (b.parentElement && b.parentElement.querySelector("pre"));
        const c = pre && pre.querySelector("code");
        return c ? c.textContent : "";
      }, idx);

      await btn.click();

      // Done when the runner re-enables the button. The first click on a page
      // also pays for the ~6 MB WASM download.
      const wait = n === 0 ? 150_000 : 90_000;
      try {
        await expect(btn).toBeEnabled({ timeout: wait });
      } catch {
        failures.push({ n: n + 1, why: "did not finish", head: code.split("\n")[0].slice(0, 70) });
        results.push("timeout");
        continue;
      }

      // Read the rendered output. book-runner writes straight into .bl-output;
      // playground nests a .bl-result inside it.
      const output = await page.evaluate((i) => {
        const b = document.querySelectorAll(".bl-run-btn")[i];
        const wrap = b && (b.closest("div") || b.parentElement);
        let out = wrap && wrap.querySelector(".bl-output");
        if (!out) {
          // book-runner puts the output as a sibling after the <pre>
          const pre = wrap && wrap.querySelector("pre");
          out = pre && pre.nextElementSibling &&
            pre.nextElementSibling.classList.contains("bl-output")
            ? pre.nextElementSibling
            : null;
        }
        if (!out) return "";
        const res = out.querySelector(".bl-result");
        return (res || out).textContent || "";
      }, idx);

      const errored =
        output.includes("✖") ||
        /^Runtime error:/m.test(output) ||
        /Failed to load WASM/.test(output);

      if (!errored) {
        results.push("ok");
        continue;
      }
      const verdict = isAcceptable(output, code, pageRules, n + 1);
      if (verdict) {
        results.push(verdict);
        continue;
      }
      results.push("FAIL");
      failures.push({
        n: n + 1,
        why: output.replace(/\s+/g, " ").trim().slice(0, 160),
        head: code.split("\n").find((l) => l.trim()) ?.slice(0, 70) || "",
      });
    }

    const tally = results.reduce((a, r) => ((a[r] = (a[r] || 0) + 1), a), {});
    const summary = Object.entries(tally).map(([k, v]) => `${k}:${v}`).join("  ");
    test.info().annotations.push({ type: "blocks", description: summary });

    // A hard JS error breaks the page even if individual blocks looked fine.
    expect(consoleErrors, `uncaught page errors on ${pg.rel}`).toEqual([]);

    expect(
      failures,
      `${failures.length} of ${budget} Run buttons failed unexpectedly on ${pg.rel}\n` +
        failures.map((f) => `  #${f.n}  ${f.why}\n        ${f.head}`).join("\n") +
        `\n\n  If an example is meant to fail, add a marker comment to it ` +
        `(e.g. "# expect: error") or list it in tests/e2e/expected-errors.json.`
    ).toEqual([]);
  });
}
