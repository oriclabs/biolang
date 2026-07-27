import { ArrowUpDown, ChevronLeft, ChevronRight, Copy, Dna, Download, FileSearch, Grid3X3, Search, Table2 } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import type { DataPreview } from "../types";
import { previewExportFormats } from "../viewers";

function reverseComplement(sequence: string) {
  const complements: Record<string, string> = {
    A: "T",
    T: "A",
    C: "G",
    G: "C",
    U: "A",
    N: "N",
  };
  return sequence
    .toUpperCase()
    .split("")
    .reverse()
    .map((base) => complements[base] ?? base)
    .join("");
}

export function DataPreviewPane({
  name,
  path,
  preview,
  onExport,
}: {
  name: string;
  path: string;
  preview: DataPreview;
  onExport: (path: string, format: string) => void | Promise<void>;
}) {
  const [filter, setFilter] = useState("");
  const [sortColumn, setSortColumn] = useState<number>();
  const [descending, setDescending] = useState(false);
  const [sequenceMode, setSequenceMode] = useState<"forward" | "reverse">("forward");
  const [selectedRecord, setSelectedRecord] = useState(0);
  const [sequenceSearch, setSequenceSearch] = useState("");
  const [activeMatch, setActiveMatch] = useState(0);
  const [viewMode, setViewMode] = useState<"table" | "heatmap">("table");
  const [exportFormat, setExportFormat] = useState(previewExportFormats(preview)[0]);
  const sequenceRecord = preview.sequences?.[selectedRecord];
  const forwardSequence = sequenceRecord?.sequence ?? preview.sequence;
  const sequence = sequenceMode === "reverse" && forwardSequence
    ? reverseComplement(forwardSequence)
    : forwardSequence;
  const visibleSequence = sequence?.slice(0, 20_000);
  const sequenceMatches = useMemo(() => {
    const needle = sequenceSearch.trim().toUpperCase();
    if (!visibleSequence || !needle) return [];
    const matches: number[] = [];
    let offset = 0;
    while (matches.length < 1_000) {
      const index = visibleSequence.toUpperCase().indexOf(needle, offset);
      if (index < 0) break;
      matches.push(index);
      offset = index + 1;
    }
    return matches;
  }, [sequenceSearch, visibleSequence]);
  const exportFormats = previewExportFormats(preview);
  const rows = useMemo(() => {
    const needle = filter.trim().toLowerCase();
    const filtered = needle
      ? preview.rows.filter((row) => row.some((cell) => cell.toLowerCase().includes(needle)))
      : preview.rows;
    if (sortColumn == null) return filtered;
    return [...filtered].sort((left, right) => {
      const comparison = (left[sortColumn] ?? "").localeCompare(
        right[sortColumn] ?? "",
        undefined,
        { numeric: true },
      );
      return descending ? -comparison : comparison;
    });
  }, [descending, filter, preview.rows, sortColumn]);
  const numericColumns = useMemo(
    () => preview.columns.map((_, index) => {
      const values = rows.map((row) => Number(row[index])).filter(Number.isFinite);
      return {
        index,
        values,
        min: values.length ? Math.min(...values) : 0,
        max: values.length ? Math.max(...values) : 0,
      };
    }).filter((column) => column.values.length >= Math.max(1, Math.floor(rows.length * 0.8))),
    [preview.columns, rows],
  );
  const numericColumnIndexes = useMemo(
    () => new Map(numericColumns.map((column) => [column.index, column])),
    [numericColumns],
  );

  useEffect(() => {
    setSelectedRecord(0);
    setSequenceSearch("");
    setActiveMatch(0);
    setSequenceMode("forward");
  }, [path]);

  useEffect(() => {
    setActiveMatch(0);
  }, [sequenceSearch, selectedRecord, sequenceMode]);

  useEffect(() => {
    const position = sequenceMatches[activeMatch];
    if (position == null) return;
    document.getElementById(`sequence-${path}-${Math.floor(position / 10)}`)
      ?.scrollIntoView({ block: "nearest", inline: "nearest" });
  }, [activeMatch, path, sequenceMatches]);

  const sort = (index: number) => {
    if (sortColumn === index) setDescending((value) => !value);
    else {
      setSortColumn(index);
      setDescending(false);
    }
  };

  return (
    <div className="data-preview">
      <header className="preview-header">
        <span className="preview-kind"><Dna size={15} />{preview.kind.toUpperCase()}</span>
        <strong>{name}</strong>
        <span>{preview.summary.join(" | ")}</span>
        <span>{(preview.totalBytes / 1024).toFixed(1)} KB</span>
        {preview.truncated && <b>Sampled preview</b>}
      </header>
      <div className="preview-tools">
        {!!preview.columns.length && <label><Search size={13} /><input value={filter} onChange={(event) => setFilter(event.target.value)} placeholder="Filter preview" /></label>}
        {!!preview.columns.length && <span>{rows.length} rows shown</span>}
        {!!numericColumns.length && <div className="segmented-control preview-view-mode" aria-label="Preview view">
          <button type="button" title="Table" aria-label="Table" className={viewMode === "table" ? "active" : ""} onClick={() => setViewMode("table")}><Table2 size={13} /></button>
          <button type="button" title="Heatmap" aria-label="Heatmap" className={viewMode === "heatmap" ? "active" : ""} onClick={() => setViewMode("heatmap")}><Grid3X3 size={13} /></button>
        </div>}
        {preview.sequence && <>
          {!!preview.sequences?.length && <div className="sequence-record-controls">
            <button type="button" title="Previous record" aria-label="Previous sequence record" disabled={selectedRecord === 0} onClick={() => setSelectedRecord((value) => Math.max(0, value - 1))}><ChevronLeft size={13} /></button>
            <select aria-label="Sequence record" value={selectedRecord} onChange={(event) => setSelectedRecord(Number(event.target.value))}>
              {preview.sequences.map((record, index) => <option value={index} key={`${record.name}-${index}`}>{index + 1}. {record.name}</option>)}
            </select>
            <button type="button" title="Next record" aria-label="Next sequence record" disabled={selectedRecord >= preview.sequences.length - 1} onClick={() => setSelectedRecord((value) => Math.min(preview.sequences!.length - 1, value + 1))}><ChevronRight size={13} /></button>
          </div>}
          <div className="segmented-control preview-orientation" aria-label="Sequence orientation">
            <button type="button" className={sequenceMode === "forward" ? "active" : ""} onClick={() => setSequenceMode("forward")}>Forward</button>
            <button type="button" className={sequenceMode === "reverse" ? "active" : ""} onClick={() => setSequenceMode("reverse")}>Reverse complement</button>
          </div>
          <button type="button" className="preview-icon-button" title="Copy sequence" aria-label="Copy sequence" onClick={() => void navigator.clipboard.writeText(sequence ?? "")}><Copy size={13} /></button>
        </>}
        <div className="preview-export">
          <select aria-label="Export format" value={exportFormat} onChange={(event) => setExportFormat(event.target.value)}>
            {exportFormats.map((format) => <option key={format} value={format}>{format.toUpperCase()}</option>)}
          </select>
          <button type="button" className="preview-icon-button" title={`Export ${exportFormat.toUpperCase()}`} aria-label={`Export ${exportFormat.toUpperCase()}`} onClick={() => void onExport(path, exportFormat)}><Download size={13} /></button>
        </div>
      </div>
      {sequence && <section className="sequence-inspector">
        <div className="sequence-search">
          <label><Search size={12} /><input aria-label="Search sequence" placeholder="Find motif in sequence" value={sequenceSearch} onChange={(event) => setSequenceSearch(event.target.value)} /></label>
          <span>{sequenceMatches.length ? `${activeMatch + 1} of ${sequenceMatches.length}` : sequenceSearch ? "No matches" : `${sequence.length} bases`}</span>
          <button type="button" disabled={!sequenceMatches.length} onClick={() => setActiveMatch((value) => (value - 1 + sequenceMatches.length) % sequenceMatches.length)}><ChevronLeft size={12} /></button>
          <button type="button" disabled={!sequenceMatches.length} onClick={() => setActiveMatch((value) => (value + 1) % sequenceMatches.length)}><ChevronRight size={12} /></button>
        </div>
        <div className="sequence-preview" aria-label="Sequence preview">
          {visibleSequence?.match(/.{1,10}/g)?.map((part, index) => {
            const start = index * 10;
            const end = start + part.length;
            const matched = sequenceMatches.some((match) =>
              match < end && match + sequenceSearch.trim().length > start);
            const active = sequenceMatches[activeMatch] != null
              && sequenceMatches[activeMatch] < end
              && sequenceMatches[activeMatch] + sequenceSearch.trim().length > start;
            return <span id={`sequence-${path}-${index}`} className={`${matched ? "match" : ""} ${active ? "active-match" : ""}`} key={index}><small>{start + 1}</small>{part}</span>;
          })}
        </div>
        {sequence.length > 20_000 && <small className="sequence-limit">Showing the first 20,000 bases of this record</small>}
      </section>}
      {(preview.kind === "image" || preview.kind === "svg") && preview.content && <div className="media-preview"><img src={preview.content} alt={name} /></div>}
      {preview.kind === "pdf" && preview.content && <div className="media-preview"><iframe src={preview.content} title={name} /></div>}
      {preview.kind === "newick" && <div className="newick-preview">
        <FileSearch size={20} />
        <strong>Newick tree</strong>
        <pre>{preview.content}</pre>
      </div>}
      {preview.kind === "structure" && !!preview.rows.length && <StructurePreview preview={preview} />}
      {!!preview.columns.length && <div className="preview-table-wrap">
        <table className="preview-table">
          <thead><tr>{preview.columns.map((column, index) => <th key={`${column}-${index}`}><button type="button" onClick={() => sort(index)}>{column}<ArrowUpDown size={11} /></button></th>)}</tr></thead>
          <tbody>{rows.map((row, rowIndex) => <tr key={rowIndex}>{preview.columns.map((_, columnIndex) => {
            const value = row[columnIndex] ?? "";
            const numeric = numericColumnIndexes.get(columnIndex);
            const range = numeric ? numeric.max - numeric.min : 0;
            const intensity = numeric && range ? (Number(value) - numeric.min) / range : 0;
            return <td
              key={columnIndex}
              title={value}
              className={viewMode === "heatmap" && numeric ? "heatmap-cell" : ""}
              style={viewMode === "heatmap" && numeric ? { "--heat": String(intensity) } as React.CSSProperties : undefined}
            >{value}</td>;
          })}</tr>)}</tbody>
        </table>
      </div>}
      {preview.provenance && <footer className="preview-provenance">
        <span><strong>Format</strong>{preview.provenance.format || preview.kind}</span>
        <span><strong>Path</strong>{preview.provenance.path}</span>
        {preview.provenance.sha256 && <span title={preview.provenance.sha256}><strong>SHA-256</strong>{preview.provenance.sha256.slice(0, 16)}...</span>}
        {preview.provenance.importedFrom && <span title={preview.provenance.importedFrom}><strong>Imported from</strong>{preview.provenance.importedFrom}</span>}
      </footer>}
    </div>
  );
}

function StructurePreview({ preview }: { preview: DataPreview }) {
  const points = useMemo(() => {
    const xIndex = preview.columns.indexOf("X");
    const yIndex = preview.columns.indexOf("Y");
    if (xIndex < 0 || yIndex < 0) return [];
    const raw = preview.rows.map((row) => ({
      x: Number(row[xIndex]),
      y: Number(row[yIndex]),
      element: row[preview.columns.indexOf("Element")] || row[preview.columns.indexOf("Atom")] || "C",
    })).filter((point) => Number.isFinite(point.x) && Number.isFinite(point.y));
    if (!raw.length) return [];
    const minX = Math.min(...raw.map((point) => point.x));
    const maxX = Math.max(...raw.map((point) => point.x));
    const minY = Math.min(...raw.map((point) => point.y));
    const maxY = Math.max(...raw.map((point) => point.y));
    return raw.map((point) => ({
      ...point,
      left: 4 + ((point.x - minX) / Math.max(1, maxX - minX)) * 92,
      top: 4 + ((point.y - minY) / Math.max(1, maxY - minY)) * 92,
    }));
  }, [preview]);
  if (!points.length) return null;
  return <div className="structure-preview" aria-label="Structure projection">
    {points.slice(0, 500).map((point, index) => <i
      key={index}
      title={point.element}
      data-element={point.element.toUpperCase().slice(0, 1)}
      style={{ left: `${point.left}%`, top: `${point.top}%` }}
    />)}
  </div>;
}
