#!/usr/bin/env node
/** Guard the one-payload npm layout after scripts/build-npm.mjs runs. */
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const npmRoot = path.join(root, "npm");
const packageJson = JSON.parse(readFileSync(path.join(npmRoot, "package.json"), "utf8"));
const required = "wasm/bl_wasm_bg.wasm";
const forbidden = ["pkg-node/bl_wasm_bg.wasm", "pkg-web/bl_wasm_bg.wasm"];
const packageScopes = [
  ["pkg-node/package.json", "commonjs"],
  ["pkg-web/package.json", "module"],
];

if (!packageJson.files.includes(required)) {
  throw new Error(`npm files[] does not include ${required}`);
}
for (const item of forbidden) {
  if (packageJson.files.includes(item) || existsSync(path.join(npmRoot, item))) {
    throw new Error(`duplicate WASM payload remains at ${item}`);
  }
}
if (!existsSync(path.join(npmRoot, required))) {
  throw new Error(`shared WASM payload is missing at ${required}; run npm run build`);
}
for (const [item, expectedType] of packageScopes) {
  if (!packageJson.files.includes(item)) {
    throw new Error(`npm files[] does not include the required ${item} package scope`);
  }
  const location = path.join(npmRoot, item);
  if (!existsSync(location)) {
    throw new Error(`required ${item} package scope is missing; run npm run build`);
  }
  const scope = JSON.parse(readFileSync(location, "utf8"));
  if (scope.type !== expectedType) {
    throw new Error(`${item} must declare type=${expectedType}, found ${scope.type ?? "missing"}`);
  }
}
if (existsSync(path.join(npmRoot, "pkg-bundler"))) {
  throw new Error("orphan npm/pkg-bundler output must not be retained");
}

const nodeGlue = readFileSync(path.join(npmRoot, "pkg-node", "bl_wasm.js"), "utf8");
const webGlue = readFileSync(path.join(npmRoot, "pkg-web", "bl_wasm.js"), "utf8");
if (!nodeGlue.includes("../wasm/bl_wasm_bg.wasm")) {
  throw new Error("Node loader does not reference the shared WASM payload");
}
if (!webGlue.includes("../wasm/bl_wasm_bg.wasm")) {
  throw new Error("browser loader does not reference the shared WASM payload");
}

console.log("npm package layout contains one shared WASM payload and explicit Node/browser module scopes.");
