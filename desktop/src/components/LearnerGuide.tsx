import { ArrowRight, Check, GraduationCap, X } from "lucide-react";

/**
 * The first-run guide shown in Learner mode.
 *
 * Learner mode used to be cosmetic — it changed a placeholder string and put
 * labels under the activity-bar icons, which is no help at all to somebody who
 * has never seen a workbench. This gives it something to do.
 *
 * Steps are derived from what the student has actually done rather than
 * advanced by a Next button, so the guide cannot get out of step with the
 * screen: if they open a file before being asked, that step is already ticked.
 */

export type LearnerProgress = {
  hasWorkspace: boolean;
  /** Desktop only: execution stays blocked until the folder is trusted. */
  needsTrust: boolean;
  hasOpenFile: boolean;
  hasRun: boolean;
  hasReadOutput: boolean;
  problemCount: number;
};

type Step = {
  id: string;
  title: string;
  detail: string;
  done: boolean;
};

/** The ordered checklist, with each step's completion read from live state. */
export function learnerSteps(progress: LearnerProgress): Step[] {
  const steps: Step[] = [
    {
      id: "workspace",
      title: "Open a folder",
      detail: "Everything you write lives in a workspace folder. Pick one to get started.",
      done: progress.hasWorkspace,
    },
  ];

  // Trust only appears on Desktop when a workspace is open and still restricted.
  // Browser / Studio Web is always trusted, so the step would be noise there.
  if (progress.hasWorkspace && progress.needsTrust) {
    steps.push({
      id: "trust",
      title: "Trust the workspace",
      detail: "Desktop blocks Run, terminals, and packages until you trust the folder. Use Trust Workspace in the banner, or press play — it will trust and run.",
      done: false,
    });
  }

  steps.push(
    {
      id: "file",
      title: "Open a BioLang file",
      detail: "Click a .bl file in the Explorer on the left. That panel lists everything in your folder.",
      done: progress.hasOpenFile,
    },
    {
      id: "run",
      title: "Run it",
      detail: "Press Ctrl+Enter, or the play button in the top right. BioLang runs the whole file.",
      done: progress.hasRun,
    },
    {
      id: "output",
      title: "Read the Output panel",
      detail: "Results appear at the bottom. Printed text, tables, and plots each get their own tab.",
      done: progress.hasReadOutput,
    },
  );

  return steps;
}

/** The step to show: the first unfinished one. */
export function currentStep(progress: LearnerProgress): Step | undefined {
  return learnerSteps(progress).find((step) => !step.done);
}

export function LearnerGuide({
  progress,
  onDismiss,
}: {
  progress: LearnerProgress;
  onDismiss: () => void;
}) {
  const steps = learnerSteps(progress);
  const step = currentStep(progress);
  const finished = steps.filter((entry) => entry.done).length;

  return (
    <aside className={`learner-guide${step ? "" : " complete"}`} aria-label="Getting started">
      <header>
        <GraduationCap size={14} />
        <strong>{step ? step.title : "You have the basics"}</strong>
        <button type="button" aria-label="Dismiss the guide" onClick={onDismiss}><X size={13} /></button>
      </header>

      {step && (
        <>
          <p>{step.detail}</p>

          {/* Errors are where a beginner gets stuck, so the guide says where to
              look rather than leaving them to find the panel. */}
          {step.id === "output" && progress.problemCount > 0 && (
            <p className="learner-guide-problem">
              There {progress.problemCount === 1 ? "is 1 problem" : `are ${progress.problemCount} problems`} to look at
              in the Problems tab. BioLang errors explain what went wrong and usually suggest a fix.
            </p>
          )}

          <ol>
            {steps.map((entry) => (
              <li key={entry.id} className={entry.done ? "done" : entry.id === step.id ? "current" : ""}>
                {entry.done ? <Check size={11} /> : entry.id === step.id ? <ArrowRight size={11} /> : <span className="learner-dot" />}
                {entry.title}
              </li>
            ))}
          </ol>

          <footer>{finished} of {steps.length}</footer>
        </>
      )}
    </aside>
  );
}
