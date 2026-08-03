import type { Job, JobLogChunk, StructuredResult } from "./types";

export type OutputTab = "summary" | "text" | "tables" | "plots" | "files" | "errors" | "provenance";

export interface OutputTable {
  name: string;
  resultIndex: number;
  resultId: string;
  paged: boolean;
  columns: string[];
  rows: unknown[][];
  totalRows: number;
  truncated: boolean;
}

export interface OutputPlot {
  name: string;
  resultId: string;
  svg: string;
}

function scalar(value: unknown): unknown {
  if (value && typeof value === "object" && "kind" in value) {
    const result = value as StructuredResult;
    if ("value" in result) return result.value;
    if (typeof result.display === "string") return result.display;
  }
  return value;
}

function recordTable(result: StructuredResult, index: number): OutputTable | undefined {
  const name = typeof result.name === "string" ? result.name : `Table ${index + 1}`;
  const resultId = typeof result.id === "string" ? result.id : `${result.kind}-${index}`;
  if (result.kind === "table" && Array.isArray(result.columns) && Array.isArray(result.rows)) {
    return {
      name,
      resultIndex: result.resultIndex ?? index,
      resultId,
      paged: typeof result.dataRef === "string",
      columns: result.columns,
      rows: result.rows.map((row) => row.map(scalar)),
      totalRows: Number(result.totalRows ?? result.rows.length),
      truncated: Boolean(result.truncated),
    };
  }
  if (result.kind === "matrix" && Array.isArray(result.rows)) {
    const columns = Array.isArray(result.columnNames)
      ? result.columnNames.map(String)
      : Array.from({ length: result.rows[0]?.length ?? 0 }, (_, column) => `C${column + 1}`);
    return {
      name: typeof result.name === "string" ? result.name : `Matrix ${index + 1}`,
      resultIndex: result.resultIndex ?? index,
      resultId,
      paged: typeof result.dataRef === "string",
      columns,
      rows: result.rows,
      totalRows: Number(result.totalRows ?? result.rows.length),
      truncated: Boolean(result.truncated),
    };
  }
  if (result.kind === "list" && Array.isArray(result.items)) {
    const records = result.items.filter((item) => item.kind === "record" && item.value && typeof item.value === "object");
    if (records.length !== result.items.length || !records.length) return undefined;
    const columns = [...new Set(records.flatMap((item) => Object.keys(item.value as object)))];
    return {
      name: typeof result.name === "string" ? result.name : `Records ${index + 1}`,
      resultIndex: result.resultIndex ?? index,
      resultId,
      paged: typeof result.dataRef === "string",
      columns,
      rows: records.map((item) => columns.map((column) =>
        scalar((item.value as Record<string, unknown>)[column]))),
      totalRows: Number(result.totalItems ?? records.length),
      truncated: Boolean(result.truncated),
    };
  }
  return undefined;
}

const svgPattern = /<svg\b[\s\S]*?<\/svg>/gi;

export function outputTables(job: Job | undefined): OutputTable[] {
  return (job?.results ?? []).flatMap((result, index) => {
    const table = recordTable(result, index);
    return table ? [table] : [];
  });
}

export function outputPlots(job: Job | undefined): OutputPlot[] {
  const typed = (job?.results ?? []).flatMap((result, index) =>
    result.kind === "plot" && result.format === "svg" && typeof result.data === "string"
      ? [{
          name: typeof result.name === "string" ? result.name : `Plot ${index + 1}`,
          resultId: typeof result.id === "string" ? result.id : `plot-${index}`,
          svg: result.data,
        }]
      : []);
  const seen = new Set(typed.map((plot) => plot.svg));
  const logs = (job?.log ?? []).flatMap((chunk) =>
    [...chunk.text.matchAll(svgPattern)].flatMap((match, index) => {
      if (seen.has(match[0])) return [];
      seen.add(match[0]);
      return [{ name: `Log plot ${index + 1}`, resultId: `log-plot-${index}`, svg: match[0] }];
    }));
  return [...typed, ...logs];
}

export function outputErrors(job: Job | undefined): JobLogChunk[] {
  return (job?.log ?? []).filter((chunk) => chunk.stream === "stderr");
}

export function availableOutputTabs(job: Job | undefined): OutputTab[] {
  const tabs: OutputTab[] = ["summary", "text"];
  if (outputTables(job).length) tabs.push("tables");
  if (outputPlots(job).length) tabs.push("plots");
  if (job?.artifacts?.length) tabs.push("files");
  if (outputErrors(job).length) tabs.push("errors");
  if (job?.provenance) tabs.push("provenance");
  return tabs;
}

export function valuePreview(value: unknown): string {
  if (value == null) return "";
  if (typeof value === "string") return value;
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  return JSON.stringify(value);
}

export function semanticResultPairs(left: Job, right: Job) {
  const keyed = (job: Job) => {
    const occurrences = new Map<string, number>();
    return new Map((job.results ?? []).map((result) => {
      const base = result.id || result.name || result.kind;
      const occurrence = (occurrences.get(base) ?? 0) + 1;
      occurrences.set(base, occurrence);
      return [`${base}${occurrence > 1 ? `#${occurrence}` : ""}`, result] as const;
    }));
  };
  const leftResults = keyed(left);
  const rightResults = keyed(right);
  return [...new Set([...leftResults.keys(), ...rightResults.keys()])].sort().map((key) => ({
    key,
    left: leftResults.get(key),
    right: rightResults.get(key),
  }));
}
