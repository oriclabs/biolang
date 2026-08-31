export type BioPrimitive = null | undefined | boolean | number | bigint | string;
export type BioArgument =
  | BioPrimitive
  | BioExpression
  | BioArgument[]
  | { [key: string]: BioArgument };
export type BioRow = BioExpression & { readonly [field: string]: BioExpression };

export class BioExpression {
  readonly source: string;
  toBioLang(): string;
  pipe(...stages: BioArgument[]): BioExpression;
  field(name: string): BioExpression;
  at(index: BioArgument): BioExpression;
  eq(other: BioArgument): BioExpression;
  ne(other: BioArgument): BioExpression;
  gt(other: BioArgument): BioExpression;
  gte(other: BioArgument): BioExpression;
  lt(other: BioArgument): BioExpression;
  lte(other: BioArgument): BioExpression;
  and(other: BioArgument): BioExpression;
  or(other: BioArgument): BioExpression;
  add(other: BioArgument): BioExpression;
  sub(other: BioArgument): BioExpression;
  mul(other: BioArgument): BioExpression;
  div(other: BioArgument): BioExpression;
  pow(other: BioArgument): BioExpression;
  not(): BioExpression;
  neg(): BioExpression;
  filter(predicate: BioArgument): BioExpression;
  column(name: string): BioExpression;
  select(...names: string[]): BioExpression;
  head(count?: number): BioExpression;
  mean(): BioExpression;
  median(): BioExpression;
  summary(): BioExpression;
  histogram(options?: Record<string, BioArgument>): BioExpression;
  run(executor: { run(source: BioExpression, options?: unknown): unknown }, options?: unknown): unknown;
  runOn(executor: { run(source: BioExpression, options?: unknown): unknown }, options?: unknown): unknown;
}

export class BioProgram {
  readonly source: string;
  toBioLang(): string;
  run(executor: { run(source: BioProgram, options?: unknown): unknown }, options?: unknown): unknown;
  runOn(executor: { run(source: BioProgram, options?: unknown): unknown }, options?: unknown): unknown;
}

export class BioStatement {
  readonly source: string;
  toBioLang(): string;
}

export function raw(source: string): BioExpression;
export function ref(name: string): BioExpression;
export function literal(value: BioArgument): BioExpression;
export function call(name: string, ...args: BioArgument[]): BioExpression;
export function lambda(parameter: string, build: (value: BioRow) => BioExpression): BioExpression;
/** String items are trusted raw BioLang statements; use literal() for data. */
export function program(...items: Array<BioExpression | BioStatement | BioProgram | string | Array<BioExpression | BioStatement>>): BioProgram;
export function let_(name: string, value: BioArgument): BioStatement;
export function assign(name: string, value: BioArgument): BioStatement;
export function return_(value?: BioArgument): BioStatement;
export function if_(condition: BioArgument, thenBranch: unknown, elseBranch?: unknown): BioStatement;
export function for_(pattern: string | object, iterable: BioArgument, body: unknown, options?: object): BioStatement;
export function while_(condition: BioArgument, body: unknown): BioStatement;
export function function_(name: string, parameters: Array<string | object>, body: unknown, options?: object): BioStatement;
export function expr_(value: BioArgument): BioStatement;
export function const_(name: string, value: BioArgument): BioStatement;
export function indexAssign(name: string, index: BioArgument, value: BioArgument): BioStatement;
export function break_(): BioStatement;
export function continue_(): BioStatement;
export function yield_(value: BioArgument): BioStatement;
export function defer_(value: BioArgument): BioStatement;
export function assert_(condition: BioArgument, message?: BioArgument): BioStatement;
export function import_(path: string, alias?: string | null): BioStatement;
export function fromImport(path: string, names: string[]): BioStatement;
export function nilAssign(name: string, value: BioArgument): BioStatement;
export function unary(operator: string, value: BioArgument): BioExpression;
export function binary(operator: string, left: BioArgument, right: BioArgument): BioExpression;
export function pipe(left: BioArgument, right: BioArgument): BioExpression;
export function tapPipe(left: BioArgument, right: BioArgument): BioExpression;
export function pipeInto(value: BioArgument, name: string): BioExpression;
export function field(object: BioArgument, name: string, optional?: boolean): BioExpression;
export function index(object: BioArgument, key: BioArgument): BioExpression;
export function slice(object: BioArgument, start?: BioArgument, end?: BioArgument, step?: BioArgument): BioExpression;
export function named(name: string, value: BioArgument): object;
export function spread(value: BioArgument): object;
export function callExpr(name: string, args?: unknown[]): BioExpression;
export function invoke(callee: string | BioExpression, args?: unknown[]): BioExpression;
/** Structural frontend helper; authored callbacks should use lambda(). */
export function lambdaExpr(parameters: unknown[], body: BioArgument): BioExpression;
export function blockExpr(body: unknown): BioExpression;
export function ifExpr(condition: BioArgument, thenBody: unknown, elseBody?: unknown): BioExpression;
export function tryCatch(body: unknown, errorVariable: string | null, catchBody: unknown): BioExpression;
export function stringText(value: string): object;
export function stringValue(value: BioArgument): object;
export function stringFormatted(value: BioArgument, spec: string): object;
export function stringInterp(parts: object[]): BioExpression;
export function wildcardPattern(): object;
export function literalPattern(value: BioPrimitive): object;
export function identPattern(name: string): object;
export function enumPattern(name: string, bindings?: string[]): object;
export function typePattern(name: string, binding?: string | null): object;
export function orPattern(patterns: object[]): object;
export function matchArm(pattern: object, body: BioArgument, guard?: BioArgument | null): object;
export function matchExpr(value: BioArgument, arms: object[]): BioExpression;
export function fieldEntry(name: string, value: BioArgument): object;
export function spreadEntry(value: BioArgument): object;
export function record(entries: object[]): BioExpression;
export function formula(value: BioArgument): BioExpression;
export function coalesce(left: BioArgument, right: BioArgument): BioExpression;
export function range(start: BioArgument, end: BioArgument, options?: object): BioExpression;
export function ternary(condition: BioArgument, value: BioArgument, fallback: BioArgument): BioExpression;
export function in_(left: BioArgument, right: BioArgument, options?: object): BioExpression;
export function cast(value: BioArgument, target: string): BioExpression;
export function tuple(values: BioArgument[]): BioExpression;
export function set(values: BioArgument[]): BioExpression;
export function dna(value: string): BioExpression;
export function rna(value: string): BioExpression;
export function protein(value: string): BioExpression;
export function quality(value: string): BioExpression;
export function param(name: string, options?: object): object;
export function listPattern(names: string[]): object;
export function recordPattern(names: string[]): object;
export function tuplePattern(names: string[]): object;
/** String values are trusted raw BioLang source; use literal() for data. */
export function sourceOf(value: string | BioExpression | BioStatement | BioProgram): string;
