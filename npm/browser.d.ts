/** Result of evaluating BioLang source. */
export interface RunResult {
  /** Whether evaluation completed without an error. */
  ok: boolean;
  /** The final expression, formatted for display. */
  value: string | null;
  /** Runtime type of `value`, e.g. "Int", "DNA", "Table". */
  type: string | null;
  /** Everything `print` and `println` wrote. */
  output: string;
  /** A table or chart as JSON, when the value is one. */
  structured: unknown | null;
  /** Every displayed value, for a notebook-style UI. */
  results: unknown[];
  /** Which line produced what. */
  trace: Array<{ line: number; text: string }>;
  /** Error message when `ok` is false. */
  error: string | null;
}

export interface BioLangOptions {
  /**
   * Synchronous reader for file and URL access.
   *
   * The interpreter calls this mid-evaluation and cannot await. Without one,
   * the fallback is a synchronous XMLHttpRequest, which is deprecated on the
   * main thread and unavailable in workers - so supply this if you have an
   * in-memory workspace or a cache to read from.
   */
  fetchSync?: (url: string) => string;
}

export interface Builtin {
  name: string;
  signature: string;
  category: string;
}

/** A BioLang interpreter. State persists across `run` calls. */
export class BioLang {
  static create(options?: BioLangOptions): Promise<BioLang>;
  run(source: string): RunResult;
  reset(): void;
  builtins(): Builtin[];
  variables(): unknown;
  format(source: string, indent?: number): string;
  tokenize(source: string): unknown;
  import(source: string, format: string, filename?: string): unknown;
  readonly raw: unknown;
}

/** Load a fresh interpreter and evaluate once. */
export function run(source: string, options?: BioLangOptions): Promise<RunResult>;

