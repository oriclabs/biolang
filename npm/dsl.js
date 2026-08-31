/**
 * JavaScript construction layer for BioLang.
 *
 * These objects never evaluate scientific operations in JavaScript. They build
 * BioLang source which is parsed and executed by the existing WASM runtime (or
 * sent to SOMER). Keeping source as the interchange format gives this layer the
 * same parser, type rules and implementation as handwritten `.bl` programs.
 */

const IDENTIFIER = /^[A-Za-z_][A-Za-z0-9_]*$/;
let activeArena = null;

export class BioExpression {
  constructor(source, children = []) {
    this.source = source;
    this.children = children;
    activeArena?.add(this);
  }

  toBioLang() {
    return this.source;
  }

  pipe(...stages) {
    return stages.reduce((value, stage) => {
      const next = expression(stage);
      return new BioExpression(`(${value.source}) |> ${next.source}`, [value, next]);
    }, this);
  }

  field(name) {
    assertIdentifier(name, "field");
    return new BioExpression(`(${this.source}).${name}`, [this]);
  }

  at(index) {
    const key = expression(index);
    return new BioExpression(`(${this.source})[${key.source}]`, [this, key]);
  }

  eq(other) { return binary("==", this, other); }
  ne(other) { return binary("!=", this, other); }
  gt(other) { return binary(">", this, other); }
  gte(other) { return binary(">=", this, other); }
  lt(other) { return binary("<", this, other); }
  lte(other) { return binary("<=", this, other); }
  and(other) { return binary("and", this, other); }
  or(other) { return binary("or", this, other); }
  add(other) { return binary("+", this, other); }
  sub(other) { return binary("-", this, other); }
  mul(other) { return binary("*", this, other); }
  div(other) { return binary("/", this, other); }
  pow(other) { return binary("**", this, other); }
  not() { return unary("not", this); }
  neg() { return unary("-", this); }

  filter(predicate) { return call("filter", this, predicate); }
  column(name) { return call("col", this, name); }
  select(...names) { return call("select", this, names.flat()); }
  head(count = 6) { return call("head", this, count); }
  mean() { return call("mean", this); }
  median() { return call("median", this); }
  summary() { return call("summary", this); }
  histogram(options = {}) { return call("histogram", this, options); }

  run(executor, options) {
    if (!executor || typeof executor.run !== "function") {
      throw new TypeError("run() requires a BioLang session or SOMER executor");
    }
    return executor.run(this, options);
  }

  runOn(executor, options) {
    return this.run(executor, options);
  }

  [Symbol.toPrimitive]() {
    throw new TypeError(
      "BioLang expressions cannot use JavaScript operators directly at runtime. "
      + "Use .eq(), .gte(), .and(), and related methods, or enable the BioLang "
      + "JavaScript source transform.",
    );
  }
}

export class BioProgram {
  constructor(items) {
    this.items = items.flat().map(statementSource);
    this.source = this.items.join("\n");
  }

  toBioLang() {
    return this.source;
  }

  run(executor, options) {
    if (!executor || typeof executor.run !== "function") {
      throw new TypeError("run() requires a BioLang session or SOMER executor");
    }
    return executor.run(this, options);
  }

  runOn(executor, options) {
    return this.run(executor, options);
  }
}

export class BioStatement {
  constructor(source) {
    this.source = source;
  }

  toBioLang() {
    return this.source;
  }
}

export function raw(source) {
  if (typeof source !== "string") throw new TypeError("raw() requires a string");
  return new BioExpression(source);
}

export function ref(name) {
  assertIdentifier(name, "variable");
  return new BioExpression(name);
}

export function literal(value) {
  return expression(value);
}

export function call(name, ...args) {
  assertIdentifier(name, "builtin");
  return invokeSource(name, args);
}

export function lambda(parameter, build) {
  assertIdentifier(parameter, "lambda parameter");
  if (typeof build !== "function") throw new TypeError("lambda() requires a builder function");

  const previousArena = activeArena;
  const arena = new Set();
  activeArena = arena;
  let result;
  try {
    result = build(expressionProxy(new BioExpression(parameter)));
  } finally {
    activeArena = previousArena;
  }

  if (!(result instanceof BioExpression)) {
    throw new TypeError(
      "A BioLang lambda must return a BioLang expression. JavaScript operators "
      + "such as === and ! return booleans; use .eq() and .not(), or enable the source transform.",
    );
  }

  const reachable = new Set();
  visit(result, reachable);
  const orphaned = [...arena].filter((node) => !reachable.has(node));
  if (orphaned.length) {
    throw new TypeError(
      "A BioLang expression was constructed but discarded. This usually means "
      + "JavaScript && or || was used; use .and()/.or(), or enable the source transform.",
    );
  }

  return new BioExpression(`|${parameter}| ${result.source}`, [result]);
}

export function program(...items) {
  return new BioProgram(items);
}

export function let_(name, value) {
  assertIdentifier(name, "variable");
  return new BioStatement(`let ${name} = ${expression(value).source}`);
}

export function assign(name, value) {
  assertIdentifier(name, "variable");
  return new BioStatement(`${name} = ${expression(value).source}`);
}

export function return_(value) {
  return new BioStatement(value === undefined ? "return" : `return ${expression(value).source}`);
}

export function if_(condition, thenBranch, elseBranch) {
  const yes = blockSource(thenBranch);
  const no = elseBranch === undefined ? "" : ` else {\n${indent(blockSource(elseBranch))}\n}`;
  return new BioStatement(
    `if ${expression(condition).source} {\n${indent(yes)}\n}${no}`,
  );
}

export function while_(condition, body) {
  return new BioStatement(
    `while ${expression(condition).source} {\n${indent(blockSource(body))}\n}`,
  );
}

export function function_(name, parameters, body, options = {}) {
  assertIdentifier(name, "function");
  if (!Array.isArray(parameters)) throw new TypeError("function_() parameters must be an array");
  const prefix = `${options.async ? "async " : ""}fn${options.generator ? "*" : ""}`;
  return new BioStatement(
    `${prefix} ${name}(${parameters.map(parameterSource).join(", ")}) {\n${indent(blockSource(body))}\n}`,
  );
}

// Structural builders used by the BioLang-to-JavaScript frontend. They are
// deliberately explicit: generated JavaScript contains no hidden BioLang
// source string and every AST edge remains visible and inspectable.
class BioCallArgument {
  constructor(kind, value, name = null) { this.kind = kind; this.value = value; this.name = name; }
}
class BioRecordEntry {
  constructor(kind, value, name = null) { this.kind = kind; this.value = value; this.name = name; }
}
class BioParameter {
  constructor(name, options = {}) { this.name = name; this.options = options; }
}

export function expr_(value) { return new BioStatement(expression(value).source); }
export function const_(name, value) {
  assertIdentifier(name, "constant");
  return new BioStatement(`const ${name} = ${expression(value).source}`);
}
export function indexAssign(name, indexValue, value) {
  assertIdentifier(name, "variable");
  return new BioStatement(`${name}[${expression(indexValue).source}] = ${expression(value).source}`);
}
export function break_() { return new BioStatement("break"); }
export function continue_() { return new BioStatement("continue"); }
export function yield_(value) { return new BioStatement(`yield ${expression(value).source}`); }
export function defer_(value) { return new BioStatement(`defer ${expression(value).source}`); }
export function assert_(condition, message = null) {
  const suffix = message === null ? "" : `, ${expression(message).source}`;
  return new BioStatement(`assert ${expression(condition).source}${suffix}`);
}
export function import_(path, alias = null) {
  const suffix = alias === null ? "" : ` as ${identifierSource(alias, "module alias")}`;
  return new BioStatement(`import ${quote(path)}${suffix}`);
}
export function fromImport(path, names) {
  names.forEach((name) => assertIdentifier(name, "import name"));
  return new BioStatement(`from ${quote(path)} import ${names.join(", ")}`);
}
export function nilAssign(name, value) {
  assertIdentifier(name, "variable");
  return new BioStatement(`${name} ?= ${expression(value).source}`);
}

export function unary(operator, value) {
  if (!["-", "!", "not"].includes(operator)) throw new TypeError(`Unsupported unary operator '${operator}'`);
  const operand = expression(value);
  return new BioExpression(`${operator}${operator === "not" ? " " : ""}(${operand.source})`, [operand]);
}
export function binary(operator, left, right) {
  const allowed = ["+", "-", "*", "/", "%", "**", "==", "!=", "<", ">", "<=", ">=", "&&", "||", "and", "or", "&", "^", "<<", ">>", "++"];
  if (!allowed.includes(operator)) throw new TypeError(`Unsupported binary operator '${operator}'`);
  const spelling = operator === "&&" ? "and" : operator === "||" ? "or" : operator;
  const lhs = expression(left);
  const rhs = expression(right);
  return new BioExpression(`(${lhs.source} ${spelling} ${rhs.source})`, [lhs, rhs]);
}
export function pipe(left, right) { return new BioExpression(`(${expression(left).source}) |> ${expression(right).source}`); }
export function tapPipe(left, right) { return new BioExpression(`(${expression(left).source}) |>> ${expression(right).source}`); }
export function pipeInto(value, name) {
  assertIdentifier(name, "pipe binding");
  return new BioExpression(`(${expression(value).source}) |> into ${name}`);
}
export function field(object, name, optional = false) {
  assertIdentifier(name, "field");
  return new BioExpression(`(${expression(object).source})${optional ? "?." : "."}${name}`);
}
export function index(object, key) { return new BioExpression(`(${expression(object).source})[${expression(key).source}]`); }
export function slice(object, start = null, end = null, step = null) {
  const part = (value) => value === null ? "" : expression(value).source;
  return new BioExpression(`(${expression(object).source})[${part(start)}:${part(end)}${step === null ? "" : `:${part(step)}`}]`);
}
export function named(name, value) {
  assertIdentifier(name, "argument name");
  return new BioCallArgument("named", value, name);
}
export function spread(value) { return new BioCallArgument("spread", value); }
export function callExpr(name, args = []) {
  assertIdentifier(name, "function");
  return invokeSource(name, args);
}
export function invoke(callee, args = []) {
  if (typeof callee === "string") {
    assertIdentifier(callee, "function");
    return invokeSource(callee, args);
  }
  return invokeSource(expression(callee).source, args);
}
export function lambdaExpr(parameters, body) {
  const rendered = parameters.map(parameterSource);
  // This is the structural transpiler path, not a JavaScript callback. There
  // is no callback arena to audit; emit_expr has already made every AST edge
  // explicit. Authored callback lambdas should use lambda().
  const result = expression(body);
  return new BioExpression(`|${rendered.join(", ")}| ${result.source}`, [result]);
}
export function blockExpr(body) { return new BioExpression(`{\n${indent(blockSource(body))}\n}`); }
export function ifExpr(condition, thenBody, elseBody = null) {
  const suffix = elseBody === null ? "" : ` else {\n${indent(blockSource(elseBody))}\n}`;
  return new BioExpression(`if ${expression(condition).source} {\n${indent(blockSource(thenBody))}\n}${suffix}`);
}
export function fieldEntry(name, value) { return new BioRecordEntry("field", value, name); }
export function spreadEntry(value) { return new BioRecordEntry("spread", value); }
export function record(entries) {
  const rendered = entries.map((entry) => {
    if (!(entry instanceof BioRecordEntry)) throw new TypeError("record() entries must use fieldEntry() or spreadEntry()");
    return entry.kind === "spread" ? `...${expression(entry.value).source}` : `${recordKey(entry.name)}: ${expression(entry.value).source}`;
  });
  return new BioExpression(`{${rendered.join(", ")}}`);
}
export function formula(value) { return new BioExpression(`~${expression(value).source}`); }
export function coalesce(left, right) { return new BioExpression(`(${expression(left).source}) ?? (${expression(right).source})`); }
export function range(start, end, options = {}) { return new BioExpression(`${expression(start).source}..${options.inclusive ? "=" : ""}${expression(end).source}`); }
export function ternary(condition, value, fallback) { return new BioExpression(`${expression(value).source} if ${expression(condition).source} else ${expression(fallback).source}`); }
export function in_(left, right, options = {}) { return new BioExpression(`(${expression(left).source}) ${options.negated ? "not in" : "in"} (${expression(right).source})`); }
export function cast(value, target) { return new BioExpression(`(${expression(value).source}) as ${identifierSource(target, "type")}`); }
export function tuple(values) { return new BioExpression(`(${values.map((value) => expression(value).source).join(", ")}${values.length === 1 ? "," : ""})`); }
export function set(values) { return new BioExpression(`#{${values.map((value) => expression(value).source).join(", ")}}`); }
export function dna(value) { return new BioExpression(`dna${quote(value)}`); }
export function rna(value) { return new BioExpression(`rna${quote(value)}`); }
export function protein(value) { return new BioExpression(`protein${quote(value)}`); }
export function quality(value) { return new BioExpression(`qual${quote(value)}`); }
export function param(name, options = {}) { assertIdentifier(name, "parameter"); return new BioParameter(name, options); }

export function for_(pattern, iterable, body, options = {}) {
  const guard = options.when === undefined ? "" : ` when ${expression(options.when).source}`;
  const fallback = options.elseBody === undefined ? "" : ` else {\n${indent(blockSource(options.elseBody))}\n}`;
  return new BioStatement(`for ${patternSource(pattern)} in ${expression(iterable).source}${guard} {\n${indent(blockSource(body))}\n}${fallback}`);
}
export function listPattern(names) { return { kind: "list", names }; }
export function recordPattern(names) { return { kind: "record", names }; }
export function tuplePattern(names) { return { kind: "tuple", names }; }

function invokeSource(callee, args) {
  const children = [];
  const rendered = args.map((arg) => {
    if (arg instanceof BioCallArgument) {
      const value = expression(arg.value);
      children.push(value);
      if (arg.kind === "spread") return `...${value.source}`;
      return `${arg.name}: ${value.source}`;
    }
    const value = expression(arg);
    children.push(value);
    return value.source;
  });
  return new BioExpression(`${callee}(${rendered.join(", ")})`, children);
}
function parameterSource(parameter) {
  if (typeof parameter === "string") return identifierSource(parameter, "parameter");
  if (!(parameter instanceof BioParameter)) throw new TypeError("parameters must use param()");
  const rest = parameter.options.rest ? "..." : "";
  const fallback = parameter.options.default === undefined ? "" : ` = ${expression(parameter.options.default).source}`;
  return `${rest}${parameter.name}${fallback}`;
}
function patternSource(pattern) {
  if (typeof pattern === "string") return identifierSource(pattern, "loop variable");
  pattern.names.forEach((name) => assertIdentifier(name, "pattern name"));
  const delimiters = pattern.kind === "record" ? ["{", "}"] : pattern.kind === "tuple" ? ["(", ")"] : ["[", "]"];
  return `${delimiters[0]}${pattern.names.join(", ")}${delimiters[1]}`;
}
function identifierSource(value, label) { assertIdentifier(value, label); return value; }

/**
 * Return BioLang source for execution. A string is treated as trusted raw
 * BioLang source, not as a data value; use literal() for untrusted strings.
 */
export function sourceOf(value) {
  if (typeof value === "string") return value;
  if (value && typeof value.toBioLang === "function") return value.toBioLang();
  if (value instanceof BioExpression || value instanceof BioProgram || value instanceof BioStatement) {
    return value.source;
  }
  throw new TypeError("Expected BioLang source, expression, statement, or program");
}

function expression(value) {
  if (value instanceof BioExpression) return value;
  if (value instanceof BioProgram || value instanceof BioStatement) {
    throw new TypeError("A statement or program cannot be used where an expression is required");
  }
  if (value === null || value === undefined) return new BioExpression("nil");
  if (typeof value === "boolean") return new BioExpression(value ? "true" : "false");
  if (typeof value === "number") {
    if (!Number.isFinite(value)) throw new TypeError("BioLang numeric literals must be finite");
    return new BioExpression(Object.is(value, -0) ? "-0.0" : String(value));
  }
  if (typeof value === "bigint") return new BioExpression(value.toString());
  if (typeof value === "string") return new BioExpression(quote(value));
  if (Array.isArray(value)) {
    const items = value.map(expression);
    return new BioExpression(`[${items.map((item) => item.source).join(", ")}]`, items);
  }
  if (isPlainObject(value)) {
    const entries = Object.entries(value).map(([key, item]) => [key, expression(item)]);
    return new BioExpression(
      `{${entries.map(([key, item]) => `${recordKey(key)}: ${item.source}`).join(", ")}}`,
      entries.map(([, item]) => item),
    );
  }
  throw new TypeError(`Cannot encode ${Object.prototype.toString.call(value)} as a BioLang literal`);
}

function expressionProxy(target) {
  return new Proxy(target, {
    get(object, property, receiver) {
      if (property === "then") return undefined;
      if (property === Symbol.iterator) {
        throw new TypeError("BioLang expressions cannot be spread or destructured");
      }
      if (typeof property === "symbol") {
        const value = Reflect.get(object, property, object);
        return typeof value === "function" ? value.bind(object) : value;
      }
      if (property in object) {
        const value = Reflect.get(object, property, object);
        if (typeof value !== "function") throw fieldCollision(property);
        const bound = value.bind(object);
        return new Proxy(bound, {
          get(method, member, receiver) {
            if (typeof member === "symbol" || member in method) return Reflect.get(method, member, receiver);
            throw fieldCollision(property);
          },
        });
      }
      return object.field(property);
    },
  });
}

function fieldCollision(name) {
  return new TypeError(
    `Column '${name}' collides with a BioLang expression builder property; use .field(${JSON.stringify(name)})`,
  );
}

function visit(node, seen) {
  if (!(node instanceof BioExpression) || seen.has(node)) return;
  seen.add(node);
  node.children.forEach((child) => visit(child, seen));
}

function statementSource(value) {
  if (value instanceof BioStatement || value instanceof BioProgram) return value.source;
  if (value instanceof BioExpression) return value.source;
  // Strings here are trusted BioLang statements. Data strings must first pass
  // through literal(), call(), or another builder that applies quote().
  if (typeof value === "string") return value;
  throw new TypeError("program() accepts expressions, statements, programs, and raw source strings");
}

function blockSource(value) {
  const values = Array.isArray(value) ? value : [value];
  return values.flat().map(statementSource).join("\n");
}

function indent(source) {
  return source.split("\n").map((line) => `  ${line}`).join("\n");
}

function recordKey(key) {
  return IDENTIFIER.test(key) ? key : quote(key);
}

function quote(value) {
  let output = '"';
  for (const character of value) {
    switch (character) {
      case "\\": output += "\\\\"; break;
      case '"': output += '\\"'; break;
      case "\n": output += "\\n"; break;
      case "\r": output += "\\r"; break;
      case "\t": output += "\\t"; break;
      case "\0": output += "\\0"; break;
      default: {
        const code = character.codePointAt(0);
        output += code < 0x20 || code === 0x7f ? `\\u{${code.toString(16)}}` : character;
      }
    }
  }
  return `${output}"`;
}

function assertIdentifier(value, label) {
  if (typeof value !== "string" || !IDENTIFIER.test(value)) {
    throw new TypeError(`${label} name '${String(value)}' is not a valid BioLang identifier`);
  }
}

function isPlainObject(value) {
  if (value === null || typeof value !== "object") return false;
  const prototype = Object.getPrototypeOf(value);
  return prototype === Object.prototype || prototype === null;
}
