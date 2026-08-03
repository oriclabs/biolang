import type { Job, JobLogChunk, JobLogStream } from "./types";

const streams = new Set<JobLogStream>(["stdout", "stderr", "system", "success"]);
const cliProgressLine = /^\s*(?:▶\s+running\b|✓\s+done in\b).*$/u;

export function stripCliProgress(text: string): string {
  return text
    .split(/(?<=\n)/)
    .filter((line) => !cliProgressLine.test(line.trimEnd()))
    .join("");
}

export function normalizeJobLog(value: unknown): JobLogChunk[] {
  if (typeof value === "string") {
    return value ? [{ stream: "stdout", text: value }] : [];
  }
  if (!Array.isArray(value)) return [];
  return value.flatMap((chunk): JobLogChunk[] => {
    if (
      typeof chunk !== "object"
      || chunk == null
      || !("stream" in chunk)
      || !("text" in chunk)
      || typeof chunk.text !== "string"
      || !streams.has(chunk.stream as JobLogStream)
    ) {
      return [];
    }
    return [{ stream: chunk.stream as JobLogStream, text: chunk.text }];
  });
}

export function appendJobLog(
  chunks: JobLogChunk[],
  stream: JobLogStream,
  text: string,
): JobLogChunk[] {
  if (!text) return chunks;
  const previous = chunks.at(-1);
  if (previous?.stream === stream) {
    return [...chunks.slice(0, -1), { stream, text: previous.text + text }];
  }
  return [...chunks, { stream, text }];
}

export function jobLogText(chunks: JobLogChunk[] | undefined): string {
  return chunks?.map((chunk) => chunk.text).join("") ?? "";
}

export function remoteJobLog(stdout: string, stderr: string): JobLogChunk[] {
  let chunks: JobLogChunk[] = [];
  chunks = appendJobLog(chunks, "stdout", stdout);
  chunks = appendJobLog(chunks, "stderr", stderr);
  return chunks;
}

export function latestJobForFile(jobs: Job[], path: string | undefined): Job | undefined {
  if (!path) return undefined;
  return jobs.find((job) => job.file === path);
}
