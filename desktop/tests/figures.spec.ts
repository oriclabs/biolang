import { expect, test } from "@playwright/test";

async function replaceActiveSource(page: import("@playwright/test").Page, source: string) {
  const editor = page.locator(".editor-host .monaco-editor").first();
  await expect(editor).toBeVisible({ timeout: 20_000 });
  await editor.click();
  await page.keyboard.press("Control+A");
  await page.keyboard.press("Backspace");
  await page.keyboard.insertText(source);
  // Monaco auto-closes the first `([` seen by a bulk insertion and leaves the
  // generated `])` after the inserted text. The caret remains before it.
  for (let index = 0; index < 4; index += 1) await page.keyboard.press("Delete");
}

async function renderedInkFraction(page: import("@playwright/test").Page) {
  const image = page.getByRole("img", { name: "BioLang plot output" });
  await expect(image).toBeVisible({ timeout: 20_000 });
  return image.evaluate(async (node) => {
    const plotNode = node instanceof HTMLImageElement || node instanceof SVGSVGElement
      ? node
      : node.querySelector("img, svg");
    if (!plotNode) throw new Error(`Plot wrapper ${node.tagName} contains no image or SVG`);
    let rendered: HTMLImageElement;
    let naturalWidth: number;
    let naturalHeight: number;
    if (plotNode instanceof HTMLImageElement) {
      await plotNode.decode();
      rendered = plotNode;
      naturalWidth = plotNode.naturalWidth;
      naturalHeight = plotNode.naturalHeight;
    } else if (plotNode instanceof SVGSVGElement) {
      const viewBox = plotNode.viewBox.baseVal;
      naturalWidth = viewBox.width || Number(plotNode.getAttribute("width"));
      naturalHeight = viewBox.height || Number(plotNode.getAttribute("height"));
      const blob = new Blob([new XMLSerializer().serializeToString(plotNode)], { type: "image/svg+xml" });
      const url = URL.createObjectURL(blob);
      rendered = new Image();
      rendered.src = url;
      await rendered.decode();
      URL.revokeObjectURL(url);
    } else {
      throw new Error(`Unexpected plot element ${plotNode.tagName}`);
    }
    const width = Math.min(500, naturalWidth);
    const height = Math.min(500, naturalHeight);
    const canvas = document.createElement("canvas");
    canvas.width = width;
    canvas.height = height;
    const context = canvas.getContext("2d", { willReadFrequently: true });
    if (!context) throw new Error("Canvas 2D context is unavailable");
    context.fillStyle = "#ffffff";
    context.fillRect(0, 0, width, height);
    context.drawImage(rendered, 0, 0, width, height);
    const pixels = context.getImageData(0, 0, width, height).data;
    let ink = 0;
    for (let index = 0; index < pixels.length; index += 4) {
      if (pixels[index] < 245 || pixels[index + 1] < 245 || pixels[index + 2] < 245) ink += 1;
    }
    return { width: naturalWidth, height: naturalHeight, fraction: ink / (pixels.length / 4) };
  });
}

test("figures export at print resolution as well as screen resolution", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("main").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.locator('.tree-row[data-path="analysis.bl"]').click();
  const editor = page.locator(".editor-host .monaco-editor").first();
  await expect(editor).toBeVisible({ timeout: 20_000 });
  await editor.click();
  await page.keyboard.press("Control+A");
  await page.keyboard.insertText('phylo_tree("(TP53:1,BRCA1:1);")');
  await page.getByLabel("Run active BioLang file").click();
  await expect(page.getByRole("img", { name: "BioLang plot output" })).toBeVisible({ timeout: 20_000 });

  // 72 DPI was the only option before; journals ask for 300.
  await expect(page.getByRole("button", { name: "PNG", exact: true })).toBeVisible();
  const print = page.getByRole("button", { name: "PNG 4x" });
  await expect(print).toBeVisible();
  await expect(print).toHaveAttribute("title", /300 DPI/);
});

test("the run bundle is named for what it is", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("main").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.locator('.tree-row[data-path="analysis.bl"]').click();
  await expect(page.locator(".monaco-editor")).toBeVisible({ timeout: 20_000 });
  await page.getByLabel("Run active BioLang file").click();
  await expect(page.locator(".output-view")).toContainText("Process completed", { timeout: 20_000 });

  await page.getByLabel("More Output actions").click();
  await expect(page.getByRole("button", { name: "Export reproducibility bundle", exact: true })).toBeVisible();
});

test("WASM renders circular tracks and composed publication panels with real pixels", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("main").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.locator('.tree-row[data-path="analysis.bl"]').click();

  await replaceActiveSource(page, `
let segments = table([
  {chrom: "chr1", end: 100},
  {chrom: "chr2", end: 60}
])
let links = table([
  {source_chr: "chr1", source_start: 10, source_end: 24,
   target_chr: "chr2", target_start: 30, target_end: 44, count: 12}
])
let coverage = table([
  {chrom: "chr1", pos: 10, depth: 2},
  {chrom: "chr1", pos: 55, depth: 9},
  {chrom: "chr2", pos: 35, depth: 5}
])
circos({segments: segments, links: links, tracks: [
  {name: "coverage", type: "line", data: coverage}
]}, {theme: "publication", title: "Circular genome", width: 520, height: 520})
`.trim());
  await page.getByLabel("Run active BioLang file").click();
  const circular = await renderedInkFraction(page);
  expect(circular.width).toBe(520);
  expect(circular.height).toBe(520);
  expect(circular.fraction).toBeGreaterThan(0.015);

  await replaceActiveSource(page, `
let left = histogram([1, 1, 2, 3, 5, 8], {theme: "publication", title: "Counts"})
let right = histogram([2, 3, 3, 4, 7, 9], {theme: "publication", title: "Response"})
plot_grid([left, right], {
  columns: 2, title: "Figure 1", shared_xlabel: "measurement",
  width: 900, height: 430
})
`.trim());
  await page.getByLabel("Run active BioLang file").click();
  const composition = await renderedInkFraction(page);
  expect(composition.width).toBe(900);
  expect(composition.height).toBe(430);
  expect(composition.fraction).toBeGreaterThan(0.02);
});
