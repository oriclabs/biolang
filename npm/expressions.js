/**
 * Structural BioLang source builders.
 *
 * These functions construct BioLang expressions; they do not execute them.
 * Keeping them on an explicit subpath prevents `mean(...)` from having two
 * incompatible meanings at the package root. Use a BioLang session for direct
 * execution (`bl.mean(values)`) and this module for source construction.
 */
export { dna, protein, range, rna, set, slice } from "./generated-builtins.js";
export * from "./dsl.js";
export * from "./generated-builtins.js";
export * from "./objects.js";
