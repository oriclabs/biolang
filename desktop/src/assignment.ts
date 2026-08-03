import type { TestResult } from "./types";

/**
 * Assignments, backed by the test runner.
 *
 * Tools become a first choice by being what the course uses, and an assignment
 * is naturally a workspace plus a set of checks the student has to make pass —
 * which is exactly what `bl test` already does. This adds the thin layer around
 * it: a manifest naming the tasks, progress read from real test results, and a
 * submission bundle.
 *
 * The format is deliberately plain files. There is no server, so an assignment
 * is a folder an instructor can zip or commit, and a submission is a bundle the
 * student hands back by whatever route the course already uses.
 *
 * `assignment.toml` at the workspace root:
 *
 * ```toml
 * title = "Week 3 — Sequence QC"
 * instructions = "Complete the functions in qc.bl until every check passes."
 *
 * [[task]]
 * name = "GC content"
 * test = "test_gc_content_is_a_fraction"
 * hint = "gc_content returns a fraction between 0 and 1, not a percentage."
 * ```
 */

export type AssignmentTask = {
  name: string;
  /** The `test_*` function that decides whether this task is done. */
  test: string;
  hint?: string;
};

export type Assignment = {
  title: string;
  instructions?: string;
  tasks: AssignmentTask[];
};

export const ASSIGNMENT_MANIFEST = "assignment.toml";

/**
 * Parse the manifest.
 *
 * A hand-rolled reader for the small subset the format uses, rather than
 * pulling in a TOML parser for the browser bundle: the manifest is written by
 * instructors, so the failure mode that matters is a clear message, not
 * exhaustive spec coverage.
 */
export function parseAssignment(text: string): Assignment | undefined {
  const lines = text.split(/\r?\n/);
  let title = "";
  let instructions: string | undefined;
  const tasks: AssignmentTask[] = [];
  let current: Partial<AssignmentTask> | undefined;

  const value = (line: string): string | undefined => {
    const match = line.match(/^\s*[A-Za-z_]+\s*=\s*"((?:[^"\\]|\\.)*)"\s*$/);
    return match ? match[1].replace(/\\"/g, '"').replace(/\\\\/g, "\\") : undefined;
  };
  const key = (line: string): string | undefined =>
    line.match(/^\s*([A-Za-z_]+)\s*=/)?.[1];

  const flush = () => {
    if (current?.name && current.test) {
      tasks.push({ name: current.name, test: current.test, hint: current.hint });
    }
    current = undefined;
  };

  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;
    if (trimmed === "[[task]]") {
      flush();
      current = {};
      continue;
    }
    const name = key(trimmed);
    const text = value(trimmed);
    if (!name || text === undefined) continue;
    if (current) {
      if (name === "name") current.name = text;
      else if (name === "test") current.test = text;
      else if (name === "hint") current.hint = text;
    } else if (name === "title") {
      title = text;
    } else if (name === "instructions") {
      instructions = text;
    }
  }
  flush();

  if (!title && !tasks.length) return undefined;
  return { title: title || "Assignment", instructions, tasks };
}

export type TaskProgress = AssignmentTask & {
  passed: boolean;
  /** True when no result for this task appeared in the last run. */
  missing: boolean;
  message?: string;
};

/**
 * Match each task to its test result.
 *
 * A task whose test did not run is reported as missing rather than failing:
 * "you have not got there yet" and "you got it wrong" are different messages,
 * and conflating them is discouraging for no reason.
 */
export function taskProgress(
  assignment: Assignment,
  results: TestResult[],
): TaskProgress[] {
  return assignment.tasks.map((task) => {
    const result = results.find((entry) => entry.name === task.test);
    return {
      ...task,
      passed: Boolean(result?.passed),
      missing: !result,
      message: result?.message,
    };
  });
}

export function completedCount(progress: TaskProgress[]): number {
  return progress.filter((task) => task.passed).length;
}

/** Everything passing, and at least one task to pass. */
export function isComplete(progress: TaskProgress[]): boolean {
  return progress.length > 0 && progress.every((task) => task.passed);
}
