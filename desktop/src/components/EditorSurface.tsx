import { Blocks, ChevronRight, Dna } from "lucide-react";
import { lazy, Suspense, type ReactNode } from "react";
import type { BeforeMount, OnMount } from "@monaco-editor/react";
import type { NotebookCellOutput, OpenFile } from "../types";
import { fileIcon } from "./FileTree";
import { ErrorBoundary } from "./ErrorBoundary";

const CodeEditor = lazy(() => import("./CodeEditor"));
const DataPreviewPane = lazy(() => import("./DataPreviewPane").then((module) => ({ default: module.DataPreviewPane })));
const NotebookPane = lazy(() => import("./NotebookPane").then((module) => ({ default: module.NotebookPane })));
const PipelineViewer = lazy(() => import("./PipelineViewer").then((module) => ({ default: module.PipelineViewer })));
const WorkflowPane = lazy(() => import("./WorkflowPane").then((module) => ({ default: module.WorkflowPane })));

/**
 * One group's content surface (code, notebook, workflow, or data preview).
 *
 * Split groups share the same OpenFile buffer via parent state; only the primary
 * group wires Monaco into the LSP document lifecycle.
 */
export function EditorSurface({
  file,
  workspaceName,
  group,
  pipelineView,
  onPipelineView,
  editorTheme,
  fontSize,
  tabSize,
  wordWrap,
  minimap,
  beforeMount,
  onMount,
  onChange,
  output,
  cellOutputs,
  running,
  onRun,
  onRunCell,
  onStop,
  onCellMount,
  onCellChange,
  onCellUnmount,
  onInvalidateCell,
  onExportPreview,
  empty,
}: {
  file: OpenFile | undefined;
  workspaceName?: string;
  group: "primary" | "secondary";
  pipelineView: boolean;
  onPipelineView: (value: boolean) => void;
  editorTheme: string;
  fontSize: number;
  tabSize: number;
  wordWrap: boolean;
  minimap: boolean;
  beforeMount: BeforeMount;
  onMount?: OnMount;
  onChange: (value: string | undefined) => void;
  output?: string;
  cellOutputs?: Record<number, NotebookCellOutput>;
  running?: boolean;
  onRun: () => void;
  onRunCell: (cellIndex: number) => void;
  onStop: () => void;
  onCellMount: (...args: never[]) => void;
  onCellChange: (...args: never[]) => void;
  onCellUnmount: (...args: never[]) => void;
  onInvalidateCell: (cellIndex: number) => void;
  onExportPreview: (path: string, format: string) => void | Promise<void>;
  empty?: ReactNode;
}) {
  if (!file) {
    return (
      <div className="editor-surface">
        {empty ?? (
          <div className="workspace-welcome compact editor-group-empty">
            <Dna size={28} />
            <span>Open a file in this group</span>
          </div>
        )}
      </div>
    );
  }

  if (file.preview) {
    return (
      <div className="editor-surface">
        <ErrorBoundary label="Data preview">
          <Suspense fallback={<div className="editor-loading">Preparing data preview...</div>}>
            <DataPreviewPane name={file.name} path={file.path} preview={file.preview} onExport={onExportPreview} />
          </Suspense>
        </ErrorBoundary>
      </div>
    );
  }

  if (file.viewer === "notebook") {
    return (
      <div className="editor-surface">
        <ErrorBoundary label="Notebook">
          <Suspense fallback={<div className="editor-loading">Preparing notebook...</div>}>
            <NotebookPane
              name={file.name}
              path={file.path}
              content={file.content}
              output={output ?? ""}
              cellOutputs={cellOutputs ?? {}}
              running={Boolean(running)}
              editorTheme={editorTheme as "biolang-dark"}
              fontSize={fontSize}
              tabSize={tabSize}
              wordWrap={wordWrap}
              beforeMount={beforeMount}
              onChange={onChange}
              onRun={onRun}
              onRunCell={onRunCell}
              onStop={onStop}
              onCellMount={onCellMount as never}
              onCellChange={onCellChange as never}
              onCellUnmount={onCellUnmount as never}
              onInvalidateCell={onInvalidateCell}
            />
          </Suspense>
        </ErrorBoundary>
      </div>
    );
  }

  if (file.viewer === "workflow") {
    return (
      <div className="editor-surface">
        <ErrorBoundary label="Workflow editor">
          <Suspense fallback={<div className="editor-loading">Preparing workflow...</div>}>
            <WorkflowPane
              content={file.content}
              running={Boolean(running)}
              onChange={onChange}
              onRun={onRun}
              onStop={onStop}
            />
          </Suspense>
        </ErrorBoundary>
      </div>
    );
  }

  return (
    <div className="editor-surface code-surface">
      <div className="breadcrumbs">
        <span>{workspaceName}</span>
        <ChevronRight size={12} />
        {(file.untitled ? [file.name] : file.path.split("/")).map((part, index, parts) => (
          <span key={`${part}-${index}`}>
            {index > 0 && <ChevronRight size={12} />}
            {index === parts.length - 1 && fileIcon(file.untitled ? file.name : file.path)}
            {part}
          </span>
        ))}
        {file.path.endsWith(".bl") && (
          <button type="button" className={pipelineView ? "active" : ""} onClick={() => onPipelineView(!pipelineView)}>
            <Blocks size={12} />Pipeline
          </button>
        )}
      </div>
      <div className="editor-host">
        {pipelineView ? (
          <ErrorBoundary label="Pipeline viewer">
            <Suspense fallback={<div className="editor-loading">Inspecting pipeline...</div>}>
              <PipelineViewer source={file.content} onOpenSource={() => onPipelineView(false)} />
            </Suspense>
          </ErrorBoundary>
        ) : (
          <ErrorBoundary label="Code editor">
            <Suspense fallback={<div className="editor-loading">Loading editor...</div>}>
              <CodeEditor
                beforeMount={beforeMount}
                onMount={onMount}
                path={`file:///workspace/${file.path}${group === "secondary" ? "?group=secondary" : ""}`}
                language={file.language}
                value={file.content}
                onChange={onChange}
                theme={editorTheme}
                options={{
                  automaticLayout: true,
                  fontFamily: '"Cascadia Code", "SFMono-Regular", Consolas, monospace',
                  fontSize,
                  lineHeight: 21,
                  minimap: { enabled: minimap, scale: 1, showSlider: "mouseover" },
                  scrollBeyondLastLine: false,
                  padding: { top: 14 },
                  renderLineHighlight: "gutter",
                  smoothScrolling: true,
                  tabSize,
                  wordWrap: wordWrap ? "on" : "off",
                  guides: { indentation: true, bracketPairs: true },
                  bracketPairColorization: { enabled: true },
                  quickSuggestions: true,
                  suggest: { showWords: false },
                  stickyScroll: { enabled: true, maxLineCount: 3 },
                  gotoLocation: {
                    multipleReferences: "goto",
                    multipleDefinitions: "goto",
                    multipleDeclarations: "goto",
                    multipleImplementations: "goto",
                    multipleTypeDefinitions: "goto",
                  },
                  rulers: [100],
                  formatOnType: true,
                  formatOnPaste: true,
                }}
              />
            </Suspense>
          </ErrorBoundary>
        )}
      </div>
    </div>
  );
}
