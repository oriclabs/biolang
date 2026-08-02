import { Check, CircleDashed, FileArchive, LoaderCircle, Play, X } from "lucide-react";
import type { Assignment, TaskProgress } from "../assignment";
import { completedCount, isComplete } from "../assignment";

/**
 * The task list for an assignment.
 *
 * Deliberately not a test report. A student wants to know what is left and what
 * to try next, so tasks lead with their own name rather than a function name,
 * and a failure shows the instructor's hint before the assertion message.
 */
export function AssignmentPane({
  assignment,
  progress,
  running,
  canRun,
  onRun,
  onSubmit,
}: {
  assignment: Assignment;
  progress: TaskProgress[];
  running: boolean;
  canRun: boolean;
  onRun: () => void;
  onSubmit: () => void;
}) {
  const done = completedCount(progress);
  const complete = isComplete(progress);

  return (
    <div className="assignment-pane">
      <header>
        <div>
          <strong>{assignment.title}</strong>
          <span>{done} of {progress.length} complete</span>
        </div>
        <div className="assignment-actions">
          <button type="button" disabled={!canRun || running} onClick={onRun}>
            {running ? <LoaderCircle size={13} className="spin" /> : <Play size={13} />}
            Check my work
          </button>
          <button
            type="button"
            className={complete ? "assignment-submit ready" : "assignment-submit"}
            disabled={!canRun}
            onClick={onSubmit}
          ><FileArchive size={13} />Export submission</button>
        </div>
      </header>

      {assignment.instructions && <p className="assignment-instructions">{assignment.instructions}</p>}

      <div
        className="assignment-progress"
        role="progressbar"
        aria-valuenow={done}
        aria-valuemin={0}
        aria-valuemax={progress.length}
      >
        <i style={{ width: `${progress.length ? (done / progress.length) * 100 : 0}%` }} />
      </div>

      <ol className="assignment-tasks">
        {progress.map((task) => (
          <li key={task.test} className={task.passed ? "passed" : task.missing ? "pending" : "failed"}>
            <span className="assignment-status">
              {task.passed ? <Check size={13} /> : task.missing ? <CircleDashed size={13} /> : <X size={13} />}
            </span>
            <div>
              <strong>{task.name}</strong>
              {/* The instructor's hint is more use than the raw assertion, so
                  it comes first when both are available. */}
              {!task.passed && task.hint && <small className="assignment-hint">{task.hint}</small>}
              {!task.passed && !task.missing && task.message && (
                <small className="assignment-message">{task.message}</small>
              )}
              {task.missing && <small>Not checked yet — run the checks to see where you are.</small>}
            </div>
          </li>
        ))}
      </ol>

      {complete && (
        <p className="assignment-complete">
          <Check size={13} />Every check passes. Export your submission to hand it in.
        </p>
      )}
    </div>
  );
}
