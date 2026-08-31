import { call, let_, program, ref, sourceOf } from "./dsl.js";
import { matrixValue, sequenceValue, tableFromCsv, tableValue } from "./objects.js";
import {
  BioMatrixValue,
  BioQualityValue,
  BioSequenceValue,
  BioTableValue,
  BioValueHandle,
  decodeBioValue,
  encodeBioValue,
  isPromiseLike,
} from "./values.js";

const DEFAULT_INLINE_BYTES = 1024 * 1024;

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
  #callbacks;
  #callbackBridge;
  #inCallback;
  #valueHost;

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
    this.#callbacks = new Map();
    this.#inCallback = false;
    this.#valueHost = {};
    Object.assign(this.#valueHost, {
      decode: (value) => decodeBioValue(value, this.#valueHost),
      page: (id, generation, offset, limit) => this.#call(
        () => this.#session.handle_page(id, generation, offset, limit),
      ),
      float64: (id, generation) => this.#call(
        () => this.#session.handle_float64(id, generation),
      ),
      release: (id, generation) => this.#call(
        () => this.#session.release_handle(id, generation),
      ),
    });
    this.#callbackBridge = {
      call: (name, args) => this.#invokeHostCallback(name, args),
    };
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
    if (this.#inCallback) {
      throw new Error("A JavaScript host callback cannot re-enter its BioLang session");
    }
    // The Rust fetch hook resolves `window.__blFetch` at call time. Reinstall
    // this session's bridge before every operation so another session cannot
    // silently change its cwd, network policy, or browser file provider.
    this.#activateBridge();
    globalThis.__blCallbacks = this.#callbackBridge;
    return callback();
  }

  #invokeHostCallback(name, rawArguments) {
    const registration = this.#callbacks.get(name);
    if (!registration) throw new Error(`JavaScript host callback '${name}' is not registered`);
    if (this.#inCallback) throw new Error("Recursive JavaScript host callbacks are not supported");
    const args = Array.from(rawArguments, (value) => decodeBioValue(value, this.#valueHost));
    validateCallbackArguments(name, args, registration.parameters);
    this.#inCallback = true;
    try {
      const result = registration.callback(...args);
      if (isPromiseLike(result)) {
        throw new TypeError(
          `JavaScript host callback '${name}' returned a Promise; callbacks must be synchronous`,
        );
      }
      validateCallbackReturn(name, result, registration.returns);
      return encodeBioValue(result);
    } catch (error) {
      throw new Error(
        `JavaScript host callback '${name}' failed: ${String(error?.message ?? error)}`,
      );
    } finally {
      this.#inCallback = false;
      // A callback may legitimately use a different BioLang session. Restore
      // this evaluation's bridges before the interpreter invokes the next
      // callback or performs another read.
      this.#activateBridge();
      globalThis.__blCallbacks = this.#callbackBridge;
    }
  }

  run(source) {
    return normalizeRunResult(JSON.parse(
      this.#call(() => this.#session.evaluate(sourceOf(source))),
    ));
  }

  /** Evaluate source and return actual JavaScript data or a large-value handle. */
  evalValue(source, options = {}) {
    const maximumInlineBytes = options.maximumInlineBytes ?? DEFAULT_INLINE_BYTES;
    const raw = this.#call(
      () => this.#session.eval_value(sourceOf(source), maximumInlineBytes),
    );
    return decodeBioValue(raw, this.#valueHost);
  }

  /** Invoke a BioLang function with marshalled JavaScript arguments. */
  callValue(name, args = [], options = {}) {
    if (!Array.isArray(args)) throw new TypeError("callValue args must be an array");
    const maximumInlineBytes = options.maximumInlineBytes ?? DEFAULT_INLINE_BYTES;
    const raw = this.#call(
      () => this.#session.call_value(name, encodeBioValue(args), maximumInlineBytes),
    );
    return decodeBioValue(raw, this.#valueHost);
  }

  /** Define a BioLang variable without serializing it into source code. */
  setValue(name, value) {
    this.#call(() => this.#session.set_value(name, encodeBioValue(value)));
    return ref(name);
  }

  /** Read a BioLang variable as JavaScript data or a large-value handle. */
  getValue(name, options = {}) {
    const maximumInlineBytes = options.maximumInlineBytes ?? DEFAULT_INLINE_BYTES;
    const raw = this.#call(() => this.#session.get_value(name, maximumInlineBytes));
    return decodeBioValue(raw, this.#valueHost);
  }

  /** Register a synchronous, session-local JavaScript function for BioLang. */
  registerFunction(name, options, callback) {
    if (typeof options === "function") {
      callback = options;
      options = {};
    }
    options ??= {};
    if (typeof callback !== "function") throw new TypeError("registerFunction requires a callback");
    if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(name)) {
      throw new TypeError(`'${name}' is not a valid BioLang identifier`);
    }
    if (this.supports(name)) throw new Error(`Cannot replace BioLang builtin '${name}'`);
    const parameters = options.parameters ?? null;
    if (parameters != null && !Array.isArray(parameters)) {
      throw new TypeError("callback parameters must be an array of BioLang type names");
    }
    parameters?.forEach((type, index) => validateCallbackType(type, `parameters[${index}]`));
    validateCallbackType(options.returns ?? "Any", "returns");
    const inferred = parameters?.length ?? callback.length;
    const minimumArguments = options.minimumArguments ?? inferred;
    const maximumArguments = options.variadic
      ? 0xffffffff
      : (options.maximumArguments ?? inferred);
    if (!Number.isSafeInteger(minimumArguments) || minimumArguments < 0) {
      throw new RangeError("minimumArguments must be a non-negative safe integer");
    }
    if (!Number.isSafeInteger(maximumArguments) || maximumArguments < minimumArguments) {
      throw new RangeError("maximumArguments must be a safe integer at least minimumArguments");
    }
    this.#call(() => this.#session.register_host_function(
      name,
      minimumArguments,
      maximumArguments,
    ));
    this.#callbacks.set(name, {
      callback,
      parameters,
      returns: options.returns ?? "Any",
    });
    // Returning the function as a session handle lets direct calls place it
    // inside ordinary JavaScript argument structures without source strings:
    // callValue("map", [[1, 2], callbackHandle]).
    return this.getValue(name, { maximumInlineBytes: 0 });
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
    this.#callbacks.clear();
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
    this.#callbacks.clear();
  }
}

function validateCallbackArguments(name, args, parameters) {
  if (!parameters) return;
  for (let index = 0; index < Math.min(args.length, parameters.length); index += 1) {
    if (!matchesBioType(args[index], parameters[index])) {
      throw new TypeError(
        `JavaScript host callback '${name}' argument ${index + 1} must be ${parameters[index]}`,
      );
    }
  }
}

function validateCallbackReturn(name, value, expected) {
  if (!matchesBioType(value, expected)) {
    throw new TypeError(`JavaScript host callback '${name}' must return ${expected}`);
  }
}

function matchesBioType(value, expected = "Any") {
  const type = String(expected).toLowerCase();
  if (type === "any") return true;
  if (type === "nil") return value == null;
  if (type === "bool" || type === "boolean") return typeof value === "boolean";
  if (type === "str" || type === "string") return typeof value === "string";
  if (type === "int" || type === "integer") {
    return typeof value === "bigint" || (typeof value === "number" && Number.isInteger(value));
  }
  if (type === "float" || type === "number") return typeof value === "number";
  if (type === "numeric") return typeof value === "number" || typeof value === "bigint";
  if (type === "list") return Array.isArray(value);
  if (type === "record") {
    return value != null && typeof value === "object" && !Array.isArray(value);
  }
  if (type === "table") return value instanceof BioTableValue || value instanceof BioValueHandle;
  if (type === "matrix" || type === "sparsematrix") {
    return value instanceof BioMatrixValue || value instanceof BioValueHandle;
  }
  if (["dna", "rna", "protein"].includes(type)) {
    return value instanceof BioSequenceValue && value.kind === type;
  }
  if (type === "sequence") return value instanceof BioSequenceValue;
  if (type === "quality") return value instanceof BioQualityValue;
  return false;
}

const CALLBACK_TYPES = new Set([
  "any", "nil", "bool", "boolean", "str", "string", "int", "integer",
  "float", "number", "numeric", "list", "record", "table", "matrix",
  "sparsematrix", "dna", "rna", "protein", "sequence", "quality",
]);

function validateCallbackType(value, option) {
  if (typeof value !== "string" || !CALLBACK_TYPES.has(value.toLowerCase())) {
    throw new TypeError(`callback ${option} is not a supported BioLang type`);
  }
}
