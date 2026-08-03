import { Download, FileArchive, RefreshCw } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { bridge, onRunHistoryChanged } from "../bridge";
import { normalizeJobLog } from "../jobLogs";
import { outputPlots, outputTables, valuePreview } from "../outputModel";
import { buildOutputExport } from "../outputExport";
import { buildRunBundle } from "../runBundle";
import type { Job } from "../types";
import { JobLog } from "./JobLog";

type DetachedTab = "text" | "tables" | "plots" | "provenance";

async function loadJobs(): Promise<Job[]> {
  return (await bridge.loadRunHistory())
    .map((job) => ({ ...job, log: normalizeJobLog(job.log) }));
}

export function DetachedOutput() {
  const requestedId = new URLSearchParams(window.location.search).get("jobId") ?? "";
  const [jobs, setJobs] = useState<Job[]>([]);
  const [selectedId, setSelectedId] = useState(requestedId);
  const [tab, setTab] = useState<DetachedTab>("text");
  const job = jobs.find((candidate) => candidate.id === selectedId) ?? jobs[0];
  const tables = useMemo(() => outputTables(job), [job]);
  const plots = useMemo(() => outputPlots(job), [job]);

  useEffect(() => {
    let disposed = false;
    const refresh = () => {
      void loadJobs().then((next) => {
        if (!disposed) setJobs(next);
      });
    };
    refresh();
    let unlisten: () => void = () => undefined;
    void onRunHistoryChanged(refresh).then((dispose) => {
      if (disposed) dispose();
      else unlisten = dispose;
    });
    return () => {
      disposed = true;
      unlisten();
    };
  }, []);

  const exportText = async () => {
    if (!job) return;
    await bridge.exportText(`${job.displayName ?? "biolang"}-output.log`, buildOutputExport(job.log, "log", job));
  };

  const exportBundle = async () => {
    if (!job) return;
    const bundle = buildRunBundle(job);
    await bridge.exportBinary(bundle.name, bundle.bytes);
  };

  return (
    <main className="detached-output-shell">
      <header>
        <div>
          <strong>BioLang Output</strong>
          <span>{job ? `${job.backend} | ${job.status}` : "No recorded runs"}</span>
        </div>
        <select aria-label="Detached output run" value={job?.id ?? ""} onChange={(event) => setSelectedId(event.target.value)}>
          {jobs.map((run) => <option value={run.id} key={run.id}>{run.displayName ?? run.file} | {new Date(run.startedAt).toLocaleString()}</option>)}
        </select>
        <button type="button" title="Refresh run history" aria-label="Refresh run history" onClick={() => void loadJobs().then(setJobs)}><RefreshCw size={14} /></button>
        <button type="button" title="Export output" aria-label="Export output" disabled={!job} onClick={() => void exportText()}><Download size={14} /></button>
        <button type="button" title="Export run bundle" aria-label="Export run bundle" disabled={!job} onClick={() => void exportBundle()}><FileArchive size={14} /></button>
      </header>
      <nav aria-label="Detached output views">
        {(["text", "tables", "plots", "provenance"] as DetachedTab[]).map((view) => (
          <button type="button" className={tab === view ? "active" : ""} onClick={() => setTab(view)} key={view}>{view}</button>
        ))}
      </nav>
      <section>
        {tab === "text" && <JobLog className="output-view" chunks={job?.log ?? []} emptyText={job?.status === "running" ? "Waiting for output..." : "No output yet."} />}
        {tab === "tables" && (
          <div className="detached-tables">
            {tables.map((table) => <article key={table.name}>
              <header><strong>{table.name}</strong><span>{table.totalRows.toLocaleString()} rows</span></header>
              <div><table><thead><tr>{table.columns.map((column) => <th key={column}>{column}</th>)}</tr></thead>
                <tbody>{table.rows.slice(0, 200).map((row, index) => <tr key={index}>{row.map((value, cell) => <td key={cell}>{valuePreview(value)}</td>)}</tr>)}</tbody>
              </table></div>
            </article>)}
            {!tables.length && <p>No structured tables in this run.</p>}
          </div>
        )}
        {tab === "plots" && (
          <div className="detached-plots">
            {plots.map((plot, index) => <figure key={`${plot.name}-${index}`}><img alt={plot.name} src={`data:image/svg+xml;charset=utf-8,${encodeURIComponent(plot.svg)}`} /><figcaption>{plot.name}</figcaption></figure>)}
            {!plots.length && <p>No plots in this run.</p>}
          </div>
        )}
        {tab === "provenance" && <pre className="detached-provenance">{JSON.stringify(job?.provenance ?? {}, null, 2)}</pre>}
      </section>
    </main>
  );
}
