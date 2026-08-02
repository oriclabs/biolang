import type { BeforeMount, Monaco, OnMount } from "@monaco-editor/react";
import {
  ChevronDown,
  ChevronRight,
  Check,
  CircleStop,
  Copy,
  Database,
  Eraser,
  LoaderCircle,
  MemoryStick,
  Play,
  RefreshCw,
  RotateCcw,
  TerminalSquare,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type * as MonacoEditor from "monaco-editor";
import { bridge } from "../bridge";
import { formatConsoleBytes } from "../console";
import type { ConsoleEnvironment, ConsoleResponse } from "../types";
import CodeEditor from "./CodeEditor";

type TranscriptEntry = {
  id: number;
  source: string;
  response?: ConsoleResponse;
  interrupted?: boolean;
};

const emptyEnvironment: ConsoleEnvironment = { variables: [], totalBytes: 0 };
const consoleDocumentPath = "__notebook__/console/session.bl";

function storageKey(kind: "transcript" | "history", root: string) {
  return `biolang.desktop.console.${kind}.${root}`;
}

function loadStored<T>(key: string, fallback: T): T {
  try {
    const value = window.localStorage.getItem(key);
    return value ? JSON.parse(value) as T : fallback;
  } catch {
    return fallback;
  }
}

function ConsoleResult({ response }: { response: ConsoleResponse }) {
  const value = response.value;
  return <div className={`console-response ${response.status}`}>
    {response.output && <pre className="console-stream">{response.output}</pre>}
    {response.error && <pre className="console-error">{response.error}</pre>}
    {value?.kind === "table" && <div className="console-table-wrap">
      <table>
        <thead><tr>{value.columns.map((column) => <th key={column}>{column}</th>)}</tr></thead>
        <tbody>{value.rows.map((row, rowIndex) =>
          <tr key={rowIndex}>{row.map((cell, columnIndex) => <td key={columnIndex}>{cell}</td>)}</tr>)}</tbody>
      </table>
      {value.truncated && <span className="console-truncated">Showing the first {value.rows.length} rows</span>}
    </div>}
    {value?.kind === "sequence" && <div className="console-sequence">
      <header><span>{value.typeName}</span><button type="button" title="Copy sequence" aria-label="Copy sequence" onClick={() => void bridge.copyText(value.sequence ?? "")}><Copy size={11} /></button></header>
      <pre>{value.sequence}</pre>
      {value.truncated && <span className="console-truncated">Sequence preview truncated</span>}
    </div>}
    {value?.kind === "text" && <pre className="console-value">{value.text}</pre>}
    <small className="console-duration">{response.status === "ok" ? <Check size={10} /> : <CircleStop size={10} />}{response.durationMs} ms</small>
  </div>;
}

/**
 * Code pushed in from the editor by Shift+Enter. The id is what makes a repeat
 * of the identical selection run a second time.
 */
export type ConsoleSubmission = { id: number; source: string };

export function ConsolePane({
  workspaceRoot,
  editorTheme,
  fontSize,
  tabSize,
  beforeMount,
  onDocumentMount,
  onDocumentChange,
  onDocumentUnmount,
  showNotice,
  submission,
}: {
  workspaceRoot: string;
  editorTheme: string;
  fontSize: number;
  tabSize: number;
  beforeMount: BeforeMount;
  onDocumentMount: (path: string, content: string, monaco: Monaco) => void | Promise<void>;
  onDocumentChange: (path: string, content: string) => void;
  onDocumentUnmount: (path: string) => void;
  showNotice: (message: string) => void;
  submission?: ConsoleSubmission;
}) {
  const transcriptKey = storageKey("transcript", workspaceRoot);
  const historyKey = storageKey("history", workspaceRoot);
  const [source, setSource] = useState("");
  const [transcript, setTranscript] = useState<TranscriptEntry[]>(() =>
    loadStored<TranscriptEntry[]>(transcriptKey, []));
  const [history, setHistory] = useState<string[]>(() => loadStored<string[]>(historyKey, []));
  const [historyIndex, setHistoryIndex] = useState(-1);
  const [environment, setEnvironment] = useState<ConsoleEnvironment>(emptyEnvironment);
  const [busy, setBusy] = useState(false);
  const [connected, setConnected] = useState(false);
  const [environmentVisible, setEnvironmentVisible] = useState(true);
  const requestGeneration = useRef(0);
  const nextEntryId = useRef(Date.now());
  const editorRef = useRef<MonacoEditor.editor.IStandaloneCodeEditor>();
  const intelligenceDisposablesRef = useRef<MonacoEditor.IDisposable[]>([]);
  const environmentRef = useRef(environment);
  const transcriptRef = useRef<HTMLDivElement>(null);
  const runRef = useRef<() => void>(() => undefined);
  const stopRef = useRef<() => void>(() => undefined);
  const busyRef = useRef(false);
  const recallHistoryRef = useRef<(direction: -1 | 1) => void>(() => undefined);
  environmentRef.current = environment;

  useEffect(() => {
    let disposed = false;
    void bridge.startConsole()
      .then((response) => {
        if (disposed) return;
        setEnvironment(response.environment);
        setConnected(true);
      })
      .catch((error) => showNotice(String(error)));
    return () => {
      disposed = true;
      for (const disposable of intelligenceDisposablesRef.current) disposable.dispose();
      intelligenceDisposablesRef.current = [];
      onDocumentUnmount(consoleDocumentPath);
    };
  }, [onDocumentUnmount, showNotice, workspaceRoot]);

  useEffect(() => {
    try {
      window.localStorage.setItem(transcriptKey, JSON.stringify(transcript.slice(-100)));
    } catch {
      // Large scientific results can exceed browser storage; the live transcript still remains.
    }
    window.requestAnimationFrame(() => {
      if (transcriptRef.current) transcriptRef.current.scrollTop = transcriptRef.current.scrollHeight;
    });
  }, [transcript, transcriptKey]);

  useEffect(() => {
    window.localStorage.setItem(historyKey, JSON.stringify(history.slice(-200)));
  }, [history, historyKey]);

  const runSource = useCallback(async (candidate: string, clearInput: boolean) => {
    const input = candidate.trim();
    if (!input || busy) return;
    const id = ++nextEntryId.current;
    const generation = requestGeneration.current;
    setTranscript((current) => [...current, { id, source: input }]);
    setHistory((current) => [...current.filter((item) => item !== input), input].slice(-200));
    setHistoryIndex(-1);
    if (clearInput) {
      setSource("");
      onDocumentChange(consoleDocumentPath, "");
    }
    setBusy(true);
    try {
      const response = await bridge.evaluateConsole(input);
      if (generation !== requestGeneration.current) return;
      setTranscript((current) => current.map((entry) => entry.id === id ? { ...entry, response } : entry));
      setEnvironment(response.environment);
      setConnected(true);
    } catch (error) {
      if (generation !== requestGeneration.current) return;
      const message = String(error);
      setTranscript((current) => current.map((entry) => entry.id === id
        ? { ...entry, response: {
            protocol: "biolang.console/v1",
            id,
            status: "error",
            output: "",
            error: message,
            durationMs: 0,
            environment,
          } }
        : entry));
      setConnected(false);
    } finally {
      if (generation === requestGeneration.current) setBusy(false);
      // Only pull focus back for input the console owns. Code sent from the
      // file editor has to leave the caret where it is, or stepping through a
      // script line by line would need a click between every line.
      if (clearInput) window.requestAnimationFrame(() => editorRef.current?.focus());
    }
  }, [busy, environment, onDocumentChange]);

  const run = useCallback(() => runSource(source, true), [runSource, source]);

  // Code sent from the editor evaluates without disturbing whatever half-typed
  // expression is sitting in the console input.
  const lastSubmissionRef = useRef(0);
  // Captured on the first render, before any effect has had a chance to consume
  // it: code already waiting here means the pane was opened by Shift+Enter from
  // the file editor rather than by someone wanting to type at the prompt.
  const openedBySubmissionRef = useRef(Boolean(submission));
  useEffect(() => {
    // `busy` is a dependency rather than an early bail so that code sent while
    // an evaluation is still running is held and dispatched afterwards instead
    // of being marked consumed and silently dropped.
    if (!submission || submission.id === lastSubmissionRef.current || busy) return;
    lastSubmissionRef.current = submission.id;
    void runSource(submission.source, false);
  }, [busy, runSource, submission]);

  const stop = useCallback(async () => {
    if (!busy) return;
    requestGeneration.current += 1;
    setBusy(false);
    setConnected(false);
    setEnvironment(emptyEnvironment);
    setTranscript((current) => {
      const lastPending = [...current].reverse().find((entry) => !entry.response);
      return current.map((entry) => entry.id === lastPending?.id ? { ...entry, interrupted: true } : entry);
    });
    try {
      await bridge.stopConsole();
      showNotice("Console evaluation stopped; session state was cleared");
    } catch (error) {
      showNotice(String(error));
    }
  }, [busy, showNotice]);
  busyRef.current = busy;
  stopRef.current = () => void stop();

  const restart = useCallback(async () => {
    if (busy) await bridge.stopConsole();
    requestGeneration.current += 1;
    setBusy(false);
    try {
      const response = busy ? await bridge.startConsole() : await bridge.resetConsole();
      setEnvironment(response.environment);
      setConnected(true);
      setTranscript((current) => [...current, {
        id: ++nextEntryId.current,
        source: ":restart",
        response: { ...response, output: "Session restarted. User objects were removed.\n" },
      }]);
    } catch (error) {
      setConnected(false);
      showNotice(String(error));
    }
  }, [busy, showNotice]);

  const refreshEnvironment = useCallback(async () => {
    try {
      const response = await bridge.inspectConsole();
      setEnvironment(response.environment);
      setConnected(true);
    } catch (error) {
      setConnected(false);
      showNotice(String(error));
    }
  }, [showNotice]);

  const recallHistory = useCallback((direction: -1 | 1) => {
    if (!history.length) return;
    const next = direction < 0
      ? Math.min(history.length - 1, historyIndex < 0 ? history.length - 1 : historyIndex + 1)
      : Math.max(-1, historyIndex - 1);
    setHistoryIndex(next);
    const value = next < 0 ? "" : history[next];
    setSource(value);
    onDocumentChange(consoleDocumentPath, value);
    editorRef.current?.setValue(value);
    editorRef.current?.setPosition({ lineNumber: 1, column: value.length + 1 });
  }, [history, historyIndex, onDocumentChange]);
  runRef.current = () => void run();
  recallHistoryRef.current = recallHistory;

  const onMount: OnMount = useCallback((editor, monaco) => {
    editorRef.current = editor;
    if (!intelligenceDisposablesRef.current.length) {
      intelligenceDisposablesRef.current = [
        monaco.languages.registerCompletionItemProvider("biolang", {
          triggerCharacters: ["."],
          provideCompletionItems(model, position) {
            if (!model.uri.path.endsWith(consoleDocumentPath)) return { suggestions: [] };
            const before = model.getValueInRange({
              startLineNumber: position.lineNumber,
              startColumn: 1,
              endLineNumber: position.lineNumber,
              endColumn: position.column,
            });
            const target = before.match(/([A-Za-z_]\w*)\.[A-Za-z_]*$/)?.[1];
            const word = model.getWordUntilPosition(position);
            const range = {
              startLineNumber: position.lineNumber,
              endLineNumber: position.lineNumber,
              startColumn: word.startColumn,
              endColumn: position.column,
            };
            if (target) {
              const variable = environmentRef.current.variables.find((entry) => entry.name === target);
              return {
                suggestions: (variable?.members ?? [])
                  .filter((member) => !member.startsWith("_"))
                  .map((member) => ({
                    label: member,
                    kind: monaco.languages.CompletionItemKind.Field,
                    detail: `runtime field of ${target}`,
                    insertText: member,
                    range,
                  })),
              };
            }
            return {
              suggestions: environmentRef.current.variables.map((variable) => ({
                label: variable.name,
                kind: monaco.languages.CompletionItemKind.Variable,
                detail: variable.typeName,
                documentation: { value: `Current value: \`${variable.preview}\`` },
                insertText: variable.name,
                range,
                sortText: `0_${variable.name}`,
              })),
            };
          },
        }),
        monaco.languages.registerHoverProvider("biolang", {
          provideHover(model, position) {
            if (!model.uri.path.endsWith(consoleDocumentPath)) return null;
            const word = model.getWordAtPosition(position)?.word;
            if (!word) return null;
            const variable = environmentRef.current.variables.find((entry) => entry.name === word);
            if (!variable) return null;
            return {
              contents: [
                { value: `**${variable.name}**: \`${variable.typeName}\`` },
                { value: `Current value: \`${variable.preview}\`` },
              ],
            };
          },
        }),
      ];
    }
    editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter, () => runRef.current());
    editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyC, () => {
      if (busyRef.current) {
        stopRef.current();
        return;
      }
      editor.trigger("console", "editor.action.clipboardCopyAction", null);
    });
    editor.addCommand(monaco.KeyCode.UpArrow, () => {
      const position = editor.getPosition();
      if (position?.lineNumber === 1 && position.column === 1) recallHistoryRef.current(-1);
      else editor.trigger("console", "cursorUp", null);
    });
    editor.addCommand(monaco.KeyCode.DownArrow, () => {
      const position = editor.getPosition();
      const model = editor.getModel();
      if (position && model && position.lineNumber === model.getLineCount()) recallHistoryRef.current(1);
      else editor.trigger("console", "cursorDown", null);
    });
    void onDocumentMount(consoleDocumentPath, source, monaco);
    // Mounting because the file editor pushed code in must not move focus: the
    // author is stepping through a script and expects the caret to stay there.
    if (!openedBySubmissionRef.current) editor.focus();
  }, [onDocumentMount, source]);

  // Clicking a name evaluates it, which is what View() means in practice: the
  // transcript already renders tables and sequences, and unlike a throwaway
  // viewer window the result stays in the record of what was inspected.
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const toggleExpanded = (name: string) => {
    setExpanded((current) => {
      const next = new Set(current);
      if (!next.delete(name)) next.add(name);
      return next;
    });
  };

  const sessionLabel = connected ? `${environment.variables.length} object${environment.variables.length === 1 ? "" : "s"}` : "starting";
  const environmentRows = useMemo(() => environment.variables, [environment.variables]);

  return <div className={`console-pane${environmentVisible ? " environment-open" : ""}`}>
    <section className="console-main">
      <header className="console-toolbar">
        <span className={`console-connection ${connected ? "connected" : ""}`}><i />Session 1 <small>{sessionLabel}</small></span>
        <div>
          <button type="button" title="Clear console transcript" aria-label="Clear console transcript" onClick={() => setTranscript([])}><Eraser size={13} /></button>
          <button type="button" title="Restart console session" aria-label="Restart console session" onClick={() => void restart()}><RotateCcw size={13} /></button>
          <button type="button" className={environmentVisible ? "active" : ""} title="Toggle Environment" aria-label="Toggle Environment" onClick={() => setEnvironmentVisible((value) => !value)}><Database size={13} /></button>
        </div>
      </header>
      <div className="console-transcript" ref={transcriptRef} aria-live="polite">
        {!transcript.length && <div className="console-empty"><TerminalSquare size={18} /><span>BioLang Console</span><small>State is retained between evaluations in this workspace.</small></div>}
        {transcript.map((entry, index) => <article className="console-entry" key={entry.id}>
          <div className="console-prompt"><span>In [{index + 1}]</span><pre>{entry.source}</pre></div>
          {entry.interrupted
            ? <div className="console-interrupted"><CircleStop size={11} />Evaluation stopped; session state cleared.</div>
            : entry.response
              ? <ConsoleResult response={entry.response} />
              : <div className="console-running"><LoaderCircle size={12} className="spin" />Evaluating...</div>}
        </article>)}
      </div>
      <div className="console-input">
        <span className="console-input-prompt">&gt;</span>
        <div className="console-editor">
          <CodeEditor
            beforeMount={beforeMount}
            onMount={onMount}
            path="file:///workspace/__notebook__/console/session.bl"
            language="biolang"
            value={source}
            onChange={(value) => {
              const next = value ?? "";
              setSource(next);
              onDocumentChange(consoleDocumentPath, next);
            }}
            theme={editorTheme}
            options={{
              ariaLabel: "BioLang Console input",
              automaticLayout: true,
              contextmenu: true,
              fontFamily: '"Cascadia Code", "SFMono-Regular", Consolas, monospace',
              fontSize,
              glyphMargin: false,
              lineDecorationsWidth: 4,
              lineHeight: 20,
              lineNumbers: "off",
              minimap: { enabled: false },
              overviewRulerBorder: false,
              padding: { top: 8, bottom: 8 },
              renderLineHighlight: "none",
              scrollBeyondLastLine: false,
              scrollbar: { verticalScrollbarSize: 6 },
              tabSize,
              wordWrap: "on",
            }}
          />
        </div>
        {busy
          ? <button type="button" className="console-run stop" title="Stop evaluation" aria-label="Stop console evaluation" onClick={() => void stop()}><CircleStop size={14} /></button>
          : <button type="button" className="console-run" title="Evaluate (Ctrl+Enter)" aria-label="Evaluate console input" disabled={!source.trim()} onClick={() => void run()}><Play size={14} fill="currentColor" /></button>}
      </div>
    </section>
    {environmentVisible && <aside className="console-environment">
      <header><span><MemoryStick size={13} />Environment</span><button type="button" title="Refresh Environment" aria-label="Refresh Environment" onClick={() => void refreshEnvironment()}><RefreshCw size={12} /></button></header>
      <div className="console-memory"><strong>{formatConsoleBytes(environment.totalBytes)}</strong><span>Estimated object memory</span></div>
      <div className="console-variable-head"><span>Name</span><span>Type</span><span>Size</span></div>
      <div className="console-variables">
        {environmentRows.length
          ? environmentRows.map((variable) => <div className="console-variable-entry" key={variable.name}>
              <div className="console-variable">
                {variable.members.length
                  ? <button
                      type="button"
                      className="console-variable-disclosure"
                      aria-label={`${expanded.has(variable.name) ? "Hide" : "Show"} fields of ${variable.name}`}
                      aria-expanded={expanded.has(variable.name)}
                      onClick={() => toggleExpanded(variable.name)}
                    >{expanded.has(variable.name) ? <ChevronDown size={11} /> : <ChevronRight size={11} />}</button>
                  : <i className="console-variable-bullet" />}
                <button
                  type="button"
                  className="console-variable-open"
                  title={`Evaluate ${variable.name}`}
                  onClick={() => void runSource(variable.name, false)}
                >
                  <span>{variable.name}<small>{variable.preview}</small></span>
                  <code>{variable.typeName}</code>
                  <span className="console-variable-size">{formatConsoleBytes(variable.sizeBytes)}</span>
                </button>
              </div>
              {expanded.has(variable.name) && <div className="console-variable-members">
                {variable.members.map((member) => <button
                  type="button"
                  key={member}
                  title={`Evaluate ${variable.name}.${member}`}
                  onClick={() => void runSource(`${variable.name}.${member}`, false)}
                >{member}</button>)}
              </div>}
            </div>)
          : <div className="console-no-variables">No user objects in this session.</div>}
      </div>
    </aside>}
  </div>;
}
