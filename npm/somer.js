import { sourceOf } from "./dsl.js";

export class SomerRun {
  constructor(client, job) {
    this.client = client;
    this.job = job;
  }

  get id() { return this.job.id; }
  get status() { return this.job.status; }

  async refresh() {
    this.job = await this.client.getJob(this.id);
    return this.job;
  }

  async wait(options = {}) {
    this.job = await this.client.waitForJob(
      this.id,
      options.onUpdate,
      options.signal,
      options.intervalMs,
    );
    return this.job;
  }

  events(onEvent, signal) {
    return this.client.streamEvents(this.id, onEvent, signal);
  }

  cancel() {
    return this.client.cancelJob(this.id);
  }

  retry() {
    return this.client.retryJob(this.id);
  }

  artifacts() {
    return this.client.artifacts(this.id);
  }

  download(artifact) {
    return this.client.downloadArtifact(this.id, artifact);
  }
}

export class SomerExecutor {
  constructor(client) {
    this.client = client;
  }

  serviceInfo() { return this.client.serviceInfo(); }
  resourceProfiles() { return this.client.resourceProfiles(); }

  async run(source, options = {}) {
    const inputs = (options.inputs ?? []).map(normalizeInput);
    let job = await this.client.submitJob({
      executor: "biolang",
      name: options.name ?? "BioLang JavaScript job",
      entrypoint: options.entrypoint ?? "main.bl",
      source: sourceOf(source),
      sourceFiles: options.sourceFiles,
      inputFiles: inputs.map(({ path, size, sha256 }) => ({
        path,
        size,
        ...(sha256 ? { sha256 } : {}),
      })),
      environment: options.environment,
      priority: options.priority,
      tags: options.tags,
      dependsOn: options.dependsOn,
      retryPolicy: options.retryPolicy,
      retentionDays: options.retentionDays,
      runtimeVersion: options.runtimeVersion,
      resources: options.resources,
    });
    if (inputs.length) {
      const chunkSize = options.chunkSize ?? 4 * 1024 * 1024;
      if (!Number.isSafeInteger(chunkSize) || chunkSize < 1) {
        throw new TypeError("Somer chunkSize must be a positive safe integer");
      }
      for (const input of inputs) {
        for (let offset = 0; offset < input.size; offset += chunkSize) {
          const body = input.data.slice(offset, Math.min(input.size, offset + chunkSize));
          const bodySize = typeof body.size === "number" ? body.size : body.byteLength;
          await this.client.uploadInput(job.id, input.path, body, {
            offset,
            chunkSize: bodySize,
            totalSize: input.size,
          });
        }
      }
      job = await this.client.finalizeInputs(job.id);
    }
    return new SomerRun(this.client, job);
  }
}

function normalizeInput(input) {
  if (!input || typeof input.path !== "string" || !input.path) {
    throw new TypeError("Each Somer input requires a non-empty path");
  }
  const data = input.data;
  let normalized;
  if (typeof data === "string") {
    normalized = new TextEncoder().encode(data);
  } else if (data instanceof Uint8Array) {
    normalized = data;
  } else if (data instanceof ArrayBuffer) {
    normalized = new Uint8Array(data);
  } else if (ArrayBuffer.isView(data)) {
    normalized = new Uint8Array(data.buffer, data.byteOffset, data.byteLength);
  } else if (typeof Blob !== "undefined" && data instanceof Blob) {
    normalized = data;
  } else {
    throw new TypeError(
      `Somer input '${input.path}' data must be a string, Blob/File, ArrayBuffer, or typed array`,
    );
  }
  const size = typeof normalized.size === "number" ? normalized.size : normalized.byteLength;
  if (size === 0) {
    throw new TypeError(`Somer input '${input.path}' cannot be empty`);
  }
  return { path: input.path, data: normalized, size, sha256: input.sha256 };
}
