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

function directValueType(value) {
  if (value === null || value === undefined) return "Nil";
  if (Array.isArray(value)) return "List";
  if (value instanceof BioSequenceValue) return value.kind.toUpperCase();
  if (value instanceof BioTableValue) return "Table";
  if (typeof value === "bigint") return "Int";
  if (typeof value === "number") return "Float";
  if (typeof value === "string") return "Str";
  if (typeof value === "boolean") return "Bool";
  return value?.constructor?.name ?? typeof value;
}

function equalDirectValues(left, right) {
  if (left === right) return true;
  if (typeof left === "number" && typeof right === "number") return left === right;
  if (left instanceof BioSequenceValue && right instanceof BioSequenceValue) {
    return left.kind === right.kind && left.data === right.data;
  }
  if (Array.isArray(left) && Array.isArray(right)) {
    return left.length === right.length && left.every((value, index) => equalDirectValues(value, right[index]));
  }
  if (left instanceof BioTableValue && right instanceof BioTableValue) {
    return equalDirectValues(left.columns, right.columns) && equalDirectValues(left.rows, right.rows);
  }
  if (left instanceof Map && right instanceof Map) {
    return left.size === right.size && [...left].every(([key, value]) => right.has(key) && equalDirectValues(value, right.get(key)));
  }
  if (left instanceof Set && right instanceof Set) {
    return left.size === right.size && [...left].every(value => [...right].some(item => equalDirectValues(value, item)));
  }
  if (left && right && typeof left === "object" && typeof right === "object") {
    const leftKeys = Object.keys(left).sort();
    const rightKeys = Object.keys(right).sort();
    return equalDirectValues(leftKeys, rightKeys)
      && leftKeys.every(key => equalDirectValues(left[key], right[key]));
  }
  return false;
}

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
  #callbackHandles;
  #builtinReferences;
  #callbackSequence;
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
    this.#callbackHandles = new WeakMap();
    this.#builtinReferences = new WeakMap();
    this.#callbackSequence = 0;
    this.#inCallback = [];
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
      field: (descriptor, name, optional = false) => this.#call(
        () => this.#session.field_value(
          encodeBioValue(descriptor), name, optional, DEFAULT_INLINE_BYTES,
        ),
      ),
    });
    this.#callbackBridge = {
      call: (name, args) => this.#invokeHostCallback(name, args),
    };
    return new Proxy(this, {
      get(target, property) {
        if (property === "then") return undefined;
        if (typeof property !== "string" || hasSdkMember(target, property)) {
          const value = Reflect.get(target, property, target);
          return typeof value === "function" ? value.bind(target) : value;
        }
        const callable = (...args) => target.#callDynamic(property, args);
        target.#builtinReferences.set(callable, bioLangBuiltinName(property));
        return callable;
      },
    });
  }

  #callDynamic(name, args) {
    try {
      const resolved = this.supports(name) ? name : bioLangBuiltinName(name);
      return this.callValue(this.supports(resolved) ? resolved : name, args);
    } catch (error) {
      const message = String(error?.message ?? error ?? "");
      if (!/undefined (?:variable|function)/i.test(message)) throw error;
      const suggestion = closestBuiltin(bioLangBuiltinName(name), this.builtins().map((builtin) => builtin.name));
      const suffix = suggestion ? ` Did you mean '${javascriptBuiltinName(suggestion)}'?` : "";
      throw new TypeError(`Unknown BioLang builtin or function '${name}'.${suffix}`, {
        cause: error,
      });
    }
  }

  #call(callback) {
    if (this.#disposed) throw new Error("This BioLang session has been disposed");
    if (this.#inCallback.length) {
      throw new Error("A JavaScript host callback cannot re-enter its BioLang session");
    }
    return this.#callInterop(callback);
  }

  #callInterop(callback) {
    if (this.#disposed) throw new Error("This BioLang session has been disposed");
    // The Rust fetch hook resolves `window.__blFetch` at call time. Reinstall
    // this session's bridge before every operation so another session cannot
    // silently change its cwd, network policy, or browser file provider.
    this.#activateBridge();
    globalThis.__blCallbacks = this.#callbackBridge;
    try {
      return callback();
    } catch (error) {
      if (error instanceof Error) throw error;
      const message = typeof error === "string"
        ? error
        : String(error?.message ?? error ?? "Unknown BioLang WASM error");
      throw new Error(message, { cause: error });
    }
  }

  #invokeHostCallback(name, rawArguments) {
    const registration = this.#callbacks.get(name);
    if (!registration) throw new Error(`JavaScript host callback '${name}' is not registered`);
    if (this.#inCallback.includes(name)) {
      throw new Error(`Recursive JavaScript host callback '${name}' is not supported`);
    }
    if (this.#inCallback.length >= 32) {
      throw new Error("JavaScript host callbacks exceeded the maximum nesting depth of 32");
    }
    const args = Array.from(rawArguments, (value) => decodeBioValue(value, this.#valueHost));
    validateCallbackArguments(name, args, registration.parameters);
    this.#inCallback.push(name);
    try {
      let result;
      try {
        result = registration.callback(...args);
      } catch (error) {
        throw new Error(
          `JavaScript host callback '${name}' failed: ${String(error?.message ?? error)}`,
          { cause: error },
        );
      }
      if (isPromiseLike(result)) {
        throw new TypeError(
          `JavaScript host callback '${name}' returned a Promise; callbacks must be synchronous`,
        );
      }
      validateCallbackReturn(name, result, registration.returns);
      return encodeBioValue(result);
    } finally {
      this.#inCallback.pop();
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
    const prepared = args.map((value) => this.#prepareArgument(value));
    const raw = this.#callInterop(
      () => this.#session.call_value(name, encodeBioValue(prepared), maximumInlineBytes),
    );
    return decodeBioValue(raw, this.#valueHost);
  }

  /** Invoke a BioLang function with explicit named arguments. */
  callNamed(name, positional = [], named = {}, options = {}) {
    if (!Array.isArray(positional)) throw new TypeError("callNamed positional arguments must be an array");
    if (!named || Object.getPrototypeOf(named) !== Object.prototype) {
      throw new TypeError("callNamed named arguments must be an ordinary object");
    }
    const maximumInlineBytes = options.maximumInlineBytes ?? DEFAULT_INLINE_BYTES;
    const preparedPositional = positional.map((value) => this.#prepareArgument(value));
    const preparedNamed = Object.fromEntries(
      Object.entries(named).map(([key, value]) => [key, this.#prepareArgument(value)]),
    );
    const raw = this.#callInterop(() => this.#session.call_value_named(
      name,
      encodeBioValue(preparedPositional),
      encodeBioValue(preparedNamed),
      maximumInlineBytes,
    ));
    return decodeBioValue(raw, this.#valueHost);
  }

  /** Call a JavaScript-transpiled BioLang function using BioLang named args. */
  callNamedFunction(callable, positional = [], named = {}) {
    if (typeof callable !== "function") throw new TypeError("callNamedFunction requires a function");
    if (!Array.isArray(positional)) throw new TypeError("callNamedFunction positional arguments must be an array");
    if (!named || Object.getPrototypeOf(named) !== Object.prototype) {
      throw new TypeError("callNamedFunction named arguments must be an ordinary object");
    }
    const parameters = callable.__biolangParameters;
    if (!Array.isArray(parameters)) throw new TypeError("function has no BioLang parameter metadata");
    const args = positional.slice();
    for (const [name, value] of Object.entries(named)) {
      const index = parameters.indexOf(name);
      if (index < 0) throw new TypeError(`unknown named argument '${name}'`);
      while (args.length <= index) args.push(undefined);
      args[index] = value;
    }
    return callable(...args);
  }

  /** Preserve BioLang's overloaded `+` semantics in direct JavaScript. */
  addValues(left, right) {
    if (Array.isArray(left) && Array.isArray(right)) return left.concat(right);
    if (typeof left === "string" && typeof right === "string") return left + right;
    if (typeof left === "number" && typeof right === "number") return left + right;
    if (typeof left === "bigint" && typeof right === "bigint") return left + right;
    if (typeof left === "bigint" && typeof right === "number") return Number(left) + right;
    if (typeof left === "number" && typeof right === "bigint") return left + Number(right);
    if (left instanceof BioSequenceValue && right instanceof BioSequenceValue && left.kind === right.kind) {
      return new BioSequenceValue(left.kind, left.data + right.data);
    }
    throw new TypeError(`cannot add ${directValueType(left)} and ${directValueType(right)}`);
  }

  /** Preserve structural BioLang equality rather than JavaScript identity. */
  equalValues(left, right) {
    return equalDirectValues(left, right);
  }

  /** Preserve BioLang indexing, including negative list/string indices. */
  indexValue(target, index) {
    if (typeof index === "bigint") index = Number(index);
    if (typeof index === "number" && Number.isInteger(index)) {
      const length = target instanceof BioTableValue
        ? target.length
        : target instanceof BioSequenceValue || target instanceof BioQualityValue
          ? target.length
          : Array.isArray(target) || typeof target === "string"
            ? target.length
            : null;
      if (length !== null) {
        const resolved = index < 0 ? length + index : index;
        if (resolved < 0 || resolved >= length) {
          throw new RangeError(`index ${resolved} out of bounds (len ${length})`);
        }
        if (target instanceof BioTableValue) return target.toRows()[resolved];
        if (target instanceof BioSequenceValue) return [...target.data][resolved];
        if (target instanceof BioQualityValue) return target.data[resolved];
        return [...target][resolved];
      }
    }
    if (typeof index === "string") {
      if (target instanceof BioTableValue) return target.column(index);
      if (target && typeof target === "object" && Object.hasOwn(target, index)) return target[index];
    }
    throw new TypeError(`cannot index ${directValueType(target)} with ${directValueType(index)}`);
  }

  /** Format a value exactly as a BioLang f-string field would. */
  formatValue(value, spec = "") {
    return this.#callInterop(() => this.#session.format_value(encodeBioValue(value), spec));
  }

  #prepareArgument(value) {
    if (typeof value === "function") {
      const builtin = this.#builtinReferences.get(value);
      if (builtin && this.supports(builtin)) {
        return this.getValue(builtin, { maximumInlineBytes: 0 });
      }
      let handle = this.#callbackHandles.get(value);
      if (handle) return handle;
      const name = `__js_callback_${++this.#callbackSequence}`;
      handle = this.registerFunction(name, {
        minimumArguments: value.length,
        maximumArguments: value.length,
      }, value);
      this.#callbackHandles.set(value, handle);
      return handle;
    }
    if (Array.isArray(value)) return value.map((item) => this.#prepareArgument(item));
    if (value && Object.getPrototypeOf(value) === Object.prototype) {
      return Object.fromEntries(
        Object.entries(value).map(([key, item]) => [key, this.#prepareArgument(item)]),
      );
    }
    return value;
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
    // Auto-generated callback names cannot collide with user-facing builtins
    // and may be registered while another callback is active. Avoid a
    // re-entrant catalog lookup here; Rust still rejects any real collision.
    if (!name.startsWith("__js_callback_") && this.supports(name)) {
      throw new Error(`Cannot replace BioLang builtin '${name}'`);
    }
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
    this.#callInterop(() => this.#session.register_host_function(
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
    const raw = this.#callInterop(() => this.#session.get_value(name, 0));
    return decodeBioValue(raw, this.#valueHost);
  }

  define(name, value) {
    const result = this.run(program(let_(name, value)));
    if (!result.ok) throw new Error(result.error || `Cannot define ${name}`);
    return ref(name);
  }

  ref(name) { return ref(name); }

  invoke(name, ...args) { return this.run(call(name, ...args)); }

  csvExpression(path, options) { return tableFromCsv(path, options); }
  tableExpression(rows) { return tableValue(rows); }
  sequence(value, kind = "dna") { return sequenceValue(value, kind); }
  matrixExpression(value) { return matrixValue(value); }

  reset() {
    this.#call(() => this.#session.reset());
    this.#builtins = null;
    this.#callbacks.clear();
    this.#callbackHandles = new WeakMap();
    this.#callbackSequence = 0;
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

  formatSource(source, indent = 4) {
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
    this.#disposed = true;
    this.#builtins = null;
    this.#callbacks.clear();
    const session = this.#session;
    // wasm-bindgen cannot take ownership of a Rust class while one of its
    // methods is still borrowed. `dispose()` can be reached from a synchronous
    // host callback, so release on the next microtask after that WASM frame has
    // returned instead of panicking inside `WasmSession.free()`.
    queueMicrotask(() => {
      try {
        session.free?.();
      } catch {
        // A trapped WASM call can leave wasm-bindgen's receiver borrow set.
        // The wrapper is already unusable and detached at this point; retain
        // the Rust allocation rather than turning cleanup into an uncaught
        // microtask exception that terminates the host process.
      }
    });
  }
}

function bioLangBuiltinName(name) {
  return name.replace(/[A-Z]/g, (letter) => `_${letter.toLowerCase()}`);
}

function hasSdkMember(target, property) {
  let owner = target;
  while (owner && owner !== Object.prototype) {
    if (Object.prototype.hasOwnProperty.call(owner, property)) return true;
    owner = Object.getPrototypeOf(owner);
  }
  return false;
}

function javascriptBuiltinName(name) {
  return name.replace(/_([a-z0-9])/g, (_, character) => character.toUpperCase());
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

function closestBuiltin(name, candidates) {
  let closest = null;
  let distance = Number.POSITIVE_INFINITY;
  for (const candidate of candidates) {
    const current = editDistance(name, candidate);
    if (current < distance || (current === distance && candidate < closest)) {
      closest = candidate;
      distance = current;
    }
  }
  return distance <= Math.max(2, Math.floor(name.length / 3)) ? closest : null;
}

function editDistance(left, right) {
  let previous = Array.from({ length: right.length + 1 }, (_, index) => index);
  for (let leftIndex = 0; leftIndex < left.length; leftIndex += 1) {
    const current = [leftIndex + 1];
    for (let rightIndex = 0; rightIndex < right.length; rightIndex += 1) {
      current.push(Math.min(
        current[rightIndex] + 1,
        previous[rightIndex + 1] + 1,
        previous[rightIndex] + (left[leftIndex] === right[rightIndex] ? 0 : 1),
      ));
    }
    previous = current;
  }
  return previous[right.length];
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
