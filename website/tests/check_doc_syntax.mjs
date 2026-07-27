/**
 * Scan doc code blocks for syntax the language does not have.
 *
 * These are constructs that look plausible (and appear in the docs) but are
 * rejected by the lexer or parser, so every example using them fails for the
 * reader. Each pattern below was verified against the real interpreter.
 *
 * Usage: node tests/check_doc_syntax.mjs [--dir docs] [--json out.json]
 */
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const argv = process.argv.slice(2);
const pick = (f) => (argv.indexOf(f) >= 0 ? argv[argv.indexOf(f) + 1] : null);
const ONLY = pick("--dir");
const JSON_OUT = pick("--json");

/** Each rule: what it matches, and what to write instead. */
const RULES = [
  {
    id: "semicolon-separator",
    // `;` is not a statement separator — the lexer rejects the character.
    re: /;/,
    why: "';' is not a statement separator (lexer: unexpected character)",
    fix: "put each statement on its own line",
  },
  {
    id: "tuple-index",
    // `.0` / `.1` positional access — the parser wants a field NAME.
    // The dot must follow an identifier or closing bracket; without that guard
    // this also matches the fractional part of a decimal like `dexp(x, 0.5)`.
    re: /(?<=[A-Za-z_)\]])\.\d+\b/,
    why: "'.0'/'.1' tuple indexing is not supported (parser: expected field name)",
    fix: "index with [0]/[1], or use a record with named fields",
  },
  {
    id: "for-destructuring",
    // `for a, b in xs` — the parser expects `in` right after one binding.
    re: /\bfor\s+[A-Za-z_]\w*\s*,\s*[A-Za-z_]\w*\s+in\b/,
    why: "for-loop tuple destructuring is not supported (parser: expected 'in', found ',')",
    fix: "bind one variable and index it, e.g. `for p in xs { p[0] }`",
  },
  {
    id: "named-arg-equals",
    // Named arguments use `:` not `=`.
    re: /\(([^()"']*,)?\s*[A-Za-z_]\w*\s*=\s*[^=]/,
    why: "named arguments use ':' not '=' (parser: expected ')', found '=')",
    fix: "write f(arg: value)",
  },
];

const E = { "&lt;": "<", "&gt;": ">", "&amp;": "&", "&quot;": '"', "&#39;": "'", "&nbsp;": " " };
const decode = (s) =>
  s.replace(/&(?:lt|gt|amp|quot|nbsp);|&#39;/g, (m) => E[m] ?? m)
   .replace(/&#(\d+);/g, (_, d) => String.fromCharCode(+d));

function blocks(html) {
  const out = [];
  const re = /<code[^>]*class="[^"]*\blanguage-(?:bio|biolang|biorun)\b[^"]*"[^>]*>([\s\S]*?)<\/code>/g;
  let m;
  while ((m = re.exec(html)) !== null) out.push(decode(m[1].replace(/<[^>]*>/g, "")));
  return out;
}

function htmlFiles(dir) {
  const out = [];
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) {
      if (e.name === "node_modules" || e.name === "wasm") continue;
      out.push(...htmlFiles(p));
    } else if (e.name.endsWith(".html") && e.name !== "print.html") out.push(p);
  }
  return out;
}

/** Blank out string literals and comments so rules don't match inside them. */
function stripNoise(line) {
  let out = line.replace(/"(?:[^"\\]|\\.)*"/g, '""');
  const hash = out.indexOf("#");
  if (hash >= 0) out = out.slice(0, hash);
  return out;
}

const roots = ONLY ? [path.join(ROOT, ONLY)] : [path.join(ROOT, "books"), path.join(ROOT, "docs")];
const findings = [];
let scanned = 0;

for (const root of roots) {
  if (!fs.existsSync(root)) continue;
  for (const file of htmlFiles(root)) {
    for (const code of blocks(fs.readFileSync(file, "utf8"))) {
      scanned++;
      const lines = code.split("\n");
      for (let i = 0; i < lines.length; i++) {
        const clean = stripNoise(lines[i]);
        if (!clean.trim()) continue;
        for (const r of RULES) {
          if (r.re.test(clean)) {
            findings.push({
              file: path.relative(ROOT, file),
              rule: r.id,
              why: r.why,
              fix: r.fix,
              line: lines[i].trim().slice(0, 90),
            });
          }
        }
      }
    }
  }
}

console.log(`code blocks scanned : ${scanned}`);
console.log(`unsupported-syntax hits : ${findings.length}\n`);

const byRule = new Map();
for (const f of findings) {
  if (!byRule.has(f.rule)) byRule.set(f.rule, []);
  byRule.get(f.rule).push(f);
}
for (const [rule, v] of [...byRule].sort((a, b) => b[1].length - a[1].length)) {
  const files = new Set(v.map((x) => x.file));
  console.log(`${String(v.length).padStart(4)}  ${rule}   (${files.size} pages)`);
  console.log(`      ${v[0].why}`);
  console.log(`      fix: ${v[0].fix}`);
  for (const ex of v.slice(0, 3)) console.log(`        ${ex.file}: ${ex.line}`);
  console.log("");
}

const byFile = new Map();
for (const f of findings) byFile.set(f.file, (byFile.get(f.file) || 0) + 1);
console.log("worst pages:");
for (const [f, n] of [...byFile].sort((a, b) => b[1] - a[1]).slice(0, 10))
  console.log(`  ${String(n).padStart(4)}  ${f}`);

if (JSON_OUT) {
  fs.writeFileSync(JSON_OUT, JSON.stringify(findings, null, 1));
  console.log(`\nfull detail → ${JSON_OUT}`);
}
process.exit(findings.length ? 1 : 0);
