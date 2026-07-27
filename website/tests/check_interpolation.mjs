import fs from "node:fs";
import path from "node:path";

const root = path.resolve(process.argv[2] ?? "docs");
const files = [];

function collect(dir) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) collect(fullPath);
    else if (entry.isFile() && entry.name.endsWith(".html")) files.push(fullPath);
  }
}

function lineAt(text, offset) {
  return text.slice(0, offset).split(/\r?\n/).length;
}

function suspiciousStrings(code) {
  const findings = [];
  let index = 0;

  while (index < code.length) {
    if (code[index] === "#") {
      while (index < code.length && code[index] !== "\n") index += 1;
      continue;
    }
    if (code[index] !== '"') {
      index += 1;
      continue;
    }

    const start = index;
    index += 1;
    let content = "";
    while (index < code.length) {
      if (code[index] === "\\") {
        content += code.slice(index, index + 2);
        index += 2;
        continue;
      }
      if (code[index] === '"') {
        index += 1;
        break;
      }
      content += code[index++];
    }

    const isFormatString = start > 0 && code[start - 1] === "f";
    if (isFormatString) {
      while (index < code.length && code[index] !== "\n") index += 1;
      continue;
    }

    const lineStart = code.lastIndexOf("\n", start) + 1;
    const beforeLiteral = code.slice(lineStart, start);
    const isFormatTemplate = /\bformat\s*\(\s*$/.test(beforeLiteral);
    const hasPlaceholder = /#\{|\$\{|\{(?:[A-Za-z_'])[^{}\r\n]*\}/.test(content);
    if (!isFormatTemplate && hasPlaceholder) {
      findings.push({ offset: start, literal: `"${content}"` });
    }
  }

  return findings;
}

collect(root);
const failures = [];
const codePattern = /<code\b[^>]*class=["'][^"']*\blanguage-biolang\b[^"']*["'][^>]*>([\s\S]*?)<\/code>/gi;

for (const file of files) {
  const html = fs.readFileSync(file, "utf8");
  for (const match of html.matchAll(codePattern)) {
    for (const finding of suspiciousStrings(match[1])) {
      const offset = match.index + match[0].indexOf(match[1]) + finding.offset;
      failures.push({
        file: path.relative(process.cwd(), file),
        line: lineAt(html, offset),
        literal: finding.literal,
      });
    }
  }
}

if (failures.length > 0) {
  for (const failure of failures) {
    console.error(`${failure.file}:${failure.line}: non-format string contains interpolation: ${failure.literal}`);
  }
  process.exitCode = 1;
} else {
  console.log(`Interpolation audit passed for ${files.length} HTML files.`);
}
