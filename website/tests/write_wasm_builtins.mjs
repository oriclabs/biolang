import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const wasmDir = path.join(root, "wasm");
const bytes = fs.readFileSync(path.join(wasmDir, "bl_wasm_bg.wasm"));

globalThis.window = globalThis;
globalThis.__blFiles = {};
globalThis.__blFetch = { sync: () => "ERROR:404 offline catalog generation" };

const wasm = await import("../wasm/bl_wasm.js");
await wasm.default({ module_or_path: bytes });
wasm.init();

const builtins = JSON.parse(wasm.list_builtins())
  .map((entry) => entry.name)
  .filter(Boolean)
  .sort();

const dataRoot = path.join(root, "books", "data");
const dataFiles = fs.existsSync(dataRoot)
  ? fs.readdirSync(dataRoot, { withFileTypes: true })
      .filter((entry) => entry.isFile())
      .map((entry) => entry.name)
      .sort()
  : [];

fs.writeFileSync(
  path.join(wasmDir, "builtins.json"),
  `${JSON.stringify({ generated: true, builtins, dataFiles }, null, 2)}\n`,
);
console.log(
  `Wrote ${builtins.length} browser builtins and ${dataFiles.length} data files to wasm/builtins.json`,
);
