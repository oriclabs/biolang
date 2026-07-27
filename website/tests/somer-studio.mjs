import { chromium } from "@playwright/test";

const browser = await chromium.launch({
  headless: true,
  executablePath: "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe",
});
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
const errors = [];
const network = [];
page.on("pageerror", (error) => errors.push(error.message));
page.on("response", (response) => {
  if (response.url().includes("/v1/jobs")) {
    void response.json()
      .then((body) => network.push(`${response.status()} ${response.url()} ${body.status || ""}`))
      .catch(() => network.push(`${response.status()} ${response.url()}`));
  }
});

try {
  await page.goto("http://127.0.0.1:3000/studio.html");
  await page.locator('[data-tab="editor"]').click();
  await page.locator("#editor-code").fill('println("SOMER_STUDIO_OK")');
  await page.locator("#execution-target").selectOption("somer");
  await page.locator("#somer-token").fill("dev-user");
  await page.locator("#runtime-test").click();
  await page.getByText("SOMER 0.1.0 as Developer").waitFor();
  await page.locator("#runtime-close").click();
  await page.locator("#editor-run").click();
  await page.waitForTimeout(3_000);
  const output = await page.locator("#editor-output").innerText();
  if (!output.includes("SOMER_STUDIO_OK") || !output.includes("SOMER job succeeded")) {
    throw new Error(`Incomplete Studio output: ${output}; browser errors: ${errors.join("; ")}; network: ${network.join("; ")}`);
  }
  if (errors.length) throw new Error(`Browser errors: ${errors.join("; ")}`);
  console.log("Studio SOMER execution passed");
} finally {
  await browser.close();
}
