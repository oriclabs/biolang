import { chromium } from "@playwright/test";

const browser = await chromium.launch({
  headless: true,
  executablePath: "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe",
});
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
const errors = [];
page.on("pageerror", error => errors.push(error.message));

const cases = [
  {
    name: "analysis.py",
    mimeType: "text/x-python",
    content: "x = 1\nprint(x)\n",
    output: "analysis.bl",
    marker: "let x = 1",
  },
  {
    name: "analysis.R",
    mimeType: "text/plain",
    content: "x <- 2\nprint(x)\n",
    output: "analysis.bl",
    marker: "let x = 2",
  },
  {
    name: "analysis.ipynb",
    mimeType: "application/x-ipynb+json",
    content: JSON.stringify({
      cells: [
        { cell_type: "markdown", metadata: {}, source: ["# Analysis"] },
        {
          cell_type: "code",
          execution_count: null,
          metadata: {},
          outputs: [],
          source: ["x = 3\n", "print(x)\n"],
        },
      ],
      metadata: { kernelspec: { language: "python", name: "python3" } },
      nbformat: 4,
      nbformat_minor: 5,
    }),
    output: "analysis.bln",
    marker: "let x = 3",
  },
  {
    name: "analysis.Rmd",
    mimeType: "text/markdown",
    content: "# Analysis\n\n```{r}\nx <- 4\nprint(x)\n```\n",
    output: "analysis.bln",
    marker: "let x = 4",
  },
];

try {
  await page.goto("http://127.0.0.1:3000/studio.html");
  await page.locator("#wasm-status").filter({ hasText: "WASM ready" }).waitFor({ timeout: 30_000 });
  await page.locator('[data-tab="editor"]').click();

  for (const item of cases) {
    await page.locator("#code-import-input").setInputFiles({
      name: item.name,
      mimeType: item.mimeType,
      buffer: Buffer.from(item.content),
    });
    await page.locator("#import-dialog").waitFor({ state: "visible" });
    await page.locator("#import-source-name").filter({ hasText: item.name }).waitFor();
    const validation = await page.locator("#import-validation").innerText();
    if (!validation.match(/validated|syntax issue/)) {
      throw new Error(`Missing validation result for ${item.name}: ${validation}`);
    }
    const outputName = await page.locator("#import-output-name").inputValue();
    if (outputName !== item.output) {
      throw new Error(`Unexpected output name for ${item.name}: ${outputName}`);
    }
    await page.locator("#import-accept").click();
    await page.locator("#import-dialog").waitFor({ state: "hidden" });
    const converted = await page.locator("#editor-code").inputValue();
    if (!converted.includes(item.marker)) {
      throw new Error(`Converted editor content for ${item.name} is missing ${item.marker}`);
    }
  }

  if (errors.length) throw new Error(`Browser errors: ${errors.join("; ")}`);
  console.log("Studio code import passed for Python, R, Jupyter, and R Markdown");
} finally {
  await browser.close();
}
