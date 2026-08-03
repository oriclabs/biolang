/**
 * Parse every BioLang-labelled documentation block with the real CLI parser.
 *
 * Usage:
 *   node tests/check_cli_syntax.mjs [--dir docs] [--bl ../target/debug/bl.exe]
 *                                   [--json out.json]
 */
import { spawn } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const argv = process.argv.slice(2);
const pick = (flag) => {
  const index = argv.indexOf(flag);
  return index >= 0 ? argv[index + 1] : null;
};
const ONLY = pick("--dir");
const JSON_OUT = pick("--json");
const BL = path.resolve(ROOT, pick("--bl") || "../target/debug/bl.exe");
const CONCURRENCY = Math.max(1, Number(pick("--concurrency") || 8));

const ENTITIES = {
  "&lt;": "<",
  "&gt;": ">",
  "&amp;": "&",
  "&quot;": '"',
  "&#39;": "'",
  "&nbsp;": " ",
};
const decode = (source) =>
  source
    .replace(/&(?:lt|gt|amp|quot|nbsp);|&#39;/g, (match) => ENTITIES[match] ?? match)
    .replace(/&#(\d+);/g, (_, decimal) => String.fromCharCode(Number(decimal)));

function codeBlocks(html) {
  const result = [];
  const pattern =
    /<code[^>]*class="[^"]*\blanguage-(?:bio|biolang|biorun)\b[^"]*"[^>]*>([\s\S]*?)<\/code>/g;
  let match;
  while ((match = pattern.exec(html)) !== null) {
    result.push(decode(match[1].replace(/<[^>]*>/g, "")));
  }
  return result;
}

function htmlFiles(directory) {
  const result = [];
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const item = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      if (entry.name !== "node_modules" && entry.name !== "wasm") {
        result.push(...htmlFiles(item));
      }
    } else if (entry.name.endsWith(".html") && entry.name !== "print.html") {
      result.push(item);
    }
  }
  return result;
}

function parseFile(file) {
  return new Promise((resolve) => {
    const child = spawn(BL, ["check", file], { windowsHide: true });
    let output = "";
    child.stdout.on("data", (chunk) => (output += chunk));
    child.stderr.on("data", (chunk) => (output += chunk));
    child.on("error", (error) => resolve({ ok: false, output: error.message }));
    child.on("close", (code) => resolve({ ok: code === 0, output: output.trim() }));
  });
}

async function mapConcurrent(items, limit, mapper) {
  const results = new Array(items.length);
  let next = 0;
  async function worker() {
    while (next < items.length) {
      const index = next++;
      results[index] = await mapper(items[index], index);
    }
  }
  await Promise.all(Array.from({ length: Math.min(limit, items.length) }, worker));
  return results;
}

if (!fs.existsSync(BL)) {
  console.error(`BioLang CLI not found: ${BL}`);
  process.exit(2);
}

const roots = ONLY
  ? [path.join(ROOT, ONLY)]
  : [path.join(ROOT, "books"), path.join(ROOT, "docs")];
const snippets = [];
for (const root of roots) {
  if (!fs.existsSync(root)) continue;
  for (const file of htmlFiles(root)) {
    codeBlocks(fs.readFileSync(file, "utf8")).forEach((source, block) => {
      snippets.push({ file: path.relative(ROOT, file), block: block + 1, source });
    });
  }
}

const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "biolang-doc-check-"));
try {
  for (let i = 0; i < snippets.length; i++) {
    snippets[i].temp = path.join(tempRoot, `${i}.bl`);
    fs.writeFileSync(snippets[i].temp, snippets[i].source);
  }

  const checked = await mapConcurrent(snippets, CONCURRENCY, async (snippet) => ({
    ...snippet,
    ...(await parseFile(snippet.temp)),
  }));
  const failures = checked
    .filter((item) => !item.ok)
    .map(({ file, block, source, output }) => ({
      file,
      block,
      firstLine: source.split("\n").find((line) => line.trim())?.trim() || "",
      error: output.replaceAll(tempRoot, "<temp>"),
    }));

  console.log(`BioLang blocks parsed : ${snippets.length}`);
  console.log(`syntax failures       : ${failures.length}`);
  for (const failure of failures) {
    console.log(`\n${failure.file} block ${failure.block}: ${failure.firstLine}`);
    console.log(failure.error);
  }
  if (JSON_OUT) {
    fs.writeFileSync(JSON_OUT, JSON.stringify(failures, null, 2));
    console.log(`\nfull detail -> ${JSON_OUT}`);
  }
  process.exitCode = failures.length ? 1 : 0;
} finally {
  fs.rmSync(tempRoot, { recursive: true, force: true });
}
