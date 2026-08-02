import type { Monaco } from "@monaco-editor/react";
import type * as MonacoEditor from "monaco-editor";

/**
 * Run actions above pipeline stages.
 *
 * A pipeline is a sequence of `stage "name" -> expr` steps, and the slow part
 * of a bio pipeline is almost never the step you are editing — it is the
 * alignment three stages up. Because the console keeps state between
 * evaluations, sending a single stage's expression re-runs exactly that step
 * against results already in memory.
 */

/** What a lens does when clicked, supplied by the workbench. */
type StageRunner = (source: string, label: string) => void;

let runner: StageRunner | undefined;

export function setStageRunner(run: StageRunner | undefined) {
  runner = run;
}

const PIPELINE = /^\s*pipeline\s+(?:"([^"]*)"|([A-Za-z_]\w*))/;
const STAGE = /^(\s*)stage\s+(?:"([^"]*)"|([A-Za-z_]\w*))\s*->\s*(.*)$/;

type Stage = {
  /** 1-based line of the `stage` keyword. */
  line: number;
  name: string;
  /** The expression, joined across continuation lines. */
  source: string;
};

type Pipeline = {
  /** 1-based line of the `pipeline` keyword. */
  line: number;
  name: string;
  stages: Stage[];
};

/**
 * Find pipelines and their stages.
 *
 * Exported for testing: the regex handling of continuation lines is the part
 * most likely to drift as the language grows.
 */
export function findPipelines(lines: readonly string[]): Pipeline[] {
  const pipelines: Pipeline[] = [];
  let current: Pipeline | undefined;
  let depth = 0;
  let open = false;

  lines.forEach((line, index) => {
    const pipelineMatch = !open ? PIPELINE.exec(line) : null;
    if (pipelineMatch) {
      current = {
        line: index + 1,
        name: pipelineMatch[1] ?? pipelineMatch[2] ?? "pipeline",
        stages: [],
      };
      pipelines.push(current);
      depth = 0;
      open = false;
    }

    if (current) {
      const before = depth;
      depth += (line.match(/[{[(]/g) ?? []).length;
      depth -= (line.match(/[}\])]/g) ?? []).length;
      if (!open && depth > 0) open = true;
      // The block closed on this line, so anything after it is outside.
      if (open && depth <= 0 && before > 0) {
        current = undefined;
        open = false;
        return;
      }
    }

    const stageMatch = STAGE.exec(line);
    if (stageMatch && current) {
      current.stages.push({
        line: index + 1,
        name: stageMatch[2] ?? stageMatch[3] ?? `stage ${current.stages.length + 1}`,
        source: stageMatch[4].trim(),
      });
      return;
    }

    // A continuation of the previous stage's expression, such as a wrapped
    // pipe chain, belongs to that stage.
    const last = current?.stages.at(-1);
    if (last && last.line === index) return;
    if (last && /^\s*(\||\.|\))/.test(line) && line.trim()) {
      last.source += `\n${line.trim()}`;
    }
  });

  return pipelines;
}

export const RUN_STAGE_COMMAND = "biolang.runStage";

export function registerPipelineLens(monaco: Monaco): MonacoEditor.IDisposable {
  monaco.editor.registerCommand(RUN_STAGE_COMMAND, (_accessor, source: string, label: string) => {
    runner?.(source, label);
  });

  return monaco.languages.registerCodeLensProvider("biolang", {
    provideCodeLenses(model) {
      if (!runner) return { lenses: [], dispose: () => undefined };
      const lines = Array.from(
        { length: model.getLineCount() },
        (_, index) => model.getLineContent(index + 1),
      );
      const lenses: MonacoEditor.languages.CodeLens[] = [];

      for (const pipeline of findPipelines(lines)) {
        if (!pipeline.stages.length) continue;
        const allStages = pipeline.stages.map((stage) => stage.source).join("\n");
        lenses.push({
          range: { startLineNumber: pipeline.line, startColumn: 1, endLineNumber: pipeline.line, endColumn: 1 },
          command: {
            id: RUN_STAGE_COMMAND,
            title: `▷ Run all ${pipeline.stages.length} stages`,
            arguments: [allStages, pipeline.name],
          },
        });

        pipeline.stages.forEach((stage, index) => {
          const range = { startLineNumber: stage.line, startColumn: 1, endLineNumber: stage.line, endColumn: 1 };
          lenses.push({
            range,
            command: {
              id: RUN_STAGE_COMMAND,
              title: "▷ Run stage",
              arguments: [stage.source, stage.name],
            },
          });
          if (index > 0) {
            lenses.push({
              range,
              command: {
                id: RUN_STAGE_COMMAND,
                title: "Run to here",
                arguments: [
                  pipeline.stages.slice(0, index + 1).map((entry) => entry.source).join("\n"),
                  `${pipeline.name} → ${stage.name}`,
                ],
              },
            });
          }
        });
      }

      return { lenses, dispose: () => undefined };
    },
  });
}
