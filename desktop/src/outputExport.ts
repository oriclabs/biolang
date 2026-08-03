import type { Job, JobLogChunk } from "./types";

export type OutputExportFormat = "log" | "text" | "json" | "svg" | "html";

export interface OutputExportOption {
  format: OutputExportFormat;
  label: string;
  extension: string;
}

const svgPattern = /<svg\b[\s\S]*?<\/svg>/gi;

function programText(chunks: JobLogChunk[]): string {
  return chunks
    .filter((chunk) => chunk.stream === "stdout" || chunk.stream === "stderr")
    .map((chunk) => chunk.text)
    .join("");
}

function parsedJson(chunks: JobLogChunk[]): string | undefined {
  const candidate = programText(chunks).trim();
  if (!candidate) return undefined;
  try {
    return `${JSON.stringify(JSON.parse(candidate), null, 2)}\n`;
  } catch {
    return undefined;
  }
}

function svgOutputs(chunks: JobLogChunk[]): string[] {
  return chunks.flatMap((chunk) => [...chunk.text.matchAll(svgPattern)].map((match) => match[0]));
}

function escapeHtml(value: string): string {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;");
}

function reportBody(chunks: JobLogChunk[]): string {
  return chunks.map((chunk) => {
    let offset = 0;
    const parts: string[] = [];
    for (const match of chunk.text.matchAll(svgPattern)) {
      const index = match.index ?? 0;
      if (index > offset) {
        parts.push(`<pre class="${chunk.stream}">${escapeHtml(chunk.text.slice(offset, index))}</pre>`);
      }
      const source = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(match[0])}`;
      parts.push(`<figure><img alt="BioLang plot output" src="${escapeHtml(source)}"></figure>`);
      offset = index + match[0].length;
    }
    if (offset < chunk.text.length) {
      parts.push(`<pre class="${chunk.stream}">${escapeHtml(chunk.text.slice(offset))}</pre>`);
    }
    return parts.join("");
  }).join("");
}

function htmlReport(job: Job | undefined, chunks: JobLogChunk[]): string {
  const title = job?.file ?? "BioLang output";
  const metadata = job
    ? `${job.backend} | ${job.status}${job.durationMs == null ? "" : ` | ${(job.durationMs / 1000).toFixed(2)}s`}`
    : "No job metadata";
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${escapeHtml(title)} output</title>
<style>
body{max-width:1100px;margin:32px auto;padding:0 24px;color:#20242a;background:#fff;font:14px/1.55 system-ui,sans-serif}
h1{margin-bottom:4px;font-size:22px}header p{margin-top:0;color:#66707c}
main{margin-top:24px;border-top:1px solid #dfe3e8;padding-top:16px}
pre{margin:0;white-space:pre-wrap;overflow-wrap:anywhere;font:13px/1.55 ui-monospace,monospace}
pre.stderr{color:#a32424}pre.system{color:#59636f}pre.success{color:#187343}
figure{margin:20px 0;padding:12px;border:1px solid #dfe3e8}figure img{display:block;max-width:100%;height:auto;margin:auto}
</style>
</head>
<body>
<header><h1>${escapeHtml(title)}</h1><p>${escapeHtml(metadata)}</p></header>
<main>${reportBody(chunks) || "<p>No output was recorded.</p>"}</main>
</body>
</html>
`;
}

export function outputExportOptions(chunks: JobLogChunk[] | undefined): OutputExportOption[] {
  const value = chunks ?? [];
  const options: OutputExportOption[] = [
    { format: "log", label: "Complete log", extension: "log" },
    { format: "text", label: "Program text", extension: "txt" },
  ];
  if (parsedJson(value)) options.push({ format: "json", label: "JSON", extension: "json" });
  if (svgOutputs(value).length) options.push({ format: "svg", label: "First SVG plot", extension: "svg" });
  options.push({ format: "html", label: "HTML report", extension: "html" });
  return options;
}

export function buildOutputExport(
  chunks: JobLogChunk[] | undefined,
  format: OutputExportFormat,
  job?: Job,
): string {
  const value = chunks ?? [];
  if (format === "text") return programText(value);
  if (format === "json") return parsedJson(value) ?? programText(value);
  if (format === "svg") return svgOutputs(value)[0] ?? "";
  if (format === "html") return htmlReport(job, value);
  return value.map((chunk) => chunk.text).join("");
}
