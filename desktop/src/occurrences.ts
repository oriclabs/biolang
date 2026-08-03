import type * as MonacoEditor from "monaco-editor";

/**
 * Finding where an identifier is actually used.
 *
 * Shared by rename, the reference provider, and the References panel so all
 * three agree on what counts as a use. Monaco's own `findMatches` would happily
 * match the word inside a string literal or a comment, which is exactly the
 * kind of edit that makes people stop trusting rename.
 */

export type Occurrence = {
  line: number;
  /** 1-based column of the first character, as Monaco counts. */
  column: number;
  /** The full text of the line, for previewing the hit. */
  preview: string;
};

/** Blank out string and comment spans, preserving every column position. */
export function stripStringsAndComments(line: string): string {
  let out = "";
  let inString = false;
  let escaped = false;
  for (let index = 0; index < line.length; index += 1) {
    const character = line[index];
    if (inString) {
      out += " ";
      if (escaped) escaped = false;
      else if (character === "\\") escaped = true;
      else if (character === '"') inString = false;
      continue;
    }
    if (character === '"') {
      inString = true;
      out += " ";
      continue;
    }
    // `#{` opens string interpolation; anything else runs to end of line.
    if (character === "#" && line[index + 1] !== "{") {
      return out + " ".repeat(line.length - index);
    }
    out += character;
  }
  return out;
}

/**
 * Every standalone use of `name` in `lines`, skipping strings, comments, and
 * member positions such as the `end` in `region.end`.
 */
export function findOccurrences(lines: readonly string[], name: string): Occurrence[] {
  if (!/^[A-Za-z_]\w*$/.test(name)) return [];
  const pattern = new RegExp(`(^|[^.\\w])(${name})(?![\\w])`, "g");
  const found: Occurrence[] = [];

  lines.forEach((line, index) => {
    const code = stripStringsAndComments(line);
    pattern.lastIndex = 0;
    for (let match = pattern.exec(code); match; match = pattern.exec(code)) {
      found.push({
        line: index + 1,
        column: match.index + match[1].length + 1,
        preview: line.trim(),
      });
    }
  });
  return found;
}

/** The same search against a Monaco model, as editor ranges. */
export function identifierOccurrences(
  model: MonacoEditor.editor.ITextModel,
  name: string,
): MonacoEditor.IRange[] {
  const lines = Array.from(
    { length: model.getLineCount() },
    (_, index) => model.getLineContent(index + 1),
  );
  return findOccurrences(lines, name).map((found) => ({
    startLineNumber: found.line,
    startColumn: found.column,
    endLineNumber: found.line,
    endColumn: found.column + name.length,
  }));
}
