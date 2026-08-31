import assert from "node:assert/strict";
import test from "node:test";

import { mean } from "../generated-builtins.js";
import { BioLangSession } from "../session.js";

function fakeWasm() {
  const calls = [];
  return {
    calls,
    evaluate(source) {
      calls.push(["evaluate", source]);
      return JSON.stringify({ ok: true, value: "2", type: "Float" });
    },
    reset() { calls.push(["reset"]); },
    list_builtins() { return JSON.stringify([{ name: "mean", arity: "Exact(1)" }]); },
    list_variables() { return "[]"; },
    inspect_variable(name, offset, limit) {
      calls.push(["inspect", name, offset, limit]);
      return JSON.stringify({ ok: true, page: { name } });
    },
    export_variable(name, format, maximumBytes) {
      calls.push(["export", name, format, maximumBytes]);
      return new Uint8Array([1, 2, 3]);
    },
    register_module(path, source) { calls.push(["module", path, source]); },
    runtime_version() { return "1.5.0"; },
    format(source) { return source; },
    tokenize() { return "[]"; },
    import_source() { return JSON.stringify({ ok: true }); },
    validate_import(source, notebook) {
      calls.push(["validate", source, notebook]);
      return JSON.stringify({ ok: true });
    },
    qc_metrics(kind, text) {
      calls.push(["qc", kind, text]);
      return JSON.stringify({ kind, records: 1 });
    },
  };
}

test("session executes generated expressions through WASM", () => {
  const wasm = fakeWasm();
  const session = new BioLangSession(wasm);
  const result = session.run(mean([1, 2, 3]));
  assert.equal(result.ok, true);
  assert.equal(result.value, "2");
  assert.deepEqual(wasm.calls[0], ["evaluate", "mean([1, 2, 3])"]);
  assert.equal(session.supports("mean"), true);
  assert.equal(session.runtimeVersion(), "1.5.0");
});

test("session creates typed lazy JavaScript data objects", () => {
  const wasm = fakeWasm();
  const session = new BioLangSession(wasm);
  const analysis = session
    .csv("nhanes.csv")
    .where({ Age: { gte: 18 }, BMI: { lt: 40 } })
    .column("BMI")
    .mean();
  assert.match(analysis.toBioLang(), /^mean\(col\(filter\(read_csv/);
  assert.match(analysis.toBioLang(), /Age >= 18/);
  analysis.run(session);
  assert.equal(wasm.calls[0][0], "evaluate");
});

test("session exposes inspect, export, and in-memory modules", () => {
  const wasm = fakeWasm();
  const session = new BioLangSession(wasm);
  session.inspectVariable("table", { offset: 20, limit: 10 });
  session.exportVariable("table", { format: "csv", maximumBytes: 1024 });
  session.registerModule("example", "export let answer = 42");
  assert.deepEqual(wasm.calls, [
    ["inspect", "table", 20, 10],
    ["export", "table", "csv", 1024],
    ["module", "example", "export let answer = 42"],
  ]);
});

test("session exposes import validation and QC utilities", () => {
  const wasm = fakeWasm();
  const session = new BioLangSession(wasm);
  assert.deepEqual(session.validateImport("let x = 1", { notebook: true }), { ok: true });
  assert.deepEqual(session.qcMetrics("fastq", "@read\nAC\n+\nII"), {
    kind: "fastq",
    records: 1,
  });
  assert.deepEqual(wasm.calls, [
    ["validate", "let x = 1", true],
    ["qc", "fastq", "@read\nAC\n+\nII"],
  ]);
});
