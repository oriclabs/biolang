export type Activity = "explorer" | "search" | "packages" | "apis" | "jobs" | "help";
export type BottomPanel = "problems" | "output" | "console" | "terminal" | "jobs";
export type HelpKind = "language" | "builtin" | "tutorial" | "example";

export interface HelpEntry {
  id: string;
  kind: HelpKind;
  title: string;
  category: string;
  collection: string;
  summary: string;
  body: string;
  signature?: string;
  example?: string;
  returnType?: string;
  code?: string;
  sourcePath?: string;
  keywords: string;
}

export interface HelpIndex {
  schemaVersion: number;
  counts: Record<HelpKind, number>;
  entries: HelpEntry[];
}

export interface FileEntry {
  name: string;
  path: string;
  kind: "file" | "directory";
  size: number;
  children: FileEntry[];
}

export interface WorkspaceSnapshot {
  name: string;
  root: string;
  entries: FileEntry[];
  truncated: boolean;
}

export interface GitFileStatus {
  path: string;
  indexStatus: string;
  worktreeStatus: string;
}

export interface GitStatusSnapshot {
  available: boolean;
  branch?: string;
  files: GitFileStatus[];
}

export interface EnvironmentInfo {
  platform: string;
  architecture: string;
  workspace: string;
  blPath?: string;
  blVersion?: string;
  lspAvailable: boolean;
}

export interface ConsoleVariable {
  name: string;
  typeName: string;
  preview: string;
  sizeBytes: number;
}

export interface ConsoleEnvironment {
  variables: ConsoleVariable[];
  totalBytes: number;
}

export interface ConsoleValue {
  kind: "text" | "table" | "sequence";
  typeName: string;
  text: string;
  columns: string[];
  rows: string[][];
  sequence?: string;
  truncated: boolean;
}

export interface ConsoleResponse {
  protocol: "biolang.console/v1";
  id: number;
  status: "ok" | "error";
  output: string;
  value?: ConsoleValue;
  error?: string;
  durationMs: number;
  environment: ConsoleEnvironment;
}

export interface OpenFile {
  path: string;
  name: string;
  content: string;
  savedContent: string;
  language: string;
  untitled?: boolean;
  preferredDirectory?: string;
  preview?: DataPreview;
  viewer?: "editor" | "data" | "notebook" | "workflow";
}

export interface PackageInfo {
  name: string;
  version?: string;
  source: string;
  installed: boolean;
}

export type JobLogStream = "stdout" | "stderr" | "system" | "success";

export interface JobLogChunk {
  stream: JobLogStream;
  text: string;
}

export interface Job {
  id: string;
  file: string;
  status: "staging" | "running" | "succeeded" | "failed" | "cancelled" | "disconnected";
  startedAt: number;
  durationMs?: number;
  exitCode?: number | null;
  backend: string;
  targetId?: string;
  remoteId?: string;
  cellIndex?: number;
  log: JobLogChunk[];
}

export interface NotebookCellOutput {
  text: string;
  status: "running" | "succeeded" | "failed" | "cancelled";
  stale?: boolean;
}

export interface SearchHit {
  path: string;
  line: number;
  column: number;
  preview: string;
}

export interface DataPreview {
  kind: "fasta" | "fastq" | "vcf" | "bed" | "gff" | "sam" | "newick"
    | "structure" | "image" | "pdf" | "svg" | "table" | "json" | "text";
  columns: string[];
  rows: string[][];
  sequence?: string;
  sequences?: Array<{ name: string; sequence: string }>;
  content?: string;
  summary: string[];
  truncated: boolean;
  totalBytes: number;
  provenance?: FileProvenance;
}

export interface FileProvenance {
  path: string;
  format: string;
  size: number;
  modifiedMs?: number;
  importedFrom?: string;
  importedAtMs?: number;
  sha256?: string;
}

export interface ImportValidationDiagnostic {
  unit: string;
  line: number;
  column: number;
  message: string;
  rendered: string;
}

export interface ImportValidationReport {
  valid: boolean;
  unitsChecked: number;
  diagnostics: ImportValidationDiagnostic[];
}

export interface CodeImportResult {
  sourceFormat: "python" | "r" | "ipynb" | "rmd";
  sourceName: string;
  sourceContent: string;
  suggestedName: string;
  notebook: boolean;
  content: string;
  validation: ImportValidationReport;
}

export interface SomerProfile {
  id: string;
  name: string;
  baseUrl: string;
  resourceProfile: string;
  connectionMode?: "direct" | "proxy" | "ssh";
  proxyUrl?: string;
  sshHost?: string;
  sshUser?: string;
  sshPort?: number;
  sshIdentityFile?: string;
}

export interface Problem {
  path: string;
  message: string;
  severity: 1 | 2 | 3 | 4;
  line: number;
  column: number;
}

export interface JobOutputEvent {
  jobId: number;
  stream: "stdout" | "stderr";
  data: string;
}

export interface JobFinishedEvent {
  jobId: number;
  exitCode?: number | null;
  durationMs: number;
}

export interface TerminalOutputEvent {
  sessionId: number;
  data: string;
}
