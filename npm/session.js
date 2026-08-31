import { call, let_, program, ref, sourceOf } from "./dsl.js";
import { matrixValue, sequenceValue, tableFromCsv, tableValue } from "./objects.js";

/**
 * Public WASM exports and the JavaScript SDK entry point which owns each one.
 * The coverage check compares this map to the built artifact so adding a Rust
 * export without adding its JavaScript API becomes a build failure.
 */
export const WASM_API_COVERAGE = Object.freeze({
  default: "BioLang.create loader",
  init: "BioLang.create loader",
  initSync: "raw loader",
  evaluate: "BioLangSession.run",
  export_variable: "BioLangSession.exportVariable",
  format: "BioLangSession.format",
  import_source: "BioLangSession.import",
  inspect_variable: "BioLangSession.inspectVariable",
  language_completions: "BioLangSession.completions",
  language_diagnostics: "BioLangSession.diagnostics",
  language_signature: "BioLangSession.signature",
  list_builtins: "BioLangSession.builtins/supports",
  list_variables: "BioLangSession.variables",
  qc_metrics: "BioLangSession.qcMetrics",
  register_module: "BioLangSession.registerModule",
  reset: "BioLangSession.reset",
  runtime_version: "BioLangSession.runtimeVersion",
  tokenize: "BioLangSession.tokenize",
  validate_import: "BioLangSession.validateImport",
  transpile_javascript: "BioLangSession.transpileJavaScript",
  WasmSession: "BioLangSession isolated interpreter ownership",
});

export function normalizeRunResult(parsed) {
  return {
    ok: parsed.ok ?? false,
    value: parsed.value ?? null,
    type: parsed.type ?? null,
    output: parsed.output ?? "",
    structured: parsed.structured ?? null,
    results: parsed.results ?? [],
    trace: parsed.trace ?? [],
    error: parsed.error ?? null,
  };
}

/** Shared session implementation for Node and browser WASM loaders. */
export class BioLangSession {
  #wasm;
  #session;
  #activateBridge;
  #builtins;
  #disposed;

  constructor(wasm, session = null, activateBridge = () => {}) {
    this.#wasm = wasm;
    // The fallback keeps custom/mock embedders using the old module-level API
    // working. Official builds always expose WasmSession and are isolated.
    this.#session = session ?? (
      typeof wasm.WasmSession === "function" ? new wasm.WasmSession() : wasm
    );
    this.#activateBridge = activateBridge;
    this.#builtins = null;
    this.#disposed = false;
    return new Proxy(this, {
      get(target, property) {
        if (property === "then") return undefined;
        if (typeof property !== "string" || property in target) {
          const value = Reflect.get(target, property, target);
          return typeof value === "function" ? value.bind(target) : value;
        }
        return (...args) => target.run(call(property, ...args));
      },
    });
  }

  #call(callback) {
    if (this.#disposed) throw new Error("This BioLang session has been disposed");
    // The Rust fetch hook resolves `window.__blFetch` at call time. Reinstall
    // this session's bridge before every operation so another session cannot
    // silently change its cwd, network policy, or browser file provider.
    this.#activateBridge();
    return callback();
  }

  run(source) {
    return normalizeRunResult(JSON.parse(
      this.#call(() => this.#session.evaluate(sourceOf(source))),
    ));
  }

  define(name, value) {
    const result = this.run(program(let_(name, value)));
    if (!result.ok) throw new Error(result.error || `Cannot define ${name}`);
    return ref(name);
  }

  ref(name) { return ref(name); }

  invoke(name, ...args) { return this.run(call(name, ...args)); }

  csv(path, options) { return tableFromCsv(path, options); }
  table(rows) { return tableValue(rows); }
  sequence(value, kind = "dna") { return sequenceValue(value, kind); }
  matrix(value) { return matrixValue(value); }

  reset() {
    this.#call(() => this.#session.reset());
    this.#builtins = null;
  }

  builtins() {
    this.#builtins ??= JSON.parse(this.#call(() => this.#wasm.list_builtins()))
      .sort((left, right) => left.name.localeCompare(right.name));
    return this.#builtins.slice();
  }

  supports(name) {
    return this.builtins().some((builtin) => builtin.name === name);
  }

  variables() {
    return JSON.parse(this.#call(() => this.#session.list_variables()));
  }

  inspectVariable(name, options = {}) {
    const offset = options.offset ?? 0;
    const limit = options.limit ?? 100;
    return JSON.parse(this.#call(
      () => this.#session.inspect_variable(name, offset, limit),
    ));
  }

  exportVariable(name, options = {}) {
    const format = options.format ?? "json";
    const maximumBytes = options.maximumBytes ?? 64 * 1024 * 1024;
    return this.#call(
      () => this.#session.export_variable(name, format, maximumBytes),
    );
  }

  registerModule(path, source) {
    this.#call(() => this.#session.register_module(path, source));
  }

  runtimeVersion() {
    return typeof this.#wasm.runtime_version === "function"
      ? this.#call(() => this.#wasm.runtime_version())
      : null;
  }

  format(source, indent = 4) {
    return this.#call(() => this.#wasm.format(source, indent));
  }

  tokenize(source) {
    return JSON.parse(this.#call(() => this.#wasm.tokenize(source)));
  }

  diagnostics(source) {
    return JSON.parse(this.#call(() => this.#wasm.language_diagnostics(source)));
  }

  completions(prefix = "") {
    return JSON.parse(this.#call(() => this.#wasm.language_completions(prefix)));
  }

  signature(name) {
    return JSON.parse(this.#call(() => this.#wasm.language_signature(name)));
  }

  transpileJavaScript(source) {
    const result = JSON.parse(this.#call(() => this.#wasm.transpile_javascript(source)));
    if (!result.ok) throw new Error(result.error || "Cannot translate BioLang to JavaScript");
    return result.source;
  }

  import(source, format, filename = "input") {
    return JSON.parse(this.#call(() => this.#wasm.import_source(source, format, filename)));
  }

  validateImport(source, options = {}) {
    return JSON.parse(this.#call(
      () => this.#wasm.validate_import(source, options.notebook ?? false),
    ));
  }

  qcMetrics(kind, text) {
    return JSON.parse(this.#call(() => this.#wasm.qc_metrics(kind, text)));
  }

  async connectSomer(options) {
    const { SomerClient } = options.client
      ? { SomerClient: null }
      : await import("@somer/client");
    const client = options.client ?? new SomerClient(options);
    const { SomerExecutor } = await import("./somer.js");
    return new SomerExecutor(client);
  }

  get raw() {
    return this.#wasm;
  }

  /** Release the Rust interpreter owned by this wrapper. */
  dispose() {
    if (this.#disposed) return;
    this.#session.free?.();
    this.#disposed = true;
    this.#builtins = null;
  }
}
