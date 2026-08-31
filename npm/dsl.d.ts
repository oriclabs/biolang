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
export function program(...items: Array<BioExpression | BioStatement | BioProgram | string | Array<BioExpression | BioStatement>>): BioProgram;
export function let_(name: string, value: BioArgument): BioStatement;
export function assign(name: string, value: BioArgument): BioStatement;
export function return_(value: BioArgument): BioStatement;
export function if_(condition: BioArgument, thenBranch: unknown, elseBranch?: unknown): BioStatement;
export function for_(name: string, iterable: BioArgument, body: unknown): BioStatement;
export function while_(condition: BioArgument, body: unknown): BioStatement;
export function function_(name: string, parameters: string[], body: unknown): BioStatement;
export function sourceOf(value: string | BioExpression | BioStatement | BioProgram): string;
