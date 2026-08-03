/**
 * Static check: does every builtin call in the docs match the real signature?
 *
 * Reads `bl metadata --format json` for the authoritative parameter lists, then
 * scans every code block on the site and flags calls whose argument count
 * cannot satisfy the builtin's arity. This catches the "docs drifted from the
 * API" class without executing anything, so it is fast and has no data or
 * network dependencies.
 *
 * Arity is derived from the metadata parameter list: a trailing `?` marks an
 * optional parameter, so `kmer_count(seq, k, top_n?)` accepts 2..3.
 *
 * Usage:
 *   bl metadata --format json > meta.json
 *   node tests/check_call_signatures.mjs meta.json [--dir docs] [--json out.json]
 *   node tests/check_call_signatures.mjs meta.json --source --dir ../books
 */
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const argv = process.argv.slice(2);
const META = argv[0];
const pick = (f) => (argv.indexOf(f) >= 0 ? argv[argv.indexOf(f) + 1] : null);
const ONLY = pick("--dir");
const JSON_OUT = pick("--json");
const UNKNOWN_OUT = pick("--unknown-json");
const ALL_CODE = argv.includes("--all-code");
const SOURCE = argv.includes("--source");

if (!META || !fs.existsSync(META)) {
  console.error("usage: node check_call_signatures.mjs <metadata.json> [--dir d]");
  process.exit(2);
}

// ── authoritative arities ───────────────────────────────────────────────────
const meta = JSON.parse(fs.readFileSync(META, "utf8").replace(/^\uFEFF/, ""));
const arity = new Map(); // name -> {min, max, sig}
for (const b of meta.builtins || []) {
  if (!b.name || !Array.isArray(b.parameters)) continue;
  const ps = b.parameters;
  const declared = b.arity;
  // Structured runtime arity is authoritative. Parameter text is curated
  // documentation and can lag an overloaded implementation.
  const optional = ps.filter((p) => /\?$|=|\.\.\./.test(p)).length;
  const variadic = ps.some((p) => /\.\.\./.test(p));
  arity.set(b.name, {
    min: Number.isInteger(declared?.minimum) ? declared.minimum : ps.length - optional,
    max:
      declared?.kind === "atLeast"
        ? Infinity
        : Number.isInteger(declared?.maximum)
          ? declared.maximum
          : variadic
            ? Infinity
            : ps.length,
    sig: b.signature || `${b.name}(${ps.join(", ")})`,
  });
}

// ── code blocks ─────────────────────────────────────────────────────────────
const E = { "&lt;": "<", "&gt;": ">", "&amp;": "&", "&quot;": '"', "&#39;": "'", "&nbsp;": " " };
const decode = (s) =>
  s
    .replace(/&(?:lt|gt|amp|quot|nbsp);|&#39;/g, (m) => E[m] ?? m)
    .replace(/&#(\d+);/g, (_, d) => String.fromCharCode(+d));

function blocks(html) {
  const out = [];
  const languages = ALL_CODE ? "(?:bio|biolang|biorun|text)" : "(?:bio|biolang|biorun)";
  const re = new RegExp(
    `<code[^>]*class="[^"]*\\blanguage-${languages}\\b[^"]*"[^>]*>([\\s\\S]*?)<\\/code>`,
    "g"
  );
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

function sourceFiles(dir) {
  const out = [];
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) {
      if (e.name === "node_modules" || e.name === "target") continue;
      out.push(...sourceFiles(p));
    } else if (e.name.endsWith(".bl")) {
      out.push(p);
    }
  }
  return out;
}

/** Replace comments with spaces while preserving strings, newlines, and offsets. */
function maskComments(src) {
  let out = "";
  let inString = false;
  let escaped = false;
  let inComment = false;
  for (const c of src) {
    if (inComment) {
      if (c === "\n") {
        inComment = false;
        out += c;
      } else {
        out += " ";
      }
      continue;
    }
    if (inString) {
      out += c;
      if (escaped) escaped = false;
      else if (c === "\\") escaped = true;
      else if (c === '"') inString = false;
      continue;
    }
    if (c === '"') {
      inString = true;
      out += c;
    } else if (c === "#") {
      inComment = true;
      out += " ";
    } else {
      out += c;
    }
  }
  return out;
}

/** True if this call sits directly after a `|>`, which supplies one implicit
 *  leading argument. */
function isPiped(src, nameStart) {
  const before = src.slice(0, nameStart).replace(/\s+$/, "");
  return before.endsWith("|>");
}

/** Split a call's argument text at top-level commas. */
function countArgs(src, open) {
  let depth = 0, n = 0, seen = false, inLambdaParams = false, i = open;
  for (; i < src.length; i++) {
    const c = src[i];
    if (c === '"') {
      i++;
      while (i < src.length && src[i] !== '"') i += src[i] === "\\" ? 2 : 1;
      seen = true;
      continue;
    }
    if (
      c === "|" &&
      src[i + 1] !== ">" &&
      src[i + 1] !== "|" &&
      src[i - 1] !== "|"
    ) {
      inLambdaParams = !inLambdaParams;
      seen = true;
      continue;
    }
    if ("([{".includes(c)) { depth++; if (depth === 1) continue; }
    else if (")]}".includes(c)) {
      depth--;
      if (depth === 0) return seen ? n + 1 : 0;
      continue;
    }
    if (depth === 1 && c === "," && !inLambdaParams) { n++; continue; }
    if (depth >= 1 && !/\s/.test(c)) seen = true;
  }
  return -1; // unbalanced
}

const roots = ONLY ? [path.join(ROOT, ONLY)] : [path.join(ROOT, "books"), path.join(ROOT, "docs")];
const findings = [];
const unknownCalls = [];
let calls = 0;
const syntaxCalls = new Set(["if", "for", "while", "match", "catch", "fn", "in"]);

for (const root of roots) {
  if (!fs.existsSync(root)) continue;
  for (const file of SOURCE ? sourceFiles(root) : htmlFiles(root)) {
    const contents = fs.readFileSync(file, "utf8");
    const pageBlocks = SOURCE ? [contents] : blocks(contents);
    const pageDefined = new Set();
    for (const code of pageBlocks) {
      for (const match of code.matchAll(/\b(?:fn|pipeline)\s+([a-z_][a-z0-9_]*)\s*\(/g)) {
        pageDefined.add(match[1]);
      }
    }
    for (const code of pageBlocks) {
      // Mask comments so `# kmer_encode(sequence, k)` prose is not treated as
      // a call, while preserving offsets and interpolation markers in strings.
      const src = maskComments(code);
      const scanSource = src.replace(
        /"(?:[^"\\]|\\.)*"/g,
        (literal) => " ".repeat(literal.length)
      );
      const locallyDefined = new Set([
        ...pageDefined,
        [...scanSource.matchAll(/\bfn\s+([a-z_][a-z0-9_]*)\s*\(/g)].map((match) => match[1])
      ].flat());
      for (const match of scanSource.matchAll(/\blet\s+([a-z_][a-z0-9_]*)\s*=\s*\|/g)) {
        locallyDefined.add(match[1]);
      }
      const re = /(^|[^A-Za-z0-9_.])([a-z_][a-z0-9_]*)[ \t]*\(/g;
      let m;
      while ((m = re.exec(scanSource)) !== null) {
        const name = m[2];
        const spec = arity.get(name);
        // `assert condition, message` is statement syntax. Parentheses around
        // its condition are grouping, not a call to the registry entry.
        if (!spec) {
          if (!locallyDefined.has(name) && !syntaxCalls.has(name)) {
            const line = (src.slice(0, m.index).match(/\n/g) || []).length + 1;
            unknownCalls.push({
              file: path.relative(ROOT, file),
              name,
              line,
              sourceLine: code.split("\n")[line - 1]?.trim() || "",
            });
          }
          continue;
        }
        if (name === "assert" || locallyDefined.has(name)) continue;
        const open = m.index + m[0].length - 1;
        const n = countArgs(src, open);
        if (n < 0) continue;
        calls++;
        // A piped call receives one extra implicit argument: `x |> f(a)`
        const piped = isPiped(scanSource, m.index + m[1].length);
        const eff = piped ? n + 1 : n;
        if (eff < spec.min || eff > spec.max) {
          const line = (src.slice(0, m.index).match(/\n/g) || []).length + 1;
          findings.push({
            file: path.relative(ROOT, file),
            name,
            given: n,
            piped,
            effective: eff,
            expected: spec.max === Infinity ? `${spec.min}+` : spec.min === spec.max ? `${spec.min}` : `${spec.min}-${spec.max}`,
            sig: spec.sig,
            line,
            sourceLine: code.split("\n")[line - 1]?.trim() || "",
            call: src.slice(m.index + m[1].length, open + 1).trim(),
          });
        }
      }
    }
  }
}

console.log(`builtins with a known arity : ${arity.size}`);
console.log(`calls checked               : ${calls}`);
console.log(`arity mismatches            : ${findings.length}`);

const byName = new Map();
for (const f of findings) {
  if (!byName.has(f.name)) byName.set(f.name, []);
  byName.get(f.name).push(f);
}
console.log("\nby builtin:");
for (const [n, v] of [...byName].sort((a, b) => b[1].length - a[1].length).slice(0, 25)) {
  const ex = v[0];
  console.log(
    `  ${String(v.length).padStart(3)}  ${n}  — got ${ex.effective}, expects ${ex.expected}`
  );
  console.log(`         ${ex.sig}`);
  console.log(`         e.g. ${ex.file}`);
}

if (JSON_OUT) {
  fs.writeFileSync(JSON_OUT, JSON.stringify(findings, null, 1));
  console.log(`\nfull detail → ${JSON_OUT}`);
}

if (UNKNOWN_OUT) {
  fs.writeFileSync(UNKNOWN_OUT, JSON.stringify(unknownCalls, null, 2));
  console.log(`unknown call candidates      : ${unknownCalls.length}`);
  console.log(`unknown detail -> ${UNKNOWN_OUT}`);
}
process.exit(findings.length ? 1 : 0);
