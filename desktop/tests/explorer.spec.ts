import { expect, test } from "@playwright/test";
import { flattenVisible } from "../src/components/FileTree";
import type { FileEntry } from "../src/types";

const sampleTree: FileEntry[] = [
  {
    name: "data",
    path: "data",
    kind: "directory",
    size: 0,
    children: [
      { name: "reads.fastq", path: "data/reads.fastq", kind: "file", size: 10, children: [] },
      { name: "genes.gff3", path: "data/genes.gff3", kind: "file", size: 10, children: [] },
    ],
  },
  { name: "analysis.bl", path: "analysis.bl", kind: "file", size: 20, children: [] },
];

test("flattenVisible expands open folders and skips collapsed ones", () => {
  const open = flattenVisible(sampleTree, new Set());
  expect(open.map((row) => row.entry.path)).toEqual([
    "data",
    "data/reads.fastq",
    "data/genes.gff3",
    "analysis.bl",
  ]);

  const collapsed = flattenVisible(sampleTree, new Set(["data"]));
  expect(collapsed.map((row) => row.entry.path)).toEqual(["data", "analysis.bl"]);
});

test("Explorer rows are draggable for move and OS drop", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("main").getByRole("button", { name: "Open Browser Workspace" }).click();
  await expect(page.locator('.tree-row[data-path="analysis.bl"]')).toBeVisible();
  await expect(page.locator('.tree-row[data-path="analysis.bl"]')).toHaveAttribute("draggable", "true");
  await expect(page.locator('.tree-row.directory[data-path="data"]')).toHaveAttribute("draggable", "true");
  await expect(page.locator(".file-tree")).toBeVisible();
});
