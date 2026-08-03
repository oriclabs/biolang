/**
 * Ensure BioLang source embedded in HTML cannot be reinterpreted as markup.
 *
 * Usage:
 *   node tests/check_code_block_html.mjs [--dir docs] [--fix]
 */
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const argv = process.argv.slice(2);
const pick = (flag) => argv.indexOf(flag) >= 0 ? argv[argv.indexOf(flag) + 1] : null;
const target = path.join(ROOT, pick("--dir") ?? "docs");
const fix = argv.includes("--fix");
const codeBlock = /(<code[^>]*class="[^"]*\blanguage-(?:bio|biolang|biorun)\b[^"]*"[^>]*>)([\s\S]*?)(<\/code>)/g;

function htmlFiles(directory) {
  const files = [];
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const absolute = path.join(directory, entry.name);
    if (entry.isDirectory()) files.push(...htmlFiles(absolute));
    else if (entry.name.endsWith(".html")) files.push(absolute);
  }
  return files;
}

let findings = 0;
let changed = 0;
for (const file of htmlFiles(target)) {
  const original = fs.readFileSync(file, "utf8");
  let fileFindings = 0;
  const updated = original.replace(codeBlock, (block, open, source, close) => {
    const rawAngles = source.match(/</g)?.length ?? 0;
    if (!rawAngles) return block;
    findings += rawAngles;
    fileFindings += rawAngles;
    return `${open}${fix ? source.replaceAll("<", "&lt;") : source}${close}`;
  });
  if (fileFindings) {
    console.log(`${path.relative(ROOT, file)}: ${fileFindings} raw '<' character${fileFindings === 1 ? "" : "s"}`);
  }
  if (fix && updated !== original) {
    fs.writeFileSync(file, updated);
    changed += 1;
  }
}

console.log(`BioLang code-block HTML findings: ${findings}${fix ? `; fixed ${changed} files` : ""}`);
process.exit(fix || findings === 0 ? 0 : 1);
