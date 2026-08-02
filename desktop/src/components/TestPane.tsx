import { Check, CircleStop, FlaskConical, LoaderCircle, Play, X } from "lucide-react";
import type { TestResult, TestRun } from "../types";

/**
 * Results of `bl test`, grouped by file.
 *
 * Analysis code is rarely tested, and a runner nobody can see is a runner
 * nobody uses. Failures lead and carry their assertion message, because the
 * message is the only part that tells you what to do next.
 */

function groupByFile(results: TestResult[]): Array<[string, TestResult[]]> {
  const groups = new Map<string, TestResult[]>();
  for (const result of results) {
    groups.set(result.file, [...(groups.get(result.file) ?? []), result]);
  }
  // Files with failures first: the reason you opened this panel is at the top.
  return [...groups].sort(([, left], [, right]) => {
    const leftFailed = left.some((result) => !result.passed) ? 0 : 1;
    const rightFailed = right.some((result) => !result.passed) ? 0 : 1;
    return leftFailed - rightFailed;
  });
}

export function TestPane({
  run,
  activeFile,
  canRun,
  onRun,
  onOpenFailure,
}: {
  run: TestRun | undefined;
  activeFile: string | undefined;
  canRun: boolean;
  onRun: (path?: string) => void;
  onOpenFailure: (file: string) => void;
}) {
  const running = run?.status === "running";

  return (
    <div className="test-pane">
      <header className="test-toolbar">
        <button type="button" disabled={!canRun || running} onClick={() => onRun()}>
          {running ? <LoaderCircle size={13} className="spin" /> : <Play size={13} />}
          Run all tests
        </button>
        <button
          type="button"
          disabled={!canRun || running || !activeFile}
          onClick={() => onRun(activeFile)}
        >
          <FlaskConical size={13} />
          Run this file
        </button>
        {run?.status === "finished" && (
          <span className={run.failed ? "test-summary failed" : "test-summary passed"}>
            {run.failed
              ? `${run.failed} failed, ${run.passed} passed`
              : `${run.passed} passed`}
            {run.durationMs != null && ` in ${(run.durationMs / 1000).toFixed(2)}s`}
          </span>
        )}
      </header>

      {run?.status === "failed" && (
        <div className="test-error" role="alert">{run.error}</div>
      )}

      {!run && (
        <div className="test-empty">
          <FlaskConical size={20} />
          <span>No tests run yet</span>
          <small>Name a function <code>test_something</code> and it becomes a test.</small>
        </div>
      )}

      {run?.status === "finished" && !run.results.length && (
        <div className="test-empty">
          <FlaskConical size={20} />
          <span>No tests found</span>
          <small>Name a zero-argument function <code>test_something</code> to make it one.</small>
        </div>
      )}

      {run && groupByFile(run.results).map(([file, results]) => (
        <section className="test-file" key={file}>
          <button type="button" className="test-file-heading" onClick={() => onOpenFailure(file)}>
            <span>{file}</span>
            <small>{results.filter((result) => result.passed).length}/{results.length}</small>
          </button>
          {results.map((result) => (
            <div className={`test-result ${result.passed ? "passed" : "failed"}`} key={result.name}>
              {result.passed ? <Check size={12} /> : <X size={12} />}
              <span>{result.label || result.name}</span>
              {result.durationMs != null && <small>{result.durationMs} ms</small>}
              {result.message && <p>{result.message}</p>}
            </div>
          ))}
        </section>
      ))}

      {running && (
        <div className="test-running" role="status">
          <LoaderCircle size={13} className="spin" />
          Running tests
          <CircleStop size={12} className="test-running-hint" />
        </div>
      )}
    </div>
  );
}
