import { parseNotebook } from "./notebooks";
import type { CodeImportResult } from "./types";

export type ImportOutputKind = "script" | "notebook";

export interface ConversionSummary {
  converted: number;
  approximated: number;
  unsupported: number;
}

export function importOutputKind(result: CodeImportResult): ImportOutputKind {
  return result.notebook ? "notebook" : "script";
}

export function convertImportOutput(
  content: string,
  from: ImportOutputKind,
  to: ImportOutputKind,
  sourceName: string,
) {
  if (from === to) return content;
  if (to === "notebook") {
    return `# Imported from ${sourceName}\n\n\`\`\`biolang\n${content.trim()}\n\`\`\`\n`;
  }
  const code = parseNotebook(content)
    .filter((block) => block.type === "code" && !block.directives.includes("skip"))
    .map((block) => block.content.trim())
    .filter(Boolean)
    .join("\n\n");
  return `# Converted from notebook: ${sourceName}\n\n${code}\n`;
}

export function outputNameForKind(name: string, kind: ImportOutputKind) {
  const stem = name
    .replace(/\.bl\.md$/i, "")
    .replace(/\.(?:bln|bl)$/i, "");
  return `${stem}.${kind === "notebook" ? "bln" : "bl"}`;
}

export function summarizeConversion(content: string, kind: ImportOutputKind): ConversionSummary {
  const lines = content.split(/\r?\n/);
  const approximated = lines.filter((line) => /\bapproximat(?:e|ion|ed)\b/i.test(line)).length;
  const unsupported = lines.filter((line) =>
    /\b(?:TODO|unsupported|cannot convert|manual attention)\b/i.test(line)
    && !line.startsWith("# Conversion complete:")
    && !/\bapproximat(?:e|ion|ed)\b/i.test(line)).length;
  const converted = kind === "notebook"
    ? parseNotebook(content).filter((block) => block.type === "code").length
    : 1;
  return { converted, approximated, unsupported };
}

export function importDestination(directory: string, name: string) {
  const cleanDirectory = directory.replaceAll("\\", "/").replace(/^\/+|\/+$/g, "");
  const cleanName = name.trim();
  if (!cleanName || /[\\/]/.test(cleanName) || cleanName === "." || cleanName === "..") {
    return undefined;
  }
  return cleanDirectory ? `${cleanDirectory}/${cleanName}` : cleanName;
}
