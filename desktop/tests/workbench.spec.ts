import { expect, test } from "@playwright/test";
import {
  completionReplacementRange,
  diagnosticMarkerRange,
  pathToFileUri,
  replaceProblemsForPath,
  startLspListening,
} from "../src/lspProtocol";
import {
  createNotebookRunPlan,
  NotebookOutputRouter,
  parseNotebook,
  setNotebookDirective,
} from "../src/notebooks";
import {
  appendJobLog,
  jobLogText,
  latestJobForFile,
  normalizeJobLog,
  stripCliProgress,
} from "../src/jobLogs";
import {
  convertImportOutput,
  importDestination,
  outputNameForKind,
  summarizeConversion,
} from "../src/codeImport";
import { snapshotDelta } from "../src/somerOutput";
import { formatConsoleBytes } from "../src/console";
import { buildOutputExport, outputExportOptions } from "../src/outputExport";
import { availableOutputTabs, outputPlots, outputTables, semanticResultPairs } from "../src/outputModel";
import { buildRunBundle, createZip } from "../src/runBundle";
import type { Job } from "../src/types";
import {
  topologicalWorkflowNodes,
  validateWorkflow,
  workflowToBioLang,
  type WorkflowDocument,
} from "../src/workflows";

test.use({ viewport: { width: 1440, height: 900 } });

async function choosePythonImport(page: import("@playwright/test").Page) {
  const chooserPromise = page.waitForEvent("filechooser");
  await page.getByRole("menuitem", { name: "Import Script from File..." }).click();
  const chooser = await chooserPromise;
  await chooser.setFiles({
    name: "analysis.py",
    mimeType: "text/x-python",
    buffer: Buffer.from('sequence = "ATCGATCG"\nprint(gc_content(sequence))\n'),
  });
}

test("job logs migrate, retain stream types, and select the latest file run", () => {
  const migrated = normalizeJobLog("legacy output\n");
  expect(migrated).toEqual([{ stream: "stdout", text: "legacy output\n" }]);

  let chunks = appendJobLog(migrated, "stdout", "next line\n");
  chunks = appendJobLog(chunks, "stderr", "failed line\n");
  expect(chunks).toEqual([
    { stream: "stdout", text: "legacy output\nnext line\n" },
    { stream: "stderr", text: "failed line\n" },
  ]);
  expect(jobLogText(chunks)).toBe("legacy output\nnext line\nfailed line\n");

  const jobs: Job[] = [
    {
      id: "new",
      file: "analysis.bl",
      status: "succeeded",
      startedAt: 2,
      backend: "Local",
      log: chunks,
    },
    {
      id: "old",
      file: "analysis.bl",
      status: "failed",
      startedAt: 1,
      backend: "Local",
      log: [],
    },
  ];
  expect(latestJobForFile(jobs, "analysis.bl")?.id).toBe("new");
});

test("remote output cursors continue across bounded SOMER snapshots", () => {
  expect(snapshotDelta("abcdef", 0, 3)).toEqual({ data: "def", cursor: 6 });
  expect(snapshotDelta("defghi", 3, 6)).toEqual({ data: "ghi", cursor: 9 });
  expect(snapshotDelta("tail", 100, 20)).toEqual({ data: "tail", cursor: 104 });
});

test("desktop removes duplicate CLI progress while retaining program output", () => {
  expect(stripCliProgress("▶ running s1.bl\n")).toBe("");
  expect(stripCliProgress("✓ done in 53.47ms\n")).toBe("");
  expect(stripCliProgress("{\n  \"n_cells\": 265\n}\n")).toBe("{\n  \"n_cells\": 265\n}\n");
});

test("output export preserves logs and exposes rich formats when available", () => {
  const chunks = [
    { stream: "system" as const, text: "running analysis.bl\n" },
    { stream: "stdout" as const, text: "{\"cells\":265}\n" },
    { stream: "stdout" as const, text: '<svg xmlns="http://www.w3.org/2000/svg"></svg>\n' },
  ];
  const options = outputExportOptions(chunks).map((option) => option.format);
  expect(options).toEqual(["log", "text", "svg", "html"]);
  expect(buildOutputExport(chunks, "text")).not.toContain("running analysis.bl");
  expect(buildOutputExport(chunks, "svg")).toContain("<svg");
  expect(buildOutputExport(chunks, "html")).toContain("<!doctype html>");

  const jsonOnly = [{ stream: "stdout" as const, text: "{\"cells\":265}\n" }];
  expect(outputExportOptions(jsonOnly).map((option) => option.format)).toContain("json");
  expect(buildOutputExport(jsonOnly, "json")).toContain('"cells": 265');
});

test("structured run results expose tables, plots, and contextual tabs", () => {
  const job: Job = {
    id: "structured",
    file: "analysis.bl",
    status: "succeeded",
    startedAt: 10,
    backend: "Local",
    log: [{ stream: "stderr", text: "analysis.bl:3:5: invalid value\n" }],
    results: [
      {
        kind: "table",
        columns: ["gene", "count"],
        rows: [[{ kind: "string", value: "TP53" }, { kind: "integer", value: 4 }]],
        totalRows: 1,
      },
      {
        kind: "plot",
        format: "svg",
        data: '<svg xmlns="http://www.w3.org/2000/svg"><rect width="10" height="10"/></svg>',
      },
    ],
    provenance: {
      packages: { singlecell: "0.1.0" },
      backend: "Local",
      entrypoint: "analysis.bl",
      parameters: {},
    },
  };
  expect(outputTables(job)[0].rows).toEqual([["TP53", 4]]);
  expect(outputPlots(job)).toHaveLength(1);
  expect(availableOutputTabs(job)).toEqual([
    "summary",
    "text",
    "tables",
    "plots",
    "errors",
    "provenance",
  ]);
});

test("run comparison matches stable result identities instead of emission order", () => {
  const run = (id: string, results: Job["results"]): Job => ({
    id,
    file: "analysis.bl",
    status: "succeeded",
    startedAt: 1,
    backend: "Local",
    log: [],
    results,
  });
  const pairs = semanticResultPairs(
    run("a", [
      { kind: "float", id: "qc-score", name: "QC score", value: 0.8 },
      { kind: "integer", id: "cells", name: "Cells", value: 100 },
    ]),
    run("b", [
      { kind: "integer", id: "cells", name: "Cells", value: 120 },
      { kind: "float", id: "qc-score", name: "QC score", value: 0.9 },
    ]),
  );
  expect(pairs.map((pair) => pair.key)).toEqual(["cells", "qc-score"]);
  expect(pairs[0].left?.value).toBe(100);
  expect(pairs[0].right?.value).toBe(120);
});

test("run bundles are valid ZIP containers with reproducibility files", () => {
  const bareZip = createZip([{ name: "hello.txt", content: "hello" }], 0);
  expect(Array.from(bareZip.slice(0, 4))).toEqual([0x50, 0x4b, 0x03, 0x04]);

  const bundle = buildRunBundle({
    id: "bundle",
    file: "analysis.bl",
    status: "succeeded",
    startedAt: 1_700_000_000_000,
    durationMs: 20,
    backend: "Local",
    log: [{ stream: "stdout", text: "complete\n" }],
    provenance: {
      biolangVersion: "1.0.0",
      packages: {},
      backend: "Local",
      entrypoint: "analysis.bl",
      sourceSnapshot: "let result = 42\n",
      parameters: {},
    },
    artifacts: [{ name: "plots/qc.svg", size: 11, mediaType: "image/svg+xml" }],
  }, new Map([["plots/qc.svg", new TextEncoder().encode("<svg></svg>")]]));
  const archiveText = new TextDecoder().decode(bundle.bytes);
  expect(bundle.name).toMatch(/analysis\.bl.*\.zip$/);
  expect(archiveText).toContain("provenance.json");
  expect(archiveText).toContain("environment.json");
  expect(archiveText).toContain("checksums.sha256");
  expect(archiveText).toContain("REPRODUCE.txt");
  expect(archiveText).toContain("source.bl");
  expect(archiveText).toContain("output.log");
  expect(archiveText).toContain("artifacts/plots/qc.svg");
  expect(archiveText).toContain("<svg></svg>");
});

test("detached Output opens recorded runs without loading the IDE shell", async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem("biolang.desktop.jobs", JSON.stringify([{
      id: "detached-run",
      file: "plots.bl",
      displayName: "QC plots",
      status: "succeeded",
      startedAt: Date.now(),
      durationMs: 40,
      backend: "Local",
      log: [{ stream: "stdout", text: "analysis complete\n" }],
      results: [{
        kind: "plot",
        format: "svg",
        data: '<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20"><rect width="20" height="20"/></svg>',
      }],
    }]));
  });
  await page.goto("http://127.0.0.1:1420/?detachedOutput=1&jobId=detached-run");
  await expect(page.locator(".detached-output-shell")).toBeVisible();
  await expect(page.locator(".app-shell")).toHaveCount(0);
  await expect(page.getByText("analysis complete")).toBeVisible();
  await page.getByRole("button", { name: "plots" }).click();
  await expect(page.locator(".detached-plots img")).toBeVisible();
});

test("Output moves independently and drags between bottom, right, and editor", async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem("biolang.desktop.experienceMode", JSON.stringify("expert"));
    localStorage.setItem("biolang.desktop.outputLocation", JSON.stringify("bottom"));
    localStorage.setItem("biolang.desktop.bottomVisible", JSON.stringify(false));
  });
  await page.goto("http://127.0.0.1:1420");

  await page.getByRole("button", { name: "View", exact: true }).click();
  await page.getByRole("menuitem", { name: "Output at Right" }).click();
  await expect(page.locator(".output-side-panel")).toHaveCount(0);
  await page.getByRole("main").getByRole("button", { name: "Open Browser Workspace" }).click();
  await expect(page.locator(".output-side-panel")).toBeVisible();
  await expect(page.locator(".bottom-panel")).toHaveCount(0);
  await expect(page.getByLabel("More Output actions")).toBeVisible();

  await expect(page.locator('.tree-row[data-path="analysis.bl"]')).toBeVisible();
  await page.locator('.tree-row[data-path="analysis.bl"]').click({ button: "right" });
  await page.getByRole("menuitem", { name: "Run", exact: true }).click();
  await expect(page.locator(".output-view")).toContainText("Process completed", { timeout: 20_000 });
  await page.locator(".editor-tab", { hasText: "analysis.bl" }).click({ button: "right" });
  await expect(page.getByRole("menuitem", { name: "Run", exact: true })).toBeVisible();
  await page.keyboard.press("Escape");
  await page.getByRole("button", { name: "Run", exact: true }).click();
  await page.getByRole("menuitem", { name: "BioLang Console Ctrl+Shift+`", exact: true }).click();
  await expect(page.locator(".console-pane")).toBeVisible();
  await expect(page.locator(".output-side-panel")).toBeVisible();
  await expect(page.getByLabel("Dock shared panel right")).toHaveCount(0);
  await expect(page.locator(".workbench")).not.toHaveClass(/panel-dock-right/);

  const dragOutputTo = async (label: "editor" | "bottom") => {
    const handle = await page.getByLabel("Drag Output to dock").boundingBox();
    expect(handle).not.toBeNull();
    await page.mouse.move(handle!.x + handle!.width / 2, handle!.y + handle!.height / 2);
    await page.mouse.down();
    await page.mouse.move(handle!.x - 12, handle!.y + handle!.height / 2, { steps: 3 });
    const target = page.getByLabel(`Dock Output in ${label}`);
    await expect(target).toBeVisible();
    const bounds = await target.boundingBox();
    expect(bounds).not.toBeNull();
    await page.mouse.move(bounds!.x + bounds!.width / 2, bounds!.y + bounds!.height / 2, { steps: 6 });
    await expect(target).toHaveClass(/active/);
    await page.mouse.up();
  };

    await dragOutputTo("editor");
    await expect(page.locator(".output-editor-tab")).toBeVisible();
    await expect(page.locator(".output-pane-editor")).toBeVisible();
    await expect(page.locator(".console-pane")).toBeVisible();

    await page.locator(".editor-tab", { hasText: "analysis.bl" }).getByRole("button", { name: "analysis.bl", exact: true }).click();
    await expect(page.locator(".output-editor-tab")).toBeVisible();
    await expect(page.locator(".output-editor-tab")).not.toHaveClass(/active/);
    await expect(page.locator(".output-pane-editor")).toHaveCount(0);
    await page.locator(".output-editor-tab").getByRole("button", { name: "Output", exact: true }).click();
    await expect(page.locator(".output-editor-tab")).toHaveClass(/active/);
    await expect(page.locator(".output-pane-editor")).toBeVisible();

    await dragOutputTo("bottom");
    await expect(page.locator(".bottom-panel")).toBeVisible();
  await expect(page.locator(".output-side-panel")).toHaveCount(0);
});

test("View menu controls persistent bottom-panel tabs", async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem("biolang.desktop.experienceMode", JSON.stringify("expert"));
    localStorage.setItem("biolang.desktop.outputLocation", JSON.stringify("bottom"));
    localStorage.setItem("biolang.desktop.bottomVisible", JSON.stringify(true));
  });
  await page.goto("http://127.0.0.1:1420");
  await page.getByRole("main").getByRole("button", { name: "Open Browser Workspace" }).click();

  const panelTabs = page.locator(".panel-tabs");
  await panelTabs.getByRole("button", { name: "problems", exact: true }).click();
  await page.getByRole("button", { name: "View", exact: true }).click();
  await page.getByRole("menuitem", { name: "Problems Tab", exact: true }).click();
  await expect(panelTabs.getByRole("button", { name: "problems", exact: true })).toHaveCount(0);
  await expect(panelTabs.getByRole("button", { name: "output", exact: true })).toHaveClass(/active/);

  await page.getByRole("button", { name: "View", exact: true }).click();
  await page.getByRole("menuitem", { name: "Console Tab", exact: true }).click();
  await expect(panelTabs.getByRole("button", { name: "console", exact: true })).toHaveCount(0);
  await page.reload();
  await expect(page.locator(".panel-tabs").getByRole("button", { name: "console", exact: true })).toHaveCount(0);

  await page.getByRole("button", { name: "View", exact: true }).click();
  await page.getByRole("menuitem", { name: "Console Tab", exact: true }).click();
  await expect(page.locator(".panel-tabs").getByRole("button", { name: "console", exact: true })).toBeVisible();
});

test("welcome hints stay compact and Help removes irrelevant editor chrome", async ({ page }) => {
  await page.setViewportSize({ width: 1024, height: 700 });
  await page.addInitScript(() => {
    localStorage.setItem("biolang.desktop.experienceMode", JSON.stringify("expert"));
    localStorage.setItem("biolang.desktop.bottomVisible", JSON.stringify(true));
  });
  await page.goto("http://127.0.0.1:1420");

  await expect(page.locator(".bottom-panel")).toHaveCount(0);
  await expect(page.getByLabel("Run active BioLang file")).toHaveCount(0);
  await expect(page.locator(".welcome-comparison")).not.toHaveAttribute("open", "");
  await expect(page.getByRole("button", { name: "View", exact: true })).toBeVisible();

  await page.getByRole("main").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.locator('.tree-row[data-path="analysis.bl"]').click();
  await expect(page.getByLabel("Run active BioLang file")).toBeVisible();
  await page.getByRole("button", { name: "output", exact: true }).click();
  await page.locator(".activity-bar").getByLabel("Help Center").click();

  await expect(page.getByLabel("Run active BioLang file")).toHaveCount(0);
  await expect(page.locator(".bottom-panel")).toBeHidden();
  await expect(page.locator(".help-tab")).toBeVisible();
});

test("plot output exposes zoom and scientific export controls", async ({ page }) => {
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
  await expect(page.getByLabel("Zoom in")).toBeVisible();
  await expect(page.getByRole("button", { name: "SVG", exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "PNG", exact: true })).toBeVisible();
  await expect(page.getByLabel("Print or save plot as PDF")).toBeVisible();
});

test("console memory labels remain compact and readable", () => {
  expect(formatConsoleBytes(512)).toBe("512 B");
  expect(formatConsoleBytes(1536)).toBe("1.5 KB");
  expect(formatConsoleBytes(2 * 1024 * 1024)).toBe("2.0 MB");
});

test("LSP protocol adapters preserve Windows URIs and exact editor ranges", () => {
  expect(pathToFileUri("C:\\Research Data\\genome#1")).toBe(
    "file:///C:/Research%20Data/genome%231",
  );
  expect(pathToFileUri("/opt/biolang/project")).toBe("file:///opt/biolang/project");
  expect(completionReplacementRange(4, 13, 8)).toEqual({
    startLineNumber: 4,
    endLineNumber: 4,
    startColumn: 8,
    endColumn: 13,
  });
  expect(diagnosticMarkerRange({
    start: { line: 1, character: 20 },
    end: { line: 3, character: 2 },
  })).toEqual({
    startLineNumber: 2,
    startColumn: 21,
    endLineNumber: 4,
    endColumn: 3,
  });
  expect(replaceProblemsForPath(
    [{ path: "analysis.bl", message: "old" }, { path: "other.bl", message: "keep" }],
    "analysis.bl",
    [],
  )).toEqual([{ path: "other.bl", message: "keep" }]);
});

test("failed LSP startup disposes its message listener before retry", async () => {
  let listeners = 0;
  let disposals = 0;
  const result = await startLspListening(
    async () => {
      listeners += 1;
      return () => {
        disposals += 1;
      };
    },
    async () => false,
  );
  expect(result.started).toBe(false);
  expect(listeners).toBe(1);
  expect(disposals).toBe(1);
});

test("notebook execution plans preserve cells and route framed output", () => {
  const plan = createNotebookRunPlan(
    "# Analysis\n\n```biolang\nprintln(\"one\")\n```\n\n```biolang\nprintln(\"two\")\n```\n",
    "test",
  );
  expect(plan.cellIndexes).toEqual([0, 1]);
  expect(plan.reportedCellIndexes).toEqual([0, 1]);
  expect(plan.notebookSource).toContain("println(\"one\")");
  expect(plan.notebookSource).toContain("__BIOLANG_DESKTOP_CELL_test_1");

  const router = new NotebookOutputRouter(plan.markerPrefix, plan.cellIndexes);
  expect(router.stdout("first\n__BIOLANG_DESKTOP_CELL_test_0\nsec")).toEqual({
    visible: "first\n",
    chunks: [{ cellIndex: 0, data: "first\n" }],
  });
  expect(router.stdout("ond\n__BIOLANG_DESKTOP_CELL_test_1\n")).toEqual({
    visible: "second\n",
    chunks: [{ cellIndex: 1, data: "second\n" }],
  });
});

test("notebook parser matches CLI fences, legacy cells, metadata, and directives", () => {
  const source = `---
title: Replication origins
authors: A. Researcher
---
# Analysis

\`\`\`
# @hide-code
# @hide-output
println("bare")
\`\`\`

\`\`\`python
print("prose example")
\`\`\`

---
# @skip
println("legacy")
---
`;
  const blocks = parseNotebook(source);
  expect(blocks.map((block) => block.type)).toEqual(["metadata", "markdown", "code", "markdown", "code"]);
  const code = blocks.filter((block) => block.type === "code");
  expect(code).toHaveLength(2);
  expect(code[0].directives).toEqual(["hide", "hide-output"]);
  expect(code[0].content).toBe('println("bare")');
  expect(code[1].syntax).toBe("dash");
  expect(code[1].directives).toEqual(["skip"]);
  expect(blocks.find((block) => block.type === "markdown")?.content).toContain("# Analysis");
  expect(blocks.filter((block) => block.type === "markdown").at(-1)?.content).toContain("```python");
});

test("invalid front matter remains legacy notebook content and directives only apply at the top", () => {
  const source = `---
this is code
---
\`\`\`bl
println("before")
# @skip
println("after")
\`\`\``;
  const blocks = parseNotebook(source);
  expect(blocks[0].type).toBe("code");
  const fenced = blocks.filter((block) => block.type === "code").at(-1);
  expect(fenced?.directives).toEqual([]);
  expect(fenced?.content).toContain("# @skip");
});

test("notebook directives can be toggled without rewriting cell code", () => {
  const source = "```biolang\nprintln(\"kept\")\n```\n";
  const block = parseNotebook(source).find((candidate) => candidate.type === "code")!;
  const hidden = setNotebookDirective(source, block, "hide-output", true);
  expect(hidden).toContain('# @hide-output\nprintln("kept")');
  const hiddenBlock = parseNotebook(hidden).find((candidate) => candidate.type === "code")!;
  expect(setNotebookDirective(hidden, hiddenBlock, "hide-output", false)).toBe(source);
});

test("notebook plans skip cells, translate chat remotely, and suppress hidden output", () => {
  const source = `\`\`\`biolang
# @skip
println("skip")
\`\`\`
\`\`\`biolang
# @chat
# @hide-output
Summarize this sequence.
\`\`\``;
  const plan = createNotebookRunPlan(source, "directives");
  expect(plan.cellIndexes).toEqual([1]);
  expect(plan.reportedCellIndexes).toEqual([1]);
  expect(plan.hiddenOutputCellIndexes).toEqual([1]);
  expect(plan.scriptSource).toContain('println(chat("Summarize this sequence."))');
  expect(plan.notebookSource).toContain("# @chat");

  const router = new NotebookOutputRouter(
    plan.markerPrefix,
    plan.cellIndexes,
    plan.hiddenOutputCellIndexes,
  );
  expect(router.stdout(`secret\n${plan.markerPrefix}1\n`)).toEqual({ visible: "", chunks: [] });
  expect(router.stderr("also secret")).toEqual({ visible: "", chunks: [] });
});

test("single-cell notebook runs replay preceding cells without replacing their results", () => {
  const source = `\`\`\`biolang
let sequence = dna"ACGT"
\`\`\`
\`\`\`biolang
println(gc_content(sequence))
\`\`\``;
  const plan = createNotebookRunPlan(source, "single", 1);
  expect(plan.cellIndexes).toEqual([0, 1]);
  expect(plan.reportedCellIndexes).toEqual([1]);
  expect(plan.hiddenOutputCellIndexes).toEqual([0]);
  expect(plan.scriptSource.indexOf('let sequence = dna"ACGT"')).toBeLessThan(plan.scriptSource.indexOf("gc_content(sequence)"));
  expect(plan.notebookSource).toContain("# @hide-output");
});

test("import review converts output formats, destinations, and summaries predictably", () => {
  const script = "# TODO: unsupported call\nlet x = pca(data) # approximation\n";
  const notebook = convertImportOutput(script, "script", "notebook", "analysis.py");
  expect(notebook).toContain("```biolang");
  expect(convertImportOutput(notebook, "notebook", "script", "analysis.py")).toContain("let x = pca(data)");
  expect(outputNameForKind("analysis.bl", "notebook")).toBe("analysis.bln");
  expect(outputNameForKind("analysis.bln", "script")).toBe("analysis.bl");
  expect(importDestination("reports", "analysis.bln")).toBe("reports/analysis.bln");
  expect(importDestination("reports", "../analysis.bln")).toBeUndefined();
  expect(summarizeConversion(script, "script")).toEqual({
    converted: 1,
    approximated: 1,
    unsupported: 1,
  });
});

test("workflow DAGs sort dependencies and generate all incoming branches", () => {
  const node = (id: string, operation: string) => ({
    id,
    operation,
    arguments: [],
    parameters: [],
    strategy: "standard" as const,
    x: 0,
    y: 0,
  });
  const workflow: WorkflowDocument = {
    schemaVersion: 1,
    name: "Branched analysis",
    nodes: [node("merge", "zip"), node("right", "take"), node("source", "read_fasta"), node("left", "filter")],
    edges: [
      { from: "source", to: "left" },
      { from: "source", to: "right" },
      { from: "left", to: "merge" },
      { from: "right", to: "merge" },
    ],
  };
  expect(topologicalWorkflowNodes(workflow).map((candidate) => candidate.id)).toEqual([
    "source", "left", "right", "merge",
  ]);
  const source = workflowToBioLang(workflow);
  expect(source).toContain("let merge = zip(left, right)");
  expect(source.indexOf("let source")).toBeLessThan(source.indexOf("let left"));
  expect(validateWorkflow({
    ...workflow,
    edges: [...workflow.edges, { from: "merge", to: "source" }],
  })).toContain("Workflow contains a cycle");
  expect(validateWorkflow({
    ...workflow,
    nodes: [node("same-id", "take"), node("same id", "filter")],
    edges: [],
  })).toContain("Node ids 'same-id' and 'same id' generate the same BioLang variable");
});

test("core desktop workflow remains inside the workbench", async ({ page }) => {
  await page.goto("http://127.0.0.1:1420");
  await expect(page.getByText("No workspace open", { exact: true })).toBeVisible();
  await expect(page.getByRole("heading", { name: "BioLang Workbench Web" })).toBeVisible();
  await page.locator(".workspace-welcome").getByRole("button", { name: "Open Browser Workspace" }).click();
  await expect(page.getByText("analysis.bl", { exact: true }).first()).toBeVisible();
  await page.getByText("analysis.bl", { exact: true }).first().click();
  await expect(page.locator(".editor-tab")).toHaveCount(1);
  await expect(page.locator(".monaco-editor")).toBeVisible({ timeout: 20_000 });

  await page.getByRole("button", { name: "File", exact: true }).click();
  await expect(page.getByRole("menuitem", { name: "Save Ctrl+S", exact: true })).toBeVisible();
  await expect(page.getByRole("menuitem", { name: "Save All Ctrl+K S", exact: true })).toBeVisible();
  await page.keyboard.press("Escape");

  await page.getByLabel("Bio APIs").click();
  await page.getByText("uniprot_entry", { exact: true }).click();
  await expect(page.getByText("API reference", { exact: true })).toBeVisible();
  await expect(page.getByText("uniprot_entry(accession) → Record", { exact: true })).toBeVisible();

  await page.getByLabel("Run active BioLang file").click();
  await expect(page.locator(".output-view").getByText("GC content: 0.4090909090909091", { exact: false })).toBeVisible({ timeout: 3_000 });
  await expect(page.getByText("Process completed", { exact: false })).toBeVisible({ timeout: 3_000 });
  await expect(page.locator(".output-run-content").getByRole("status")).toHaveCount(0);
  await page.locator(".panel-tabs").getByRole("button", { name: "jobs" }).click();
  await page.getByLabel("Rerun analysis.bl").click();
  await expect(page.getByText("Process completed", { exact: false })).toBeVisible({ timeout: 3_000 });

  await page.getByLabel("Packages").click();
  await expect(page.getByText("oric", { exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "Install dependencies" }).last()).toBeDisabled();

  await page.keyboard.press("Control+Shift+P");
  await expect(page.getByPlaceholder("Search files, > for commands, @ for symbols")).toBeVisible();
  await expect(page.getByText("BioLang: Run Active File", { exact: true })).toBeVisible();
});

test("Explorer expands and collapses the complete workspace tree", async ({ page }) => {
  await page.goto("http://127.0.0.1:1420");
  await page.locator(".workspace-welcome").getByRole("button", { name: "Open Browser Workspace" }).click();

  await expect(page.getByText("sequences.fasta", { exact: true })).toBeVisible();
  await page.getByLabel("Collapse all folders").click();
  await expect(page.getByText("data", { exact: true })).toBeVisible();
  await expect(page.getByText("sequences.fasta", { exact: true })).toBeHidden();

  await page.getByLabel("Expand all folders").click();
  await expect(page.getByText("sequences.fasta", { exact: true })).toBeVisible();

  await page.getByRole("button", { name: "View", exact: true }).click();
  await page.getByRole("menuitem", { name: "Collapse All Explorer Folders" }).click();
  await expect(page.getByText("sequences.fasta", { exact: true })).toBeHidden();

  await page.keyboard.press("Control+Shift+P");
  await page.getByPlaceholder("Search files, > for commands, @ for symbols").fill(">expand all folders");
  await page.getByText("Explorer: Expand All Folders", { exact: true }).click();
  await expect(page.getByText("sequences.fasta", { exact: true })).toBeVisible();

  await page.getByRole("button", { name: "File", exact: true }).click();
  await expect(page.getByRole("menuitem", { name: "Import Script from File..." })).toBeVisible();
  await expect(page.getByRole("menuitem", { name: "Import Script from URL..." })).toBeVisible();
  await expect(page.getByRole("menuitem", { name: "Import Data..." })).toBeVisible();
});

test("output follows the active file and preserves stream coloring in job history", async ({ page }) => {
  await page.goto("http://127.0.0.1:1420");
  await page.locator(".workspace-welcome").getByRole("button", { name: "Open Browser Workspace" }).click();

  await page.locator('.tree-row[data-path="analysis.bl"]').click();
  await page.getByLabel("Run active BioLang file").click();
  await expect(page.locator(".output-view")).toContainText("Process completed", { timeout: 3_000 });
  await expect(page.locator(".output-pane-toolbar")).toContainText("analysis.bl");
  await expect(page.locator(".output-view .job-log-chunk.system")).toContainText("running analysis.bl");
  await expect(page.locator(".output-view .job-log-chunk.stdout")).toContainText("GC content: 0.4090909090909091");

  await page.locator('.tree-row[data-path="pipelines/qc.bl"]').click();
  await page.getByLabel("Run active BioLang file").click();
  await expect(page.locator(".output-view")).toContainText("Process completed", { timeout: 3_000 });
  await expect(page.locator(".output-pane-toolbar")).toContainText("qc.bl");
  await expect(page.locator(".output-view")).toContainText("running qc.bl");
  await expect(page.locator(".output-view")).not.toContainText("running analysis.bl");

  await page.locator('.tree-row[data-path="analysis.bl"]').click();
  await expect(page.locator(".output-pane-toolbar")).toContainText("analysis.bl");
  await expect(page.locator(".output-view")).toContainText("running analysis.bl");
  await expect(page.locator(".output-view")).not.toContainText("running qc.bl");

  await page.locator(".panel-tabs").getByRole("button", { name: "jobs" }).click();
  await page.getByLabel("View logs for analysis.bl").click();
  await expect(page.locator(".job-log-view")).toContainText("GC content: 0.4090909090909091");
  await expect(page.locator(".job-log-view .job-log-chunk.system")).toContainText("running analysis.bl");
});

test("centered navigation switches between persisted learner and expert modes", async ({ page }) => {
  await page.goto("http://127.0.0.1:1420");
  const commandCenter = await page.getByLabel("Command center").boundingBox();
  expect(commandCenter).not.toBeNull();
  expect(Math.abs(commandCenter!.x + commandCenter!.width / 2 - 720)).toBeLessThan(2);

  await page.getByRole("group", { name: "Interface mode" }).getByRole("button", { name: "Learner" }).click();
  await expect(page.locator(".app-shell")).toHaveClass(/learner-mode/);
  await expect(page.locator(".activity-bar").getByText("Explorer", { exact: true })).toBeVisible();
  await expect(page.locator(".app-menu").getByRole("button", { name: "File", exact: true })).toBeVisible();
  await expect(page.locator(".app-menu").getByRole("button", { name: "Run", exact: true })).toBeVisible();
  await expect(page.locator(".app-menu").getByRole("button", { name: "Help", exact: true })).toBeVisible();
  await expect(page.locator(".app-menu").getByRole("button", { name: "Selection", exact: true })).toHaveCount(0);

  await page.reload();
  await expect(page.locator(".app-shell")).toHaveClass(/learner-mode/);
  await expect(page.getByLabel("Command center")).toContainText("Search files, help, and commands");

  await page.getByRole("group", { name: "Interface mode" }).getByRole("button", { name: "Expert" }).click();
  await expect(page.locator(".app-shell")).toHaveClass(/expert-mode/);
  await expect(page.locator(".activity-bar").getByText("Explorer", { exact: true })).toBeHidden();
  await expect(page.locator(".app-menu").getByRole("button", { name: "Selection", exact: true })).toBeVisible();
});

test("external database browser runs examples through the BioLang backend", async ({ page }) => {
  await page.route("https://eutils.ncbi.nlm.nih.gov/**", async (route) => {
    const url = new URL(route.request().url());
    await route.fulfill({
      contentType: "application/json",
      body: url.pathname.endsWith("/esearch.fcgi")
        ? JSON.stringify({ esearchresult: { idlist: ["672"] } })
        : JSON.stringify({
            result: {
              "672": {
                name: "BRCA1",
                description: "BRCA1 DNA repair associated",
                organism: { scientificname: "Homo sapiens" },
                chromosome: "17",
              },
            },
          }),
    });
  });
  await page.goto("http://127.0.0.1:1420");
  await page.locator(".workspace-welcome").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.getByLabel("Bio APIs").click();

  await expect(page.getByRole("group", { name: "API browser scope" })).toBeVisible();
  await expect(page.getByRole("button", { name: "External DBs" })).toHaveClass(/active/);
  await expect(page.getByRole("button", { name: "ncbi_gene", exact: true })).toBeVisible();
  await expect(page.locator(".api-detail")).toContainText("NCBI");

  await page.getByRole("button", { name: "Run example" }).click();
  await expect(page.locator(".panel-tabs").getByRole("button", { name: "jobs" })).toHaveClass(/active/);
  await expect(page.locator(".job-log-view")).toContainText("API test/ncbi_gene");
  await expect(page.locator(".job-log-view")).toContainText("Process completed", { timeout: 3_000 });
  await expect(page.locator(".job-log-view")).toContainText("BRCA1");

  await page.getByRole("button", { name: "All builtins" }).click();
  await page.getByPlaceholder("Search functions").fill("gc_content");
  await expect(page.getByRole("button", { name: "gc_content", exact: true })).toBeVisible();
});

test("code import previews validation and saves converted BioLang", async ({ page }) => {
  await page.goto("http://127.0.0.1:1420");
  await page.locator(".workspace-welcome").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.getByRole("button", { name: "File", exact: true }).click();
  await choosePythonImport(page);

  await expect(page.getByRole("region", { name: "Import code" })).toBeVisible();
  await expect(page.getByText("Syntax valid", { exact: true })).toBeVisible();
  await expect(page.getByText("Read only", { exact: true })).toBeVisible();
  await expect(page.locator(".import-review-editors > section").nth(0).locator(".monaco-editor")).toContainText("gc_content");
  await expect(page.locator(".import-review-editors > section").nth(1).locator(".monaco-editor")).toContainText("gc_content");
  await page.getByRole("group", { name: "Import output format" }).getByRole("button", { name: "Notebook" }).click();
  await expect(page.getByLabel("Output file")).toHaveValue("analysis.bln");
  await expect(page.getByText("Validation outdated", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Revalidate", exact: true }).last().click();
  await expect(page.getByText("Syntax valid", { exact: true })).toBeVisible();
  await page.getByLabel("Import destination folder").selectOption("reports");
  await page.getByRole("button", { name: "Save and Open" }).click();

  await expect(page.locator(".editor-tab", { hasText: "analysis.bln" })).toBeVisible();
  await expect(page.locator(".notebook-pane")).toBeVisible();
  await expect(page.getByText("Imported and validated analysis.py", { exact: true })).toBeVisible();
});

test("URL script import downloads into the same validation review", async ({ page }) => {
  await page.route("https://example.org/remote-analysis.py", (route) => route.fulfill({
    contentType: "text/x-python",
    body: 'sequence = "ATCGATCG"\nprint(gc_content(sequence))\n',
  }));
  await page.goto("http://127.0.0.1:1420");
  await page.locator(".workspace-welcome").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.getByRole("button", { name: "File", exact: true }).click();
  await page.getByRole("menuitem", { name: "Import Script from URL..." }).click();

  const dialog = page.getByRole("form", { name: "Import script from URL" });
  await dialog.getByLabel("HTTP or HTTPS URL").fill("https://example.org/remote-analysis.py");
  await dialog.getByRole("button", { name: "Download and Review" }).click();

  await expect(page.getByRole("region", { name: "Import code" })).toBeVisible();
  await expect(page.getByText("Import remote-analysis.py", { exact: true })).toBeVisible();
  await expect(page.getByText("Syntax valid", { exact: true })).toBeVisible();
  await expect(page.getByText("Review only. Imported code is not executed automatically.", { exact: true })).toBeVisible();
});

test("edited imports require revalidation and can be saved only as reviewed drafts", async ({ page }) => {
  await page.goto("http://127.0.0.1:1420");
  await page.locator(".workspace-welcome").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.getByRole("button", { name: "File", exact: true }).click();
  await choosePythonImport(page);

  const converted = page.getByLabel("Converted BioLang preview");
  await converted.focus();
  await converted.press("Control+A");
  await page.keyboard.type("let =");
  await expect(page.getByText("Validation outdated", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Revalidate", exact: true }).last().click();
  await expect(page.locator(".import-diagnostics")).toContainText("expected identifier");
  await expect(page.getByRole("button", { name: "Save Draft" })).toBeVisible();

  await page.getByLabel("Output file").fill("review-needed.bl");
  await page.getByLabel("Import destination folder").selectOption("reports");
  await page.getByRole("button", { name: "Save Draft" }).click();
  await expect(page.locator(".editor-tab", { hasText: "review-needed.bl" })).toBeVisible();
  await expect(page.locator(".monaco-editor")).toContainText("let =");
  await expect(page.getByText("Imported analysis.py as a draft", { exact: true })).toBeVisible();
});

test("Save As creates a workspace copy and moves the active editor", async ({ page }) => {
  await page.goto("http://127.0.0.1:1420");
  await page.locator(".workspace-welcome").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.getByText("analysis.bl", { exact: true }).first().click();
  await expect(page.locator(".monaco-editor")).toBeVisible({ timeout: 20_000 });

  await page.getByRole("button", { name: "File", exact: true }).click();
  await page.getByRole("menuitem", { name: "Save As... Ctrl+Shift+S", exact: true }).click();

  await expect(page.locator(".editor-tab", { hasText: "analysis copy.bl" })).toBeVisible();
  await expect(page.locator('.tree-row[data-path="analysis copy.bl"]')).toBeVisible();
  await expect(page.locator(".monaco-editor")).toContainText("BRCA1 sequence quality snapshot");
  await expect(page.getByText("Saved as analysis copy.bl", { exact: true })).toBeVisible();
});

test("compact desktop layout has no horizontal overflow", async ({ page }) => {
  await page.setViewportSize({ width: 1024, height: 700 });
  await page.goto("http://127.0.0.1:1420");
  await page.locator(".workspace-welcome").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.getByText("analysis.bl", { exact: true }).first().click();
  await expect(page.locator(".monaco-editor")).toBeVisible({ timeout: 20_000 });
  const dimensions = await page.evaluate(() => ({
    body: document.body.scrollWidth,
    viewport: document.documentElement.clientWidth,
    height: document.body.scrollHeight,
    viewportHeight: document.documentElement.clientHeight,
  }));
  expect(dimensions.body).toBeLessThanOrEqual(dimensions.viewport);
  expect(dimensions.height).toBeLessThanOrEqual(dimensions.viewportHeight);
});

test("Explorer creates, renames, and deletes a file", async ({ page }) => {
  await page.goto("http://127.0.0.1:1420");
  await page.locator(".workspace-welcome").getByRole("button", { name: "Open Browser Workspace" }).click();

  await page.getByLabel("New file").click();
  await expect(page.locator(".editor-tab", { hasText: "Untitled-1.bl" })).toBeVisible();
  await expect(page.locator('.tree-row[data-path="Untitled-1.bl"]')).toHaveCount(0);
  await page.getByLabel("Save file").click();
  await expect(page.getByText("Save Untitled File", { exact: true })).toBeVisible();
  await page.getByLabel("Name").fill("scratch.bl");
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await expect(page.getByText("scratch.bl", { exact: true }).first()).toBeVisible();
  await expect(page.locator(".editor-tab", { hasText: "scratch.bl" })).toBeVisible();

  await page.locator(".tree-row", { hasText: "scratch.bl" }).click({ button: "right" });
  await page.getByRole("menuitem", { name: "Rename..." }).click();
  await page.getByLabel("Name").fill("renamed.bl");
  await page.getByRole("button", { name: "Rename" }).click();
  await expect(page.getByText("renamed.bl", { exact: true }).first()).toBeVisible();

  await page.locator(".tree-row", { hasText: "renamed.bl" }).click({ button: "right" });
  await page.getByRole("menuitem", { name: "Delete" }).click();
  await page.getByRole("alertdialog").getByRole("button", { name: "Delete" }).click();
  await expect(page.locator(".tree-row", { hasText: "renamed.bl" })).toHaveCount(0);
});

test("untitled BioLang buffers run without being saved", async ({ page }) => {
  await page.goto("http://127.0.0.1:1420");
  await page.locator(".workspace-welcome").getByRole("button", { name: /Open (Folder|Browser Workspace)/ }).click();

  await page.getByLabel("New file").click();
  await expect(page.locator(".editor-tab", { hasText: "Untitled-1.bl" })).toBeVisible();
  await page.locator(".monaco-editor").click();
  await page.keyboard.insertText('println("untitled works")');
  await expect(page.getByLabel("Run active BioLang file")).toBeEnabled();
  await page.getByLabel("Run active BioLang file").click();

  await expect(page.locator(".output-view")).toContainText("untitled works", { timeout: 3_000 });
  await expect(page.locator(".output-view")).toContainText("Process completed", { timeout: 3_000 });
  await expect(page.locator('.tree-row[data-path^="__untitled__"]')).toHaveCount(0);
});

test("renaming a demo directory rebases all descendant paths", async ({ page }) => {
  await page.goto("http://127.0.0.1:1420");
  await page.locator(".workspace-welcome").getByRole("button", { name: "Open Browser Workspace" }).click();

  await page.locator('.tree-row[data-path="data"]').click({ button: "right" });
  await page.getByRole("menuitem", { name: "Rename..." }).click();
  await page.getByLabel("Name").fill("inputs");
  await page.getByRole("button", { name: "Rename" }).click();
  await expect(page.locator('.tree-row[data-path="inputs/sequences.fasta"]')).toBeVisible();
  await page.locator('.tree-row[data-path="inputs/sequences.fasta"]').click();
  await expect(page.locator(".preview-kind")).toHaveText("FASTA");

  await page.locator('.tree-row[data-path="inputs"]').click({ button: "right" });
  await page.getByRole("menuitem", { name: "Rename..." }).click();
  await page.getByLabel("Name").fill("data");
  await page.getByRole("button", { name: "Rename" }).click();
  await expect(page.locator('.tree-row[data-path="data/sequences.fasta"]')).toBeVisible();
});

test("workspace content search opens the matching source location", async ({ page }) => {
  await page.goto("http://127.0.0.1:1420");
  await page.locator(".workspace-welcome").getByRole("button", { name: "Open Browser Workspace" }).click();

  await page.getByLabel("Search").click();
  await page.getByPlaceholder("Search file contents").fill("gc_content");
  // Hits group under a per-file heading, so each row carries only line:column.
  const analysisGroup = page.locator(".search-file-group")
    .filter({ has: page.locator(".search-file-heading > span", { hasText: /^analysis\.bl$/ }) });
  await expect(analysisGroup.locator(".search-file-heading")).toBeVisible();
  await expect(analysisGroup.getByText("4:10", { exact: true })).toBeVisible();
  await expect(page.locator(".content-hit").first()).toContainText("let gc = gc_content(sequence)");
  await page.locator(".content-hit").first().click();

  await expect(page.locator(".monaco-editor")).toBeVisible({ timeout: 20_000 });
  await expect(page.locator(".editor-tab.active")).toContainText("analysis.bl");
});

test("bio and tabular files open in bounded interactive previews", async ({ page }) => {
  await page.goto("http://127.0.0.1:1420");
  await page.locator(".workspace-welcome").getByRole("button", { name: "Open Browser Workspace" }).click();

  await page.getByText("sequences.fasta", { exact: true }).click();
  await expect(page.locator(".preview-kind")).toHaveText("FASTA");
  await expect(page.locator(".statusbar")).toContainText("bases");
  await expect(page.locator(".statusbar")).toContainText("GC");
  await expect(page.locator(".preview-table")).toContainText("ori_candidate");
  await expect(page.getByLabel("Sequence preview")).toContainText("TAAACGTGAG");
  await page.getByRole("button", { name: "Reverse complement" }).click();
  await expect(page.getByLabel("Sequence preview")).toContainText("ATACCACACG");
  await page.getByLabel("Sequence record", { exact: true }).selectOption({ label: "2. control" });
  await page.getByRole("button", { name: "Forward" }).click();
  await expect(page.getByLabel("Sequence preview")).toContainText("CCAGATC");
  await page.getByLabel("Search sequence").fill("CGCG");
  await expect(page.locator(".sequence-search")).toContainText("1 of 1");

  await page.getByText("expression.csv", { exact: true }).click();
  await expect(page.locator(".preview-kind")).toHaveText("TABLE");
  await expect(page.locator(".preview-table")).toContainText("BRCA1");
  await page.getByPlaceholder("Filter preview").fill("TP53");
  await expect(page.locator(".preview-table tbody tr")).toHaveCount(1);
  await expect(page.locator(".preview-table tbody")).toContainText("TP53");
});

test("extended biological viewers expose provenance and visualization modes", async ({ page }) => {
  await page.goto("http://127.0.0.1:1420");
  await page.locator(".workspace-welcome").getByRole("button", { name: "Open Browser Workspace" }).click();

  await page.getByText("genes.gff3", { exact: true }).click();
  await expect(page.locator(".preview-kind")).toHaveText("GFF");
  await expect(page.locator(".preview-table")).toContainText("BRCA1");
  await expect(page.locator(".preview-provenance")).toContainText("data/genes.gff3");
  await page.getByLabel("Heatmap").click();
  await expect(page.locator(".heatmap-cell").first()).toBeVisible();
  await page.getByLabel("Export format").selectOption("json");
  await expect(page.getByLabel("Export JSON")).toBeVisible();

  await page.getByText("helix.pdb", { exact: true }).click();
  await expect(page.locator(".preview-kind")).toHaveText("STRUCTURE");
  await expect(page.getByLabel("Interactive molecular structure")).toBeVisible();
  await page.getByLabel("Structure representation").selectOption("ball+stick");
  await expect(page.locator(".structure-stage canvas")).toHaveCount(1);
  await expect.poll(() => page.locator(".structure-stage canvas").evaluate((canvas: HTMLCanvasElement) => {
    const context = canvas.getContext("webgl2") || canvas.getContext("webgl");
    if (!context) return 0;
    const pixels = new Uint8Array(canvas.width * canvas.height * 4);
    context.readPixels(0, 0, canvas.width, canvas.height, context.RGBA, context.UNSIGNED_BYTE, pixels);
    let colored = 0;
    for (let index = 0; index < pixels.length; index += 4) {
      if (pixels[index] !== 16 || pixels[index + 1] !== 21 || pixels[index + 2] !== 25) colored += 1;
    }
    return colored;
  }), { timeout: 10_000 }).toBeGreaterThan(100);

  await page.getByText("species.nwk", { exact: true }).click();
  await expect(page.getByText("Newick tree", { exact: true })).toBeVisible();
  await expect(page.locator(".newick-preview")).toContainText("Mammals");
});

test("literate notebooks edit and run through the workbench", async ({ page }) => {
  await page.goto("http://127.0.0.1:1420");
  await page.locator(".workspace-welcome").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.getByText("origin-analysis.bl.md", { exact: true }).click();

  await expect(page.locator(".notebook-pane")).toBeVisible();
  await expect(page.locator(".notebook-cell")).toHaveCount(2);
  await expect(page.locator(".notebook-cell .monaco-editor")).toHaveCount(2);
  await expect(page.getByRole("heading", { name: "Origin candidate analysis" })).toBeVisible();
  await page.getByRole("button", { name: "Run all" }).click();
  await expect(page.locator(".notebook-cell-output")).toHaveCount(2);
  await expect(page.locator(".notebook-cell").nth(0).locator(".notebook-cell-output")).toContainText("GC content: 0.4090909090909091");
  await expect(page.locator(".notebook-cell").nth(1).locator(".notebook-cell-output")).toContainText("ACGT | 2");
  await expect(page.locator(".bottom-panel")).toContainText("Process completed");

  await page.getByRole("button", { name: "Run code cell 2" }).click();
  await expect(page.locator(".notebook-cell").nth(1).locator(".notebook-cell-output")).toContainText("succeeded");

  await page.locator(".notebook-cell").nth(0).getByLabel("Cell directives").click();
  await page.locator(".notebook-cell").nth(0).getByText("Hide output", { exact: true }).click();
  await expect(page.locator(".notebook-cell").nth(0).locator(".notebook-directives")).toContainText("@hide-output");
  await page.getByRole("button", { name: "Run code cell 1" }).click();
  await expect(page.locator(".notebook-cell").nth(0).locator(".notebook-cell-output")).toContainText("Output hidden by directive");

  await page.getByLabel("Source", { exact: true }).click();
  await expect(page.locator(".notebook-source .monaco-editor")).toBeVisible();
  await page.getByLabel("Notebook").click();
  await expect(page.locator(".notebook-cell .monaco-editor")).toHaveCount(2);
});

test("source pipelines and typed workflows have dedicated visual surfaces", async ({ page }) => {
  await page.goto("http://127.0.0.1:1420");
  await page.locator(".workspace-welcome").getByRole("button", { name: "Open Browser Workspace" }).click();

  await page.getByText("qc.bl", { exact: true }).click();
  await expect(page.locator(".monaco-editor")).toBeVisible({ timeout: 20_000 });
  await page.getByRole("button", { name: "Pipeline", exact: true }).click();
  await expect(page.locator(".pipeline-viewer")).toBeVisible();
  await expect(page.locator(".pipeline-viewer")).toContainText("read_fastq");
  await expect(page.locator(".pipeline-viewer")).toContainText("filter");
  await expect(page.locator(".pipeline-viewer")).toContainText("take");
  await expect(page.locator(".pipeline-viewer")).toContainText("each");
  await page.getByRole("button", { name: "Source", exact: true }).click();
  await expect(page.locator(".monaco-editor")).toBeVisible();

  await page.getByText("sequence-qc.blflow", { exact: true }).click();
  await expect(page.locator(".workflow-pane")).toBeVisible();
  await expect(page.locator(".workflow-node")).toHaveCount(3);
  await page.getByRole("button", { name: "Add step" }).click();
  await expect(page.locator(".workflow-node")).toHaveCount(4);
  await page.getByRole("button", { name: "Run", exact: true }).last().click();
  await expect(page.getByText("Workflow pipelines/sequence-qc.blflow completed", { exact: false })).toBeVisible();
});

test("Explorer duplicates a file and exposes native file actions", async ({ page }) => {
  await page.goto("http://127.0.0.1:1420");
  await page.locator(".workspace-welcome").getByRole("button", { name: "Open Browser Workspace" }).click();

  await page.locator('.tree-row[data-path="analysis.bl"]').click({ button: "right" });
  await expect(page.getByRole("menu", { name: "analysis.bl actions" })).toBeVisible();
  await expect(page.getByRole("menuitem", { name: "Open", exact: true })).toBeFocused();
  await expect(page.getByRole("menuitem", { name: "Copy Relative Path" })).toBeVisible();
  await expect(page.getByRole("menuitem", { name: "Reveal in File Manager" })).toBeVisible();
  await page.keyboard.press("ArrowDown");
  await expect(page.getByRole("menuitem", { name: "Run", exact: true })).toBeFocused();
  await page.keyboard.press("ArrowDown");
  await expect(page.getByRole("menuitem", { name: "Rename..." })).toBeFocused();
  await page.getByRole("menuitem", { name: "Duplicate" }).click();
  await expect(page.getByText("analysis copy.bl", { exact: true }).first()).toBeVisible();

  await page.locator(".tree-row", { hasText: "analysis copy.bl" }).click({ button: "right" });
  await page.getByRole("menuitem", { name: "Delete" }).click();
  await page.getByRole("alertdialog").getByRole("button", { name: "Delete" }).click();
  await expect(page.locator(".tree-row", { hasText: "analysis copy.bl" })).toHaveCount(0);

  await page.locator('.tree-row[data-path="data"]').click({ button: "right" });
  await expect(page.getByRole("menuitem", { name: "New File" })).toBeVisible();
  await expect(page.getByRole("menuitem", { name: "New Folder..." })).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("menu")).toHaveCount(0);

  await page.locator(".workspace-heading").click({ button: "right" });
  await expect(page.getByRole("menu", { name: "genome-workbench workspace actions" })).toBeVisible();
  await expect(page.getByRole("menuitem", { name: "Copy Workspace Path" })).toBeVisible();
  await expect(page.getByRole("menuitem", { name: "Refresh" })).toBeVisible();
});

test("context menus stay in the viewport and editor tabs support close actions", async ({ page }) => {
  await page.goto("http://127.0.0.1:1420");
  await page.locator(".workspace-welcome").getByRole("button", { name: "Open Browser Workspace" }).click();
  const analysis = page.locator('.tree-row[data-path="analysis.bl"]');
  await analysis.evaluate((element) => element.dispatchEvent(new MouseEvent("contextmenu", {
    bubbles: true,
    clientX: window.innerWidth - 1,
    clientY: window.innerHeight - 1,
  })));
  const menuBox = await page.getByRole("menu").boundingBox();
  expect(menuBox).not.toBeNull();
  expect(menuBox!.x + menuBox!.width).toBeLessThanOrEqual(1440);
  expect(menuBox!.y + menuBox!.height).toBeLessThanOrEqual(900);
  await page.keyboard.press("Escape");

  await page.getByText("analysis.bl", { exact: true }).first().click();
  await page.getByText("README.md", { exact: true }).first().click();
  await page.locator(".editor-tab", { hasText: "README.md" }).click({ button: "right" });
  await expect(page.getByRole("menu", { name: "Editor tab actions" })).toBeVisible();
  await page.getByRole("menuitem", { name: "Close Others" }).click();
  await expect(page.locator(".editor-tab")).toHaveCount(1);
  await expect(page.locator(".editor-tab")).toContainText("README.md");
});

test("workbench controls expose symbols, shortcuts, output state, and resizable panes", async ({ page }) => {
  await page.goto("http://127.0.0.1:1420");
  await page.keyboard.press("Control+`");
  await expect(page.locator(".toast")).toContainText("Open a workspace");

  await page.locator(".workspace-welcome").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.locator('.tree-row[data-path="analysis.bl"]').click();
  await expect(page.locator(".outline-item", { hasText: "sequence" })).toBeVisible();

  await page.keyboard.press("Control+Shift+O");
  await expect(page.locator(".command-palette")).toContainText("sequence");
  await expect(page.locator(".command-palette")).toContainText("Binding · line 2");
  await page.keyboard.press("Escape");

  await page.keyboard.press("Control+,");
  await expect(page.locator(".settings-dialog")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.locator(".settings-dialog")).toHaveCount(0);

  const sidebarBefore = await page.locator(".sidebar").boundingBox();
  const handle = await page.getByRole("separator", { name: "Resize sidebar" }).boundingBox();
  expect(sidebarBefore).not.toBeNull();
  expect(handle).not.toBeNull();
  await page.mouse.move(handle!.x + handle!.width / 2, handle!.y + 100);
  await page.mouse.down();
  await page.mouse.move(handle!.x + 42, handle!.y + 100);
  await page.mouse.up();
  const sidebarAfter = await page.locator(".sidebar").boundingBox();
  expect(sidebarAfter!.width).toBeGreaterThan(sidebarBefore!.width + 30);

  await page.getByLabel("Toggle bottom panel").click();
  await page.getByRole("button", { name: "output", exact: true }).click();
  await page.getByLabel("Run active BioLang file").click();
  await expect(page.locator(".output-view")).toContainText("Process completed", { timeout: 3_000 });
  await page.getByLabel("More Output actions").click();
  await page.locator(".output-compact-menu").getByRole("button", { name: "Clear output", exact: true }).click();
  await expect(page.locator(".output-view")).toHaveText("This run produced no output.");
  await page.getByLabel("Maximize panel").click();
  await expect(page.locator(".workbench")).toHaveClass(/panel-maximized/);
  await expect(page.locator(".editor-workspace")).toHaveCSS("visibility", "hidden");
  await page.getByLabel("Restore panel").click();
  await expect(page.locator(".workbench")).not.toHaveClass(/panel-maximized/);
  await page.getByLabel("Maximize panel").click();
  await page.keyboard.press("Escape");
  await expect(page.locator(".workbench")).not.toHaveClass(/panel-maximized/);

  await page.getByRole("button", { name: "Help", exact: true }).click();
  await page.getByRole("menuitem", { name: "Keyboard Shortcuts" }).click();
  await expect(page.locator(".shortcuts-dialog")).toContainText("Go to Symbol");
  await page.keyboard.press("Escape");
  await expect(page.locator(".shortcuts-dialog")).toHaveCount(0);
});

test("learner Output is compact, bottom dock has one close action, and data previews reclaim empty output space", async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem("biolang.desktop.experienceMode", JSON.stringify("learner"));
  });
  await page.goto("http://127.0.0.1:1420");
  await page.locator(".workspace-welcome").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.locator('.tree-row[data-path="analysis.bl"]').click();
  await page.getByLabel("Run active BioLang file").click();
  await expect(page.locator(".output-pane-bottom")).toBeVisible();
  await expect(page.locator(".output-pane-bottom").getByLabel("More Output actions")).toBeVisible();
  await expect(page.locator(".output-pane-bottom").getByLabel("Pin run")).toBeHidden();
  await page.locator(".output-pane-bottom").getByLabel("More Output actions").click();
  await expect(page.locator(".output-pane-bottom").getByRole("button", { name: "Clear output" })).toBeVisible();
  await expect(page.locator(".output-pane-bottom").getByRole("button", { name: "Export reproducibility bundle" })).toBeVisible();
  await expect(page.getByLabel("Close panel")).toHaveCount(1);
  await expect(page.locator(".output-pane-bottom").getByLabel("Close Output")).toHaveCount(0);

  await page.getByText("sequences.fasta", { exact: true }).click();
  await expect(page.locator(".preview-kind")).toHaveText("FASTA");
  await expect(page.locator(".bottom-panel")).toBeHidden();
});

test("recent workspaces reopen without a folder picker", async ({ page }) => {
  await page.goto("http://127.0.0.1:1420");
  await page.locator(".workspace-welcome").getByRole("button", { name: "Open Browser Workspace" }).click();

  await page.getByRole("button", { name: "File", exact: true }).click();
  await page.getByRole("menuitem", { name: "Close Folder", exact: true }).click();
  await expect(page.getByText("Recent", { exact: true })).toBeVisible();
  await page.locator(".recent-workspaces").getByRole("button").click();

  await expect(page.getByText("analysis.bl", { exact: true }).first()).toBeVisible();
  await expect(page.locator(".workspace-welcome.compact h1")).toHaveText("genome-workbench");
});

test("terminal panel manages multiple persistent sessions", async ({ page }) => {
  await page.goto("http://127.0.0.1:1420");
  await page.locator(".workspace-welcome").getByRole("button", { name: "Open Browser Workspace" }).click();

  await page.getByRole("button", { name: "Run", exact: true }).click();
  await expect(page.getByRole("menuitem", { name: "New Terminal Ctrl+`", exact: true })).toBeDisabled();
});

test("terminal owns its scrollbar without continuously changing layout", async ({ page }) => {
  await page.goto("http://127.0.0.1:1420");
  await page.locator(".workspace-welcome").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.getByLabel("Toggle bottom panel").click();
  await page.locator(".panel-tabs").getByRole("button", { name: "terminal", exact: true }).click();

  const panelContent = page.locator(".terminal-panel-content");
  const terminal = page.locator(".terminal-host");
  await expect(terminal).toBeVisible();
  await expect(page.locator(".terminal-wrap")).toHaveAttribute("data-state", "ready");
  await expect(panelContent).toHaveCSS("overflow", "hidden");

  const dimensions = await terminal.evaluate(async (element) => {
    const samples = new Set<string>();
    for (let frame = 0; frame < 30; frame += 1) {
      await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
      const bounds = element.getBoundingClientRect();
      samples.add(`${bounds.width}:${bounds.height}`);
    }
    return [...samples];
  });
  expect(dimensions).toHaveLength(1);
});

test("BioLang Console retains session objects and exposes environment memory", async ({ page }) => {
  await page.goto("http://127.0.0.1:1420");
  await page.locator(".workspace-welcome").getByRole("button", { name: /Open (Folder|Browser Workspace)/ }).click();

  await page.getByRole("button", { name: "Run", exact: true }).click();
  await page.getByRole("menuitem", { name: "BioLang Console Ctrl+Shift+`", exact: true }).click();
  await expect(page.locator(".console-pane")).toBeVisible();
  await expect(page.locator(".console-connection")).toContainText("Session 1");

  await page.locator(".console-editor").click();
  await page.keyboard.insertText("let x = 42");
  await page.keyboard.press("Control+Enter");
  await expect(page.locator(".console-variable", { hasText: "x" })).toContainText("42");

  await page.locator(".console-editor").click();
  await page.keyboard.insertText("x * 2");
  await page.keyboard.press("Control+Enter");
  await expect(page.locator(".console-value").last()).toHaveText("84");
  await expect(page.locator(".console-memory")).toContainText("Estimated object memory");

  await page.getByLabel("Restart console session").click();
  await expect(page.locator(".console-no-variables")).toContainText("No user objects");
});

test("offline Help Center covers language, builtins, tutorials, and examples", async ({ page }) => {
  await page.goto("http://127.0.0.1:1420");
  await page.keyboard.press("F1");
  await expect(page.getByPlaceholder("Search all BioLang help")).toBeVisible();
  await expect(page.locator(".help-total")).toHaveText(/\d+/);
  expect(Number(await page.locator(".help-total").textContent())).toBeGreaterThanOrEqual(1_000);

  await page.getByRole("tab", { name: "Built-ins" }).click();
  await page.getByPlaceholder("Search all BioLang help").fill("reverse_complement");
  await page.locator(".help-result", { hasText: "reverse_complement" }).click();
  await expect(page.locator(".help-document-header h1")).toHaveText("reverse_complement");
  await expect(page.locator(".help-document")).toContainText("reverse_complement(seq)");
  await expect(page.locator(".help-document")).toContainText("dna\"ATCG\"");

  await page.getByRole("tab", { name: "Tutorials" }).click();
  await page.getByPlaceholder("Search all BioLang help").fill("What Is Bioinformatics");
  await page.locator(".help-result", { hasText: "Day 1: What Is Bioinformatics?" }).click();
  await expect(page.locator(".help-document-header h1")).toContainText("What Is Bioinformatics");

  await page.getByRole("tab", { name: "Examples" }).click();
  await page.getByPlaceholder("Search all BioLang help").fill("hello.bl");
  await page.locator(".help-result").first().click();
  await expect(page.locator(".help-document")).toContainText("Variables are declared with let");
});

test("Help navigation follows internal links and resets stale searches", async ({ page }) => {
  await page.goto("http://127.0.0.1:1420");
  await page.keyboard.press("F1");
  await page.getByPlaceholder("Search all BioLang help").fill("gc_content");

  await page.getByRole("button", { name: "Help", exact: true }).click();
  await page.getByRole("menuitem", { name: "Language Guide", exact: true }).click();
  await expect(page.getByPlaceholder("Search all BioLang help")).toHaveValue("");

  await page.getByPlaceholder("Search all BioLang help").fill("Introduction");
  await page.locator(".help-result", { hasText: "Introduction" }).first().click();
  await expect(page.locator(".help-code").filter({ hasText: "cargo install biolang" }).getByText("Shell", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Benchmarks & Correctness", exact: true }).click();
  await expect(page.locator(".help-document-header h1")).toHaveText("Benchmarks & Correctness");
  await expect(page.getByPlaceholder("Search all BioLang help")).toHaveValue("");
});

test("Help examples open source and insert into an active BioLang file", async ({ page }) => {
  await page.goto("http://127.0.0.1:1420");
  await page.locator(".workspace-welcome").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.getByText("analysis.bl", { exact: true }).first().click();
  await expect(page.locator(".monaco-editor")).toBeVisible({ timeout: 20_000 });

  await page.keyboard.press("F1");
  await page.getByRole("tab", { name: "Built-ins" }).click();
  await page.getByPlaceholder("Search all BioLang help").fill("gc_content");
  await page.locator(".help-result", { hasText: "gc_content" }).click();
  await page.getByRole("button", { name: "Insert in editor" }).click();
  await expect(page.locator(".monaco-editor")).toBeVisible({ timeout: 20_000 });
  await expect(page.locator(".editor-tab.active .editor-tab-main > i")).toBeVisible();

  await page.keyboard.press("F1");
  await page.getByRole("tab", { name: "Examples" }).click();
  await page.getByPlaceholder("Search all BioLang help").fill("analysis.bl");
  await page.locator(".help-result").first().click();
  await expect(page.getByRole("button", { name: "Open source" })).toBeDisabled();
});

test("SOMER profile runs the active file on a remote backend", async ({ page }) => {
  await page.route("http://127.0.0.1:8787/v1/**", async (route) => {
    const url = new URL(route.request().url());
    if (url.pathname === "/v1/service-info") {
      await route.fulfill({
        json: {
          name: "SOMER",
          version: "0.1.0",
          apiVersion: "v1",
          executionMode: "integrated",
          capabilities: ["biolang", "jobs"],
        },
      });
      return;
    }
    if (url.pathname === "/v1/me") {
      await route.fulfill({
        json: { id: "developer", displayName: "Developer", roles: ["user"] },
      });
      return;
    }
    if (url.pathname === "/v1/jobs" && route.request().method() === "POST") {
      expect(route.request().headers().authorization).toBe("Bearer test-token");
      const request = route.request().postDataJSON();
      expect(request.source).toContain("gc_content");
      await route.fulfill({
        status: 202,
        json: {
          id: "0196a649-f013-7a83-9118-a194796c3321",
          status: "queued",
          stdout: "",
          stderr: "",
        },
      });
      return;
    }
    if (url.pathname.endsWith("/0196a649-f013-7a83-9118-a194796c3321/artifacts")) {
      await route.fulfill({ json: { artifacts: [] } });
      return;
    }
    if (url.pathname.includes("0196a649-f013-7a83-9118-a194796c3321")) {
      await route.fulfill({
        json: {
          id: "0196a649-f013-7a83-9118-a194796c3321",
          status: "succeeded",
          stdout: "Remote GC content: 0.4783\n",
          stderr: "",
          exitCode: 0,
        },
      });
      return;
    }
    await route.fulfill({ status: 404 });
  });

  await page.goto("http://127.0.0.1:1420");
  await page.locator(".workspace-welcome").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.getByText("analysis.bl", { exact: true }).first().click();
  await expect(page.locator(".monaco-editor")).toBeVisible({ timeout: 20_000 });

  await page.locator(".activity-bar").getByLabel("Settings").click();
  await page.getByRole("tab", { name: "Remote" }).click();
  await page.getByLabel("Bearer token").fill("test-token");
  await page.getByRole("button", { name: "Test connection" }).click();
  await expect(page.getByText("SOMER 0.1.0 as Developer")).toBeVisible();
  await page.getByRole("button", { name: "Close", exact: true }).click();

  await page.getByLabel("Execution target for Run").selectOption("somer-lab");
  await page.getByLabel("Run active BioLang file").click();
  // First remote run confirms the target.
  const confirm = page.getByRole("alertdialog");
  if (await confirm.isVisible().catch(() => false)) {
    await confirm.getByRole("button", { name: /Run on/i }).click();
  }
  await expect(page.getByText("Remote GC content: 0.4783", { exact: false })).toBeVisible();
  await page.locator(".panel-tabs").getByRole("button", { name: "jobs" }).click();
  await page.getByLabel("View logs for analysis.bl").click();
  await expect(page.locator(".job-log-view")).toContainText("Remote GC content: 0.4783");
  await expect(page.locator(".job-log-view")).toContainText("Remote job succeeded on SOMER Lab.");
});

test("SOMER cancellation uses the stable target id after a profile rename", async ({ page }) => {
  let cancelled = false;
  await page.route("http://127.0.0.1:8787/v1/**", async (route) => {
    const url = new URL(route.request().url());
    if (url.pathname === "/v1/jobs" && route.request().method() === "POST") {
      await route.fulfill({
        status: 202,
        json: { id: "stable-target-job", status: "queued", stdout: "", stderr: "" },
      });
      return;
    }
    if (url.pathname.endsWith("/stable-target-job/cancel")) {
      cancelled = true;
      await route.fulfill({
        json: { id: "stable-target-job", status: "cancelled", stdout: "", stderr: "" },
      });
      return;
    }
    if (url.pathname.endsWith("/stable-target-job")) {
      await route.fulfill({
        json: { id: "stable-target-job", status: "running", stdout: "", stderr: "" },
      });
      return;
    }
    await route.fulfill({ status: 404 });
  });

  await page.goto("http://127.0.0.1:1420");
  await page.locator(".workspace-welcome").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.getByText("analysis.bl", { exact: true }).first().click();
  await expect(page.locator(".monaco-editor")).toBeVisible({ timeout: 20_000 });
  await page.locator(".activity-bar").getByLabel("Settings").click();
  await page.getByRole("tab", { name: "Remote" }).click();
  await page.getByLabel("Bearer token").fill("test-token");
  await page.getByRole("button", { name: "Close", exact: true }).click();
  await page.getByLabel("Execution target for Run").selectOption("somer-lab");
  await page.getByLabel("Run active BioLang file").click();
  const confirm = page.getByRole("alertdialog");
  if (await confirm.isVisible().catch(() => false)) {
    await confirm.getByRole("button", { name: /Run on/i }).click();
  }
  await expect(page.getByText("Submitted analysis.bl to SOMER Lab.", { exact: false })).toBeVisible();

  await page.locator(".activity-bar").getByLabel("Settings").click();
  await page.getByRole("tab", { name: "Remote" }).click();
  await page.locator(".settings-dialog").getByRole("textbox", { name: "Name" }).fill("Renamed SOMER");
  await page.getByRole("button", { name: "Close", exact: true }).click();
  await page.getByLabel("Stop running job").click();
  await expect.poll(() => cancelled).toBe(true);
});

test("closing a workspace aborts SOMER polling and leaves the job disconnected", async ({ page }) => {
  await page.route("http://127.0.0.1:8787/v1/**", async (route) => {
    const url = new URL(route.request().url());
    if (url.pathname === "/v1/jobs" && route.request().method() === "POST") {
      await route.fulfill({
        status: 202,
        json: { id: "workspace-switch-job", status: "queued", stdout: "", stderr: "" },
      });
      return;
    }
    if (url.pathname.endsWith("/workspace-switch-job")) {
      await new Promise((resolve) => setTimeout(resolve, 300));
      await route.fulfill({
        json: { id: "workspace-switch-job", status: "running", stdout: "", stderr: "" },
      });
      return;
    }
    await route.fulfill({ status: 404 });
  });

  await page.goto("http://127.0.0.1:1420");
  await page.locator(".workspace-welcome").getByRole("button", { name: "Open Browser Workspace" }).click();
  await page.getByText("analysis.bl", { exact: true }).first().click();
  await expect(page.locator(".monaco-editor")).toBeVisible({ timeout: 20_000 });
  await page.locator(".activity-bar").getByLabel("Settings").click();
  await page.getByRole("tab", { name: "Remote" }).click();
  await page.getByLabel("Bearer token").fill("test-token");
  await page.getByRole("button", { name: "Close", exact: true }).click();
  await page.getByLabel("Execution target for Run").selectOption("somer-lab");
  await page.getByLabel("Run active BioLang file").click();
  const confirm = page.getByRole("alertdialog");
  if (await confirm.isVisible().catch(() => false)) {
    await confirm.getByRole("button", { name: /Run on/i }).click();
  }

  await page.getByRole("button", { name: "File", exact: true }).click();
  await page.getByRole("menuitem", { name: "Close Folder", exact: true }).click();
  await page.getByLabel("Jobs").click();
  await expect(page.locator(".job-sidebar-row")).toContainText("disconnected");
  await page.waitForTimeout(700);
  await expect(page.locator(".job-sidebar-row")).toContainText("disconnected");
});
