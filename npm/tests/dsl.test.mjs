import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import {
  call,
  for_,
  function_,
  if_,
  invoke,
  lambda,
  lambdaExpr,
  let_,
  literal,
  literalPattern,
  matchArm,
  matchExpr,
  pipeline_,
  program,
  ref,
  return_,
  stringFormatted,
  stringInterp,
  stringText,
  stringValue,
  tryCatch,
  wildcardPattern,
} from "../dsl.js";
import { mean, read_csv, WASM_BUILTIN_NAMES } from "../generated-builtins.js";
import * as browser from "../browser.js";
import * as nodeRoot from "../index.js";
import { range as expressionRange } from "../expressions.js";

test("browser SDK version matches the package manifest", () => {
  const manifest = JSON.parse(readFileSync(new URL("../package.json", import.meta.url), "utf8"));
  assert.equal(browser.version, manifest.version);
});

test("package root does not mix structural builders with executable sessions", () => {
  assert.equal("range" in browser, false);
  assert.equal("mean" in browser, false);
  assert.equal("range" in nodeRoot, false);
  assert.equal("mean" in nodeRoot, false);
  assert.equal(expressionRange(1, 3).toBioLang(), "range(1, 3)");
});

test("direct builtins generate BioLang without executing JavaScript algorithms", () => {
  assert.equal(mean([1, 2, 3]).toBioLang(), "mean([1, 2, 3])");
  assert.equal(read_csv("data.csv").toBioLang(), 'read_csv("data.csv")');
  assert.equal(WASM_BUILTIN_NAMES.length, new Set(WASM_BUILTIN_NAMES).size);
  assert.ok(WASM_BUILTIN_NAMES.length > 800);
});

test("object expressions build readable pipelines", () => {
  const adults = read_csv("nhanes.csv").filter(
    lambda("row", (row) => row.Age.gte(18).and(row.BMI.lt(40))),
  );
  const analysis = adults.column("BMI").mean();
  assert.match(analysis.toBioLang(), /read_csv\("nhanes\.csv"\)/);
  assert.match(analysis.toBioLang(), /row\)\.Age >= 18/);
  assert.match(analysis.toBioLang(), /and/);
  assert.match(analysis.toBioLang(), /^mean\(col\(/);
});

test("literal encoding cannot inject BioLang source", () => {
  const hostile = '") |> write_csv("stolen.csv") #';
  assert.equal(
    call("print", hostile).toBioLang(),
    'print("\\\") |> write_csv(\\\"stolen.csv\\\") #")',
  );
  assert.equal(literal({ "not an identifier": hostile }).toBioLang(),
    '{"not an identifier": "\\\") |> write_csv(\\\"stolen.csv\\\") #"}');
});

test("lambda safety rejects JavaScript operators that silently discard expressions", () => {
  assert.throws(
    () => lambda("row", (row) => row.Age && row.BMI),
    /constructed but discarded/,
  );
  assert.throws(
    () => lambda("row", (row) => row.Age === 18),
    /must return a BioLang expression/,
  );
  assert.throws(
    () => lambda("row", (row) => row.Age >= 18),
    /cannot use JavaScript operators directly/,
  );
});

test("lambda field collisions give a precise escape hatch", () => {
  assert.throws(
    () => lambda("row", (row) => row.mean.eq(5)),
    /Column 'mean'.*\.field\("mean"\)/,
  );
  assert.throws(
    () => lambda("row", (row) => row.source.eq("study")),
    /Column 'source'.*\.field\("source"\)/,
  );
  assert.equal(lambda("row", (row) => row.field("mean").eq(5)).toBioLang(), "|row| ((row).mean == 5)");
  assert.equal(lambda("row", (row) => row.mean()).toBioLang(), "|row| mean(row)");
});

test("invoke treats string callees as validated function names", () => {
  assert.equal(invoke("mean", [[1, 2]]).toBioLang(), "mean([1, 2])");
  assert.equal(invoke(ref("mean"), [[1, 2]]).toBioLang(), "mean([1, 2])");
  assert.throws(() => invoke('mean\nprint', [[1, 2]]), /not a valid BioLang identifier/);
});

test("lambdaExpr preserves the structural frontend path", () => {
  assert.equal(lambdaExpr(["value"], ref("value").gte(2)).toBioLang(), "|value| (value >= 2)");
  assert.equal(lambdaExpr(["value"], true).toBioLang(), "|value| true");
});

test("interpolated strings preserve runtime formatting and literal braces", () => {
  const source = stringInterp([
    stringText("mean={"),
    stringFormatted(ref("mu"), ".2f"),
    stringText("}"),
    stringValue(ref("suffix")),
  ]).toBioLang();
  assert.equal(source, '("mean={") ++ (f"{mu:.2f}") ++ ("}") ++ (f"{suffix}")');
  assert.throws(() => stringFormatted(ref("mu"), "wat"), /format spec/);
});

test("try/catch is represented as BioLang rather than executed by JavaScript", () => {
  assert.equal(
    tryCatch([call("fail", "bad")], "err", [ref("err")]).toBioLang(),
    'try {\n  fail("bad")\n} catch err {\n  err\n}',
  );
});

test("match expressions keep patterns, guards, and bodies structural", () => {
  const source = matchExpr(ref("base"), [
    matchArm(literalPattern("A"), "adenine"),
    matchArm(wildcardPattern(), "other"),
  ]).toBioLang();
  assert.equal(source, 'match base {\n  "A" => "adenine",\n  _ => "other"\n}');
  assert.throws(() => literalPattern(ref("not_a_literal")), /requires nil, a boolean, a number, or a string/);
});

test("statement helpers cover ordinary program structure", () => {
  const source = program(
    function_("classify", ["x"], if_(ref("x").gte(10), return_("high"), return_("low"))),
    let_("labels", []),
    for_("value", [5, 15], call("push", ref("labels"), call("classify", ref("value")))),
    ref("labels"),
  ).toBioLang();
  assert.match(source, /^fn classify\(x\)/);
  assert.match(source, /if \(x >= 10\)/);
  assert.match(source, /for value in \[5, 15\]/);
  assert.match(
    pipeline_("qc", ["sample"], return_(call("len", ref("sample")))).toBioLang(),
    /^pipeline qc\(sample\)/,
  );
});
