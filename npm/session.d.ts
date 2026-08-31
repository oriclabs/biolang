import type { BioExpression, BioProgram, BioStatement } from "./dsl.js";
import type { SomerExecutor } from "./somer.js";
import type { BioMatrix, BioSequence, BioTable } from "./objects.js";

export interface RunResult {
  ok: boolean;
  value: string | null;
  type: string | null;
  output: string;
  structured: unknown | null;
  results: unknown[];
  trace: Array<{ line: number; text: string }>;
  error: string | null;
}

export interface Builtin {
  name: string;
  arity: string;
}

export interface LanguageDiagnostic {
  severity: "error" | "warning" | "information";
  message: string;
  start: number;
  end: number;
  line: number;
  column: number;
  endLine: number;
  endColumn: number;
}

export interface LanguageCompletion {
  label: string;
  kind: "function" | "keyword";
  detail: string;
  insertText: string;
}

export interface VariableInspectionOptions {
  offset?: number;
  limit?: number;
}

export interface VariableExportOptions {
  format?: "json" | "csv" | "tsv" | "text";
  maximumBytes?: number;
}

export interface SomerConnectionOptions {
  baseUrl?: string;
  token?: string;
  fetch?: typeof fetch;
  client?: unknown;
}

export type BioLangSource = string | BioExpression | BioStatement | BioProgram;

export declare const WASM_API_COVERAGE: Readonly<Record<string, string>>;

export class BioLangSession {
  [name: string]: any;
  constructor(wasm: unknown);
  run(source: BioLangSource): RunResult;
  define(name: string, value: import("./dsl.js").BioArgument): BioExpression;
  ref(name: string): BioExpression;
  invoke(name: string, ...args: import("./dsl.js").BioArgument[]): RunResult;
  csv(path: string, options?: Record<string, import("./dsl.js").BioArgument>): BioTable;
  table(rows: import("./dsl.js").BioArgument[]): BioTable;
  sequence(value: string, kind?: "dna" | "rna" | "protein"): BioSequence;
  matrix(value: import("./dsl.js").BioArgument): BioMatrix;
  reset(): void;
  builtins(): Builtin[];
  supports(name: string): boolean;
  variables(): unknown;
  inspectVariable(name: string, options?: VariableInspectionOptions): unknown;
  exportVariable(name: string, options?: VariableExportOptions): Uint8Array;
  registerModule(path: string, source: string): void;
  runtimeVersion(): string | null;
  format(source: string, indent?: number): string;
  tokenize(source: string): unknown;
  diagnostics(source: string): LanguageDiagnostic[];
  completions(prefix?: string): LanguageCompletion[];
  signature(name: string): string | null;
  transpileJavaScript(source: string): string;
  import(source: string, format: string, filename?: string): unknown;
  validateImport(source: string, options?: { notebook?: boolean }): unknown;
  qcMetrics(kind: string, text: string): unknown;
  connectSomer(options: SomerConnectionOptions): Promise<SomerExecutor>;
  readonly raw: unknown;
}
