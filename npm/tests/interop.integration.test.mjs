import assert from "node:assert/strict";
import test from "node:test";
import * as bio from "../dsl.js";

import {
  BioLang,
  BioKmerValue,
  BioMatrixValue,
  BioQualityValue,
  BioSequenceValue,
  BioTableValue,
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

  for (const value of [Number.NaN, Number.POSITIVE_INFINITY, Number.NEGATIVE_INFINITY]) {
    assert.throws(() => bl.setValue("nonfinite", value), /BioLang Float values must be finite/);
    assert.throws(() => bl.first([value]), /BioLang Float values must be finite/);
  }
  bl.setValue("quality", new BioQualityValue(new Uint8Array([30, 31, 32])));
  assert.deepEqual([...bl.getValue("quality").data], [30, 31, 32]);

  const matrix = new BioMatrixValue({
    nrow: 2, ncol: 2, data: new Float64Array([1, 2, 3, 4]),
  });
  bl.setValue("measurements", matrix);
  assert.deepEqual([...bl.getValue("measurements").data], [1, 2, 3, 4]);
  assert.equal(bl.mean(matrix.data), 2.5);
  assert.throws(
    () => bl.setValue("invalid_matrix", new BioMatrixValue({
      nrow: 1, ncol: 2, data: new Float64Array([1, Number.NaN]),
    })),
    /matrix data containing a non-finite JavaScript number/,
  );

  const people = bl.table([
    { age: 17, height: 160, group: "A" },
    { age: 31, height: 180, group: "B" },
    { age: 44, height: null, group: "A" },
  ]);
  bl.setValue("people", people);
  const adults = bl.filter(bl.getValue("people"), (row) => row.age >= 18);
  const completeAdults = bl.dropNull(adults, "height");
  assert.ok(adults instanceof BioTableValue);
  assert.ok(completeAdults instanceof BioTableValue);
  assert.deepEqual(completeAdults.toRows(), [{ age: 31, group: "B", height: 180 }]);
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

  assert.ok(bl.table([{ x: 1 }]) instanceof BioTableValue);
  assert.ok(bl.matrix([[1, 2], [3, 4]]) instanceof BioMatrixValue);
  assert.equal(bl.format("value={}", 7), "value=7");
  assert.throws(() => bl.csv("definitely-missing.csv"), Error);
  assert.equal(typeof bl.formatSource("let   x=1"), "string");

  assert.equal(bl.mean([1, 2, 3]), 2);
  const envelope = bl.invoke("mean", [1, 2, 3]);
  assert.equal(envelope.ok, true);
  assert.equal(envelope.value, "2");
  const sequence = bl.dna("ATGC");
  assert.ok(sequence instanceof BioSequenceValue);
  assert.equal(bl.gcContent(sequence), 0.5);
  assert.equal(bl.dna("ATCGNNRYSWKM").data, "ATCGNNRYSWKM");
  assert.equal(bl.toString(bl.protein("MKT")), "MKT");
  assert.throws(
    () => bl.gcContnt(sequence),
    (error) => error instanceof TypeError && /Did you mean 'gcContent'/.test(error.message),
  );
});

test("transpiled comments and nested direct calls remain executable", async (context) => {
  const bl = await BioLang.create({ network: false });
  context.after(() => bl.dispose());
  const generated = bl.transpileJavaScript(`
# Explain the IUPAC rule
println(iupac_match(dna"AAATTC", "RAATTC")) # true: R matches A or G
println(reverse_complement(dna"AAATTC")) # typed nested result stays inside one BioLang expression
  `);
  assert.match(generated, /\/\/ Explain the IUPAC rule/);
  assert.match(generated, /\/\/ true: R matches A or G/);
  assert.match(generated, /bl\.println\(bl\.iupacMatch/);
  assert.match(generated, /bl\.println\(bl\.reverseComplement/);
  const executable = generated.replace(
    /\nresult;\s*(?:\/\/[^\n]*)?\s*$/,
    "\nreturn result;",
  );
  const result = await new Function(
    "bio", "bl", `return (async () => { ${executable} })()`,
  )(bio, bl);
  assert.equal(result, null);
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

test("session-bound handles forward fields and dispose after callback pipelines", async () => {
  const bl = await BioLang.create({ network: false });
  const record = bl.evalValue(
    '{name: "GRCh38", chromosomes: ["chr1", "chr2"]}',
    { maximumInlineBytes: 1 },
  );
  assert.ok(record instanceof BioValueHandle);
  assert.equal(record.name, "GRCh38");
  assert.deepEqual(record.chromosomes, ["chr1", "chr2"]);
  assert.equal(record.field("name"), "GRCh38");
  assert.deepEqual(bl.map(record.chromosomes, (chromosome) => chromosome + "!"), ["chr1!", "chr2!"]);
  const genome = bl.genome("GRCh38");
  assert.ok(genome instanceof BioValueHandle);
  assert.equal(genome.name, "GRCh38");
  assert.equal(genome.chromosomes.length, 24);
  assert.doesNotThrow(() => bl.dispose());
});

test("minimizer records preserve typed k-mers without retaining the whole list", async (context) => {
  const bl = await BioLang.create({ network: false });
  context.after(() => bl.dispose());
  const minimizers = bl.minimizers(bl.dna("ATCGATCGATCGATCGATCGATCG"), 7, 11);
  assert.ok(Array.isArray(minimizers));
  assert.ok(minimizers.length > 0);
  assert.ok(minimizers[0].kmer instanceof BioKmerValue);
  assert.equal(bl.kmerDecode(minimizers[0].kmer), minimizers[0].kmer.data);
  assert.deepEqual(
    bl.map(minimizers, (entry) => entry.kmer.toString()),
    minimizers.map((entry) => entry.kmer.data),
  );
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
  assert.deepEqual(first.map([1, 2, 3], (value) => value + 10), [11, 12, 13]);
  assert.deepEqual(first.filter([1, 2, 3, 4], (value) => value % 2 === 0), [2, 4]);
  const typedSequences = [first.dna("ATGC"), first.rna("AUGC"), first.dna("NNNN")];
  assert.deepEqual(
    first.filter(typedSequences, first.isDna).map((sequence) => sequence.data),
    ["ATGC", "NNNN"],
  );
  assert.deepEqual(first.sort_by([3, 1, 2], (left, right) => left - right), [1, 2, 3]);
  assert.deepEqual(
    first.map([1, 2], (value) => first.filter([1, 2, 3], (item) => item >= value).length),
    [3, 2],
  );

  first.evalValue("fn shifted(value, amount = 1) { value + amount }");
  assert.equal(first.callNamed("shifted", [10], { amount: 5 }), 15);

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

test("dispose is safe when requested from inside a synchronous host callback", async () => {
  const bl = await BioLang.create({ network: false });
  bl.registerFunction("js_dispose", () => {
    bl.dispose();
    return 1;
  });
  assert.equal(bl.evalValue("js_dispose()"), 1);
  await Promise.resolve();
});

test("exit and die stop a WASM run without trapping or poisoning disposal", async () => {
  for (const [source, message] of [
    ["exit()", /cannot terminate its host process/],
    ["exit(7)", /exit\(7\) requested/],
    ['die("invalid teaching data")', /invalid teaching data/],
  ]) {
    const bl = await BioLang.create({ network: false });
    const result = bl.run(source);
    assert.equal(result.ok, false);
    assert.match(result.error, message);
    assert.equal(bl.evalValue("1 + 1"), 2);
    assert.doesNotThrow(() => bl.dispose());
    await Promise.resolve();
  }
});
