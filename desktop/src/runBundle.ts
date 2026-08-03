import { buildOutputExport } from "./outputExport";
import type { Job } from "./types";

interface ZipEntry {
  name: string;
  content: string | Uint8Array;
}

function crc32(bytes: Uint8Array): number {
  let crc = 0xffffffff;
  for (const byte of bytes) {
    crc ^= byte;
    for (let bit = 0; bit < 8; bit += 1) {
      crc = (crc >>> 1) ^ (0xedb88320 & -(crc & 1));
    }
  }
  return (crc ^ 0xffffffff) >>> 0;
}

function write16(target: number[], value: number) {
  target.push(value & 0xff, (value >>> 8) & 0xff);
}

function write32(target: number[], value: number) {
  write16(target, value & 0xffff);
  write16(target, (value >>> 16) & 0xffff);
}

function dosDateTime(timestamp: number): { date: number; time: number } {
  const date = new Date(timestamp);
  return {
    date: ((Math.max(1980, date.getFullYear()) - 1980) << 9)
      | ((date.getMonth() + 1) << 5)
      | date.getDate(),
    time: (date.getHours() << 11) | (date.getMinutes() << 5) | Math.floor(date.getSeconds() / 2),
  };
}

export function createZip(entries: ZipEntry[], timestamp = Date.now()): Uint8Array {
  const encoder = new TextEncoder();
  const output: number[] = [];
  const central: number[] = [];
  const { date, time } = dosDateTime(timestamp);

  for (const entry of entries) {
    const name = encoder.encode(entry.name.replaceAll("\\", "/"));
    const content = typeof entry.content === "string" ? encoder.encode(entry.content) : entry.content;
    const checksum = crc32(content);
    const offset = output.length;

    write32(output, 0x04034b50);
    write16(output, 20);
    write16(output, 0x0800);
    write16(output, 0);
    write16(output, time);
    write16(output, date);
    write32(output, checksum);
    write32(output, content.length);
    write32(output, content.length);
    write16(output, name.length);
    write16(output, 0);
    output.push(...name, ...content);

    write32(central, 0x02014b50);
    write16(central, 20);
    write16(central, 20);
    write16(central, 0x0800);
    write16(central, 0);
    write16(central, time);
    write16(central, date);
    write32(central, checksum);
    write32(central, content.length);
    write32(central, content.length);
    write16(central, name.length);
    write16(central, 0);
    write16(central, 0);
    write16(central, 0);
    write16(central, 0);
    write32(central, 0);
    write32(central, offset);
    central.push(...name);
  }

  const centralOffset = output.length;
  output.push(...central);
  write32(output, 0x06054b50);
  write16(output, 0);
  write16(output, 0);
  write16(output, entries.length);
  write16(output, entries.length);
  write32(output, central.length);
  write32(output, centralOffset);
  write16(output, 0);
  return new Uint8Array(output);
}

function safeName(value: string): string {
  return value.replace(/[^A-Za-z0-9._-]+/g, "-").replace(/^-+|-+$/g, "") || "run";
}

export function buildRunBundle(
  job: Job,
  artifactContents: ReadonlyMap<string, Uint8Array> = new Map(),
): { name: string; bytes: Uint8Array } {
  const entries: ZipEntry[] = [
    {
      name: "README.txt",
      content: `BioLang run bundle
Run: ${job.displayName ?? job.file}
Status: ${job.status}
Backend: ${job.backend}
Started: ${new Date(job.startedAt).toISOString()}
Duration: ${job.durationMs == null ? "unknown" : `${job.durationMs} ms`}
`,
    },
    { name: "output.log", content: buildOutputExport(job.log, "log", job) },
    { name: "output.txt", content: buildOutputExport(job.log, "text", job) },
    { name: "job.json", content: `${JSON.stringify({ ...job, log: undefined }, null, 2)}\n` },
  ];
  if (job.provenance) {
    const checksumLines = [
      ...(job.provenance.inputs ?? []),
      ...(job.provenance.environmentFiles ?? []),
    ].filter((input) => input.sha256).map((input) => `${input.sha256}  ${input.path}`);
    entries.push({
      name: "provenance.json",
      content: `${JSON.stringify({ ...job.provenance, sourceSnapshot: undefined }, null, 2)}\n`,
    });
    entries.push({
      name: "environment.json",
      content: `${JSON.stringify({
        biolangVersion: job.provenance.biolangVersion,
        packages: job.provenance.packages,
        tools: job.provenance.tools,
        runtime: job.provenance.runtime,
        environmentFiles: job.provenance.environmentFiles,
      }, null, 2)}\n`,
    });
    entries.push({
      name: "checksums.sha256",
      content: checksumLines.length ? `${checksumLines.join("\n")}\n` : "# No input checksums were available\n",
    });
    entries.push({
      name: "REPRODUCE.txt",
      content: `1. Install BioLang ${job.provenance.biolangVersion ?? "recorded in environment.json"}.
2. Restore the package versions and environment files listed in environment.json.
3. Restore inputs and verify them with checksums.sha256.
4. Run: bl run ${job.provenance.entrypoint}
5. Compare generated results and artifacts with job.json and artifacts/manifest.json.
`,
    });
    if (job.provenance.sourceSnapshot) {
      entries.push({ name: "source.bl", content: job.provenance.sourceSnapshot });
    }
  }
  for (const [index, result] of (job.results ?? []).entries()) {
    entries.push({
      name: `results/result-${index + 1}.json`,
      content: `${JSON.stringify(result, null, 2)}\n`,
    });
    if (result.kind === "plot" && result.format === "svg" && typeof result.data === "string") {
      entries.push({ name: `plots/plot-${index + 1}.svg`, content: result.data });
    }
  }
  if (job.artifacts?.length) {
    entries.push({
      name: "artifacts/manifest.json",
      content: `${JSON.stringify(job.artifacts, null, 2)}\n`,
    });
    for (const artifact of job.artifacts) {
      const content = artifactContents.get(artifact.name);
      if (content) entries.push({ name: `artifacts/${artifact.name}`, content });
    }
  }
  return {
    name: `${safeName(job.displayName ?? job.file)}-${new Date(job.startedAt).toISOString().replaceAll(":", "-")}.zip`,
    bytes: createZip(entries, job.startedAt),
  };
}
