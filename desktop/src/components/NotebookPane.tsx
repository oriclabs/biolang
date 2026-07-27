import type { BeforeMount, Monaco } from "@monaco-editor/react";
import {
  AlignLeft,
  Braces,
  ChartNoAxesCombined,
  Check,
  CircleStop,
  Eye,
  EyeOff,
  FileCode2,
  FileText,
  Image,
  LoaderCircle,
  Pencil,
  Play,
  Settings2,
  Table2,
  X,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import {
  parseNotebook,
  setNotebookDirective,
  updateNotebookBlock,
  type NotebookBlock,
  type NotebookDirective,
} from "../notebooks";
import type { NotebookCellOutput } from "../types";
import CodeEditor from "./CodeEditor";

type OutputView = "text" | "table" | "chart" | "visual";

interface StructuredOutput {
  rows: string[][];
  header: boolean;
}

const directiveOptions: Array<{ value: NotebookDirective; label: string; description: string }> = [
  { value: "hide", label: "Hide code", description: "Run this cell without displaying its source" },
  { value: "skip", label: "Skip", description: "Do not execute this cell" },
  { value: "echo", label: "Echo", description: "Copy the source into the run transcript" },
  { value: "hide-output", label: "Hide output", description: "Execute without displaying cell output" },
  { value: "chat", label: "Chat", description: "Send this cell as a prompt through BioLang chat" },
];

function structuredOutput(text: string): StructuredOutput | undefined {
  const trimmed = text.trim();
  if (!trimmed) return undefined;
  try {
    const parsed = JSON.parse(trimmed) as unknown;
    if (Array.isArray(parsed) && parsed.length && parsed.every((value) => value && typeof value === "object" && !Array.isArray(value))) {
      const records = parsed as Array<Record<string, unknown>>;
      const columns = [...new Set(records.flatMap((record) => Object.keys(record)))];
      return {
        rows: [columns, ...records.map((record) => columns.map((column) => String(record[column] ?? "")))],
        header: true,
      };
    }
  } catch {
    // Plain output falls through to strict TSV detection.
  }

  const rows = trimmed.split(/\r?\n/).map((line) => line.split("\t"));
  const columnCount = rows[0]?.length ?? 0;
  const header = rows[0]?.every((cell) =>
    cell.trim().length > 0 && !Number.isFinite(Number(cell))) ?? false;
  const tabular = rows.length > 1
    && columnCount > 1
    && header
    && new Set(rows[0]).size === columnCount
    && rows.every((row) => row.length === columnCount)
    && (rows.length > 2 || rows.slice(1).some((row) => row.some((cell) => Number.isFinite(Number(cell)))));
  return tabular ? { rows, header: true } : undefined;
}

function svgOutput(text: string) {
  const start = text.indexOf("<svg");
  const end = text.lastIndexOf("</svg>");
  return start >= 0 && end > start ? text.slice(start, end + "</svg>".length) : undefined;
}

function OutputChart({ data }: { data: StructuredOutput }) {
  const rows = data.header ? data.rows.slice(1) : data.rows;
  const headings = data.header ? data.rows[0] : data.rows[0].map((_, index) => `Column ${index + 1}`);
  const numericColumn = headings.findIndex((_, index) =>
    index > 0 && rows.some((row) => Number.isFinite(Number(row[index]))));
  if (numericColumn < 0 || !rows.length) {
    return <div className="notebook-output-pending">No numeric column is available to plot.</div>;
  }
  const values = rows.map((row) => Number(row[numericColumn])).filter(Number.isFinite);
  const minimum = Math.min(...values);
  const maximum = Math.max(...values);
  const range = maximum - minimum || 1;
  const points = values.map((value, index) => {
    const x = values.length === 1 ? 50 : 4 + (index / (values.length - 1)) * 92;
    const y = 88 - ((value - minimum) / range) * 76;
    return `${x},${y}`;
  }).join(" ");
  return <figure className="notebook-output-chart">
    <figcaption>{headings[numericColumn]}</figcaption>
    <svg role="img" aria-label={`${headings[numericColumn]} plot`} viewBox="0 0 100 100" preserveAspectRatio="none">
      <line x1="4" y1="88" x2="96" y2="88" />
      <line x1="4" y1="12" x2="4" y2="88" />
      <polyline points={points} />
      {points.split(" ").map((point, index) => {
        const [cx, cy] = point.split(",");
        return <circle key={index} cx={cx} cy={cy} r="1.8" />;
      })}
    </svg>
    <div><span>{minimum}</span><span>{maximum}</span></div>
  </figure>;
}

function CellOutput({
  output,
  hidden,
}: {
  output: NotebookCellOutput;
  hidden: boolean;
}) {
  const structured = useMemo(() => structuredOutput(output.text), [output.text]);
  const svg = useMemo(() => svgOutput(output.text), [output.text]);
  const [view, setView] = useState<OutputView>(() => svg ? "visual" : structured ? "table" : "text");
  const effectiveView = view === "visual" && svg
    ? "visual"
    : view !== "text" && !structured
      ? "text"
      : view;

  return <section className={`notebook-cell-output ${output.status}${output.stale ? " stale" : ""}`}>
    <header>
      {output.status === "running"
        ? <LoaderCircle size={11} className="spin" />
        : output.status === "succeeded"
          ? <Check size={11} />
          : <X size={11} />}
      <span>{output.status}{output.stale ? " · stale" : ""}</span>
      {!hidden && (structured || svg) && <div className="notebook-output-views">
        <button type="button" className={effectiveView === "text" ? "active" : ""} title="Text output" aria-label="Text output" onClick={() => setView("text")}><AlignLeft size={12} /></button>
        {structured && <button type="button" className={effectiveView === "table" ? "active" : ""} title="Table output" aria-label="Table output" onClick={() => setView("table")}><Table2 size={12} /></button>}
        {structured && <button type="button" className={effectiveView === "chart" ? "active" : ""} title="Chart output" aria-label="Chart output" onClick={() => setView("chart")}><ChartNoAxesCombined size={12} /></button>}
        {svg && <button type="button" className={effectiveView === "visual" ? "active" : ""} title="Visual output" aria-label="Visual output" onClick={() => setView("visual")}><Image size={12} /></button>}
      </div>}
    </header>
    {hidden
      ? <div className="notebook-output-pending"><EyeOff size={12} />Output hidden by directive</div>
      : effectiveView === "visual" && svg
        ? <div className="notebook-output-visual"><img alt="BioLang cell visualization" src={`data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`} /></div>
        : effectiveView === "table" && structured
        ? <div className="notebook-output-table-wrap"><table>
            {structured.header && <thead><tr>{structured.rows[0].map((cell, index) => <th key={index}>{cell}</th>)}</tr></thead>}
            <tbody>{structured.rows.slice(structured.header ? 1 : 0).map((row, rowIndex) =>
              <tr key={rowIndex}>{row.map((cell, columnIndex) =>
                <td key={columnIndex}>{cell}</td>)}</tr>)}</tbody>
          </table></div>
        : effectiveView === "chart" && structured
          ? <OutputChart data={structured} />
          : output.text
            ? <pre>{output.text}</pre>
            : output.status === "running" && <div className="notebook-output-pending">Waiting for output...</div>}
  </section>;
}

function editorHeight(content: string, minimum = 92, maximum = 420) {
  return Math.max(minimum, Math.min(maximum, content.split(/\r?\n/).length * 21 + 30));
}

function virtualCellPath(notebookPath: string, cellIndex: number) {
  let hash = 2166136261;
  for (const character of notebookPath) {
    hash ^= character.charCodeAt(0);
    hash = Math.imul(hash, 16777619);
  }
  const name = notebookPath.split(/[\\/]/).at(-1)?.replace(/[^A-Za-z0-9._-]/g, "_") ?? "notebook";
  return `__notebook__/${name}-${(hash >>> 0).toString(36)}/cell-${cellIndex + 1}.bl`;
}

function modelUri(path: string) {
  return `file:///workspace/${path.split("/").map(encodeURIComponent).join("/")}`;
}

function DirectiveControls({
  block,
  onToggle,
}: {
  block: NotebookBlock;
  onToggle: (directive: NotebookDirective, enabled: boolean) => void;
}) {
  return <details className="notebook-directive-menu">
    <summary title="Cell directives" aria-label="Cell directives"><Settings2 size={12} /></summary>
    <div>
      {directiveOptions.map((option) => <label key={option.value} title={option.description}>
        <input
          type="checkbox"
          checked={block.directives.includes(option.value)}
          onChange={(event) => onToggle(option.value, event.target.checked)}
        />
        <span>{option.label}</span>
      </label>)}
    </div>
  </details>;
}

function NotebookCodeCell({
  notebookPath,
  block,
  cellIndex,
  content,
  output,
  running,
  editorTheme,
  fontSize,
  tabSize,
  wordWrap,
  beforeMount,
  onChange,
  onRun,
  onCellMount,
  onCellChange,
  onCellUnmount,
  onInvalidate,
}: {
  notebookPath: string;
  block: NotebookBlock;
  cellIndex: number;
  content: string;
  output?: NotebookCellOutput;
  running: boolean;
  editorTheme: string;
  fontSize: number;
  tabSize: number;
  wordWrap: boolean;
  beforeMount: BeforeMount;
  onChange: (content: string) => void;
  onRun: () => void | Promise<void>;
  onCellMount: (path: string, content: string, monaco: Monaco) => void | Promise<void>;
  onCellChange: (path: string, content: string) => void;
  onCellUnmount: (path: string) => void;
  onInvalidate: () => void;
}) {
  const path = useMemo(() => virtualCellPath(notebookPath, cellIndex), [cellIndex, notebookPath]);
  const hidden = block.directives.includes("hide");
  const skipped = block.directives.includes("skip");
  const chat = block.directives.includes("chat");

  useEffect(() => () => onCellUnmount(path), [onCellUnmount, path]);

  const changeCode = (value: string) => {
    onChange(updateNotebookBlock(content, block, value));
    onInvalidate();
    if (!chat) onCellChange(path, value);
  };
  const toggleDirective = (directive: NotebookDirective, enabled: boolean) => {
    onChange(setNotebookDirective(content, block, directive, enabled));
    onInvalidate();
  };

  return <section className={`notebook-cell${hidden ? " code-hidden" : ""}${skipped ? " skipped" : ""}`}>
    <div className="notebook-cell-gutter"><Braces size={13} /><span>{cellIndex + 1}</span></div>
    <div className="notebook-cell-header">
      {!!block.directives.length && <div className="notebook-directives">{block.directives.map((directive) => <code key={directive}>@{directive}</code>)}</div>}
      <div className="notebook-cell-actions">
        <DirectiveControls block={block} onToggle={toggleDirective} />
        <button
          type="button"
          title={skipped ? "Cell is skipped" : "Run cell"}
          aria-label={`Run code cell ${cellIndex + 1}`}
          disabled={running || skipped}
          onClick={() => void onRun()}
        ><Play size={12} fill="currentColor" /></button>
      </div>
    </div>
    {hidden
      ? <div className="notebook-hidden-code"><EyeOff size={13} /><span>Code hidden</span></div>
      : <div className="notebook-code-editor" style={{ height: editorHeight(block.content) }}>
          <CodeEditor
            key={chat ? "chat" : "biolang"}
            beforeMount={beforeMount}
            onMount={(editor, monaco) => {
              editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter, () => {
                if (!running && !skipped) void onRun();
              });
              if (chat) onCellUnmount(path);
              else void onCellMount(path, block.content, monaco);
            }}
            path={modelUri(path)}
            language={chat ? "markdown" : "biolang"}
            value={block.content}
            onChange={(value) => changeCode(value ?? "")}
            theme={editorTheme}
            options={{
              ariaLabel: `Code cell ${cellIndex + 1}`,
              automaticLayout: true,
              contextmenu: true,
              folding: false,
              fontFamily: '"Cascadia Code", "SFMono-Regular", Consolas, monospace',
              fontSize,
              glyphMargin: false,
              lineDecorationsWidth: 8,
              lineHeight: 21,
              lineNumbersMinChars: 2,
              minimap: { enabled: false },
              overviewRulerBorder: false,
              padding: { top: 10, bottom: 10 },
              renderLineHighlight: "gutter",
              scrollBeyondLastLine: false,
              scrollbar: { alwaysConsumeMouseWheel: false, verticalScrollbarSize: 8 },
              tabSize,
              wordWrap: wordWrap ? "on" : "off",
            }}
          />
        </div>}
    {skipped
      ? <div className="notebook-cell-state">Skipped</div>
      : output && <CellOutput output={output} hidden={block.directives.includes("hide-output")} />}
  </section>;
}

function MarkdownCell({
  block,
  content,
  editorTheme,
  fontSize,
  beforeMount,
  onChange,
}: {
  block: NotebookBlock;
  content: string;
  editorTheme: string;
  fontSize: number;
  beforeMount: BeforeMount;
  onChange: (content: string) => void;
}) {
  const [editing, setEditing] = useState(false);
  return <article className={`notebook-markdown${editing ? " editing" : ""}`}>
    <button
      type="button"
      className="notebook-markdown-edit"
      title={editing ? "Finish editing" : "Edit Markdown"}
      aria-label={editing ? "Finish editing Markdown" : "Edit Markdown"}
      onClick={() => setEditing((value) => !value)}
    >{editing ? <Check size={12} /> : <Pencil size={12} />}</button>
    {editing
      ? <div className="notebook-markdown-editor" style={{ height: editorHeight(block.content, 120, 520) }}>
          <CodeEditor
            beforeMount={beforeMount}
            language="markdown"
            value={block.content}
            onChange={(value) => onChange(updateNotebookBlock(content, block, value ?? ""))}
            theme={editorTheme}
            options={{
              ariaLabel: "Markdown cell",
              automaticLayout: true,
              fontSize,
              lineHeight: 21,
              minimap: { enabled: false },
              padding: { top: 10, bottom: 10 },
              scrollBeyondLastLine: false,
              wordWrap: "on",
            }}
          />
        </div>
      : <ReactMarkdown remarkPlugins={[remarkGfm]}>{block.content}</ReactMarkdown>}
  </article>;
}

function NotebookMetadata({ block }: { block: NotebookBlock }) {
  const entries = block.content.split(/\r?\n/)
    .map((line) => line.split(/:(.*)/s))
    .filter((parts) => parts.length > 1)
    .map(([key, value]) => [key.trim(), value.trim()]);
  return <section className="notebook-metadata">
    <FileText size={14} />
    <dl>{entries.map(([key, value]) => <div key={key}><dt>{key}</dt><dd>{value}</dd></div>)}</dl>
  </section>;
}

export function NotebookPane({
  name,
  path,
  content,
  output,
  cellOutputs,
  running,
  editorTheme,
  fontSize,
  tabSize,
  wordWrap,
  beforeMount,
  onChange,
  onRun,
  onRunCell,
  onStop,
  onCellMount,
  onCellChange,
  onCellUnmount,
  onInvalidateCell,
}: {
  name: string;
  path: string;
  content: string;
  output: string;
  cellOutputs: Record<number, NotebookCellOutput>;
  running: boolean;
  editorTheme: string;
  fontSize: number;
  tabSize: number;
  wordWrap: boolean;
  beforeMount: BeforeMount;
  onChange: (content: string) => void;
  onRun: () => void | Promise<void>;
  onRunCell: (cellIndex: number) => void | Promise<void>;
  onStop: () => void | Promise<void>;
  onCellMount: (path: string, content: string, monaco: Monaco) => void | Promise<void>;
  onCellChange: (path: string, content: string) => void;
  onCellUnmount: (path: string) => void;
  onInvalidateCell: (cellIndex: number) => void;
}) {
  const [mode, setMode] = useState<"notebook" | "source">("notebook");
  const blocks = useMemo(() => parseNotebook(content), [content]);
  const indexedBlocks = useMemo(() => {
    let cellIndex = 0;
    return blocks.map((block) => ({
      block,
      cellIndex: block.type === "code" ? cellIndex++ : undefined,
    }));
  }, [blocks]);

  return <div className="notebook-pane">
    <header className="notebook-toolbar">
      <strong>{name}</strong>
      <span>{blocks.filter((block) => block.type === "code").length} code cells</span>
      <span className="notebook-lsp-status">BioLang cells</span>
      <div className="segmented-control">
        <button type="button" title="Notebook" aria-label="Notebook" className={mode === "notebook" ? "active" : ""} onClick={() => setMode("notebook")}><Eye size={13} /></button>
        <button type="button" title="Source" aria-label="Source" className={mode === "source" ? "active" : ""} onClick={() => setMode("source")}><FileCode2 size={13} /></button>
      </div>
      {running
        ? <button type="button" className="notebook-run danger" onClick={() => void onStop()}><CircleStop size={14} />Stop</button>
        : <button type="button" className="notebook-run" onClick={() => void onRun()}><Play size={14} />Run all</button>}
    </header>
    {mode === "source"
      ? <div className="notebook-source">
          <CodeEditor
            beforeMount={beforeMount}
            path={`${modelUri(path)}?notebook-source`}
            language="markdown"
            value={content}
            onChange={(value) => onChange(value ?? "")}
            theme={editorTheme}
            options={{
              ariaLabel: `${name} source`,
              automaticLayout: true,
              fontFamily: '"Cascadia Code", "SFMono-Regular", Consolas, monospace',
              fontSize,
              lineHeight: 21,
              minimap: { enabled: false },
              padding: { top: 14 },
              scrollBeyondLastLine: false,
              tabSize,
              wordWrap: wordWrap ? "on" : "off",
            }}
          />
        </div>
      : <main className="notebook-canvas">
          {indexedBlocks.map(({ block, cellIndex }, index) =>
            block.type === "metadata"
              ? <NotebookMetadata key={`metadata-${block.start}`} block={block} />
              : block.type === "markdown"
                ? <MarkdownCell
                    key={`markdown-${block.start}-${index}`}
                    block={block}
                    content={content}
                    editorTheme={editorTheme}
                    fontSize={fontSize}
                    beforeMount={beforeMount}
                    onChange={onChange}
                  />
                : <NotebookCodeCell
                    key={`code-${cellIndex}`}
                    notebookPath={path}
                    block={block}
                    cellIndex={cellIndex ?? 0}
                    content={content}
                    output={cellOutputs[cellIndex ?? 0]}
                    running={running}
                    editorTheme={editorTheme}
                    fontSize={fontSize}
                    tabSize={tabSize}
                    wordWrap={wordWrap}
                    beforeMount={beforeMount}
                    onChange={onChange}
                    onRun={() => onRunCell(cellIndex ?? 0)}
                    onCellMount={onCellMount}
                    onCellChange={onCellChange}
                    onCellUnmount={onCellUnmount}
                    onInvalidate={() => onInvalidateCell(cellIndex ?? 0)}
                  />)}
          {!!output.trim() && !Object.keys(cellOutputs).length
            && <section className="notebook-output"><span>Run output</span><pre>{output}</pre></section>}
        </main>}
  </div>;
}
