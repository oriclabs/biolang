/**
 * How a workspace search interprets its query.
 *
 * Shared by the browser search, the replace preview, and the Rust command's
 * arguments so the three cannot drift into disagreeing about what a query
 * matches — which would show one set of hits and rewrite a different one.
 */
export type SearchOptions = {
  caseSensitive: boolean;
  wholeWord: boolean;
  regex: boolean;
};

export const defaultSearchOptions: SearchOptions = {
  caseSensitive: false,
  wholeWord: false,
  regex: false,
};

/** Minimum query length. Shorter queries match nearly every file. */
export const MIN_SEARCH_LENGTH = 2;

function escapeLiteral(query: string): string {
  return query.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

/**
 * Build the matching expression, or undefined when the query cannot compile.
 *
 * An invalid regular expression is normal while someone is still typing one, so
 * it returns undefined rather than throwing.
 */
export function searchPattern(query: string, options: SearchOptions): RegExp | undefined {
  if (query.length < MIN_SEARCH_LENGTH) return undefined;
  const body = options.regex ? query : escapeLiteral(query);
  const source = options.wholeWord ? `\\b(?:${body})\\b` : body;
  try {
    return new RegExp(source, options.caseSensitive ? "g" : "gi");
  } catch {
    return undefined;
  }
}

/**
 * Replacement text for one match.
 *
 * `$1` style group references only make sense for a regular expression search;
 * for a literal search the replacement is inserted verbatim, so a `$` in it
 * stays a `$`.
 */
export function replacementFor(
  line: string,
  pattern: RegExp,
  replacement: string,
  options: SearchOptions,
): string {
  const safe = options.regex ? replacement : replacement.replaceAll("$", "$$$$");
  return line.replace(new RegExp(pattern.source, pattern.flags), safe);
}
