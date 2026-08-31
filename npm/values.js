const TYPE_KEY = "__biolangType";

/** A small BioLang table copied into JavaScript. */
export class BioTableValue {
  constructor(columns, rows) {
    this.columns = Object.freeze([...columns]);
    this.rows = rows.map((row) => [...row]);
  }

  get length() { return this.rows.length; }

  toRows() {
    return this.rows.map((row) => Object.fromEntries(
      this.columns.map((column, index) => [column, row[index] ?? null]),
    ));
  }

  column(name) {
    const index = this.columns.indexOf(name);
    if (index < 0) throw new Error(`Unknown table column '${name}'`);
    return this.rows.map((row) => row[index] ?? null);
  }
}

/** A small dense row-major matrix copied into JavaScript. */
export class BioMatrixValue {
  constructor({ nrow, ncol, data, rowNames = null, columnNames = null }) {
    if (!(data instanceof Float64Array)) throw new TypeError("matrix data must be Float64Array");
    if (data.length !== nrow * ncol) throw new RangeError("matrix shape does not match its data");
    this.nrow = nrow;
    this.ncol = ncol;
    this.data = data;
    this.rowNames = rowNames;
    this.columnNames = columnNames;
  }

  get shape() { return [this.nrow, this.ncol]; }
  at(row, column) { return this.data[row * this.ncol + column]; }
  row(index) { return this.data.slice(index * this.ncol, (index + 1) * this.ncol); }
}

/** A DNA, RNA, or protein value with its biological kind preserved. */
export class BioSequenceValue {
  constructor(kind, data) {
    this.kind = kind;
    this.data = data;
  }
  get length() { return this.data.length; }
  toString() { return this.data; }
}

/** A Phred-score vector copied into JavaScript. */
export class BioQualityValue {
  constructor(data) {
    if (!(data instanceof Uint8Array)) throw new TypeError("quality data must be Uint8Array");
    this.data = data;
  }
  get length() { return this.data.length; }
}

/** A BioLang integer range. */
export class BioRangeValue {
  constructor(start, end, inclusive = false) {
    this.start = start;
    this.end = end;
    this.inclusive = inclusive;
  }
}

/** A genomic interval with coordinate and strand semantics preserved. */
export class BioIntervalValue {
  constructor(chrom, start, end, strand = ".") {
    this.chrom = chrom;
    this.start = start;
    this.end = end;
    this.strand = strand;
  }
}

/** A user-defined BioLang enum value. */
export class BioEnumValue {
  constructor(enumName, variant, fields = []) {
    this.enumName = enumName;
    this.variant = variant;
    this.fields = fields;
  }
}

/** A large or non-copyable value retained by its owning Rust session. */
export class BioValueHandle {
  #host;
  #disposed = false;

  constructor(descriptor, host) {
    this.session = descriptor.session;
    this.id = descriptor.id;
    this.generation = descriptor.generation;
    this.valueType = descriptor.valueType;
    this.length = descriptor.length ?? null;
    this.rows = descriptor.rows ?? null;
    this.columns = descriptor.columns ?? null;
    this.nonZero = descriptor.nonZero ?? null;
    this.#host = host;
  }

  get disposed() { return this.#disposed; }
  get shape() { return this.rows == null ? null : [this.rows, this.columns]; }

  page(options = {}) {
    this.#assertLive();
    return this.#host.decode(this.#host.page(
      this.id,
      this.generation,
      options.offset ?? 0,
      options.limit ?? 100,
    ));
  }

  toFloat64Array() {
    this.#assertLive();
    return this.#host.float64(this.id, this.generation);
  }

  dispose() {
    if (this.#disposed) return false;
    this.#disposed = true;
    return this.#host.release(this.id, this.generation);
  }

  _descriptor() {
    this.#assertLive();
    return {
      [TYPE_KEY]: "handle",
      session: this.session,
      id: this.id,
      generation: this.generation,
    };
  }

  #assertLive() {
    if (this.#disposed) throw new Error("BioLang value handle has been disposed");
  }
}

/** Decode the tagged objects emitted directly by the Rust/WASM boundary. */
export function decodeBioValue(value, host = null) {
  if (value === null || value === undefined || typeof value !== "object") return value;
  if (Array.isArray(value)) return value.map((item) => decodeBioValue(item, host));
  if (ArrayBuffer.isView(value)) return value;

  switch (value[TYPE_KEY]) {
    case "int64": return BigInt(value.value);
    case "table": return new BioTableValue(
      value.columns,
      value.rows.map((row) => row.map((item) => decodeBioValue(item, host))),
    );
    case "matrix": return new BioMatrixValue({
      nrow: value.nrow,
      ncol: value.ncol,
      data: value.data,
      rowNames: value.rowNames ?? null,
      columnNames: value.columnNames ?? null,
    });
    case "sequence": return new BioSequenceValue(value.sequenceKind, value.data);
    case "quality": return new BioQualityValue(value.data);
    case "record": {
      const result = Object.create(null);
      for (const [key, item] of Object.entries(value.entries)) {
        result[key] = decodeBioValue(item, host);
      }
      return result;
    }
    case "map": return new Map(Object.entries(value.entries).map(
      ([key, item]) => [key, decodeBioValue(item, host)],
    ));
    case "set": return new Set(value.values.map((item) => decodeBioValue(item, host)));
    case "tuple": return Object.freeze(value.values.map((item) => decodeBioValue(item, host)));
    case "range": return new BioRangeValue(
      decodeBioValue(value.start, host), decodeBioValue(value.end, host), value.inclusive,
    );
    case "interval": return new BioIntervalValue(
      value.chrom, decodeBioValue(value.start, host),
      decodeBioValue(value.end, host), value.strand,
    );
    case "enum": return new BioEnumValue(
      value.enumName, value.variant,
      value.fields.map((item) => decodeBioValue(item, host)),
    );
    case "regex": return new RegExp(value.pattern, value.flags);
    case "handle": {
      if (!host) throw new Error("A BioLang handle requires its owning session");
      return new BioValueHandle(value, host);
    }
    default: {
      if (Object.hasOwn(value, TYPE_KEY)) {
        throw new TypeError(`Unsupported BioLang value tag '${value[TYPE_KEY]}'`);
      }
      const result = Object.create(null);
      for (const [key, item] of Object.entries(value)) {
        result[key] = decodeBioValue(item, host);
      }
      return result;
    }
  }
}

/** Convert supported JavaScript data into the explicit WASM interop schema. */
export function encodeBioValue(value, seen = new WeakSet()) {
  if (value === null || value === undefined) return null;
  if (["string", "boolean", "number"].includes(typeof value)) return value;
  if (typeof value === "bigint") return { [TYPE_KEY]: "int64", value: value.toString() };
  if (typeof value === "function" || typeof value === "symbol") {
    throw new TypeError(`Cannot marshal JavaScript ${typeof value} as a BioLang value`);
  }
  if (typeof value !== "object") throw new TypeError("Unsupported JavaScript value");
  if (seen.has(value)) throw new TypeError("Cannot marshal a cyclic JavaScript value");

  if (value instanceof BioValueHandle) return value._descriptor();
  if (value instanceof BioSequenceValue) {
    return { [TYPE_KEY]: "sequence", sequenceKind: value.kind, data: value.data };
  }
  if (value instanceof BioQualityValue) {
    return { [TYPE_KEY]: "quality", data: value.data };
  }
  if (value instanceof BioRangeValue) {
    return {
      [TYPE_KEY]: "range", start: encodeBioValue(value.start, seen),
      end: encodeBioValue(value.end, seen), inclusive: value.inclusive,
    };
  }
  if (value instanceof BioIntervalValue) {
    return {
      [TYPE_KEY]: "interval", chrom: value.chrom,
      start: encodeBioValue(value.start, seen), end: encodeBioValue(value.end, seen),
      strand: value.strand,
    };
  }
  if (value instanceof BioEnumValue) {
    seen.add(value);
    const fields = value.fields.map((item) => encodeBioValue(item, seen));
    seen.delete(value);
    return {
      [TYPE_KEY]: "enum", enumName: value.enumName, variant: value.variant, fields,
    };
  }
  if (value instanceof BioTableValue) {
    seen.add(value);
    const encoded = {
      [TYPE_KEY]: "table",
      columns: [...value.columns],
      rows: value.rows.map((row) => row.map((item) => encodeBioValue(item, seen))),
    };
    seen.delete(value);
    return encoded;
  }
  if (value instanceof BioMatrixValue) {
    return {
      [TYPE_KEY]: "matrix", nrow: value.nrow, ncol: value.ncol,
      data: value.data, rowNames: value.rowNames, columnNames: value.columnNames,
    };
  }
  if (value instanceof Uint8Array) {
    throw new TypeError(
      "Uint8Array has no unambiguous BioLang type; wrap Phred scores in BioQualityValue or pass Array.from(value)",
    );
  }
  if (value instanceof Float64Array) {
    throw new TypeError(
      "Float64Array has no persistent BioLang vector type; pass Array.from(value) or wrap matrix data in BioMatrixValue",
    );
  }
  if (value instanceof RegExp) {
    return { [TYPE_KEY]: "regex", pattern: value.source, flags: value.flags };
  }
  if (value instanceof Map) {
    seen.add(value);
    const entries = Object.create(null);
    for (const [key, item] of value) {
      if (typeof key !== "string") throw new TypeError("BioLang Map keys must be strings");
      entries[key] = encodeBioValue(item, seen);
    }
    seen.delete(value);
    return { [TYPE_KEY]: "map", entries };
  }
  if (value instanceof Set) {
    seen.add(value);
    const values = [...value].map((item) => encodeBioValue(item, seen));
    seen.delete(value);
    return { [TYPE_KEY]: "set", values };
  }
  if (Array.isArray(value)) {
    seen.add(value);
    const values = value.map((item) => encodeBioValue(item, seen));
    seen.delete(value);
    return values;
  }
  if (Object.getPrototypeOf(value) !== Object.prototype && Object.getPrototypeOf(value) !== null) {
    throw new TypeError(`Cannot marshal JavaScript ${value.constructor?.name ?? "object"}`);
  }
  seen.add(value);
  const entries = Object.create(null);
  for (const [key, item] of Object.entries(value)) entries[key] = encodeBioValue(item, seen);
  seen.delete(value);
  return { [TYPE_KEY]: "record", entries };
}

export function isPromiseLike(value) {
  return value != null && (typeof value === "object" || typeof value === "function")
    && typeof value.then === "function";
}
