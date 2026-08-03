/**
 * Snippet text for BioLang completions.
 *
 * Completions used to insert a bare identifier, so accepting `align` left you
 * to type the parentheses and every argument by hand even though the signature
 * was already on screen in the detail row. These build Monaco snippet strings
 * with a tabstop per argument instead.
 */

/** Characters Monaco reads as snippet syntax rather than literal text. */
function escapePlaceholder(text: string): string {
  return text.replace(/[$}\\]/g, "\\$&");
}

/**
 * Parameter names that should get a tabstop.
 *
 * Metadata marks optional arguments with a trailing `?` and variadic tails with
 * `...`. Neither belongs in the snippet: `align(seq1, seq2)` is the call people
 * mean, and forcing them to tab through four optional scoring arguments would
 * be slower than typing it. Signature help still lists the full set.
 */
export function requiredParameters(parameters: readonly string[] | undefined): string[] {
  return (parameters ?? []).filter((name) => name !== "..." && !name.endsWith("?"));
}

/** Pull the parameter list out of `name(a, b) → T`, for metadata without one. */
export function parametersFromSignature(signature: string): string[] {
  const open = signature.indexOf("(");
  const close = signature.indexOf(")", open + 1);
  if (open < 0 || close < 0) return [];
  return signature
    .slice(open + 1, close)
    .split(",")
    .map((parameter) => parameter.trim())
    .filter(Boolean);
}

/**
 * Build `name(${1:first}, ${2:second})$0` for a call, or `name()$0` when the
 * function takes nothing — the trailing `$0` leaves the cursor after the call
 * rather than inside the parentheses you just filled in.
 */
export function callSnippet(name: string, parameters: readonly string[]): string {
  const required = requiredParameters(parameters);
  if (!required.length) return `${name}()$0`;
  const placeholders = required.map(
    (parameter, index) => `\${${index + 1}:${escapePlaceholder(parameter)}}`,
  );
  return `${name}(${placeholders.join(", ")})$0`;
}

export type ScaffoldSnippet = {
  label: string;
  detail: string;
  documentation: string;
  body: string;
};

/**
 * Block scaffolds. These are the shapes people retype constantly and get the
 * bracket or arrow wrong on — particularly `stage`, whose `->` is easy to
 * misremember as `=>` after writing a `match`.
 */
export const scaffoldSnippets: ScaffoldSnippet[] = [
  {
    label: "pipeline",
    detail: "pipeline name(params) { ... }",
    documentation: "A named pipeline with two stages.",
    body: [
      "pipeline ${1:name}(${2:input}) {",
      "\tstage \"${3:load}\" -> ${4:input}",
      "\tstage \"${5:summarize}\" -> $0",
      "}",
    ].join("\n"),
  },
  {
    label: "stage",
    detail: 'stage "name" -> expr',
    documentation: "One stage of a pipeline.",
    body: 'stage "${1:name}" -> $0',
  },
  {
    label: "fn",
    detail: "fn name(params) { ... }",
    documentation: "A function declaration.",
    body: "fn ${1:name}(${2:args}) {\n\t$0\n}",
  },
  {
    label: "for",
    detail: "for item in items { ... }",
    documentation: "Iterate a list, table, or stream.",
    body: "for ${1:item} in ${2:items} {\n\t$0\n}",
  },
  {
    label: "match",
    detail: "match expr { ... }",
    documentation: "Pattern match with a catch-all arm.",
    body: "match ${1:value} {\n\t${2:pattern} => ${3:result},\n\t_ => $0\n}",
  },
  {
    label: "if",
    detail: "if condition { ... } else { ... }",
    documentation: "Conditional with an else branch.",
    body: "if ${1:condition} {\n\t$2\n} else {\n\t$0\n}",
  },
  {
    label: "try",
    detail: "try { ... } catch { ... }",
    documentation: "Run a fallible block and handle the error.",
    body: "try {\n\t$1\n} catch ${2:error} {\n\t$0\n}",
  },
  {
    label: "import",
    detail: 'import "package" as alias',
    documentation: "Import a BioLang package under a short alias.",
    body: 'import "${1:package}" as ${2:alias}$0',
  },
];
