import { expect, test } from "@playwright/test";
import { readFileSync } from "node:fs";

test("cold load ships a startup screen before JavaScript is ready", async ({ request }) => {
  const response = await request.get("/");
  const html = await response.text();

  expect(response.ok()).toBeTruthy();
  expect(html).toContain('id="boot-splash"');
  expect(html).toContain("Loading application...");
  expect(html).toContain("background: #101216");
});

test("desktop dev startup serves the splash without a blocking predev task", () => {
  const packageJson = JSON.parse(readFileSync("package.json", "utf8")) as {
    scripts: Record<string, string>;
  };
  const tauriConfig = JSON.parse(readFileSync("src-tauri/tauri.conf.json", "utf8")) as {
    app: { windows: Array<{ backgroundColor?: string }> };
  };

  expect(packageJson.scripts.predev).toBeUndefined();
  expect(tauriConfig.app.windows[0]?.backgroundColor).toBe("#101216");
});

test("Studio Web runs BioLang WASM and restores its browser workspace", async ({ page }) => {
  await page.goto("/");

  await expect(page.getByRole("heading", { name: "BioLang Studio Web" })).toBeVisible();
  await expect(page.locator('link[rel="manifest"]')).toHaveAttribute("href", "./manifest.webmanifest");
  const manifest = await page.evaluate(async () => {
    const response = await fetch("./manifest.webmanifest");
    return response.json() as Promise<{ name: string; display: string }>;
  });
  expect(manifest).toMatchObject({ name: "BioLang Studio Web", display: "standalone" });

  await page.getByRole("main").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.locator('.tree-row[data-path="analysis.bl"]').click();
  await expect(page.locator(".monaco-editor")).toBeVisible({ timeout: 20_000 });
  await page.getByLabel("Run active BioLang file").click();
  await expect(page.locator(".output-view")).toContainText("GC content:", { timeout: 20_000 });
  await expect(page.locator(".output-view")).toContainText("Reverse complement:");
  await expect(page.locator(".output-view")).toContainText("Process completed");

  await page.reload();
  await expect(page.locator('.tree-row[data-path="analysis.bl"]')).toBeVisible();
  await expect(page.locator(".editor-tab.active")).toContainText("analysis.bl");

  await page.getByLabel("Packages").click();
  await expect(page.getByRole("button", { name: "Install dependencies" }).last()).toBeDisabled();
  await expect(page.getByText("Install packages with Desktop or on the selected SOMER runtime.")).toBeVisible();

  await page.getByRole("button", { name: "Run", exact: true }).click();
  await expect(page.getByRole("menuitem", { name: "New Terminal Ctrl+`", exact: true })).toBeDisabled();
  await expect(page.getByRole("menuitem", { name: "BioLang Console Ctrl+Shift+`", exact: true })).toBeEnabled();
});

test("Studio Web suggests package functions and inferred record fields", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("main").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.locator('.tree-row[data-path="analysis.bl"]').click();
  const editor = page.locator(".monaco-editor").first();
  await expect(editor).toBeVisible({ timeout: 20_000 });
  await editor.click();
  await page.keyboard.press("Control+A");
  await page.keyboard.insertText('import "singlecell" as sc\nsc');
  await page.keyboard.type(".");
  await page.keyboard.press("Control+Space");
  const suggestions = page.locator(".suggest-widget.visible");
  await expect(suggestions).toBeVisible();
  await page.keyboard.type("sum");
  await expect(suggestions).toContainText("summary");

  await page.keyboard.press("Escape");
  await page.keyboard.press("Control+A");
  await page.keyboard.insertText(
    'import "singlecell" as sc\nlet stats = sc.summary(cells)\nstats',
  );
  await page.keyboard.type(".");
  await page.keyboard.press("Control+Space");
  await expect(suggestions).toBeVisible();
  await expect(suggestions).toContainText("n_cells");
  await expect(suggestions).toContainText("has_clusters");
});

test("Studio Web renders SVG results and docks the panel", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("main").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.locator('.tree-row[data-path="analysis.bl"]').click();
  const editor = page.locator(".monaco-editor").first();
  await expect(editor).toBeVisible({ timeout: 20_000 });
  await editor.click();
  await page.keyboard.press("Control+A");
  await page.keyboard.insertText('phylo_tree("(TP53:1,BRCA1:1);")');
  await page.getByLabel("Run active BioLang file").click();
  await expect(page.getByRole("img", { name: "BioLang plot output" })).toBeVisible({ timeout: 20_000 });

  await page.getByLabel("More Output actions").click();
  await page.getByRole("button", { name: "Dock at right", exact: true }).click();
  await expect(page.locator(".workbench")).toHaveClass(/output-dock-right/);
  await page.getByLabel("More Output actions").click();
  await expect(page.getByRole("button", { name: "Dock at bottom", exact: true })).toBeVisible();
});

test("welcome examples open runnable analyses and browser runs expose Tables", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("region", { name: "BioLang examples" })).toBeVisible();
  await page.getByRole("button", { name: /K-mer Table/ }).click();
  await expect(page.locator(".editor-tab.active")).toContainText("kmer_table.bl");
  await page.getByLabel("Run active BioLang file").click();
  await expect(page.getByRole("button", { name: "tables", exact: true })).toBeVisible({ timeout: 20_000 });
  await page.getByRole("button", { name: "tables", exact: true }).click();
  await expect(page.locator(".output-tables-view table")).toBeVisible();
  await expect(page.locator(".output-tables-view")).toContainText("kmer");
  await expect(page.locator(".output-tables-view")).toContainText("count");
});

test("BioLang Light themes the workbench and editor together", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("main").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.locator(".activity-bar").getByLabel("Settings").click();
  await page.getByLabel("Editor theme").selectOption("biolang-light");
  await expect(page.locator(".app-shell")).toHaveClass(/light-theme/);
  // Assert the chrome actually resolves to the theme's surface token rather
  // than a specific hex, so retuning the palette does not fail the test that
  // is meant to be guarding the wiring.
  const titlebar = await page.locator(".titlebar").evaluate((element) => {
    const shell = element.closest(".app-shell") as HTMLElement;
    const token = getComputedStyle(shell).getPropertyValue("--bg-raised").trim();
    const probe = document.createElement("div");
    probe.style.color = token;
    document.body.append(probe);
    const resolved = getComputedStyle(probe).color;
    probe.remove();
    const rgb = (value: string) => (value.match(/[\d.]+/g) || []).slice(0, 3).map(Number);
    const [r, g, b] = rgb(getComputedStyle(element).backgroundColor);
    return { matchesToken: rgb(resolved).join() === [r, g, b].join(), luminance: 0.2126 * r + 0.7152 * g + 0.0722 * b };
  });
  expect(titlebar.matchesToken).toBe(true);
  expect(titlebar.luminance).toBeGreaterThan(200);
  await page.getByRole("button", { name: "Close", exact: true }).click();
  await page.locator('.tree-row[data-path="analysis.bl"]').click();
  await expect(page.locator(".monaco-editor")).toHaveClass(/vs/);
});

test("every workbench surface is themed by token, not by a per-component override", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("main").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.locator(".activity-bar").getByLabel("Settings").click();
  await page.getByLabel("Editor theme").selectOption("biolang-light");
  await page.getByRole("button", { name: "Close", exact: true }).click();
  await page.getByLabel("Help").click();
  await expect(page.locator(".help-markdown")).toBeVisible({ timeout: 20_000 });

  // The previous light theme was a hand-maintained list of component
  // selectors, so anything omitted kept a dark literal and rendered as light
  // text on a light surface. Nothing on screen may fall below AA.
  const failures = await page.evaluate(() => {
    const channel = (v: number) => {
      const s = v / 255;
      return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4);
    };
    const lum = (c: number[]) => 0.2126 * channel(c[0]) + 0.7152 * channel(c[1]) + 0.0722 * channel(c[2]);
    const parse = (s: string) => (s.match(/[\d.]+/g) || []).slice(0, 3).map(Number);
    const backdrop = (el: Element): number[] => {
      let node: Element | null = el;
      while (node) {
        const bg = getComputedStyle(node).backgroundColor;
        if (bg && bg !== "rgba(0, 0, 0, 0)" && (bg.match(/[\d.]+/g) || [])[3] !== "0") return parse(bg);
        node = node.parentElement;
      }
      return [255, 255, 255];
    };
    const bad: string[] = [];
    for (const el of Array.from(document.querySelectorAll("*"))) {
      const text = (el.textContent || "").trim();
      if (!text || el.children.length > 0) continue;
      const box = el.getBoundingClientRect();
      if (box.width < 4 || box.height < 4) continue;
      const style = getComputedStyle(el);
      if (style.visibility === "hidden" || Number(style.opacity) < 0.5) continue;
      const a = lum(parse(style.color));
      const b = lum(backdrop(el));
      const ratio = (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
      if (ratio < 4.5) bad.push(`${ratio.toFixed(2)}:1 ${style.color} on rgb(${backdrop(el).join(", ")}) "${text.slice(0, 30)}"`);
    }
    return Array.from(new Set(bad));
  });
  expect(failures, `light theme contrast failures:\n${failures.join("\n")}`).toEqual([]);
});
