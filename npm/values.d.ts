export type BioInteropScalar = null | boolean | number | bigint | string;
export type BioJsValue = BioInteropScalar | BioJsValue[] | { [key: string]: BioJsValue }
  | Map<string, BioJsValue> | Set<BioJsValue> | BioTableValue | BioMatrixValue
  | BioSequenceValue | BioQualityValue | BioRangeValue | BioIntervalValue
  | BioEnumValue | BioValueHandle;

export class BioTableValue {
  readonly columns: readonly string[];
  readonly rows: BioJsValue[][];
  constructor(columns: string[], rows: BioJsValue[][]);
  readonly length: number;
  toRows(): Array<Record<string, BioJsValue>>;
  column(name: string): BioJsValue[];
}

export class BioMatrixValue {
  readonly nrow: number;
  readonly ncol: number;
  readonly data: Float64Array;
  readonly rowNames: string[] | null;
  readonly columnNames: string[] | null;
  constructor(value: {
    nrow: number; ncol: number; data: Float64Array;
    rowNames?: string[] | null; columnNames?: string[] | null;
  });
  readonly shape: [number, number];
  at(row: number, column: number): number;
  row(index: number): Float64Array;
}

export class BioSequenceValue {
  readonly kind: "dna" | "rna" | "protein";
  readonly data: string;
  constructor(kind: "dna" | "rna" | "protein", data: string);
  readonly length: number;
  toString(): string;
}

export class BioQualityValue {
  readonly data: Uint8Array;
  constructor(data: Uint8Array);
  readonly length: number;
}

export class BioRangeValue {
  readonly start: number | bigint;
  readonly end: number | bigint;
  readonly inclusive: boolean;
  constructor(start: number | bigint, end: number | bigint, inclusive?: boolean);
}

export class BioIntervalValue {
  readonly chrom: string;
  readonly start: number | bigint;
  readonly end: number | bigint;
  readonly strand: string;
  constructor(chrom: string, start: number | bigint, end: number | bigint, strand?: string);
}

export class BioEnumValue {
  readonly enumName: string;
  readonly variant: string;
  readonly fields: BioJsValue[];
  constructor(enumName: string, variant: string, fields?: BioJsValue[]);
}

export interface BioValuePageOptions { offset?: number; limit?: number; }

export class BioValueHandle {
  readonly session: number;
  readonly id: number;
  readonly generation: number;
  readonly valueType: string;
  readonly length: number | null;
  readonly rows: number | null;
  readonly columns: number | null;
  readonly nonZero: number | null;
  readonly disposed: boolean;
  readonly shape: [number, number] | null;
  page(options?: BioValuePageOptions): BioJsValue;
  toFloat64Array(): Float64Array;
  dispose(): boolean;
}

export function decodeBioValue(value: unknown, host?: unknown): BioJsValue;
export function encodeBioValue(value: unknown): unknown;
