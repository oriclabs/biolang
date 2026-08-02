import {
  ArrowDownToLine,
  Copy,
  Download,
  ExternalLink,
  Eye,
  FileArchive,
  FileText,
  Check,
  GitCompare,
  GripVertical,
  LoaderCircle,
  Eraser,
  MoreHorizontal,
  PanelBottom,
  PanelRight,
  Pencil,
  Pin,
  PinOff,
  Printer,
  RefreshCw,
  Search,
  Trash2,
  WrapText,
  X,
  ZoomIn,
  ZoomOut,
} from "lucide-react";
import type { PointerEvent as ReactPointerEvent, ReactNode } from "react";
import { useEffect, useMemo, useRef, useState } from "react";
import { bridge } from "../bridge";
import { jobLogText } from "../jobLogs";
import {
  availableOutputTabs,
  outputErrors,
  outputPlots,
  outputTables,
  semanticResultPairs,
  type OutputTab,
  type OutputTable,
  type OutputPlot,
  valuePreview,
} from "../outputModel";
import type { OutputExportFormat, OutputExportOption } from "../outputExport";
import { bibtex, methodsDocument, methodsParagraph } from "../methods";
import type {
  Job,
  JobArtifact,
  JobProvenance,
  RestoreReport,
  ResultPageData,
  ResultPageRequest,
} from "../types";
import { JobLog } from "./JobLog";

export type OutputLocation = "bottom" | "right" | "editor";

/// True when a run wrote text a reader would want to see, ignoring the banners
/// the workbench adds itself — `running <file>` and `Process completed in ...`
/// are chrome, not program output — and ignoring raw SVG, which is shown as a
/// plot rather than read as text.
const CHROME_STREAMS = new Set(["system", "success"]);

function hasReadableLog(job: Job | undefined): boolean {
  return (job?.log ?? []).some((chunk) => !CHROME_STREAMS.has(chunk.stream)
    && chunk.text.replace(/<svg\b[\s\S]*?<\/svg>/gi, "").trim().length > 0);
}

function artifactFormat(artifact: JobArtifact) {
  const extension = artifact.name.split(".").pop()?.toLowerCase() ?? "";
  if (["fa", "fasta", "fna"].includes(extension)) return { label: "FASTA", text: true };
  if (["fq", "fastq"].includes(extension)) return { label: "FASTQ", text: true };
  if (["vcf", "bed", "gff", "gff3", "gtf", "sam"].includes(extension)) return { label: extension.toUpperCase(), text: true };
  if (["csv", "tsv"].includes(extension)) return { label: extension.toUpperCase(), text: true };
  if (["jsonl", "ndjson"].includes(extension)) return { label: "JSONL", text: true };
  if (extension === "bam") return { label: "BAM", text: false, detail: "Indexed binary alignments. Open with the workspace alignment viewer or export with its .bai index." };
  if (extension === "cram") return { label: "CRAM", text: false, detail: "Reference-compressed alignments. A matching reference and .crai index are required." };
  if (["h5", "hdf5", "h5ad"].includes(extension)) return { label: extension === "h5ad" ? "AnnData" : "HDF5", text: false, detail: "Hierarchical scientific dataset. Use the data viewer or a package reader for matrix-level access." };
  if (extension === "zarr") return { label: "Zarr", text: false, detail: "Chunked array dataset. Preview its metadata and open it with a Zarr-aware package." };
  if (["parquet", "arrow", "feather"].includes(extension)) return { label: "Columnar", text: false, detail: "Columnar dataset designed for paged analysis. Open it through a table reader." };
  return { label: extension ? extension.toUpperCase() : "FILE", text: undefined };
}

function ToolButton({
  label,
  active,
  disabled,
  className,
  onClick,
  children,
}: {
  label: string;
  active?: boolean;
  disabled?: boolean;
  className?: string;
  onClick: () => void;
  children: ReactNode;
}) {
  return (
    <button
      type="button"
      className={`icon-button ${active ? "active" : ""} ${className ?? ""}`}
      aria-label={label}
      title={label}
      disabled={disabled}
      onClick={onClick}
    >
      {children}
    </button>
  );
}

function TableResult({
  table,
  onLoadPage,
}: {
  table: OutputTable;
  onLoadPage?: (request: ResultPageRequest) => Promise<ResultPageData>;
}) {
  const [query, setQuery] = useState("");
  const [sortColumn, setSortColumn] = useState<number>();
  const [descending, setDescending] = useState(false);
  const [page, setPage] = useState(0);
  const [pageSize, setPageSize] = useState(50);
  const [hidden, setHidden] = useState<Set<number>>(() => new Set());
  const [remotePage, setRemotePage] = useState<ResultPageData>();
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState<string>();
  const visibleColumns = table.columns
    .map((name, index) => ({ name, index }))
    .filter(({ index }) => !hidden.has(index));
  const localRows = useMemo(() => {
    const needle = query.trim().toLowerCase();
    const filtered = needle
      ? table.rows.filter((row) => row.some((value) => valuePreview(value).toLowerCase().includes(needle)))
      : [...table.rows];
    if (sortColumn != null) {
      filtered.sort((left, right) => {
        const a = left[sortColumn];
        const b = right[sortColumn];
        const order = typeof a === "number" && typeof b === "number"
          ? a - b
          : valuePreview(a).localeCompare(valuePreview(b), undefined, { numeric: true });
        return descending ? -order : order;
      });
    }
    return filtered;
  }, [descending, query, sortColumn, table.rows]);
  const filteredRows = onLoadPage ? remotePage?.filteredRows ?? table.totalRows : localRows.length;
  const pages = Math.max(1, Math.ceil(filteredRows / pageSize));
  const visibleRows = onLoadPage
    ? remotePage?.rows ?? table.rows.slice(0, pageSize)
    : localRows.slice(page * pageSize, (page + 1) * pageSize);

  useEffect(() => setPage(0), [query, sortColumn, descending]);
  useEffect(() => {
    if (!onLoadPage) return;
    let active = true;
    const timer = window.setTimeout(() => {
      setLoading(true);
      setLoadError(undefined);
      void onLoadPage({
        offset: page * pageSize,
        limit: pageSize,
        search: query.trim() || undefined,
        sortColumn,
        descending,
      }).then((result) => {
        if (active) setRemotePage(result);
      }).catch((error) => {
        if (active) setLoadError(String(error));
      }).finally(() => {
        if (active) setLoading(false);
      });
    }, 180);
    return () => {
      active = false;
      window.clearTimeout(timer);
    };
  }, [descending, onLoadPage, page, pageSize, query, sortColumn]);

  const sort = (column: number) => {
    if (sortColumn === column) setDescending((value) => !value);
    else {
      setSortColumn(column);
      setDescending(false);
    }
  };

  return (
    <div className="output-table-result">
      <div className="output-result-controls">
        <label><Search size={12} /><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Filter rows" /></label>
        <details>
          <summary>Columns</summary>
          <div className="output-column-menu">
            {table.columns.map((column, index) => (
              <label key={column}>
                <input
                  type="checkbox"
                  checked={!hidden.has(index)}
                  onChange={() => setHidden((current) => {
                    const next = new Set(current);
                    if (next.has(index)) next.delete(index);
                    else next.add(index);
                    return next;
                  })}
                />
                {column}
              </label>
            ))}
          </div>
        </details>
        <span>{filteredRows.toLocaleString()} of {Math.max(table.totalRows, remotePage?.totalRows ?? 0).toLocaleString()} rows{table.truncated && !onLoadPage ? " (preview)" : ""}</span>
        {loading && <span role="status"><LoaderCircle size={12} className="spin" /> Loading page</span>}
      </div>
      {loadError && <div className="output-table-error" role="alert">{loadError}</div>}
      <div className="output-table-scroll">
        <table>
          <thead><tr>{visibleColumns.map(({ name, index }) => (
            <th key={name}><button type="button" onClick={() => sort(index)}>{name}{sortColumn === index ? descending ? " v" : " ^" : ""}</button></th>
          ))}</tr></thead>
          <tbody>{visibleRows.map((row, rowIndex) => (
            <tr key={rowIndex}>{visibleColumns.map(({ name, index }) => <td key={name}>{valuePreview(row[index])}</td>)}</tr>
          ))}</tbody>
        </table>
      </div>
      <footer>
        <label>Rows <select value={pageSize} onChange={(event) => {
          setPageSize(Number(event.target.value));
          setPage(0);
        }}><option value={50}>50</option><option value={100}>100</option><option value={250}>250</option></select></label>
        <button type="button" disabled={page === 0} onClick={() => setPage((value) => value - 1)}>Previous</button>
        <span>Page {page + 1} of {pages}</span>
        <button type="button" disabled={page + 1 >= pages} onClick={() => setPage((value) => value + 1)}>Next</button>
      </footer>
    </div>
  );
}

function CompareRuns({ left, right }: { left: Job; right: Job }) {
  const leftTables = outputTables(left);
  const rightTables = outputTables(right);
  const leftPlots = outputPlots(left);
  const rightPlots = outputPlots(right);
  const metrics = [
    ["Status", left.status, right.status],
    ["Backend", left.backend, right.backend],
    ["Duration", left.durationMs == null ? "-" : `${left.durationMs} ms`, right.durationMs == null ? "-" : `${right.durationMs} ms`],
    ["Results", String(left.results?.length ?? 0), String(right.results?.length ?? 0)],
    ["Tables", String(leftTables.length), String(rightTables.length)],
    ["Plots", String(leftPlots.length), String(rightPlots.length)],
    ["Artifacts", String(left.artifacts?.length ?? 0), String(right.artifacts?.length ?? 0)],
    ["Output", `${jobLogText(left.log).length} chars`, `${jobLogText(right.log).length} chars`],
    ["Source", left.provenance?.sourceHash?.slice(0, 12) ?? "-", right.provenance?.sourceHash?.slice(0, 12) ?? "-"],
  ];
  const resultRows = semanticResultPairs(left, right).map(({ key, left: a, right: b }) => {
    const aValue = a?.value;
    const bValue = b?.value;
    const delta = typeof aValue === "number" && typeof bValue === "number"
      ? String(bValue - aValue)
      : "-";
    return [
      key,
      a ? `${a.kind}: ${valuePreview(aValue ?? a.display ?? "")}` : "-",
      b ? `${b.kind}: ${valuePreview(bValue ?? b.display ?? "")}` : "-",
      delta,
    ];
  });
  const packageNames = [...new Set([
    ...Object.keys(left.provenance?.packages ?? {}),
    ...Object.keys(right.provenance?.packages ?? {}),
  ])].sort();
  const parameterNames = [...new Set([
    ...Object.keys(left.provenance?.parameters ?? {}),
    ...Object.keys(right.provenance?.parameters ?? {}),
  ])].sort();
  const inputPaths = [...new Set([
    ...(left.provenance?.inputs ?? []).map((input) => input.path),
    ...(right.provenance?.inputs ?? []).map((input) => input.path),
  ])].sort();
  const artifactNames = [...new Set([
    ...(left.artifacts ?? []).map((artifact) => artifact.path ?? artifact.name),
    ...(right.artifacts ?? []).map((artifact) => artifact.path ?? artifact.name),
  ])].sort();
  const provenanceRows = [
    ["Platform", left.provenance?.platform ?? "-", right.provenance?.platform ?? "-"],
    ["Architecture", left.provenance?.architecture ?? "-", right.provenance?.architecture ?? "-"],
    ["Random seed", left.provenance?.randomSeed ?? "-", right.provenance?.randomSeed ?? "-"],
    ["Inputs", String(left.provenance?.inputs?.length ?? 0), String(right.provenance?.inputs?.length ?? 0)],
    ...packageNames.map((name) => [
      `Package: ${name}`,
      left.provenance?.packages[name] ?? "-",
      right.provenance?.packages[name] ?? "-",
    ]),
    ...parameterNames.map((name) => [
      `Parameter: ${name}`,
      String(left.provenance?.parameters[name] ?? "-"),
      String(right.provenance?.parameters[name] ?? "-"),
    ]),
    ...inputPaths.map((path) => {
      const a = left.provenance?.inputs?.find((input) => input.path === path);
      const b = right.provenance?.inputs?.find((input) => input.path === path);
      return [
        `Input: ${path}`,
        a ? `${a.sha256?.slice(0, 12) ?? a.checksumStatus} | ${a.size}` : "-",
        b ? `${b.sha256?.slice(0, 12) ?? b.checksumStatus} | ${b.size}` : "-",
      ];
    }),
  ];
  return (
    <div className="output-compare">
      <header><strong>{left.displayName ?? left.file}</strong><strong>{right.displayName ?? right.file}</strong></header>
      {metrics.map(([label, a, b]) => <div key={label}><span>{label}</span><code>{a}</code><code className={a === b ? "" : "changed"}>{b}</code></div>)}
      {resultRows.length > 0 && <>
        <h3>Structured results</h3>
        <div className="output-compare-results"><strong>Result</strong><strong>Run A</strong><strong>Run B</strong><strong>Delta</strong>
          {resultRows.flatMap((row) => row.map((value, index) => <code className={index > 0 && row[1] !== row[2] ? "changed" : ""} key={`${row[0]}-${index}`}>{value}</code>))}
        </div>
      </>}
      {artifactNames.length > 0 && <>
        <h3>Artifacts</h3>
        <div className="output-compare-provenance">
          {artifactNames.map((name) => {
            const a = left.artifacts?.find((artifact) => (artifact.path ?? artifact.name) === name);
            const b = right.artifacts?.find((artifact) => (artifact.path ?? artifact.name) === name);
            const leftValue = a ? `${a.sha256?.slice(0, 12) ?? "no hash"} | ${a.size ?? 0}` : "-";
            const rightValue = b ? `${b.sha256?.slice(0, 12) ?? "no hash"} | ${b.size ?? 0}` : "-";
            return <div key={name}><span>{name}</span><code>{leftValue}</code><code className={leftValue === rightValue ? "" : "changed"}>{rightValue}</code></div>;
          })}
        </div>
      </>}
      <h3>Provenance</h3>
      <div className="output-compare-provenance">
        {provenanceRows.map(([label, a, b]) => <div key={label}><span>{label}</span><code>{a}</code><code className={a === b ? "" : "changed"}>{b}</code></div>)}
      </div>
      <section><JobLog chunks={left.log} /><JobLog chunks={right.log} /></section>
    </div>
  );
}

function PlotGallery({ plots }: { plots: OutputPlot[] }) {
  const [plotIndex, setPlotIndex] = useState(0);
  const [zoom, setZoom] = useState(100);
  const [inspection, setInspection] = useState<{ x: number; y: number; label: string }>();
  const plot = plots[Math.min(plotIndex, plots.length - 1)];
  const safeSvg = useMemo(() => {
    if (!plot) return "";
    const document = new DOMParser().parseFromString(plot.svg, "image/svg+xml");
    document.querySelectorAll("script,foreignObject,iframe,object,embed").forEach((node) => node.remove());
    document.querySelectorAll("*").forEach((node) => {
      for (const attribute of [...node.attributes]) {
        if (attribute.name.toLowerCase().startsWith("on")
          || /^(?:href|xlink:href)$/i.test(attribute.name) && /^\s*javascript:/i.test(attribute.value)) {
          node.removeAttribute(attribute.name);
        }
      }
    });
    return new XMLSerializer().serializeToString(document.documentElement);
  }, [plot]);

  useEffect(() => {
    setZoom(100);
    setInspection(undefined);
  }, [plotIndex]);

  const inspectPlot = (event: ReactPointerEvent<HTMLDivElement>) => {
    const target = event.target as SVGElement;
    if (!(target instanceof SVGElement) || target.tagName.toLowerCase() === "svg") {
      setInspection(undefined);
      return;
    }
    const bounds = event.currentTarget.getBoundingClientRect();
    const title = target.querySelector("title")?.textContent
      || target.getAttribute("aria-label")
      || target.getAttribute("data-value")
      || target.textContent?.trim();
    const details = [
      title,
      target.getAttribute("data-x") && `x=${target.getAttribute("data-x")}`,
      target.getAttribute("data-y") && `y=${target.getAttribute("data-y")}`,
      target.getAttribute("cx") && `x=${target.getAttribute("cx")}`,
      target.getAttribute("cy") && `y=${target.getAttribute("cy")}`,
      target.getAttribute("fill") && `fill=${target.getAttribute("fill")}`,
    ].filter(Boolean);
    setInspection({
      x: event.clientX - bounds.left,
      y: event.clientY - bounds.top,
      label: details.join(" | ") || target.tagName.toLowerCase(),
    });
  };

  const exportSvg = async () => {
    if (plot) await bridge.exportText(`${plot.name.replace(/\s+/g, "-").toLowerCase()}.svg`, plot.svg);
  };
  /**
   * Rasterise the plot at print resolution.
   *
   * This used to draw the SVG at its natural pixel size — about 1200x800,
   * which is roughly 72 DPI at figure width. Journals ask for 300, so a figure
   * exported for a manuscript was unusable and nothing on screen said so.
   * Scaling the backing canvas while keeping the drawn size is what actually
   * raises effective DPI.
   */
  const exportPng = async (scale: number) => {
    if (!plot) return;
    const image = new Image();
    const source = URL.createObjectURL(new Blob([plot.svg], { type: "image/svg+xml" }));
    try {
      await new Promise<void>((resolve, reject) => {
        image.onload = () => resolve();
        image.onerror = () => reject(new Error("Cannot render SVG"));
        image.src = source;
      });
      const width = Math.max(1, image.naturalWidth || 1200);
      const height = Math.max(1, image.naturalHeight || 800);
      const canvas = document.createElement("canvas");
      canvas.width = Math.round(width * scale);
      canvas.height = Math.round(height * scale);
      const context = canvas.getContext("2d");
      if (!context) throw new Error("Canvas is unavailable");
      context.scale(scale, scale);
      // Journals reject transparent backgrounds more often than anything else
      // about a figure.
      context.fillStyle = "#fff";
      context.fillRect(0, 0, width, height);
      context.drawImage(image, 0, 0, width, height);
      const blob = await new Promise<Blob>((resolve, reject) =>
        canvas.toBlob((value) => value ? resolve(value) : reject(new Error("Cannot encode PNG")), "image/png"));
      const suffix = scale > 1 ? `@${scale}x` : "";
      await bridge.exportBinary(
        `${plot.name.replace(/\s+/g, "-").toLowerCase()}${suffix}.png`,
        new Uint8Array(await blob.arrayBuffer()),
        "image/png",
      );
    } finally {
      URL.revokeObjectURL(source);
    }
  };
  const printPlot = () => {
    if (!plot) return;
    const popup = window.open("", "_blank", "width=1000,height=760");
    if (!popup) return;
    popup.document.write(`<title>${plot.name}</title><style>html,body{margin:0;height:100%;display:grid;place-items:center}svg{max-width:96vw;max-height:96vh}</style>${plot.svg}`);
    popup.document.close();
    popup.addEventListener("load", () => popup.print(), { once: true });
  };

  return (
    <div className="output-plots-view">
      <aside>{plots.map((candidate, index) => <button type="button" className={plotIndex === index ? "active" : ""} onClick={() => setPlotIndex(index)} key={`${candidate.name}-${index}`}><img alt={candidate.name} src={`data:image/svg+xml;charset=utf-8,${encodeURIComponent(candidate.svg)}`} /><span>{candidate.name}</span></button>)}</aside>
      <div className="output-plot-stage">
        <div className="output-result-controls">
          <ToolButton label="Zoom out" disabled={zoom <= 25} onClick={() => setZoom((value) => Math.max(25, value - 25))}><ZoomOut size={12} /></ToolButton>
          <span>{zoom}%</span>
          <ToolButton label="Zoom in" disabled={zoom >= 400} onClick={() => setZoom((value) => Math.min(400, value + 25))}><ZoomIn size={12} /></ToolButton>
          <button type="button" title="Vector, best for journals that accept it" onClick={() => void exportSvg()}>SVG</button>
          <button type="button" title="Screen resolution" onClick={() => void exportPng(1)}>PNG</button>
          {/* 4x a ~1200px figure is about 300 DPI at single-column width,
              which is what journals ask for. */}
          <button type="button" title="4x scale, about 300 DPI at figure width" onClick={() => void exportPng(4)}>PNG 4x</button>
          <ToolButton label="Print or save plot as PDF" onClick={printPlot}><Printer size={12} /></ToolButton>
        </div>
        <figure
          className="output-interactive-plot"
          role="img"
          aria-label="BioLang plot output"
          onPointerMove={inspectPlot}
          onPointerLeave={() => setInspection(undefined)}
        >
          <div style={{ width: `${zoom}%`, maxWidth: "none" }} dangerouslySetInnerHTML={{ __html: safeSvg }} />
          {inspection && <output className="plot-inspector" style={{ left: inspection.x, top: inspection.y }}>{inspection.label}</output>}
        </figure>
      </div>
    </div>
  );
}

/**
 * Reproducibility check for one run.
 *
 * Provenance has always recorded package versions, input checksums, the seed,
 * and a source snapshot, but only ever stored them. This turns that record into
 * the answer to "why do I get different numbers than last time", and offers to
 * put back the parts that can be put back. What cannot be restored says so
 * rather than being quietly skipped.
 */
function RestorePanel({
  provenance,
  onCompare,
  onRestore,
}: {
  provenance: JobProvenance;
  onCompare: (provenance: JobProvenance) => Promise<RestoreReport>;
  onRestore: (provenance: JobProvenance, restoreSource: boolean) => Promise<string>;
}) {
  const [report, setReport] = useState<RestoreReport>();
  const [busy, setBusy] = useState(false);

  const compare = async () => {
    setBusy(true);
    try {
      setReport(await onCompare(provenance));
    } finally {
      setBusy(false);
    }
  };

  const restorable = report?.drift.filter((entry) => entry.restorable) ?? [];

  return (
    <div className="restore-panel">
      <div className="restore-actions">
        <button type="button" disabled={busy} onClick={() => void compare()}>
          {busy ? <LoaderCircle size={13} className="spin" /> : <GitCompare size={13} />}
          Compare with now
        </button>
        {restorable.length > 0 && (
          <button
            type="button"
            className="restore-apply"
            disabled={busy}
            onClick={() => {
              void onRestore(provenance, restorable.some((entry) => entry.kind === "source"))
                .then(() => compare());
            }}
          >Restore {restorable.length} item{restorable.length === 1 ? "" : "s"}</button>
        )}
      </div>

      {report?.checked && report.drift.length === 0 && (
        <p className="restore-clean"><Check size={13} />Nothing has changed since this run.</p>
      )}

      {report && report.drift.length > 0 && (
        <table className="restore-drift">
          <thead><tr><th>What</th><th>Then</th><th>Now</th></tr></thead>
          <tbody>
            {report.drift.map((entry) => (
              <tr key={`${entry.kind}-${entry.name}`} className={entry.restorable ? "" : "unrestorable"}>
                <td>{entry.name}<small>{entry.kind}</small></td>
                <td><code>{entry.recorded}</code></td>
                <td><code>{entry.current}</code></td>
              </tr>
            ))}
          </tbody>
        </table>
      )}

      {report?.notes.map((note) => <p className="restore-note" key={note}>{note}</p>)}
    </div>
  );
}

/**
 * A methods paragraph and citation, generated from this run's provenance.
 *
 * Placed in the provenance tab because that is where the underlying record
 * already lives, and because the moment someone looks at provenance is the
 * moment they are thinking about writing the run up.
 */
function MethodsPanel({ job, onCopy }: { job: Job; onCopy: (text: string) => void }) {
  const [open, setOpen] = useState(false);
  const document = useMemo(() => methodsDocument(job), [job]);
  if (!document) return null;

  return (
    <div className="methods-panel">
      <div className="methods-actions">
        <button type="button" onClick={() => setOpen((value) => !value)}>
          <FileText size={13} />{open ? "Hide" : "Methods and citation"}
        </button>
        {open && (
          <>
            <button type="button" onClick={() => onCopy(methodsParagraph(job.provenance!))}>
              Copy paragraph
            </button>
            <button type="button" onClick={() => onCopy(bibtex())}>Copy BibTeX</button>
            <button type="button" onClick={() => onCopy(document)}>Copy all</button>
          </>
        )}
      </div>
      {open && (
        <>
          <p className="methods-note">
            Generated from what this run actually recorded. Check it against your manuscript before submitting.
          </p>
          <pre className="methods-body">{document}</pre>
        </>
      )}
    </div>
  );
}

export function OutputPane({
  runs,
  job,
  compareJob,
  fileName,
  elapsed,
  location,
  /** Learner mode: keep Export/Rerun, hide docking and power-user chrome. */
  simplified = false,
  exportFormat,
  exportOptions,
  onSelectJob,
  onCompareJob,
  onExportFormat,
  onExport,
  onExportBundle,
  onClear,
  onMove,
  onDockPointerDown,
  onClose,
  onPin,
  onRename,
  onDelete,
  onRerun,
  onDetach,
  onOpenDiagnostic,
  onReadArtifactPreview,
  onSaveArtifact,
  onReadResultPage,
  onCompareEnvironment,
  onRestoreEnvironment,
  onCopyText,
}: {
  runs: Job[];
  job: Job | undefined;
  compareJob?: Job;
  fileName?: string;
  elapsed: string;
  location: OutputLocation;
  simplified?: boolean;
  exportFormat: OutputExportFormat;
  exportOptions: OutputExportOption[];
  onSelectJob: (id: string) => void;
  onCompareJob: (id: string | undefined) => void;
  onExportFormat: (format: OutputExportFormat) => void;
  onExport: () => void;
  onExportBundle: () => void;
  onClear: () => void;
  onMove: (location: OutputLocation) => void;
  onDockPointerDown: (event: ReactPointerEvent<HTMLButtonElement>) => void;
  onClose: () => void;
  onPin: () => void;
  onRename: (name: string) => void;
  onDelete: () => void;
  onRerun: () => void;
  onDetach: () => void;
  onOpenDiagnostic: (path: string, line: number, column: number) => void;
  onReadArtifactPreview: (artifact: JobArtifact, length?: number) => Promise<Uint8Array>;
  onSaveArtifact: (artifact: JobArtifact) => void;
  onReadResultPage: (resultIndex: number, request: ResultPageRequest) => Promise<ResultPageData>;
  onCompareEnvironment: (provenance: JobProvenance) => Promise<RestoreReport>;
  onRestoreEnvironment: (provenance: JobProvenance, restoreSource: boolean) => Promise<string>;
  onCopyText: (text: string) => void;
}) {
  const tabs = useMemo(() => availableOutputTabs(job), [job]);
  const [tab, setTab] = useState<OutputTab>("summary");
  const [query, setQuery] = useState("");
  const [stream, setStream] = useState<"all" | "stdout" | "stderr" | "system">("all");
  const [wrap, setWrap] = useState(true);
  const [follow, setFollow] = useState(true);
  const [renaming, setRenaming] = useState(false);
  const [draftName, setDraftName] = useState("");
  const [tableIndex, setTableIndex] = useState(0);
  const [artifactPreview, setArtifactPreview] = useState<{ name: string; mediaType: string; text?: string; url?: string }>();
  const contentRef = useRef<HTMLDivElement>(null);
  const previousJobId = useRef<string>();
  const previousResultCount = useRef(0);
  const tables = useMemo(() => outputTables(job), [job]);
  const plots = useMemo(() => outputPlots(job), [job]);
  const errors = useMemo(() => outputErrors(job), [job]);
  const filteredChunks = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return (job?.log ?? []).filter((chunk) => {
      if (stream !== "all" && chunk.stream !== stream) return false;
      return !needle || chunk.text.toLowerCase().includes(needle);
    });
  }, [job?.log, query, stream]);

  useEffect(() => {
    if (!tabs.includes(tab)) setTab(tabs[0] ?? "summary");
  }, [tab, tabs]);
  // Land on the richer view only when the log has nothing of its own to show.
  // Printed values became typed results, so "always prefer plots or tables"
  // would now hide the prose a script deliberately printed alongside them.
  const landingTab = (candidate: Job | undefined): OutputTab => {
    if (hasReadableLog(candidate)) return "text";
    if (outputPlots(candidate).length) return "plots";
    if (outputTables(candidate).length) return "tables";
    return "text";
  };
  useEffect(() => {
    const resultCount = job?.results?.length ?? 0;
    if (job?.id !== previousJobId.current) {
      previousJobId.current = job?.id;
      previousResultCount.current = resultCount;
      setTab(landingTab(job));
      return;
    }
    if (resultCount > previousResultCount.current && !hasReadableLog(job)) {
      setTab(landingTab(job));
    }
    previousResultCount.current = resultCount;
  }, [job]);
  useEffect(() => {
    if (follow) contentRef.current?.scrollTo({ top: contentRef.current.scrollHeight });
  }, [filteredChunks, follow]);
  useEffect(() => () => {
    if (artifactPreview?.url) URL.revokeObjectURL(artifactPreview.url);
  }, [artifactPreview?.url]);

  const previewArtifact = async (artifact: JobArtifact) => {
    const format = artifactFormat(artifact);
    const mediaType = artifact.mediaType ?? "application/octet-stream";
    const visual = mediaType.startsWith("image/") || mediaType === "application/pdf";
    const bytes = await onReadArtifactPreview(
      artifact,
      visual ? Math.min(8 * 1024 * 1024, artifact.size ?? 8 * 1024 * 1024) : 1024 * 1024,
    );
    if (artifactPreview?.url) URL.revokeObjectURL(artifactPreview.url);
    if (format.text || mediaType.startsWith("text/") || mediaType === "application/json") {
      setArtifactPreview({ name: artifact.name, mediaType, text: new TextDecoder().decode(bytes) });
      return;
    }
    if (format.detail) {
      const signature = [...bytes.slice(0, 16)].map((value) => value.toString(16).padStart(2, "0")).join(" ");
      setArtifactPreview({
        name: artifact.name,
        mediaType,
        text: `${format.detail}\n\nFormat: ${format.label}\nSize: ${(artifact.size ?? bytes.length).toLocaleString()} bytes\nSHA-256: ${artifact.sha256 ?? "not available"}\nHeader bytes: ${signature || "empty file"}`,
      });
      return;
    }
    setArtifactPreview({
      name: artifact.name,
      mediaType,
      url: URL.createObjectURL(new Blob([bytes], { type: mediaType })),
    });
  };

  const beginRename = () => {
    setDraftName(job?.displayName ?? job?.file ?? "");
    setRenaming(true);
  };
  const commitRename = () => {
    onRename(draftName);
    setRenaming(false);
  };

  return (
    <section className={`output-pane output-pane-${location}${simplified ? " output-pane-simplified" : ""}`}>
      <div className="output-pane-toolbar">
        {!simplified && (
          <button
            type="button"
            className="output-drag-handle"
            aria-label="Drag Output to dock"
            title="Drag Output to bottom, right, or editor"
            onPointerDown={onDockPointerDown}
          >
            <GripVertical size={14} />
          </button>
        )}
        <select className="output-run-select" aria-label="Output run" value={job?.id ?? ""} onChange={(event) => onSelectJob(event.target.value)}>
          {!runs.length && <option value="">No runs</option>}
          {runs.map((run) => <option value={run.id} key={run.id}>
            {run.pinned ? "[pinned] " : ""}{run.displayName ?? run.file} | {new Date(run.startedAt).toLocaleTimeString()} | {run.status}
          </option>)}
        </select>
        <div className={`output-pane-title ${renaming ? "renaming" : ""}`}>
          {renaming ? (
            <input
              aria-label="Run name"
              value={draftName}
              autoFocus
              onChange={(event) => setDraftName(event.target.value)}
              onBlur={commitRename}
              onKeyDown={(event) => {
                if (event.key === "Enter") commitRename();
                if (event.key === "Escape") setRenaming(false);
              }}
            />
          ) : <strong>{job?.displayName ?? fileName ?? job?.file ?? "Output"}</strong>}
          <span>{job ? `${job.backend} | ${job.status}` : "No run selected"}</span>
        </div>
        <button
          type="button"
          className="output-text-action"
          disabled={!job || job.status === "running"}
          onClick={onRerun}
          title="Rerun"
        >
          <RefreshCw size={13} />Rerun
        </button>
        <div className="output-export-control">
          <select aria-label="Output export format" value={exportFormat} onChange={(event) => onExportFormat(event.target.value as OutputExportFormat)}>
            {exportOptions.map((option) => <option value={option.format} key={option.format}>{option.label}</option>)}
          </select>
          <button
            type="button"
            className="output-text-action"
            disabled={!job?.log.length}
            onClick={onExport}
            title={`Export output as ${exportFormat.toUpperCase()}`}
          >
            <Download size={13} />Export
          </button>
        </div>
        {!simplified && (
          <div className="output-location-tools" role="group" aria-label="Output location">
            <ToolButton label="Move Output to bottom" active={location === "bottom"} onClick={() => onMove("bottom")}><PanelBottom size={13} /></ToolButton>
            <ToolButton label="Move Output to right" active={location === "right"} onClick={() => onMove("right")}><PanelRight size={13} /></ToolButton>
            <ToolButton label="Open Output in editor" active={location === "editor"} onClick={() => onMove("editor")}><FileText size={13} /></ToolButton>
          </div>
        )}
        <details className="output-compact-menu">
          <summary aria-label="More Output actions" title="More Output actions"><MoreHorizontal size={14} /></summary>
          <div>
            {!simplified && (
              <>
                <button type="button" onClick={() => onMove("bottom")}><PanelBottom size={13} />Dock at bottom</button>
                <button type="button" onClick={() => onMove("right")}><PanelRight size={13} />Dock at right</button>
                <button type="button" onClick={() => onMove("editor")}><FileText size={13} />Open in editor</button>
                <button type="button" disabled={!job} onClick={onPin}>{job?.pinned ? <PinOff size={13} /> : <Pin size={13} />}{job?.pinned ? "Unpin run" : "Pin run"}</button>
                <button type="button" disabled={!job} onClick={beginRename}><Pencil size={13} />Rename run</button>
                <button type="button" disabled={runs.length < 2} onClick={() => onCompareJob(compareJob ? undefined : runs.find((run) => run.id !== job?.id)?.id)}><GitCompare size={13} />{compareJob ? "Stop comparing" : "Compare runs"}</button>
                <button type="button" onClick={onDetach}><ExternalLink size={13} />Detach window</button>
              </>
            )}
            <button type="button" disabled={!job} onClick={onExportBundle}><FileArchive size={13} />Export reproducibility bundle</button>
            <button type="button" disabled={!job?.log.length} onClick={onClear}><Eraser size={13} />Clear output</button>
            <button type="button" disabled={!job} onClick={onDelete}><Trash2 size={13} />Delete run</button>
          </div>
        </details>
        {location !== "bottom" && <ToolButton label="Close Output" onClick={onClose}><X size={13} /></ToolButton>}
      </div>

      {compareJob && job ? <CompareRuns left={job} right={compareJob} /> : (
        <div className="output-pane-body">
          <nav className="output-result-tabs" aria-label="Output views">
            {tabs.map((view) => <button type="button" className={tab === view ? "active" : ""} onClick={() => setTab(view)} key={view}>{view}</button>)}
          </nav>
          {job?.status === "running" && (
            <div className="active-job-progress" role="status" aria-live="polite">
              <LoaderCircle size={14} className="spin" />
              <span><strong>Running on {job.backend}</strong><small>Elapsed {elapsed}</small></span>
              <div className="progress-track" aria-hidden="true"><i /></div>
            </div>
          )}
          <div className={`output-result-content ${wrap ? "wrap" : "nowrap"}`} ref={contentRef}>
            {tab === "summary" && (
              <div className="output-summary">
                <div className="output-summary-metrics">
                  <span><strong>Status</strong>{job?.status ?? "-"}</span>
                  <span><strong>Duration</strong>{job?.durationMs == null ? elapsed : `${(job.durationMs / 1000).toFixed(2)}s`}</span>
                  <span><strong>Backend</strong>{job?.backend ?? "-"}</span>
                  <span><strong>Results</strong>{job?.results?.length ?? 0}</span>
                  <span><strong>Plots</strong>{plots.length}</span>
                  <span><strong>Tables</strong>{tables.length}</span>
                </div>
                {(job?.results ?? []).map((result, index) => (
                  <details open={index === 0} key={index}><summary>Result {index + 1}: {result.kind}</summary><pre>{JSON.stringify(result, null, 2)}</pre></details>
                ))}
                {!job?.results?.length && <p className="output-empty-reason">
                  No tables or plots in this run. Printing a table, matrix, record
                  list, or plot — <code>println(result)</code> — shows it here as a
                  sortable view alongside the text log.
                </p>}
              </div>
            )}
            {tab === "text" && (
              <div className="output-text-view">
                <div className="output-result-controls">
                  <label><Search size={12} /><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Search output" /></label>
                  <select aria-label="Output stream" value={stream} onChange={(event) => setStream(event.target.value as typeof stream)}>
                    <option value="all">All streams</option><option value="stdout">stdout</option><option value="stderr">errors</option><option value="system">system</option>
                  </select>
                  <ToolButton label="Toggle line wrapping" active={wrap} onClick={() => setWrap((value) => !value)}><WrapText size={12} /></ToolButton>
                  <ToolButton label="Follow output" active={follow} onClick={() => setFollow((value) => !value)}><ArrowDownToLine size={12} /></ToolButton>
                  <ToolButton label="Copy visible output" onClick={() => void bridge.copyText(jobLogText(filteredChunks))}><Copy size={12} /></ToolButton>
                </div>
                <JobLog
                  className="output-view"
                  chunks={filteredChunks}
                  emptyText={job?.status === "running"
                    ? "Waiting for output..."
                    : job
                      ? "This run produced no output."
                      : "No run selected. Run a BioLang file to see its output here."}
                />
              </div>
            )}
            {tab === "tables" && tables.length > 0 && (
              <div className="output-tables-view">
                <select aria-label="Result table" value={tableIndex} onChange={(event) => setTableIndex(Number(event.target.value))}>
                  {tables.map((table, index) => <option value={index} key={table.name}>{table.name}</option>)}
                </select>
                <TableResult
                  table={tables[Math.min(tableIndex, tables.length - 1)]}
                  onLoadPage={job?.remoteId || tables[Math.min(tableIndex, tables.length - 1)].paged
                    ? (request) => onReadResultPage(
                        tables[Math.min(tableIndex, tables.length - 1)].resultIndex,
                        request,
                      )
                    : undefined}
                />
              </div>
            )}
            {tab === "plots" && plots.length > 0 && <PlotGallery plots={plots} />}
            {tab === "files" && <div className="output-files-tab">
              <div className="output-files-view">{job?.artifacts?.map((artifact) => <div className="output-file-row" key={artifact.path ?? artifact.name}>
                <FileText size={14} /><span>{artifact.name}<small><b>{artifactFormat(artifact).label}</b> {artifact.path ?? artifact.mediaType}{artifact.size == null ? "" : ` | ${artifact.size.toLocaleString()} bytes`}</small></span>
                <ToolButton label={`Preview ${artifact.name}`} onClick={() => void previewArtifact(artifact)}><Eye size={13} /></ToolButton>
                <ToolButton label={`Save ${artifact.name}`} onClick={() => onSaveArtifact(artifact)}><Download size={13} /></ToolButton>
              </div>)}</div>
              {artifactPreview && <section className="output-artifact-preview">
                <header><strong>{artifactPreview.name}</strong><button type="button" aria-label="Close artifact preview" onClick={() => setArtifactPreview(undefined)}><X size={13} /></button></header>
                {artifactPreview.text != null && <pre>{artifactPreview.text}</pre>}
                {artifactPreview.url && artifactPreview.mediaType.startsWith("image/") && <img src={artifactPreview.url} alt={artifactPreview.name} />}
                {artifactPreview.url && artifactPreview.mediaType === "application/pdf" && <iframe src={artifactPreview.url} title={artifactPreview.name} />}
                {artifactPreview.url && !artifactPreview.mediaType.startsWith("image/") && artifactPreview.mediaType !== "application/pdf" && <p>Preview is unavailable for {artifactPreview.mediaType}.</p>}
              </section>}
            </div>}
            {tab === "errors" && <div className="output-errors-view">{errors.map((chunk, index) => {
              const match = chunk.text.match(/([A-Za-z]:[\\/][^:\n]+|[^:\n]+\.[A-Za-z0-9]+):(\d+):(\d+)/);
              return <button type="button" key={index} onClick={() => match && onOpenDiagnostic(match[1], Number(match[2]), Number(match[3]))}><pre>{chunk.text}</pre></button>;
            })}</div>}
            {tab === "provenance" && job?.provenance && <div className="output-provenance">
              <RestorePanel provenance={job.provenance} onCompare={onCompareEnvironment} onRestore={onRestoreEnvironment} />
              <MethodsPanel job={job} onCopy={onCopyText} />
              <dl>
                <dt>BioLang</dt><dd>{job.provenance.biolangVersion ?? "unknown"}</dd>
                <dt>Backend</dt><dd>{job.provenance.backend}</dd>
                <dt>Entrypoint</dt><dd>{job.provenance.entrypoint}</dd>
                <dt>Source SHA-256</dt><dd><code>{job.provenance.sourceHash ?? "unknown"}</code></dd>
                <dt>Workspace</dt><dd>{job.provenance.workspace ?? "unknown"}</dd>
                <dt>Started</dt><dd>{new Date(job.startedAt).toLocaleString()}</dd>
                <dt>Platform</dt><dd>{[job.provenance.platform, job.provenance.architecture].filter(Boolean).join(" / ") || "unknown"}</dd>
                <dt>Runtime</dt><dd>{[
                  job.provenance.runtime?.locale,
                  job.provenance.runtime?.timezone,
                  job.provenance.runtime?.logicalCpus && `${job.provenance.runtime.logicalCpus} logical CPUs`,
                ].filter(Boolean).join(" | ") || "unknown"}</dd>
                <dt>Random seed</dt><dd>{job.provenance.randomSeed ?? "not declared"}</dd>
              </dl>
              <h3>Packages</h3>
              <pre>{JSON.stringify(job.provenance.packages, null, 2)}</pre>
              <h3>Parameters</h3>
              <pre>{JSON.stringify(job.provenance.parameters, null, 2)}</pre>
              <h3>Input files</h3>
              <pre>{JSON.stringify(job.provenance.inputs ?? [], null, 2)}</pre>
              <h3>Tools</h3>
              <pre>{JSON.stringify(job.provenance.tools ?? [], null, 2)}</pre>
              <h3>Environment manifests</h3>
              <pre>{JSON.stringify(job.provenance.environmentFiles ?? [], null, 2)}</pre>
            </div>}
          </div>
        </div>
      )}
    </section>
  );
}
