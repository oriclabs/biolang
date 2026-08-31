import assert from "node:assert/strict";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";

import { BioLang } from "../index.js";

test("create returns isolated interpreters and session-specific file bridges", async (context) => {
  const firstRoot = mkdtempSync(path.join(tmpdir(), "biolang-session-a-"));
  const secondRoot = mkdtempSync(path.join(tmpdir(), "biolang-session-b-"));
  context.after(() => {
    rmSync(firstRoot, { recursive: true, force: true });
    rmSync(secondRoot, { recursive: true, force: true });
  });
  writeFileSync(path.join(firstRoot, "values.csv"), "value\n1\n1\n");
  writeFileSync(path.join(secondRoot, "values.csv"), "value\n9\n9\n");

  const first = await BioLang.create({ cwd: firstRoot, network: false });
  const second = await BioLang.create({ cwd: secondRoot, network: false });

  assert.equal(first.run("let secret = 42").ok, true);
  assert.equal(second.run("secret").ok, false, "variables must not leak between sessions");
  second.reset();
  assert.equal(first.run("secret").value, "42", "reset must affect only its owning session");

  const firstMean = first.run('read_csv("values.csv") |> col("value") |> mean()');
  const secondMean = second.run('read_csv("values.csv") |> col("value") |> mean()');
  const firstAgain = first.run('read_csv("values.csv") |> col("value") |> mean()');
  assert.equal(firstMean.value, "1", firstMean.error ?? "first cwd read failed");
  assert.equal(secondMean.value, "9", secondMean.error ?? "second cwd read failed");
  assert.equal(firstAgain.value, "1", "the later session must not retarget the first bridge");

  first.dispose();
  assert.throws(() => first.run("1"), /disposed/);
  second.dispose();
});
