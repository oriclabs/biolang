import { BioLangSession, type BioLangSource, type RunResult } from "./session.js";

export * from "./dsl.js";
export * from "./generated-builtins.js";
export * from "./objects.js";
export * from "./session.js";
export * from "./somer.js";
export * from "./values.js";

export interface BioLangOptions {
  cwd?: string;
  network?: boolean;
}

export class BioLang extends BioLangSession {
  static create(options?: BioLangOptions): Promise<BioLang>;
}

export function run(source: BioLangSource, options?: BioLangOptions): Promise<RunResult>;
export const version: string;

export { dna, protein, range, rna, set, slice } from "./generated-builtins.js";
