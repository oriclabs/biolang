import assert from "node:assert/strict";
import test from "node:test";

import {
  BioLang,
  BioMatrixValue,
  BioQualityValue,
  BioValueHandle,
} from "../index.js";

test("direct values cross the JavaScript and BioLang boundary", async (context) => {
  const bl = await BioLang.create({ network: false });
  context.after(() => bl.dispose());

  const record = bl.evalValue('{gene: "TP53", scores: [1, 2, 3]}');
  assert.equal(record.gene, "TP53");
  assert.deepEqual(record.scores, [1, 2, 3]);
  assert.equal(bl.evalValue("9007199254740993"), 9_007_199_254_740_993n);
  assert.throws(
    () => bl.setValue("rounded", 9_007_199_254_740_992),
    /pass a bigint/,
  );
  assert.equal(bl.callValue("mean", [[1, 2, 3]]), 2);

  bl.setValue("clinical", { age: 52, cohort: "case" });
  assert.equal(bl.evalValue("clinical.age"), 52);
  assert.equal(bl.getValue("clinical").cohort, "case");
  bl.setValue("tagged_record", { __biolangType: "user-data", value: 42 });
  assert.equal(bl.getValue("tagged_record").__biolangType, "user-data");

  bl.setValue("not_a_number", Number.NaN);
  assert.equal(Number.isNaN(bl.getValue("not_a_number")), true);
  bl.setValue("quality", new BioQualityValue(new Uint8Array([30, 31, 32])));
  assert.deepEqual([...bl.getValue("quality").data], [30, 31, 32]);

  const matrix = new BioMatrixValue({
    nrow: 2, ncol: 2, data: new Float64Array([1, 2, 3, 4]),
  });
  bl.setValue("measurements", matrix);
  assert.deepEqual([...bl.getValue("measurements").data], [1, 2, 3, 4]);
});

test("typed interop throws Error objects and builtins are not shadowed", async (context) => {
  const bl = await BioLang.create({ network: false });
  context.after(() => bl.dispose());

  for (const operation of [
    () => bl.getValue("missing"),
    () => bl.evalValue("let ="),
  ]) {
    assert.throws(operation, (error) => {
      assert.ok(error instanceof Error);
      assert.equal(typeof error.message, "string");
      assert.ok(error.stack);
      return true;
    });
  }

  assert.equal(bl.table([{ x: 1 }]).ok, true);
  assert.equal(bl.matrix([[1, 2], [3, 4]]).ok, true);
  assert.equal(JSON.parse(bl.format("value={}", 7).value), "value=7");
  assert.equal(bl.csv("definitely-missing.csv").ok, false);
  assert.equal(typeof bl.formatSource("let   x=1"), "string");
});

test("large values stay in Rust and handles remain session-bound", async (context) => {
  const first = await BioLang.create({ network: false });
  const second = await BioLang.create({ network: false });
  context.after(() => { first.dispose(); second.dispose(); });

  const handle = first.evalValue("[1, 2, 3, 4]", { maximumInlineBytes: 1 });
  assert.ok(handle instanceof BioValueHandle);
  assert.deepEqual(handle.page({ offset: 1, limit: 2 }), [2, 3]);
  assert.deepEqual([...handle.toFloat64Array()], [1, 2, 3, 4]);
  assert.equal(first.callValue("mean", [handle]), 2.5);
  assert.throws(() => second.callValue("mean", [handle]), /another session/);

  assert.equal(handle.dispose(), true);
  assert.throws(() => handle.page(), /disposed/);

  const stale = first.evalValue("[5, 6, 7]", { maximumInlineBytes: 1 });
  first.reset();
  assert.throws(() => stale.page(), (error) => {
    assert.ok(error instanceof Error);
    assert.match(error.message, /stale after reset/);
    return true;
  });
});

test("synchronous callbacks are isolated, validated, and callable by BioLang", async (context) => {
  const first = await BioLang.create({ network: false });
  const second = await BioLang.create({ network: false });
  context.after(() => { first.dispose(); second.dispose(); });

  const scale = first.registerFunction(
    "js_scale",
    { parameters: ["Number"], returns: "Number" },
    (value) => value * 2,
  );
  second.registerFunction("js_scale", (value) => value * 3);
  assert.equal(first.evalValue("js_scale(7)"), 14);
  assert.equal(second.evalValue("js_scale(7)"), 21);
  assert.throws(() => first.evalValue('js_scale("seven")'), /argument 1 must be Number/);
  assert.deepEqual(first.evalValue("map([1, 2, 3], js_scale)"), [2, 4, 6]);
  assert.deepEqual(first.callValue("map", [[1, 2, 3], scale]), [2, 4, 6]);

  first.registerFunction("js_cross_session", (value) => {
    assert.equal(second.evalValue("js_scale(1)"), 3);
    return value + 1;
  });
  assert.deepEqual(first.evalValue("map([1, 2, 3], js_cross_session)"), [2, 3, 4]);

  first.registerFunction("js_async", async (value) => value);
  assert.throws(() => first.evalValue("js_async(1)"), (error) => {
    assert.match(error.message, /must be synchronous/);
    assert.equal((error.message.match(/JavaScript host callback 'js_async'/g) ?? []).length, 1);
    return true;
  });
  first.registerFunction("js_reenter", () => first.evalValue("1"));
  assert.throws(() => first.evalValue("js_reenter()"), /cannot re-enter/);
  assert.throws(
    () => first.registerFunction("bad_type", { returns: "Numbr" }, () => 1),
    /supported BioLang type/,
  );
  first.registerFunction("wrong_return", { returns: "String" }, () => 1);
  assert.throws(() => first.evalValue("wrong_return()"), /must return String/);
  first.evalValue("let existing = 7");
  assert.throws(
    () => first.registerFunction("existing", () => 8),
    /cannot replace existing BioLang value/,
  );
});
