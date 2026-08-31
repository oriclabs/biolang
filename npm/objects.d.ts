import { BioExpression, type BioArgument, type BioRow } from "./dsl.js";

export type BioPredicate = Record<string, BioArgument | Partial<Record<"eq" | "ne" | "gt" | "gte" | "lt" | "lte", BioArgument>>>;

export class BioTable extends BioExpression {
  filter(predicate: BioArgument | ((row: BioRow) => BioExpression)): BioTable;
  where(predicate: BioPredicate): BioTable;
  column(name: string): BioColumn;
  select(...names: string[]): BioTable;
  drop(...names: string[]): BioTable;
  head(count?: number): BioTable;
  arrange(...names: string[]): BioTable;
  count(): BioScalar;
  summary(): BioExpression;
}

export class BioColumn extends BioExpression {
  mean(): BioScalar;
  median(): BioScalar;
  mode(): BioScalar;
  min(): BioScalar;
  max(): BioScalar;
  sum(): BioScalar;
  variance(): BioScalar;
  stdev(): BioScalar;
  quantile(probability: number): BioScalar;
  histogram(options?: Record<string, BioArgument>): BioPlot;
  density(options?: Record<string, BioArgument>): BioPlot;
  boxplot(options?: Record<string, BioArgument>): BioPlot;
}

export class BioScalar extends BioExpression {}
export class BioPlot extends BioExpression {}

export class BioSequence extends BioExpression {
  gcContent(): BioScalar;
  reverseComplement(): BioSequence;
  transcribe(): BioSequence;
  translate(): BioSequence;
  length(): BioScalar;
}

export class BioMatrix extends BioExpression {
  transpose(): BioMatrix;
  pca(options?: Record<string, BioArgument>): BioExpression;
  heatmap(options?: Record<string, BioArgument>): BioPlot;
}

export function tableFromCsv(path: string, options?: Record<string, BioArgument>): BioTable;
export function tableValue(rows: BioArgument[]): BioTable;
export function sequenceValue(value: string, kind?: "dna" | "rna" | "protein"): BioSequence;
export function matrixValue(value: BioArgument): BioMatrix;
