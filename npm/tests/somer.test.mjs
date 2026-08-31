import assert from "node:assert/strict";
import test from "node:test";

import { mean } from "../generated-builtins.js";
import { SomerExecutor } from "../somer.js";

test("Somer receives the same generated BioLang source", async () => {
  let submitted;
  const uploads = [];
  let finalized = false;
  const client = {
    async submitJob(request) {
      submitted = request;
      return { id: "job-1", status: "queued" };
    },
    async getJob() { return { id: "job-1", status: "succeeded" }; },
    async waitForJob() { return { id: "job-1", status: "succeeded", results: [] }; },
    async uploadInput(id, path, body, options) {
      uploads.push({ id, path, body: [...body], options });
    },
    async finalizeInputs() {
      finalized = true;
      return { id: "job-1", status: "queued" };
    },
  };
  const somer = new SomerExecutor(client);
  const run = await somer.run(mean([1, 2, 3]), {
    name: "Mean",
    resources: { cpu: 2, memoryGb: 4 },
    inputs: [{ path: "values.txt", data: "12345" }],
    chunkSize: 3,
  });
  assert.equal(run.id, "job-1");
  assert.equal(submitted.executor, "biolang");
  assert.equal(submitted.source, "mean([1, 2, 3])");
  assert.deepEqual(submitted.resources, { cpu: 2, memoryGb: 4 });
  assert.deepEqual(submitted.inputFiles, [{ path: "values.txt", size: 5 }]);
  assert.deepEqual(uploads.map(({ body }) => body), [[49, 50, 51], [52, 53]]);
  assert.equal(finalized, true);
});
