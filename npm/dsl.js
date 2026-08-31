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
  const values = args.map(expression);
  return new BioExpression(
    `${name}(${values.map((value) => value.source).join(", ")})`,
    values,
  );
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
  return new BioStatement(`return ${expression(value).source}`);
}

export function if_(condition, thenBranch, elseBranch) {
  const yes = blockSource(thenBranch);
  const no = elseBranch === undefined ? "" : ` else {\n${indent(blockSource(elseBranch))}\n}`;
  return new BioStatement(
    `if ${expression(condition).source} {\n${indent(yes)}\n}${no}`,
  );
}

export function for_(name, iterable, body) {
  assertIdentifier(name, "loop variable");
  return new BioStatement(
    `for ${name} in ${expression(iterable).source} {\n${indent(blockSource(body))}\n}`,
  );
}

export function while_(condition, body) {
  return new BioStatement(
    `while ${expression(condition).source} {\n${indent(blockSource(body))}\n}`,
  );
}

export function function_(name, parameters, body) {
  assertIdentifier(name, "function");
  if (!Array.isArray(parameters)) throw new TypeError("function_() parameters must be an array");
  parameters.forEach((parameter) => assertIdentifier(parameter, "function parameter"));
  return new BioStatement(
    `fn ${name}(${parameters.join(", ")}) {\n${indent(blockSource(body))}\n}`,
  );
}

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
      if (typeof property === "symbol" || property in object) {
        const value = Reflect.get(object, property, object);
        return typeof value === "function" ? value.bind(object) : value;
      }
      return object.field(property);
    },
  });
}

function binary(operator, left, right) {
  const lhs = expression(left);
  const rhs = expression(right);
  return new BioExpression(`(${lhs.source} ${operator} ${rhs.source})`, [lhs, rhs]);
}

function unary(operator, value) {
  const operand = expression(value);
  return new BioExpression(`(${operator} ${operand.source})`, [operand]);
}

function visit(node, seen) {
  if (!(node instanceof BioExpression) || seen.has(node)) return;
  seen.add(node);
  node.children.forEach((child) => visit(child, seen));
}

function statementSource(value) {
  if (value instanceof BioStatement || value instanceof BioProgram) return value.source;
  if (value instanceof BioExpression) return value.source;
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
