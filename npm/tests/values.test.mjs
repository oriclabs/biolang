import assert from "node:assert/strict";
import test from "node:test";

import {
  BioKmerValue,
  BioMatrixValue,
  BioQualityValue,
  BioRangeValue,
  BioSequenceValue,
  BioTableValue,
  decodeBioValue,
  encodeBioValue,
} from "../values.js";

test("value codec preserves precise integers and ordinary JavaScript structures", () => {
  const source = {
    count: 9_007_199_254_740_993n,
    groups: new Set(["case", "control"]),
    metadata: new Map([["source", "study-a"]]),
  };
  const decoded = decodeBioValue(encodeBioValue(source));
  assert.equal(decoded.count, 9_007_199_254_740_993n);
  assert.deepEqual([...decoded.groups], ["case", "control"]);
  assert.deepEqual([...decoded.metadata], [["source", "study-a"]]);
});

test("typed biological wrappers round-trip without losing their shape", () => {
  const values = [
    new BioTableValue(["gene", "score"], [["TP53", 1.5], ["EGFR", 2.5]]),
    new BioMatrixValue({ nrow: 2, ncol: 2, data: new Float64Array([1, 2, 3, 4]) }),
    new BioSequenceValue("dna", "ACGT"),
    new BioQualityValue(new Uint8Array([30, 31, 32])),
    new BioKmerValue("ACGTTGC"),
  ];
  const decoded = decodeBioValue(encodeBioValue(values));
  assert.deepEqual(decoded[0].toRows(), [
    { gene: "TP53", score: 1.5 },
    { gene: "EGFR", score: 2.5 },
  ]);
  assert.deepEqual(decoded[1].shape, [2, 2]);
  assert.deepEqual([...decoded[1].row(1)], [3, 4]);
  assert.equal(decoded[2].toString(), "ACGT");
  assert.deepEqual([...decoded[3].data], [30, 31, 32]);
  assert.equal(decoded[4].toString(), "ACGTTGC");

  const range = decodeBioValue(encodeBioValue(new BioRangeValue(1n, 10n, true)));
  assert.equal(range.start, 1n);
  assert.equal(range.end, 10n);
  assert.equal(range.inclusive, true);
});

test("the JavaScript value codec rejects non-finite numbers", () => {
  for (const value of [Number.NaN, Number.POSITIVE_INFINITY, Number.NEGATIVE_INFINITY]) {
    assert.throws(() => encodeBioValue(value), /BioLang Float values must be finite/);
    assert.throws(() => encodeBioValue([1, value]), /BioLang Float values must be finite/);
    assert.throws(
      () => encodeBioValue(new Float64Array([1, value])),
      /BioLang Float values must be finite/,
    );
  }
  assert.throws(
    () => encodeBioValue(new BioMatrixValue({
      nrow: 1, ncol: 2, data: new Float64Array([1, Number.NaN]),
    })),
    /matrix data containing a non-finite JavaScript number/,
  );
});

test("record protocol tags cannot collide with user fields", () => {
  const record = { __biolangType: "matrix", value: 42 };
  const decoded = decodeBioValue(encodeBioValue(record));
  assert.equal(decoded.__biolangType, "matrix");
  assert.equal(decoded.value, 42);
  assert.throws(
    () => decodeBioValue({ __biolangType: "future-type" }),
    /Unsupported BioLang value tag/,
  );
});

test("value codec refuses cycles and arbitrary class instances", () => {
  const cyclic = {};
  cyclic.self = cyclic;
  assert.throws(() => encodeBioValue(cyclic), /cyclic/);
  assert.throws(() => encodeBioValue(new Date()), /Date/);
  assert.throws(() => encodeBioValue(new Uint8Array([30, 31])), /BioQualityValue/);
  assert.deepEqual(encodeBioValue(new Float64Array([1, 2])), [1, 2]);
});
