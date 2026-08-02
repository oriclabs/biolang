export type Activity = "explorer" | "search" | "scm" | "packages" | "apis" | "jobs" | "help";
export type BottomPanel = "problems" | "output" | "tests" | "assignment" | "console" | "terminal" | "jobs";
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
  members: string[];
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

export interface StructuredResult {
  kind: string;
  id?: string;
  name?: string;
  resultIndex?: number;
  value?: unknown;
  display?: string;
  format?: string;
  data?: string;
  columns?: string[];
  rows?: unknown[][];
  items?: StructuredResult[];
  totalRows?: number;
  totalColumns?: number;
  totalItems?: number;
  truncated?: boolean;
  [key: string]: unknown;
}

export interface ResultPageRequest {
  offset: number;
  limit: number;
  search?: string;
  sortColumn?: number;
  descending?: boolean;
}

export interface ResultPageData {
  columns: string[];
  rows: unknown[][];
  offset: number;
  limit: number;
  totalRows: number;
  filteredRows: number;
}

export interface JobArtifact {
  name: string;
  path?: string;
  mediaType?: string;
  size?: number;
  sha256?: string;
  downloadUrl?: string;
}

export interface JobProvenance {
  biolangVersion?: string;
  packages: Record<string, string>;
  backend: string;
  targetId?: string;
  sourceHash?: string;
  sourceSnapshot?: string;
  workspace?: string;
  entrypoint: string;
  parameters: Record<string, string | number | boolean>;
  capturedAt?: string;
  platform?: string;
  architecture?: string;
  inputs?: JobInputProvenance[];
  randomSeed?: string;
  tools?: Array<{ name: string; version?: string; path?: string }>;
  runtime?: {
    locale?: string;
    timezone?: string;
    logicalCpus?: number;
    userAgent?: string;
  };
  environmentFiles?: JobInputProvenance[];
}

export interface JobInputProvenance {
  path: string;
  size: number;
  modifiedMs?: number;
  sha256?: string;
  checksumStatus: "complete" | "skipped-large" | "unavailable";
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
  results?: StructuredResult[];
  /** Values printed by the run, tagged with the source line that printed them. */
  trace?: JobTraceEntry[];
  artifacts?: JobArtifact[];
  provenance?: JobProvenance;
  displayName?: string;
  pinned?: boolean;
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
  metrics?: PreviewMetrics;
}

export interface PreviewFact {
  label: string;
  value: string;
}

export interface PreviewSeries {
  name: string;
  values: number[];
}

export interface PreviewChart {
  title: string;
  /** `line` for a positional profile, `bar` for a distribution. */
  kind: "line" | "bar";
  xLabel: string;
  yLabel: string;
  categories: string[];
  series: PreviewSeries[];
}

/** Quality metrics for formats where a table of raw lines says nothing. */
export interface PreviewMetrics {
  facts: PreviewFact[];
  charts: PreviewChart[];
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

/** One `test_*` function result, from a `bl test --events` run. */
export interface TestResult {
  file: string;
  name: string;
  label: string;
  passed: boolean;
  durationMs?: number;
  message?: string;
}

export interface TestRun {
  status: "running" | "finished" | "failed";
  results: TestResult[];
  passed: number;
  failed: number;
  durationMs?: number;
  error?: string;
}

/** A named reference genome build from `~/.biolang/references.toml`. */
export interface ReferenceBuild {
  name: string;
  assets: Record<string, string>;
  /** Asset keys whose paths do not exist, so a stale registry is visible. */
  missing: string[];
}

/** One difference between a recorded run and the workspace as it is now. */
export interface RestoreDrift {
  kind: "package" | "biolang" | "input" | "source";
  name: string;
  recorded: string;
  current: string;
  /** True when the workbench can put this back. */
  restorable: boolean;
}

export interface RestoreReport {
  /**
   * True when the workspace was actually inspected. An empty `drift` means
   * "nothing changed" only if this is true; otherwise the check could not run.
   */
  checked: boolean;
  drift: RestoreDrift[];
  /** Why some drift cannot be undone, so the report does not overpromise. */
  notes: string[];
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

export interface JobResultEvent {
  jobId: number;
  value: StructuredResult;
}

/** One printed value, attributed to the line of the statement that printed it. */
export interface JobTraceEntry {
  line: number;
  text: string;
}

export interface JobTraceEvent {
  jobId: number;
  entries: JobTraceEntry[];
}

export interface JobArtifactsEvent {
  jobId: number;
  artifacts: JobArtifact[];
}

export interface TerminalOutputEvent {
  sessionId: number;
  data: string;
}
