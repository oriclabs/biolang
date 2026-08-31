import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import test from "node:test";
import { fileURLToPath, pathToFileURL } from "node:url";

import { mean } from "../generated-builtins.js";
import { BioLangSession } from "../session.js";

const packageRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const wasmRoot = path.resolve(packageRoot, "..", "desktop", "public", "wasm");

test("generated JavaScript executes through the shipped WASM interpreter", async () => {
  globalThis.window = globalThis;
  globalThis.__blFiles = {};
  globalThis.__blFetch = { sync: () => "ERROR:offline integration test" };
  const wasm = await import(pathToFileURL(path.join(wasmRoot, "bl_wasm.js")).href);
  await wasm.default({ module_or_path: readFileSync(path.join(wasmRoot, "bl_wasm_bg.wasm")) });
  wasm.init();

  const session = new BioLangSession(wasm);
  assert.equal(session.runtimeVersion(), "1.5.0");
  const result = session.run(mean([1, 2, 3]));
  assert.equal(result.ok, true, result.error ?? "WASM execution failed");
  assert.equal(result.type, "Float");
  assert.equal(result.value, "2");

  const objectResult = session
    .table([{ Age: 20, BMI: 22 }, { Age: 15, BMI: 30 }, { Age: 40, BMI: 28 }])
    .where({ Age: { gte: 18 }, BMI: { lt: 25 } })
    .column("BMI")
    .mean()
    .run(session);
  assert.equal(objectResult.ok, true, objectResult.error ?? "Object API execution failed");
  assert.equal(objectResult.value, "22");
});
