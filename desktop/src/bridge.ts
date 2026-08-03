import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  browserConsoleResponse,
  browserQcMetrics,
  evaluateBrowserSource,
  importBrowserSource,
  resetBrowserConsole,
  setBrowserRuntimeFileReader,
  validateBrowserImport,
} from "./browserRuntime";
import { loadBrowserWorkspace, saveBrowserWorkspace } from "./browserWorkspace";
import { credentialCatalog, type CredentialStatus } from "./credentials";
import {
  defaultSearchOptions,
  replacementFor,
  searchPattern,
  type SearchOptions,
} from "./searchOptions";
import { demoFiles, demoPackages, demoWorkspace } from "./demo";
import { packFileEntries, packWorkspaceFiles, type PackBundle } from "./packs";
import type {
  DataPreview,
  CodeImportResult,
  ConsoleResponse,
  ImportValidationReport,
  EnvironmentInfo,
  GitStatusSnapshot,
  JobFinishedEvent,
  Job,
  JobInputProvenance,
  JobProvenance,
  JobOutputEvent,
  JobResultEvent,
  JobTraceEvent,
  JobArtifactsEvent,
  PackageInfo,
  ReferenceBuild,
  RestoreReport,
  ResultPageData,
  ResultPageRequest,
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

/**
 * Add a downloaded example pack to the browser workspace.
 *
 * Idempotent: re-opening the same deep link replaces the pack's folder rather
 * than appending a second copy, so a shared link is safe to follow twice. The
 * restore runs first because a pack must merge into whatever the user already
 * has, not overwrite it.
 */
export async function installPackIntoWorkspace(bundle: PackBundle): Promise<void> {
  if (isDesktop) throw new Error("Example packs install into the browser workspace only");
  // Serialised: two installs that overlap would both await the restore, both
  // find no existing folder, and both append one — which React then reports as
  // duplicate keys. StrictMode makes that the normal case in development.
  const run = packInstalls.then(() => installPackNow(bundle));
  packInstalls = run.then(() => undefined, () => undefined);
  return run;
}

let packInstalls: Promise<void> = Promise.resolve();

/**
 * The version of a pack already sitting in the workspace, if any.
 *
 * Following a deep link used to download the whole pack every time, so opening
 * a second problem from the same pack fetched all thirty-four again and
 * reinstalled them. Each link now opens its own tab, so an in-memory cache
 * would never be hit; the workspace is the only thing that persists, so that is
 * what gets asked.
 */
export async function installedPackVersion(packId: string): Promise<string | undefined> {
  if (isDesktop) return undefined;
  await restoreBrowserWorkspace();
  const manifest = demoFiles[`${packId}/pack.toml`];
  return manifest?.match(/^\s*version\s*=\s*"([^"]+)"/m)?.[1];
}

/** Read an installed pack's manifest, for resolving a problem without the bundle. */
export async function installedPackManifest(packId: string): Promise<string | undefined> {
  if (isDesktop) return undefined;
  await restoreBrowserWorkspace();
  return demoFiles[`${packId}/pack.toml`];
}

/**
 * Put the shared sample data into the workspace.
 *
 * Documentation examples read `data/counts.csv` and three dozen siblings. A
 * pack example never needed them — that is why packs were the only thing with a
 * workbench link — but a tutorial opened here failed on the first read. The
 * whole set is 61 KiB, so it is installed once and left in place.
 *
 * Quiet on failure: sample data missing is a reason for one example not to run,
 * not a reason to abandon opening the file.
 */
export async function installDataFilesIntoWorkspace(base = ""): Promise<number> {
  if (isDesktop) return 0;
  try {
    const response = await fetch(`${base}/packs/data-bundle.json`);
    if (!response.ok) return 0;
    const bundle = (await response.json()) as { files?: Record<string, string> };
    const files = bundle.files ?? {};
    if (Object.keys(files).length === 0) return 0;

    await restoreBrowserWorkspace();
    Object.assign(demoFiles, files);

    const entries = Object.keys(files)
      .sort()
      .map((path) => ({
        path,
        name: path.slice("data/".length),
        kind: "file" as const,
        size: files[path].length,
        children: [],
      }));
    const tree = {
      path: "data",
      name: "data",
      kind: "directory" as const,
      size: 0,
      children: entries,
    };
    const existing = demoWorkspace.entries.findIndex((entry) => entry.path === "data");
    if (existing >= 0) demoWorkspace.entries[existing] = tree;
    else demoWorkspace.entries.push(tree);

    demoSelected = true;
    await persistBrowserWorkspace();
    return entries.length;
  } catch {
    return 0;
  }
}

async function installPackNow(bundle: PackBundle): Promise<void> {
  await restoreBrowserWorkspace();

  const prefix = `${bundle.id}/`;
  for (const path of Object.keys(demoFiles)) {
    if (path.startsWith(prefix)) delete demoFiles[path];
  }
  Object.assign(demoFiles, packWorkspaceFiles(bundle));

  const tree = packFileEntries(bundle);
  const existing = demoWorkspace.entries.findIndex((entry) => entry.path === bundle.id);
  if (existing >= 0) demoWorkspace.entries[existing] = tree;
  else demoWorkspace.entries.push(tree);

  demoSelected = true;
  await persistBrowserWorkspace();
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

function emitBrowserJobResult(jobId: number, value: unknown) {
  window.dispatchEvent(new CustomEvent("demo-job-result", {
    detail: { jobId, value },
  }));
}

function dispatchBrowserJobResult(
  jobId: number,
  value: string,
  returned: boolean,
  type?: string,
  structured?: import("./types").StructuredResult,
  results?: import("./types").StructuredResult[],
) {
  // Values passed to print/println arrive here as typed results, so a script
  // written the idiomatic way fills the Tables and Plots views rather than
  // only the text log. The runtime already appends the program's own return
  // value to this list, so `structured` needs no second emission.
  for (const displayed of results ?? []) emitBrowserJobResult(jobId, displayed);
  // `println(...)` returns Nil. Emitting a fallback for it would report a
  // phantom extra result next to the table the user actually printed.
  if (structured || !returned) return;
  const trimmed = value.trim();
  if (!trimmed) return;
  emitBrowserJobResult(jobId, trimmed.startsWith("<svg")
    ? { kind: "plot", format: "svg", data: value }
    : { kind: "string", value, type });
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
        if (result.trace?.length) {
          window.dispatchEvent(new CustomEvent("demo-job-trace", {
            detail: { jobId, entries: result.trace },
          }));
        }
        if (result.ok) {
          const returned = Boolean(result.value)
            && !["null", "nil", "Nil", "()", "None"].includes(result.value ?? "");
          // Printed results stand on their own: `println(table)` returns Nil,
          // so gating the whole dispatch on a returned value hid them.
          if (returned || result.results?.length) {
            dispatchBrowserJobResult(
              jobId,
              result.value ?? "",
              returned,
              result.type,
              result.structured,
              result.results,
            );
          }
          // Echo the return value only when it is not already on show as a
          // typed result. Repeating it dumped whole SVG documents into the
          // text log next to the plot they render as.
          if (returned && !result.structured) {
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

  /**
   * Pick an absolute filesystem path outside the workspace model.
   *
   * Used for reference FASTA/GTF and SSH identity files. Browser returns null:
   * there is no host filesystem to browse.
   */
  async pickPath(options?: {
    title?: string;
    filters?: Array<{ name: string; extensions: string[] }>;
  }): Promise<string | null> {
    if (!isDesktop) return null;
    return invoke("pick_path", {
      title: options?.title,
      filters: options?.filters ?? [],
    });
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

  async compareRunEnvironment(provenance: JobProvenance): Promise<RestoreReport> {
    if (isDesktop) return invoke("compare_run_environment", { request: provenance });
    return {
      checked: false,
      drift: [],
      notes: ["Comparing a run environment requires BioLang Desktop."],
    };
  },

  async restoreRunEnvironment(provenance: JobProvenance, restoreSource: boolean): Promise<string> {
    if (isDesktop) {
      return invoke("restore_run_environment", { request: provenance, restoreSource });
    }
    throw new Error("Restoring a run environment requires BioLang Desktop");
  },

  async listReferenceBuilds(): Promise<ReferenceBuild[]> {
    if (isDesktop) return invoke("list_reference_builds");
    // The registry is a file in the home directory; the browser has neither.
    return [];
  },

  async saveReferenceBuild(name: string, assets: Record<string, string>): Promise<void> {
    if (isDesktop) return invoke("save_reference_build", { name, assets });
    throw new Error("Reference builds require BioLang Desktop");
  },

  async deleteReferenceBuild(name: string): Promise<void> {
    if (isDesktop) return invoke("delete_reference_build", { name });
  },

  async gitStage(paths: string[]): Promise<void> {
    if (isDesktop) return invoke("git_stage", { paths });
    throw new Error("Git requires BioLang Desktop");
  },

  async gitUnstage(paths: string[]): Promise<void> {
    if (isDesktop) return invoke("git_unstage", { paths });
    throw new Error("Git requires BioLang Desktop");
  },

  async gitCommit(message: string): Promise<string> {
    if (isDesktop) return invoke("git_commit", { message });
    throw new Error("Git requires BioLang Desktop");
  },

  async gitDiff(path: string, staged: boolean): Promise<string> {
    if (isDesktop) return invoke("git_diff", { path, staged });
    return "";
  },

  async runWorkspaceTests(path?: string): Promise<{
    results: import("./types").TestResult[];
    passed: number;
    failed: number;
    durationMs: number;
  }> {
    if (isDesktop) return invoke("run_workspace_tests", { path });
    // The browser build has no `bl` binary; the WASM runtime evaluates one
    // program at a time and cannot walk a workspace.
    throw new Error("Running tests requires BioLang Desktop");
  },

  async listCredentials(): Promise<CredentialStatus[]> {
    if (isDesktop) return invoke("list_credentials");
    // The browser build has no keyring and no child process to inject into, so
    // a stored key could not reach the WASM runtime anyway. Reporting them as
    // unconfigured is honest; the Settings UI explains why.
    return credentialCatalog.map((credential) => ({
      name: credential.name,
      configured: false,
      fromEnvironment: false,
    }));
  },

  async setCredential(name: string, value: string): Promise<void> {
    if (isDesktop) return invoke("set_credential", { name, value });
    throw new Error("Credentials require BioLang Desktop");
  },

  async deleteCredential(name: string): Promise<void> {
    if (isDesktop) return invoke("delete_credential", { name });
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

  /**
   * Move a workspace entry into `destinationDirectory` ("" = workspace root).
   * Keeps the original basename.
   */
  async moveEntry(path: string, destinationDirectory: string): Promise<string> {
    if (isDesktop) {
      return invoke("move_entry", { path, destinationDirectory });
    }
    await restoreBrowserWorkspace();
    const normalized = path.replaceAll("\\", "/");
    const dest = destinationDirectory.replaceAll("\\", "/").replace(/^\/+|\/+$/g, "");
    const separator = normalized.lastIndexOf("/");
    const parent = separator >= 0 ? normalized.slice(0, separator) : "";
    if (parent === dest) return normalized;
    if (dest === normalized || dest.startsWith(`${normalized}/`)) {
      throw new Error("Cannot move a folder into itself");
    }
    const sourceEntries = findDemoDirectory(parent);
    const index = sourceEntries.findIndex((candidate) => candidate.path === normalized);
    if (index < 0) throw new Error(`${path} was not found`);
    const [entry] = sourceEntries.splice(index, 1);
    const destEntries = findDemoDirectory(dest);
    if (destEntries.some((candidate) => candidate.name === entry.name)) {
      throw new Error(`${entry.name} already exists in the destination`);
    }
    const nextPath = dest ? `${dest}/${entry.name}` : entry.name;
    destEntries.push(entry);
    rebaseDemoEntry(entry, nextPath);
    await persistBrowserWorkspace();
    return nextPath;
  },

  /**
   * Create a new file with content (parents created as needed). Used by OS drop.
   */
  async writeNewFile(path: string, content: string | Uint8Array): Promise<string> {
    if (isDesktop) {
      const bytes = typeof content === "string"
        ? Array.from(new TextEncoder().encode(content))
        : Array.from(content);
      return invoke("write_new_file", { path, content: bytes });
    }
    await restoreBrowserWorkspace();
    const normalized = path.replaceAll("\\", "/");
    const separator = normalized.lastIndexOf("/");
    const parent = separator >= 0 ? normalized.slice(0, separator) : "";
    const name = separator >= 0 ? normalized.slice(separator + 1) : normalized;
    if (parent) {
      const parts = parent.split("/").filter(Boolean);
      let cursor = "";
      for (const part of parts) {
        const next = cursor ? `${cursor}/${part}` : part;
        try {
          findDemoDirectory(next);
        } catch {
          const siblings = findDemoDirectory(cursor);
          if (!siblings.some((entry) => entry.name === part)) {
            siblings.push({ name: part, path: next, kind: "directory", size: 0, children: [] });
          }
        }
        cursor = next;
      }
    }
    const entries = findDemoDirectory(parent);
    if (entries.some((entry) => entry.name === name)) throw new Error(`${path} already exists`);
    entries.push({ name, path: normalized, kind: "file", size: 0, children: [] });
    demoFiles[normalized] = typeof content === "string"
      ? content
      : new TextDecoder().decode(content);
    await persistBrowserWorkspace();
    return normalized;
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

  async searchWorkspace(
    query: string,
    options: SearchOptions = defaultSearchOptions,
  ): Promise<SearchHit[]> {
    if (isDesktop) {
      return invoke("search_workspace", {
        query,
        caseSensitive: options.caseSensitive,
        wholeWord: options.wholeWord,
        regex: options.regex,
      });
    }
    await restoreBrowserWorkspace();
    const pattern = searchPattern(query.trim(), options);
    if (!pattern) return [];
    const hits: SearchHit[] = [];
    for (const [path, content] of Object.entries(demoFiles)) {
      for (const [index, line] of content.split(/\r?\n/).entries()) {
        pattern.lastIndex = 0;
        const match = pattern.exec(line);
        if (match) hits.push({ path, line: index + 1, column: match.index + 1, preview: line.trim() });
      }
    }
    return hits.slice(0, 200);
  },

  /**
   * Rewrite every match across the workspace and report how many files changed.
   *
   * Search without replace meant leaving the app for `sed` to rename anything
   * across a project, which is exactly the point at which people stop treating
   * an editor as their editor.
   */
  async replaceInWorkspace(
    query: string,
    replacement: string,
    options: SearchOptions = defaultSearchOptions,
  ): Promise<number> {
    if (isDesktop) {
      return invoke("replace_in_workspace", {
        query,
        replacement,
        caseSensitive: options.caseSensitive,
        wholeWord: options.wholeWord,
        regex: options.regex,
      });
    }
    await restoreBrowserWorkspace();
    const pattern = searchPattern(query.trim(), options);
    if (!pattern) return 0;
    let changed = 0;
    for (const [path, content] of Object.entries(demoFiles)) {
      const next = replacementFor(content, pattern, replacement, options);
      if (next === content) continue;
      demoFiles[path] = next;
      changed += 1;
    }
    if (changed) await persistBrowserWorkspace();
    return changed;
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
    if (extension === "fastq" || extension === "fq") {
      const lines = content.split(/\r?\n/);
      const rows: string[][] = [];
      for (let index = 0; index + 3 < lines.length; index += 4) {
        if (!lines[index].startsWith("@")) continue;
        rows.push([lines[index].slice(1), String(lines[index + 1].length), String(lines[index + 3].length)]);
      }
      return {
        kind: "fastq",
        columns: ["Read", "Length", "Quality length"],
        rows,
        summary: [`${rows.length} reads sampled`],
        truncated: false,
        totalBytes: content.length,
        provenance,
        metrics: await browserQcMetrics("fastq", content),
      };
    }
    if (extension === "vcf") {
      const lines = content.split(/\r?\n/);
      const header = lines.find((line) => line.startsWith("#CHROM"));
      return {
        kind: "vcf",
        columns: header ? header.slice(1).split("\t") : [],
        rows: lines.filter((line) => line && !line.startsWith("#")).map((line) => line.split("\t")),
        summary: [`${lines.filter((line) => line && !line.startsWith("#")).length} variants sampled`],
        truncated: false,
        totalBytes: content.length,
        provenance,
        metrics: await browserQcMetrics("vcf", content),
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

  async exportBinary(suggestedName: string, content: Uint8Array, mediaType = "application/octet-stream"): Promise<string | null> {
    if (isDesktop) return invoke("export_binary", { suggestedName, content: Array.from(content) });
    const blob = new Blob([content], { type: mediaType });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = suggestedName;
    anchor.click();
    URL.revokeObjectURL(url);
    return anchor.download;
  },

  async loadRunHistory(): Promise<Job[]> {
    if (isDesktop) return invoke("load_run_history");
    try {
      const value = JSON.parse(localStorage.getItem("biolang.desktop.jobs") ?? "[]");
      return Array.isArray(value) ? value as Job[] : [];
    } catch {
      return [];
    }
  },

  async saveRunHistory(jobs: Job[]): Promise<void> {
    if (isDesktop) return invoke("save_run_history", { jobs });
    localStorage.setItem("biolang.desktop.jobs", JSON.stringify(jobs));
  },

  async deleteRunHistory(jobId: string): Promise<void> {
    if (isDesktop) return invoke("delete_run_history", { jobId });
    const jobs = await bridge.loadRunHistory();
    localStorage.setItem(
      "biolang.desktop.jobs",
      JSON.stringify(jobs.filter((job) => job.id !== jobId)),
    );
  },

  async checksumWorkspaceFiles(paths: string[]): Promise<JobInputProvenance[]> {
    if (isDesktop) return invoke("checksum_workspace_files", { paths });
    await restoreBrowserWorkspace();
    return paths.flatMap((path) => {
      const content = demoFiles[path];
      return content === undefined ? [] : [{
        path,
        size: new TextEncoder().encode(content).byteLength,
        checksumStatus: "unavailable" as const,
      }];
    });
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

  async readWorkspaceBinary(path: string): Promise<Uint8Array> {
    if (isDesktop) return new Uint8Array(await invoke<number[]>("read_workspace_binary", { path }));
    await restoreBrowserWorkspace();
    const value = demoFiles[path];
    if (value === undefined) throw new Error(`Cannot read ${path}`);
    return new TextEncoder().encode(value);
  },

  async readWorkspaceBinaryRange(path: string, offset = 0, length = 1024 * 1024): Promise<Uint8Array> {
    if (isDesktop) {
      return new Uint8Array(await invoke<number[]>("read_workspace_binary_range", { path, offset, length }));
    }
    await restoreBrowserWorkspace();
    const value = demoFiles[path];
    if (value === undefined) throw new Error(`Cannot read ${path}`);
    return new TextEncoder().encode(value).slice(offset, offset + length);
  },

  async readJsonlPage(path: string, request: ResultPageRequest): Promise<ResultPageData> {
    if (isDesktop) {
      const page = await invoke<Omit<ResultPageData, "columns">>("read_jsonl_page", {
        path,
        offset: request.offset,
        limit: request.limit,
        search: request.search,
        sortColumn: request.sortColumn,
        descending: request.descending ?? false,
      });
      return { ...page, columns: [] };
    }
    await restoreBrowserWorkspace();
    const content = demoFiles[path];
    if (content === undefined) throw new Error(`Cannot read ${path}`);
    const needle = request.search?.trim().toLowerCase();
    const rows = content.split(/\r?\n/).filter(Boolean).map((line) => JSON.parse(line) as unknown[])
      .filter((row) => !needle || JSON.stringify(row).toLowerCase().includes(needle));
    if (request.sortColumn !== undefined) {
      rows.sort((left, right) => String(left[request.sortColumn!]).localeCompare(String(right[request.sortColumn!]), undefined, { numeric: true })
        * (request.descending ? -1 : 1));
    }
    return {
      columns: [],
      rows: rows.slice(request.offset, request.offset + request.limit),
      offset: request.offset,
      limit: request.limit,
      totalRows: rows.length,
      filteredRows: rows.length,
    };
  },

  async writeFile(path: string, content: string): Promise<void> {
    if (isDesktop) return invoke("write_file", { path, content });
    await restoreBrowserWorkspace();
    demoFiles[path] = content;
    await persistBrowserWorkspace();
  },

  async copyText(text: string): Promise<void> {
    if (isDesktop) return invoke("write_clipboard", { text });
    await navigator.clipboard.writeText(text);
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

  async runSource(source: string): Promise<number> {
    if (isDesktop) return invoke("run_source", { source });
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

export async function onJobResult(callback: (event: JobResultEvent) => void): Promise<UnlistenFn> {
  if (isDesktop) return listen<JobResultEvent>("job-result", ({ payload }) => callback(payload));
  const handler = (event: Event) => callback((event as CustomEvent<JobResultEvent>).detail);
  window.addEventListener("demo-job-result", handler);
  return () => window.removeEventListener("demo-job-result", handler);
}

export async function onJobTrace(callback: (event: JobTraceEvent) => void): Promise<UnlistenFn> {
  if (isDesktop) return listen<JobTraceEvent>("job-trace", ({ payload }) => callback(payload));
  const handler = (event: Event) => callback((event as CustomEvent<JobTraceEvent>).detail);
  window.addEventListener("demo-job-trace", handler);
  return () => window.removeEventListener("demo-job-trace", handler);
}

export async function onJobArtifacts(callback: (event: JobArtifactsEvent) => void): Promise<UnlistenFn> {
  if (isDesktop) return listen<JobArtifactsEvent>("job-artifacts", ({ payload }) => callback(payload));
  return () => undefined;
}

export async function onJobFinished(
  callback: (event: JobFinishedEvent) => void,
): Promise<UnlistenFn> {
  if (isDesktop) return listen<JobFinishedEvent>("job-finished", ({ payload }) => callback(payload));
  const handler = (event: Event) => callback((event as CustomEvent<JobFinishedEvent>).detail);
  window.addEventListener("demo-job-finished", handler);
  return () => window.removeEventListener("demo-job-finished", handler);
}

export async function onRunHistoryChanged(callback: () => void): Promise<UnlistenFn> {
  if (isDesktop) return listen("run-history-changed", callback);
  const handler = () => callback();
  window.addEventListener("storage", handler);
  return () => window.removeEventListener("storage", handler);
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
