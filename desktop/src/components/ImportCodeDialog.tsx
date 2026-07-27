import type { Monaco, OnMount } from "@monaco-editor/react";
import type * as MonacoEditor from "monaco-editor";
import {
  AlertTriangle,
  Check,
  FileInput,
  LockKeyhole,
  RefreshCw,
  X,
} from "lucide-react";
import { lazy, Suspense, useEffect, useMemo, useRef, useState } from "react";
import {
  convertImportOutput,
  importDestination,
  importOutputKind,
  outputNameForKind,
  summarizeConversion,
  type ImportOutputKind,
} from "../codeImport";
import { registerBioLang } from "../language";
import type { CodeImportResult, ImportValidationReport } from "../types";

const CodeEditor = lazy(() => import("./CodeEditor"));

export interface ImportSaveRequest {
  path: string;
  content: string;
  notebook: boolean;
  validation: ImportValidationReport;
  validationCurrent: boolean;
}

type Props = {
  result: CodeImportResult;
  directories: string[];
  onClose: () => void;
  onValidate: (content: string, notebook: boolean) => Promise<ImportValidationReport>;
  onSave: (request: ImportSaveRequest) => Promise<void>;
};

function sourceLanguage(format: CodeImportResult["sourceFormat"]) {
  if (format === "python") return "python";
  if (format === "r") return "r";
  if (format === "ipynb") return "json";
  return "markdown";
}

function issueMarkers(
  content: string,
  validation: ImportValidationReport,
  monaco: Monaco,
): MonacoEditor.editor.IMarkerData[] {
  const diagnostics = validation.diagnostics.map((diagnostic) => ({
    startLineNumber: Math.max(1, diagnostic.line),
    startColumn: Math.max(1, diagnostic.column),
    endLineNumber: Math.max(1, diagnostic.line),
    endColumn: Math.max(2, diagnostic.column + 1),
    message: diagnostic.message,
    severity: monaco.MarkerSeverity.Error,
  }));
  const generated = content.split(/\r?\n/).flatMap((line, index) => {
    const approximate = /\bapproximat(?:e|ion|ed)\b/i.test(line);
    const unsupported = !line.startsWith("# Conversion complete:")
      && /\b(?:TODO|unsupported|cannot convert|manual attention)\b/i.test(line);
    if (!approximate && !unsupported) return [];
    return [{
      startLineNumber: index + 1,
      startColumn: 1,
      endLineNumber: index + 1,
      endColumn: Math.max(2, line.length + 1),
      message: approximate ? "Conversion uses an approximation" : "Manual conversion review required",
      severity: approximate ? monaco.MarkerSeverity.Info : monaco.MarkerSeverity.Warning,
    }];
  });
  return [...diagnostics, ...generated];
}

export function ImportCodeDialog({
  result,
  directories,
  onClose,
  onValidate,
  onSave,
}: Props) {
  const initialKind = importOutputKind(result);
  const [kind, setKind] = useState<ImportOutputKind>(initialKind);
  const [drafts, setDrafts] = useState<Partial<Record<ImportOutputKind, string>>>({
    [initialKind]: result.content,
  });
  const [directory, setDirectory] = useState("");
  const [name, setName] = useState(result.suggestedName);
  const [validation, setValidation] = useState(result.validation);
  const [validationCurrent, setValidationCurrent] = useState(true);
  const [validating, setValidating] = useState(false);
  const [saving, setSaving] = useState(false);
  const convertedEditor = useRef<MonacoEditor.editor.IStandaloneCodeEditor>();
  const convertedMonaco = useRef<Monaco>();
  const content = drafts[kind] ?? "";
  const destination = importDestination(directory, name);
  const summary = useMemo(() => summarizeConversion(content, kind), [content, kind]);

  useEffect(() => {
    const nextKind = importOutputKind(result);
    setKind(nextKind);
    setDrafts({ [nextKind]: result.content });
    setDirectory("");
    setName(result.suggestedName);
    setValidation(result.validation);
    setValidationCurrent(true);
  }, [result]);

  useEffect(() => {
    const editor = convertedEditor.current;
    const monaco = convertedMonaco.current;
    if (!editor || !monaco) return;
    monaco.editor.setModelMarkers(
      editor.getModel()!,
      "biolang-import",
      issueMarkers(content, validation, monaco),
    );
  }, [content, validation]);

  const changeKind = (nextKind: ImportOutputKind) => {
    if (nextKind === kind) return;
    setDrafts((current) => ({
      ...current,
      [nextKind]: current[nextKind]
        ?? convertImportOutput(current[kind] ?? "", kind, nextKind, result.sourceName),
    }));
    setKind(nextKind);
    setName((current) => outputNameForKind(current, nextKind));
    setValidationCurrent(false);
  };

  const changeContent = (nextContent: string) => {
    setDrafts((current) => ({ ...current, [kind]: nextContent }));
    setValidationCurrent(false);
  };

  const validate = async () => {
    setValidating(true);
    try {
      setValidation(await onValidate(content, kind === "notebook"));
      setValidationCurrent(true);
    } finally {
      setValidating(false);
    }
  };

  const save = async () => {
    if (!destination) return;
    setSaving(true);
    try {
      await onSave({
        path: destination,
        content,
        notebook: kind === "notebook",
        validation,
        validationCurrent,
      });
    } finally {
      setSaving(false);
    }
  };

  const convertedMounted: OnMount = (editor, monaco) => {
    convertedEditor.current = editor;
    convertedMonaco.current = monaco;
    monaco.editor.setModelMarkers(
      editor.getModel()!,
      "biolang-import",
      issueMarkers(content, validation, monaco),
    );
  };

  return (
    <div className="dialog-backdrop" onMouseDown={onClose}>
      <section className="import-code-dialog" aria-label="Import code" onMouseDown={(event) => event.stopPropagation()}>
        <div className="dialog-heading">
          <span><FileInput size={14} /> Import {result.sourceName}</span>
          <button type="button" className="icon-button" aria-label="Close" onClick={onClose}><X size={14} /></button>
        </div>

        <div className="import-code-summary">
          <span><strong>Source</strong>{result.sourceFormat.toUpperCase()}</span>
          <span><strong>Converted</strong>{summary.converted} {kind === "notebook" ? "cells" : "script"}</span>
          <span className={summary.approximated ? "review" : ""}><strong>Approximated</strong>{summary.approximated}</span>
          <span className={summary.unsupported ? "invalid" : ""}><strong>Unsupported</strong>{summary.unsupported}</span>
          <span className={!validationCurrent ? "review" : validation.valid ? "valid" : "invalid"}>
            {!validationCurrent
              ? <><AlertTriangle size={13} />Validation outdated</>
              : validation.valid
                ? <><Check size={13} />Syntax valid</>
                : <><AlertTriangle size={13} />{validation.diagnostics.length} diagnostics</>}
          </span>
        </div>

        <div className="import-options">
          <div className="segmented-control" role="group" aria-label="Import output format">
            <button type="button" className={kind === "script" ? "active" : ""} onClick={() => changeKind("script")}>Script</button>
            <button type="button" className={kind === "notebook" ? "active" : ""} onClick={() => changeKind("notebook")}>Notebook</button>
          </div>
          <label>Folder
            <select aria-label="Import destination folder" value={directory} onChange={(event) => setDirectory(event.target.value)}>
              <option value="">Workspace root</option>
              {directories.map((path) => <option value={path} key={path}>{path}</option>)}
            </select>
          </label>
          <label>File name
            <input aria-label="Output file" value={name} onChange={(event) => setName(event.target.value)} spellCheck={false} />
          </label>
        </div>

        <div className="import-review-editors">
          <section>
            <header><strong>Original</strong><span>Read only</span></header>
            <Suspense fallback={<div className="editor-loading">Loading source preview...</div>}>
              <CodeEditor
                beforeMount={registerBioLang}
                path={`file:///import-source/${encodeURIComponent(result.sourceName)}`}
                language={sourceLanguage(result.sourceFormat)}
                value={result.sourceContent}
                theme="biolang-dark"
                options={{
                  ariaLabel: "Original source preview",
                  automaticLayout: true,
                  fontSize: 11,
                  lineHeight: 19,
                  minimap: { enabled: false },
                  readOnly: true,
                  scrollBeyondLastLine: false,
                  wordWrap: "off",
                }}
              />
            </Suspense>
          </section>
          <section>
            <header>
              <strong>Converted BioLang</strong>
              <button type="button" disabled={validating || validationCurrent} onClick={() => void validate()}>
                <RefreshCw size={12} className={validating ? "spin" : ""} />
                {validating ? "Validating..." : "Revalidate"}
              </button>
            </header>
            <Suspense fallback={<div className="editor-loading">Loading BioLang preview...</div>}>
              <CodeEditor
                key={kind}
                beforeMount={registerBioLang}
                onMount={convertedMounted}
                path={`file:///import-converted/${encodeURIComponent(outputNameForKind(result.suggestedName, kind))}`}
                language={kind === "notebook" ? "markdown" : "biolang"}
                value={content}
                onChange={(value) => changeContent(value ?? "")}
                theme="biolang-dark"
                options={{
                  ariaLabel: "Converted BioLang preview",
                  automaticLayout: true,
                  fontSize: 11,
                  lineHeight: 19,
                  minimap: { enabled: false },
                  scrollBeyondLastLine: false,
                  wordWrap: "off",
                }}
              />
            </Suspense>
          </section>
        </div>

        {!validation.valid && validationCurrent && (
          <div className="import-diagnostics" role="status">
            {validation.diagnostics.map((diagnostic, index) => (
              <div key={`${diagnostic.unit}-${diagnostic.line}-${index}`}>
                <strong>{diagnostic.unit}:{diagnostic.line}:{diagnostic.column}</strong>
                <span>{diagnostic.message}</span>
              </div>
            ))}
          </div>
        )}

        <div className="import-safety">
          <LockKeyhole size={12} />
          <span>Review only. Imported code is not executed automatically.</span>
        </div>
        <div className="dialog-actions">
          <button type="button" onClick={onClose}>Cancel</button>
          <button type="button" disabled={validating || validationCurrent} onClick={() => void validate()}>
            {validating ? "Validating..." : "Revalidate"}
          </button>
          <button
            type="button"
            className="primary"
            disabled={saving || !destination}
            onClick={() => void save()}
          >
            {saving ? "Saving..." : validationCurrent && validation.valid ? "Save and Open" : "Save Draft"}
          </button>
        </div>
      </section>
    </div>
  );
}
