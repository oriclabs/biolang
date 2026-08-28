import type {
  CodeImportResult,
  JobTraceEntry,
  PreviewMetrics,
  ConsoleEnvironment,
  ConsoleResponse,
  ImportValidationReport,
  StructuredResult,
} from "./types";

interface WasmEvaluation {
  ok: boolean;
  value?: string;
  type?: string;
  output?: string;
  error?: string;
  structured?: StructuredResult;
  // Typed results promoted from print/println, in display order, with the
  // program's own return value appended when it is structured.
  results?: StructuredResult[];
  // Printed values tagged with the line that printed them, for the inline
  // annotations the editor draws after a run.
  trace?: JobTraceEntry[];
}

interface WasmVariable {
  name: string;
  type?: string;
  typeName?: string;
  preview: string;
  sizeBytes?: number;
  members?: string[];
}

interface BioLangWasm {
  default: (input?: URL) => Promise<unknown>;
  init: () => void;
  evaluate: (source: string) => string;
  format: (source: string, indent: number) => string;
  qc_metrics: (kind: string, text: string) => string;
  import_source: (source: string, format: string, filename: string) => string;
  list_variables: () => string;
  reset: () => void;
  validate_import: (source: string, notebook: boolean) => string;
}

interface ImportEnvelope {
  ok: boolean;
  result?: CodeImportResult;
  error?: string;
}

let runtimePromise: Promise<BioLangWasm> | undefined;
let fileReader: (path: string) => string | undefined = () => undefined;
let consoleResponseId = 0;

declare global {
  interface Window {
    __blFetch?: {
      sync: (url: string) => string;
    };
  }
}

export function setBrowserRuntimeFileReader(reader: (path: string) => string | undefined) {
  fileReader = reader;
}

function normalizeWorkspacePath(url: string) {
  return decodeURIComponent(url)
    .replaceAll("\\", "/")
    .replace(/^browser:\/\/[^/]+\//, "")
    .replace(/^\.?\//, "");
}

async function runtime(): Promise<BioLangWasm> {
  if (runtimePromise) return runtimePromise;
  runtimePromise = (async () => {
    window.__blFetch = {
      sync(url: string) {
        if (/^https?:\/\//i.test(url)) {
          try {
            const request = new XMLHttpRequest();
            request.open("GET", url, false);
            request.setRequestHeader("Accept", "application/json,text/plain,text/*,*/*");
            request.send();
            if (request.status >= 200 && request.status < 300) return request.responseText;
            return `ERROR:HTTP ${request.status || "network"} while fetching ${url}`;
          } catch (error) {
            return `ERROR:Browser network request failed for ${url}: ${String(error)}`;
          }
        }
        const normalized = normalizeWorkspacePath(url);
        const content = fileReader(normalized);
        return content === undefined
          ? `ERROR:File not found in browser workspace (${url})`
          : content;
      },
    };
    const moduleUrl = new URL("./wasm/bl_wasm.js", document.baseURI);
    const module = await import(/* @vite-ignore */ moduleUrl.href) as BioLangWasm;
    await module.default();
    module.init();
    return module;
  })();
  return runtimePromise;
}

export async function evaluateBrowserSource(
  source: string,
  reset = false,
): Promise<WasmEvaluation> {
  const wasm = await runtime();
  if (reset) wasm.reset();
  return JSON.parse(wasm.evaluate(source)) as WasmEvaluation;
}

/** Canonical layout for a document, from the same formatter `bl fmt` runs. */
export async function formatBrowserSource(source: string, indent: number): Promise<string> {
  const wasm = await runtime();
  return wasm.format(source, indent);
}

/** Sequencing quality metrics, from the same `bl-qc` the Desktop build uses. */
export async function browserQcMetrics(
  kind: string,
  text: string,
): Promise<PreviewMetrics | undefined> {
  const wasm = await runtime();
  return JSON.parse(wasm.qc_metrics(kind, text)) as PreviewMetrics ?? undefined;
}

export async function importBrowserSource(
  source: string,
  format: string,
  filename: string,
): Promise<CodeImportResult> {
  const wasm = await runtime();
  const envelope = JSON.parse(wasm.import_source(source, format, filename)) as ImportEnvelope;
  if (!envelope.ok || !envelope.result) {
    throw new Error(envelope.error || `Cannot import ${filename}`);
  }
  return envelope.result;
}

export async function validateBrowserImport(
  content: string,
  notebook: boolean,
): Promise<ImportValidationReport> {
  const wasm = await runtime();
  return JSON.parse(wasm.validate_import(content, notebook)) as ImportValidationReport;
}

function consoleEnvironment(variables: WasmVariable[]): ConsoleEnvironment {
  const mapped = variables.map((variable) => ({
    name: variable.name,
    typeName: variable.typeName ?? variable.type ?? "Value",
    preview: variable.preview,
    sizeBytes: variable.sizeBytes ?? new TextEncoder().encode(variable.preview).byteLength,
    members: variable.members ?? [],
  }));
  return {
    variables: mapped,
    totalBytes: mapped.reduce((total, variable) => total + variable.sizeBytes, 0),
  };
}

async function inspectEnvironment(wasm: BioLangWasm): Promise<ConsoleEnvironment> {
  const variables = JSON.parse(wasm.list_variables()) as WasmVariable[];
  return consoleEnvironment(variables);
}

export async function browserConsoleResponse(source?: string): Promise<ConsoleResponse> {
  const wasm = await runtime();
  const startedAt = performance.now();
  let evaluation: WasmEvaluation | undefined;
  if (source?.trim()) {
    evaluation = JSON.parse(wasm.evaluate(source)) as WasmEvaluation;
  }
  const hasValue = evaluation?.value
    && !["null", "nil", "Nil", "()", "None"].includes(evaluation.value);
  return {
    protocol: "biolang.console/v1",
    id: ++consoleResponseId,
    status: evaluation && !evaluation.ok ? "error" : "ok",
    output: evaluation?.output ?? "",
    value: evaluation?.ok && hasValue
      ? {
          kind: "text",
          typeName: evaluation.type ?? "Value",
          text: evaluation.value!,
          columns: [],
          rows: [],
          truncated: false,
        }
      : undefined,
    error: evaluation?.ok === false ? evaluation.error ?? "BioLang evaluation failed" : undefined,
    durationMs: Math.max(0, performance.now() - startedAt),
    environment: await inspectEnvironment(wasm),
  };
}

export async function resetBrowserConsole(): Promise<ConsoleResponse> {
  const wasm = await runtime();
  wasm.reset();
  return browserConsoleResponse();
}
