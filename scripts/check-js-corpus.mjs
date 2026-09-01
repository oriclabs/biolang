import { execFileSync } from "node:child_process";
import { createRequire } from "node:module";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import vm from "node:vm";

const require = createRequire(import.meta.url);
const wasm = require("../npm/pkg-node/bl_wasm.js");

const tracked = execFileSync("git", ["ls-files", "-z", "*.bl"], {
  cwd: new URL("..", import.meta.url),
}).toString("utf8").split("\0").filter(Boolean);

let transpiled = 0;
let invalid = 0;
const refusals = new Map();
const refusalFiles = new Map();

for (const path of tracked) {
  const source = readFileSync(new URL(`../${path}`, import.meta.url), "utf8");
  const result = JSON.parse(wasm.transpile_javascript(source));
  if (!result.ok) {
    const reason = refusalName(result.error);
    refusals.set(reason, (refusals.get(reason) ?? 0) + 1);
    refusalFiles.set(reason, [...(refusalFiles.get(reason) ?? []), { path, error: result.error }]);
    continue;
  }
  transpiled += 1;
  try {
    new vm.Script(`(async function (bl, bio) {\n${result.source}\n})`, { filename: path });
  } catch (error) {
    invalid += 1;
    console.error(`${path}: ${error.message}`);
  }
}

const refused = tracked.length - transpiled;
console.log(`corpus files       ${tracked.length}`);
console.log(`transpiled         ${transpiled} (${Math.round(transpiled * 100 / tracked.length)}%)`);
console.log(`refused            ${refused}`);
console.log(`invalid JS emitted ${invalid}`);
for (const [name, count] of [...refusals].sort((left, right) => right[1] - left[1])) {
  console.log(`  ${name.padEnd(28)} ${count}`);
  for (const item of refusalFiles.get(name)) {
    const detail = item.error.replace(/\s+/g, " ").trim();
    console.log(`    ${item.path}: ${detail}`);
  }
}

if (refused > 0 || invalid > 0) {
  process.exitCode = 1;
} else {
  execFileSync(process.execPath, [fileURLToPath(new URL("check-js-equivalence.mjs", import.meta.url))], {
    cwd: new URL("..", import.meta.url),
    stdio: "inherit",
  });
}

function refusalName(message) {
  const unsupported = message.match(/does not yet support (?:expression|statement) ([A-Za-z]+)/);
  if (unsupported) return unsupported[1];
  if (message.includes("expected")) return "parse error";
  return message.split("\n", 1)[0].slice(0, 80);
}
