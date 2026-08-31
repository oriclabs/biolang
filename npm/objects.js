import { BioExpression, call, lambda } from "./dsl.js";

/** A lazy table-valued BioLang expression. */
export class BioTable extends BioExpression {
  filter(predicate) {
    const condition = typeof predicate === "function" ? lambda("row", predicate) : predicate;
    return wrap(BioTable, call("filter", this, condition));
  }

  where(predicate) {
    if (!predicate || typeof predicate !== "object" || Array.isArray(predicate)) {
      throw new TypeError("where() requires an object predicate");
    }
    return this.filter((row) => predicateExpression(row, predicate));
  }

  column(name) { return wrap(BioColumn, call("col", this, name)); }
  select(...names) { return wrap(BioTable, call("select", this, names.flat())); }
  drop(...names) { return wrap(BioTable, call("drop_cols", this, names.flat())); }
  head(count = 6) { return wrap(BioTable, call("head", this, count)); }
  arrange(...names) { return wrap(BioTable, call("arrange", this, names.flat())); }
  count() { return wrap(BioScalar, call("nrow", this)); }
  summary() { return call("summary", this); }
}

/** A lazy numeric or categorical column expression. */
export class BioColumn extends BioExpression {
  mean() { return wrap(BioScalar, call("mean", this)); }
  median() { return wrap(BioScalar, call("median", this)); }
  mode() { return wrap(BioScalar, call("mode", this)); }
  min() { return wrap(BioScalar, call("min", this)); }
  max() { return wrap(BioScalar, call("max", this)); }
  sum() { return wrap(BioScalar, call("sum", this)); }
  variance() { return wrap(BioScalar, call("variance", this)); }
  stdev() { return wrap(BioScalar, call("stdev", this)); }
  quantile(probability) { return wrap(BioScalar, call("quantile", this, probability)); }
  histogram(options = {}) { return wrap(BioPlot, call("histogram", this, options)); }
  density(options = {}) { return wrap(BioPlot, call("density_plot", this, options)); }
  boxplot(options = {}) { return wrap(BioPlot, call("boxplot", this, options)); }
}

/** A scalar-valued BioLang expression. */
export class BioScalar extends BioExpression {}

/** A plot-valued BioLang expression. Rendering remains a host responsibility. */
export class BioPlot extends BioExpression {}

/** A DNA/RNA/protein-valued BioLang expression. */
export class BioSequence extends BioExpression {
  gcContent() { return wrap(BioScalar, call("gc_content", this)); }
  reverseComplement() { return wrap(BioSequence, call("reverse_complement", this)); }
  transcribe() { return wrap(BioSequence, call("transcribe", this)); }
  translate() { return wrap(BioSequence, call("translate", this)); }
  length() { return wrap(BioScalar, call("len", this)); }
}

/** A matrix-valued BioLang expression. */
export class BioMatrix extends BioExpression {
  transpose() { return wrap(BioMatrix, call("transpose", this)); }
  pca(options = {}) { return call("pca", this, options); }
  heatmap(options = {}) { return wrap(BioPlot, call("heatmap", this, options)); }
}

export function tableFromCsv(path, options) {
  const expression = options === undefined ? call("read_csv", path) : call("read_csv", path, options);
  return wrap(BioTable, expression);
}

export function tableValue(rows) {
  return wrap(BioTable, call("table", rows));
}

export function sequenceValue(value, kind = "dna") {
  if (!new Set(["dna", "rna", "protein"]).has(kind)) {
    throw new TypeError("sequence kind must be 'dna', 'rna', or 'protein'");
  }
  return wrap(BioSequence, call(kind, value));
}

export function matrixValue(value) {
  return wrap(BioMatrix, call("matrix", value));
}

function predicateExpression(row, predicate) {
  const conditions = [];
  for (const [field, rule] of Object.entries(predicate)) {
    const value = row.field(field);
    if (rule && typeof rule === "object" && !Array.isArray(rule)) {
      for (const [operator, operand] of Object.entries(rule)) {
        if (!new Set(["eq", "ne", "gt", "gte", "lt", "lte"]).has(operator)) {
          throw new TypeError(`Unsupported where() operator '${operator}'`);
        }
        conditions.push(value[operator](operand));
      }
    } else {
      conditions.push(value.eq(rule));
    }
  }
  if (!conditions.length) throw new TypeError("where() requires at least one condition");
  return conditions.slice(1).reduce((result, condition) => result.and(condition), conditions[0]);
}

function wrap(Type, expression) {
  return new Type(expression.source, expression.children);
}
