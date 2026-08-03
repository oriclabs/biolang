import type { Problem } from "./types";

/**
 * Work the converter could not finish, found in imported source.
 *
 * `bl import` leaves a `# TODO:` marker wherever it could not translate
 * something, naming what is missing. The import dialog surfaces those as editor
 * markers, but they vanished the moment the file was saved: the Problems panel
 * is fed by the language server, which only reports parse errors. So after
 * importing a 600-line pipeline you had no idea how much was left to port
 * unless you read every line.
 *
 * Scanning the saved file keeps that work list alive, and because the markers
 * are ordinary comments it keeps working after hand-editing too.
 */

export type ImportIssueSeverity = "review" | "approximation";

export type ImportIssue = {
  line: number;
  column: number;
  message: string;
  severity: ImportIssueSeverity;
  /** The marker text itself, so the panel can say what is actually missing. */
  detail: string;
};

/** A marker that needs a human, rather than one that merely warns. */
const NEEDS_REVIEW = /\b(?:TODO|FIXME|unsupported|cannot convert|not yet in BioLang|manual attention)\b/i;
const APPROXIMATE = /\bapproximat(?:e|es|ion|ed)\b/i;

/** The banner `bl import` writes at the top of a converted file. */
const CONVERSION_BANNER = /^#\s*Conversion complete:/;

/** Text after the marker keyword, which is the part that says what is missing. */
function markerDetail(line: string): string {
  const trimmed = line.trim().replace(/^#+\s*/, "");
  const withoutKeyword = trimmed.replace(/^(?:TODO|FIXME)\s*:?\s*/i, "");
  return withoutKeyword || trimmed;
}

/**
 * Find conversion markers in `content`.
 *
 * Only comment lines count. A `TODO` inside a string literal is data, and a
 * function named `todo_list` is not a conversion marker — flagging either would
 * make the panel untrustworthy, which is worse than not having it.
 */
export function findImportIssues(content: string): ImportIssue[] {
  return content.split(/\r?\n/).flatMap((line, index) => {
    const commentStart = commentIndex(line);
    if (commentStart < 0) return [];
    const comment = line.slice(commentStart);
    if (CONVERSION_BANNER.test(comment.trim())) return [];

    const review = NEEDS_REVIEW.test(comment);
    const approximate = !review && APPROXIMATE.test(comment);
    if (!review && !approximate) return [];

    return [{
      line: index + 1,
      column: commentStart + 1,
      severity: review ? ("review" as const) : ("approximation" as const),
      message: review
        ? "Needs manual porting"
        : "Converted using an approximation",
      detail: markerDetail(comment),
    }];
  });
}

/** Index of the `#` that starts a comment, or -1 when the line has none. */
function commentIndex(line: string): number {
  let inString = false;
  let escaped = false;
  for (let index = 0; index < line.length; index += 1) {
    const character = line[index];
    if (inString) {
      if (escaped) escaped = false;
      else if (character === "\\") escaped = true;
      else if (character === '"') inString = false;
      continue;
    }
    if (character === '"') inString = true;
    // `#{` opens string interpolation, not a comment.
    else if (character === "#" && line[index + 1] !== "{") return index;
  }
  return -1;
}

/**
 * Import issues as Problems.
 *
 * Reported as warning and info rather than error: the file parses and runs, and
 * calling unfinished porting an error would drown the real diagnostics.
 */
export function importProblems(path: string, content: string): Problem[] {
  return findImportIssues(content).map((issue) => ({
    path,
    message: `${issue.message}: ${issue.detail}`,
    severity: issue.severity === "review" ? 2 : 3,
    line: issue.line,
    column: issue.column,
  }));
}

/** How many markers still need a human, for a count in the import dialog. */
export function countNeedingReview(content: string): number {
  return findImportIssues(content).filter((issue) => issue.severity === "review").length;
}
