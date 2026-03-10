/* tslint:disable */
/* eslint-disable */

/**
 * Evaluate BioLang source code. Returns JSON: `{ok, value, type, output, error}`
 */
export function evaluate(source: string): string;

/**
 * Initialize the WASM module (set panic hook for better error messages).
 */
export function init(): void;

/**
 * List all builtin functions. Returns JSON array of {name, signature, category}.
 */
export function list_builtins(): string;

/**
 * List all variables in the current environment. Returns JSON array.
 */
export function list_variables(): string;

/**
 * Reset the interpreter state.
 */
export function reset(): void;

/**
 * Tokenize source code for syntax highlighting. Returns JSON array of token spans.
 */
export function tokenize(source: string): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly evaluate: (a: number, b: number) => [number, number];
    readonly init: () => void;
    readonly list_builtins: () => [number, number];
    readonly list_variables: () => [number, number];
    readonly reset: () => void;
    readonly tokenize: (a: number, b: number) => [number, number];
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
