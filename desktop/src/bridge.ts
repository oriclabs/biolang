import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  browserConsoleResponse,
  evaluateBrowserSource,
  importBrowserSource,
  resetBrowserConsole,
  setBrowserRuntimeFileReader,
  validateBrowserImport,
} from "./browserRuntime";
import { loadBrowserWorkspace, saveBrowserWorkspace } from "./browserWorkspace";
import { demoFiles, demoPackages, demoWorkspace } from "./demo";
import type {
  DataPreview,
  CodeImportResult,
  ConsoleResponse,
  ImportValidationReport,
  EnvironmentInfo,
  GitStatusSnapshot,
  JobFinishedEvent,
  JobOutputEvent,
  PackageInfo,
  SearchHit,
  TerminalOutputEvent,
  WorkspaceSnapshot,
} from "./types";

export const isDesktop = "__TAURI_INTERNALS__" in window;

const demoEnvironment: EnvironmentInfo = {
  platform: "web",
  architecture: "WASM",
  workspace: demoWorkspace.root,
  blPath: "browser://wasm",
  blVersion: "bl WASM",
  lspAvailable: false,
};

let demoJobId = 40;
let demoSelected = false;
let browserWorkspaceReady: Promise<void> | undefined;
const cancelledBrowserJobs = new Set<number>();

setBrowserRuntimeFileReader((path) => demoFiles[path]);

function restoreBrowserWorkspace(): Promise<void> {
  if (isDesktop) return Promise.resolve();
  if (!browserWorkspaceReady) {
    browserWorkspaceReady = loadBrowserWorkspace()
      .then((stored) => {
        if (!stored) return;
        demoSelected = stored.selected;
        Object.assign(demoWorkspace, structuredClone(stored.workspace));
        for (const path of Object.keys(demoFiles)) delete demoFiles[path];
        Object.assign(demoFiles, structuredClone(stored.files));
      })
      .catch((error) => {
        console.warn("BioLang browser workspace could not be restored", error);
      });
  }
  return browserWorkspaceReady;
}

async function persistBrowserWorkspace(): Promise<void> {
  if (isDesktop) return;
  await saveBrowserWorkspace({
    selected: demoSelected,
    workspace: structuredClone(demoWorkspace),
    files: structuredClone(demoFiles),
  });
}

function browserImportFormat(filename: string) {
  const lower = filename.toLowerCase();
  if (lower.endsWith(".ipynb")) return "ipynb";
  if (lower.endsWith(".rmd")) return "rmd";
  if (lower.endsWith(".r")) return "r";
  if (lower.endsWith(".py")) return "python";
  throw new Error("Choose a Python, R, Jupyter, or R Markdown file");
}

function chooseBrowserFiles(accept: string, multiple: boolean): Promise<File[]> {
  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = accept;
    input.multiple = multiple;
    input.style.display = "none";
    const finish = () => {
      resolve(Array.from(input.files ?? []));
      input.remove();
    };
    input.addEventListener("change", finish, { once: true });
    input.addEventListener("cancel", finish, { once: true });
    document.body.appendChild(input);
    input.click();
  });
}

function dispatchBrowserJobOutput(jobId: number, stream: "stdout" | "stderr", data: string) {
  if (!data) return;
  window.dispatchEvent(new CustomEvent("demo-job-output", {
    detail: { jobId, stream, data },
  }));
}

function finishBrowserJob(jobId: number, exitCode: number | null, startedAt: number) {
  window.dispatchEvent(new CustomEvent("demo-job-finished", {
    detail: {
      jobId,
      exitCode,
      durationMs: Math.max(0, Math.round(performance.now() - startedAt)),
    },
  }));
}

function runBrowserSource(source: string): number {
  const jobId = ++demoJobId;
  const startedAt = performance.now();
  window.setTimeout(() => {
    void evaluateBrowserSource(source, true)
      .then((result) => {
        if (cancelledBrowserJobs.delete(jobId)) {
          finishBrowserJob(jobId, null, startedAt);
          return;
        }
        dispatchBrowserJobOutput(jobId, "stdout", result.output ?? "");
        if (result.ok) {
          if (
            result.value
            && !["null", "nil", "Nil", "()", "None"].includes(result.value)
          ) {
            dispatchBrowserJobOutput(
              jobId,
              "stdout",
              `\u2192 ${result.value}${result.type ? ` : ${result.type}` : ""}\n`,
            );
          }
          finishBrowserJob(jobId, 0, startedAt);
        } else {
          dispatchBrowserJobOutput(jobId, "stderr", `${result.error ?? "BioLang evaluation failed"}\n`);
          finishBrowserJob(jobId, 1, startedAt);
        }
      })
      .catch((error) => {
        dispatchBrowserJobOutput(jobId, "stderr", `${String(error)}\n`);
        finishBrowserJob(jobId, 1, startedAt);
      });
  }, 0);
  return jobId;
}

function findDemoDirectory(path: string) {
  if (!path) return demoWorkspace.entries;
  let entries = demoWorkspace.entries;
  for (const part of path.split("/").filter(Boolean)) {
    const directory = entries.find((entry) => entry.kind === "directory" && entry.name === part);
    if (!directory) throw new Error(`Directory not found: ${path}`);
    entries = directory.children;
  }
  return entries;
}

function rebaseDemoEntry(entry: WorkspaceSnapshot["entries"][number], nextPath: string) {
  const previousPath = entry.path;
  entry.path = nextPath;
  if (entry.kind === "file") {
    demoFiles[nextPath] = demoFiles[previousPath] ?? "";
    delete demoFiles[previousPath];
    return;
  }
  for (const child of entry.children) {
    rebaseDemoEntry(child, `${nextPath}/${child.name}`);
  }
}

export const bridge = {
  async workspace(): Promise<WorkspaceSnapshot | null> {
    if (isDesktop) return invoke("workspace_snapshot");
    await restoreBrowserWorkspace();
    return demoSelected ? structuredClone(demoWorkspace) : null;
  },

  async gitStatus(): Promise<GitStatusSnapshot> {
    if (isDesktop) return invoke("git_status");
    return { available: false, files: [] };
  },

  async selectWorkspace(): Promise<WorkspaceSnapshot | null> {
    if (isDesktop) return invoke("select_workspace");
    await restoreBrowserWorkspace();
    demoSelected = true;
    await persistBrowserWorkspace();
    return structuredClone(demoWorkspace);
  },

  async openWorkspace(path: string): Promise<WorkspaceSnapshot> {
    if (isDesktop) return invoke("open_workspace", { path });
    await restoreBrowserWorkspace();
    if (path !== demoWorkspace.root) throw new Error("The recent workspace is no longer available");
    demoSelected = true;
    await persistBrowserWorkspace();
    return structuredClone(demoWorkspace);
  },

  async closeWorkspace(): Promise<void> {
    if (isDesktop) return invoke("close_workspace");
    await restoreBrowserWorkspace();
    demoSelected = false;
    await persistBrowserWorkspace();
  },

  async setWorkspaceTrust(root: string, trusted: boolean): Promise<void> {
    if (isDesktop) return invoke("set_workspace_trust", { root, trusted });
  },

  async getSomerSecret(profileId: string): Promise<string | null> {
    if (isDesktop) return invoke("get_somer_secret", { profileId });
    return window.sessionStorage.getItem(`biolang.desktop.somer.${profileId}`);
  },

  async setSomerSecret(profileId: string, secret: string): Promise<void> {
    if (isDesktop) return invoke("set_somer_secret", { profileId, secret });
    window.sessionStorage.setItem(`biolang.desktop.somer.${profileId}`, secret);
  },

  async deleteSomerSecret(profileId: string): Promise<void> {
    if (isDesktop) return invoke("delete_somer_secret", { profileId });
    window.sessionStorage.removeItem(`biolang.desktop.somer.${profileId}`);
  },

  async startSomerTunnel(profile: {
    id: string;
    sshHost: string;
    sshUser: string;
    sshPort: number;
    remoteHost: string;
    remotePort: number;
    identityFile?: string;
  }): Promise<string> {
    if (isDesktop) {
      return invoke("start_somer_tunnel", {
        profileId: profile.id,
        sshHost: profile.sshHost,
        sshUser: profile.sshUser,
        sshPort: profile.sshPort,
        remoteHost: profile.remoteHost,
        remotePort: profile.remotePort,
        identityFile: profile.identityFile,
      });
    }
    return `http://${profile.remoteHost}:${profile.remotePort}`;
  },

  async stopSomerTunnel(profileId: string): Promise<void> {
    if (isDesktop) return invoke("stop_somer_tunnel", { profileId });
  },

  async createEntry(path: string, kind: "file" | "directory"): Promise<void> {
    if (isDesktop) return invoke("create_entry", { path, kind });
    await restoreBrowserWorkspace();
    const normalized = path.replaceAll("\\", "/");
    const separator = normalized.lastIndexOf("/");
    const parent = separator >= 0 ? normalized.slice(0, separator) : "";
    const name = separator >= 0 ? normalized.slice(separator + 1) : normalized;
    const entries = findDemoDirectory(parent);
    if (entries.some((entry) => entry.name === name)) throw new Error(`${path} already exists`);
    entries.push({ name, path: normalized, kind, size: 0, children: [] });
    if (kind === "file") {
      demoFiles[normalized] = normalized.endsWith(".blflow")
        ? '{\n  "schemaVersion": 1,\n  "name": "BioLang workflow",\n  "nodes": [],\n  "edges": []\n}\n'
        : normalized.endsWith(".bl.md") || normalized.endsWith(".bln")
          ? '# BioLang notebook\n\n```biolang\nprintln("Hello from BioLang")\n```\n'
          : normalized.endsWith(".bl") ? "# New BioLang file\n" : "";
    }
    await persistBrowserWorkspace();
  },

  async renameEntry(path: string, newName: string): Promise<string> {
    if (isDesktop) return invoke("rename_entry", { path, newName });
    await restoreBrowserWorkspace();
    const normalized = path.replaceAll("\\", "/");
    const separator = normalized.lastIndexOf("/");
    const parent = separator >= 0 ? normalized.slice(0, separator) : "";
    const entries = findDemoDirectory(parent);
    const entry = entries.find((candidate) => candidate.path === normalized);
    if (!entry) throw new Error(`${path} was not found`);
    const nextPath = parent ? `${parent}/${newName}` : newName;
    entry.name = newName;
    rebaseDemoEntry(entry, nextPath);
    await persistBrowserWorkspace();
    return nextPath;
  },

  async deleteEntry(path: string): Promise<void> {
    if (isDesktop) return invoke("delete_entry", { path });
    await restoreBrowserWorkspace();
    const normalized = path.replaceAll("\\", "/");
    const separator = normalized.lastIndexOf("/");
    const parent = separator >= 0 ? normalized.slice(0, separator) : "";
    const entries = findDemoDirectory(parent);
    const index = entries.findIndex((candidate) => candidate.path === normalized);
    if (index >= 0) entries.splice(index, 1);
    delete demoFiles[normalized];
    await persistBrowserWorkspace();
  },

  async duplicateEntry(path: string): Promise<string> {
    if (isDesktop) return invoke("duplicate_entry", { path });
    await restoreBrowserWorkspace();
    const normalized = path.replaceAll("\\", "/");
    const separator = normalized.lastIndexOf("/");
    const parent = separator >= 0 ? normalized.slice(0, separator) : "";
    const entries = findDemoDirectory(parent);
    const entry = entries.find((candidate) => candidate.path === normalized);
    if (!entry) throw new Error(`${path} was not found`);
    const dot = entry.name.lastIndexOf(".");
    const stem = dot > 0 ? entry.name.slice(0, dot) : entry.name;
    const extension = dot > 0 ? entry.name.slice(dot) : "";
    let number = 1;
    let name = `${stem} copy${extension}`;
    while (entries.some((candidate) => candidate.name === name)) {
      number += 1;
      name = `${stem} copy ${number}${extension}`;
    }
    const nextPath = parent ? `${parent}/${name}` : name;
    entries.push({ ...structuredClone(entry), name, path: nextPath });
    if (entry.kind === "file") demoFiles[nextPath] = demoFiles[normalized] ?? "";
    await persistBrowserWorkspace();
    return nextPath;
  },

  async revealEntry(path: string): Promise<void> {
    if (isDesktop) return invoke("reveal_entry", { path });
  },

  async openExternal(url: string): Promise<void> {
    if (isDesktop) return invoke("open_external", { url });
    window.open(url, "_blank", "noopener,noreferrer");
  },

  async searchWorkspace(query: string): Promise<SearchHit[]> {
    if (isDesktop) return invoke("search_workspace", { query });
    await restoreBrowserWorkspace();
    const needle = query.trim().toLowerCase();
    if (needle.length < 2) return [];
    const hits: SearchHit[] = [];
    for (const [path, content] of Object.entries(demoFiles)) {
      for (const [index, line] of content.split(/\r?\n/).entries()) {
        const column = line.toLowerCase().indexOf(needle);
        if (column >= 0) hits.push({ path, line: index + 1, column: column + 1, preview: line.trim() });
      }
    }
    return hits.slice(0, 200);
  },

  async previewFile(path: string): Promise<DataPreview> {
    if (isDesktop) return invoke("preview_file", { path });
    await restoreBrowserWorkspace();
    const content = demoFiles[path] ?? "";
    const extension = path.split(".").pop()?.toLowerCase();
    const provenance = {
      path,
      format: extension ?? "text",
      size: content.length,
    };
    if (extension === "fasta" || extension === "fa") {
      const rows: string[][] = [];
      const sequences: Array<{ name: string; sequence: string }> = [];
      let name = "";
      let sequence = "";
      let firstSequence = "";
      for (const line of content.split(/\r?\n/)) {
        if (line.startsWith(">")) {
          if (name) {
            rows.push([name, String(sequence.length)]);
            sequences.push({ name, sequence });
            if (!firstSequence) firstSequence = sequence;
          }
          name = line.slice(1);
          sequence = "";
        } else sequence += line.trim();
      }
      if (name) {
        rows.push([name, String(sequence.length)]);
        sequences.push({ name, sequence });
        if (!firstSequence) firstSequence = sequence;
      }
      return {
        kind: "fasta",
        columns: ["Record", "Length"],
        rows,
        sequence: firstSequence,
        sequences,
        summary: [`${rows.length} records sampled`],
        truncated: false,
        totalBytes: content.length,
        provenance,
      };
    }
    if (extension === "gff" || extension === "gff3" || extension === "gtf") {
      const rows = content.split(/\r?\n/).filter((line) => line && !line.startsWith("#")).map((line) => line.split("\t"));
      return {
        kind: "gff",
        columns: ["Sequence", "Source", "Feature", "Start", "End", "Score", "Strand", "Phase", "Attributes"],
        rows,
        summary: [`${rows.length} features sampled`],
        truncated: false,
        totalBytes: content.length,
        provenance,
      };
    }
    if (["nwk", "newick", "tree"].includes(extension ?? "")) {
      return {
        kind: "newick",
        columns: [],
        rows: [],
        content,
        summary: ["7 labeled nodes"],
        truncated: false,
        totalBytes: content.length,
        provenance,
      };
    }
    if (extension === "pdb" || extension === "ent") {
      const field = (line: string, start: number, end: number) => line.slice(start, end).trim();
      const rows = content.split(/\r?\n/)
        .filter((line) => line.startsWith("ATOM  ") || line.startsWith("HETATM"))
        .map((line) => [
          field(line, 6, 11), field(line, 12, 16), field(line, 17, 20), field(line, 21, 22),
          field(line, 30, 38), field(line, 38, 46), field(line, 46, 54), field(line, 76, 78),
        ]);
      return {
        kind: "structure",
        columns: ["Serial", "Atom", "Residue", "Chain", "X", "Y", "Z", "Element"],
        rows,
        content,
        summary: [`${rows.length} atoms sampled`],
        truncated: false,
        totalBytes: content.length,
        provenance,
      };
    }
    const delimiter = extension === "csv" ? "," : "\t";
    const lines = content.split(/\r?\n/).filter(Boolean);
    return {
      kind: extension === "json" ? "json" : "table",
      columns: lines[0]?.split(delimiter) ?? [],
      rows: lines.slice(1, 501).map((line) => line.split(delimiter)),
      summary: [`${Math.max(0, lines.length - 1)} rows sampled`],
      truncated: false,
      totalBytes: content.length,
      provenance,
    };
  },

  async importFiles(): Promise<string[]> {
    if (isDesktop) return invoke("import_files");
    await restoreBrowserWorkspace();
    const files = await chooseBrowserFiles(
      ".fa,.fasta,.fq,.fastq,.csv,.tsv,.vcf,.bed,.gff,.gff3,.gtf,.json,.nwk,.newick,.pdb,.txt",
      true,
    );
    const entries = findDemoDirectory("data");
    const imported: string[] = [];
    for (const file of files) {
      let name = file.name.replace(/[\\/]/g, "_");
      const dot = name.lastIndexOf(".");
      const stem = dot > 0 ? name.slice(0, dot) : name;
      const extension = dot > 0 ? name.slice(dot) : "";
      let number = 1;
      while (entries.some((entry) => entry.name === name)) {
        number += 1;
        name = `${stem}-${number}${extension}`;
      }
      const path = `data/${name}`;
      const content = await file.text();
      demoFiles[path] = content;
      entries.push({ name, path, kind: "file", size: file.size, children: [] });
      imported.push(path);
    }
    if (imported.length) await persistBrowserWorkspace();
    return imported;
  },

  async importCode(): Promise<CodeImportResult | null> {
    if (isDesktop) return invoke("import_code");
    const [file] = await chooseBrowserFiles(".py,.r,.R,.ipynb,.rmd,.Rmd", false);
    if (!file) return null;
    const source = await file.text();
    return importBrowserSource(source, browserImportFormat(file.name), file.name);
  },

  async importCodeUrl(url: string): Promise<CodeImportResult> {
    if (isDesktop) return invoke("import_code_url", { url });
    const sourceName = new URL(url).pathname.split("/").filter(Boolean).at(-1) ?? "analysis.py";
    const response = await fetch(url);
    if (!response.ok) throw new Error(`Import failed: HTTP ${response.status}`);
    const source = await response.text();
    return importBrowserSource(source, browserImportFormat(sourceName), sourceName);
  },

  async validateImportCode(content: string, notebook: boolean): Promise<ImportValidationReport> {
    if (isDesktop) return invoke("validate_import_code", { content, notebook });
    return validateBrowserImport(content, notebook);
  },

  async exportPreview(path: string, format: string): Promise<string | null> {
    if (isDesktop) return invoke("export_preview", { path, format });
    const preview = await bridge.previewFile(path);
    const delimiter = format === "csv" ? "," : "\t";
    const content = format === "json"
      ? JSON.stringify(preview, null, 2)
      : format === "fasta" && preview.sequence
        ? `>${path.split("/").pop()}\n${preview.sequence}\n`
        : format === "newick"
          ? preview.content ?? ""
          : [preview.columns, ...preview.rows].map((row) => row.join(delimiter)).join("\n");
    const blob = new Blob([content], { type: "text/plain" });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = `${path.split("/").pop()}.preview.${format}`;
    anchor.click();
    URL.revokeObjectURL(url);
    return anchor.download;
  },

  async exportText(suggestedName: string, content: string): Promise<string | null> {
    if (isDesktop) return invoke("export_text", { suggestedName, content });
    const blob = new Blob([content], { type: "text/plain" });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = suggestedName;
    anchor.click();
    URL.revokeObjectURL(url);
    return anchor.download;
  },

  async environment(): Promise<EnvironmentInfo> {
    if (isDesktop) return invoke("get_environment");
    await restoreBrowserWorkspace();
    return { ...demoEnvironment, workspace: demoWorkspace.root };
  },

  async readFile(path: string): Promise<string> {
    if (isDesktop) return invoke("read_file", { path });
    await restoreBrowserWorkspace();
    const value = demoFiles[path];
    if (value === undefined) throw new Error(`Cannot read ${path}`);
    return value;
  },

  async writeFile(path: string, content: string): Promise<void> {
    if (isDesktop) return invoke("write_file", { path, content });
    await restoreBrowserWorkspace();
    demoFiles[path] = content;
    await persistBrowserWorkspace();
  },

  async saveFileAs(path: string, content: string): Promise<string | null> {
    if (isDesktop) return invoke("save_file_as", { path, content });
    const nextPath = await bridge.duplicateEntry(path);
    demoFiles[nextPath] = content;
    await persistBrowserWorkspace();
    return nextPath;
  },

  async runFile(path: string): Promise<number> {
    if (isDesktop) return invoke("run_file", { path });
    await restoreBrowserWorkspace();
    const source = demoFiles[path];
    if (source === undefined) throw new Error(`Cannot read ${path}`);
    return runBrowserSource(source);
  },

  async runNotebook(path: string): Promise<number> {
    if (isDesktop) return invoke("run_notebook", { path });
    const jobId = ++demoJobId;
    window.setTimeout(() => {
      window.dispatchEvent(new CustomEvent("demo-job-output", {
        detail: {
          jobId,
          stream: "stdout",
          data: "# Origin analysis\nGC content: 0.4783\nCandidate windows: 2\n",
        },
      }));
    }, 180);
    window.setTimeout(() => {
      window.dispatchEvent(new CustomEvent("demo-job-finished", {
        detail: { jobId, exitCode: 0, durationMs: 390 },
      }));
    }, 420);
    return jobId;
  },

  async runNotebookSource(source: string): Promise<number> {
    if (isDesktop) return invoke("run_notebook_source", { source });
    return runBrowserSource(source);
  },

  async runWorkflow(path: string): Promise<number> {
    if (isDesktop) return invoke("run_workflow", { path });
    const jobId = ++demoJobId;
    window.setTimeout(() => {
      window.dispatchEvent(new CustomEvent("demo-job-output", {
        detail: { jobId, stream: "stdout", data: `Workflow ${path} completed\n3 records\n` },
      }));
    }, 180);
    window.setTimeout(() => {
      window.dispatchEvent(new CustomEvent("demo-job-finished", {
        detail: { jobId, exitCode: 0, durationMs: 460 },
      }));
    }, 500);
    return jobId;
  },

  async stopJob(jobId: number): Promise<void> {
    if (isDesktop) return invoke("stop_job", { jobId });
    cancelledBrowserJobs.add(jobId);
  },

  async packages(): Promise<PackageInfo[]> {
    return isDesktop ? invoke("list_packages") : demoPackages;
  },

  async installPackages(): Promise<string> {
    if (isDesktop) return invoke("install_packages");
    throw new Error("Package installation requires BioLang Desktop or a remote SOMER runtime");
  },

  async startConsole(): Promise<ConsoleResponse> {
    return isDesktop ? invoke("start_console") : browserConsoleResponse();
  },

  async evaluateConsole(source: string): Promise<ConsoleResponse> {
    return isDesktop ? invoke("evaluate_console", { source }) : browserConsoleResponse(source);
  },

  async inspectConsole(): Promise<ConsoleResponse> {
    return isDesktop ? invoke("inspect_console") : browserConsoleResponse();
  },

  async resetConsole(): Promise<ConsoleResponse> {
    if (isDesktop) return invoke("reset_console");
    return resetBrowserConsole();
  },

  async stopConsole(): Promise<void> {
    if (isDesktop) return invoke("stop_console");
  },

  async closeConsole(): Promise<void> {
    if (isDesktop) return invoke("close_console");
    await resetBrowserConsole();
  },

  async startTerminal(cols: number, rows: number): Promise<number> {
    return isDesktop ? invoke("start_terminal", { cols, rows }) : 1;
  },

  async writeTerminal(sessionId: number, data: string): Promise<void> {
    if (isDesktop) return invoke("terminal_write", { sessionId, data });
  },

  async resizeTerminal(sessionId: number, cols: number, rows: number): Promise<void> {
    if (isDesktop) return invoke("terminal_resize", { sessionId, cols, rows });
  },

  async closeTerminal(sessionId: number): Promise<void> {
    if (isDesktop) return invoke("close_terminal", { sessionId });
  },

  async startLsp(): Promise<boolean> {
    return isDesktop ? invoke("start_lsp") : false;
  },

  async sendLsp(message: unknown): Promise<void> {
    if (isDesktop) return invoke("send_lsp", { message });
  },
};

export async function onJobOutput(callback: (event: JobOutputEvent) => void): Promise<UnlistenFn> {
  if (isDesktop) return listen<JobOutputEvent>("job-output", ({ payload }) => callback(payload));
  const handler = (event: Event) => callback((event as CustomEvent<JobOutputEvent>).detail);
  window.addEventListener("demo-job-output", handler);
  return () => window.removeEventListener("demo-job-output", handler);
}

export async function onJobFinished(
  callback: (event: JobFinishedEvent) => void,
): Promise<UnlistenFn> {
  if (isDesktop) return listen<JobFinishedEvent>("job-finished", ({ payload }) => callback(payload));
  const handler = (event: Event) => callback((event as CustomEvent<JobFinishedEvent>).detail);
  window.addEventListener("demo-job-finished", handler);
  return () => window.removeEventListener("demo-job-finished", handler);
}

export async function onTerminalOutput(
  callback: (event: TerminalOutputEvent) => void,
): Promise<UnlistenFn> {
  if (isDesktop) return listen<TerminalOutputEvent>("terminal-output", ({ payload }) => callback(payload));
  return () => undefined;
}

export async function onLspMessage(callback: (message: unknown) => void): Promise<UnlistenFn> {
  if (isDesktop) return listen("lsp-message", ({ payload }) => callback(payload));
  return () => undefined;
}
