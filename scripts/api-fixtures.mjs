#!/usr/bin/env node
/**
 * Record and replay the HTTP responses the network-dependent pack examples need.
 *
 * Four Rosalind problems fetch from NCBI or UniProt: frmt, gbk and need in the
 * Armory, mprt in the Stronghold. They ran only in an advisory job, because a
 * rate-limited NCBI is not a BioLang regression -- so nothing gated their
 * answers, and a real break in `ncbi_sequence` or `uniprot_fasta` could reach
 * main behind a red X everyone had learned to ignore.
 *
 * These are real recorded responses, not hand-written ones: --record forwards to
 * the live services and saves exactly what they returned. --serve then replays
 * them with no network at all, so the examples can move into the gating job. The
 * live job stays, still advisory, as the check that the recordings have not
 * drifted from what the services actually say today.
 *
 * A miss under --serve is a loud 504 rather than a pass-through: a fixture that
 * silently fell back to the network would put the flakiness straight back.
 *
 *   node scripts/api-fixtures.mjs --record   # refresh from the live services
 *   node scripts/api-fixtures.mjs --serve    # replay for CI
 *
 * Both print the base URLs to export as BIOLANG_NCBI_URL and BIOLANG_UNIPROT_URL.
 */

import { createHash } from "node:crypto";
import { createServer } from "node:http";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const FIXTURES = path.join(root, "tests", "api-fixtures");

/** Upstreams, keyed by the first path segment the examples are pointed at. */
const UPSTREAM = {
  ncbi: "https://eutils.ncbi.nlm.nih.gov",
  uniprot: "https://rest.uniprot.org",
};

const record = process.argv.includes("--record");
const port = Number(
  (process.argv.find((a) => a.startsWith("--port=")) ?? "--port=8787").split("=")[1],
);

/**
 * An api_key is per-developer and rate-limit-scoped, so it is stripped before
 * keying: a recording made with one must replay for someone who has none.
 * Query order is normalised because it is not semantically meaningful here.
 */
function keyFor(service, rawUrl) {
  const url = new URL(rawUrl, "http://placeholder");
  url.searchParams.delete("api_key");
  const query = [...url.searchParams.entries()]
    .sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))
    .map(([k, v]) => `${k}=${v}`)
    .join("&");
  return `${service} ${url.pathname}${query ? `?${query}` : ""}`;
}

const fileFor = (key) =>
  path.join(FIXTURES, `${createHash("sha256").update(key).digest("hex").slice(0, 32)}.json`);

function split(rawUrl) {
  const [, service, ...rest] = rawUrl.split("?")[0].split("/");
  return { service, remainder: `/${rest.join("/")}${rawUrl.includes("?") ? `?${rawUrl.split("?").slice(1).join("?")}` : ""}` };
}

const server = createServer(async (request, response) => {
  const { service, remainder } = split(request.url ?? "/");
  if (!UPSTREAM[service]) {
    response.writeHead(404, { "content-type": "text/plain" });
    response.end(`Unknown service "${service}". Expected one of: ${Object.keys(UPSTREAM).join(", ")}`);
    return;
  }

  const key = keyFor(service, remainder);
  const file = fileFor(key);

  if (!record) {
    try {
      const fixture = JSON.parse(await readFile(file, "utf8"));
      response.writeHead(fixture.status, { "content-type": fixture.contentType });
      response.end(fixture.body);
    } catch {
      // Loud, not a pass-through. See the header comment.
      response.writeHead(504, { "content-type": "text/plain" });
      response.end(`No fixture for: ${key}\nRe-record with: node scripts/api-fixtures.mjs --record`);
      console.error(`  MISS  ${key}`);
    }
    return;
  }

  const target = UPSTREAM[service] + remainder;
  try {
    const upstream = await fetch(target, { headers: { "user-agent": "biolang-api-fixtures" } });
    const body = await upstream.text();
    const contentType = upstream.headers.get("content-type") ?? "text/plain";
    await mkdir(FIXTURES, { recursive: true });
    await writeFile(
      file,
      `${JSON.stringify({ key, status: upstream.status, contentType, body }, null, 2)}\n`,
      "utf8",
    );
    console.error(`  saved ${key} (${upstream.status}, ${body.length} bytes)`);
    response.writeHead(upstream.status, { "content-type": contentType });
    response.end(body);
  } catch (error) {
    response.writeHead(502, { "content-type": "text/plain" });
    response.end(String(error));
    console.error(`  FAILED ${key}: ${error}`);
  }
});

server.listen(port, "127.0.0.1", () => {
  const base = `http://127.0.0.1:${port}`;
  console.error(`${record ? "Recording" : "Replaying"} API fixtures on ${base}`);
  console.error(`  BIOLANG_NCBI_URL=${base}/ncbi/entrez/eutils`);
  console.error(`  BIOLANG_UNIPROT_URL=${base}/uniprot`);
});
