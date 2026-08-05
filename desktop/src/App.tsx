import {
  AlertCircle,
  AlertTriangle,
  Blocks,
  BookOpen,
  Braces,
  Check,
  ChevronDown,
  ChevronRight,
  ChevronsDown,
  ChevronsUp,
  CircleStop,
  Columns2,
  Command,
  Copy,
  Database,
  Dna,
  Download,
  File,
  FileCode2,
  FileInput,
  FileJson,
  FilePlus2,
  FileSearch,
  FileText,
  Files,
  Folder,
  GitBranch,
  FolderOpen,
  FolderPlus,
  FlaskConical,
  Globe2,
  HardDrive,
  GraduationCap,
  LoaderCircle,
  Library,
  Info,
  Minus,
  Package,
  PanelBottom,
  PanelRight,
  Pencil,
  Plus,
  Play,
  Redo2,
  KeyRound,
  Replace,
  RefreshCw,
  Save,
  Search,
  Server,
  Settings,
  TerminalSquare,
  Trash2,
  Undo2,
  Upload,
  WrapText,
  X,
  Zap,
} from "lucide-react";
import { lazy, Suspense, useCallback, useDeferredValue, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import {
  bridge,
  installDataFilesIntoWorkspace,
  installedPackManifest,
  installedPackVersion,
  installPackIntoWorkspace,
  isDesktop,
} from "./bridge";
import {
  fetchPackBundle,
  fetchPackIndex,
  packSummary,
  parsePackLink,
  problemPath,
  problemPathFromManifest,
} from "./packs";
import { EditorSurface } from "./components/EditorSurface";
import { FileTree, fileIcon } from "./components/FileTree";
import { VirtualList } from "./components/VirtualList";
import { ErrorBoundary } from "./components/ErrorBoundary";
import type { ImportSaveRequest } from "./components/ImportCodeDialog";
import { ImportUrlDialog } from "./components/ImportUrlDialog";
import { JobLog } from "./components/JobLog";
import { LearnerGuide } from "./components/LearnerGuide";
import { OutputPane, type OutputLocation } from "./components/OutputPane";
import { SettingsDialog, type SettingsSection } from "./components/SettingsDialog";
import { TerminalManager } from "./components/TerminalManager";
import { useJobManager } from "./hooks/useJobManager";
import { useLspManager } from "./hooks/useLspManager";
import { usePwa } from "./hooks/usePwa";
import { loadRecoverySession, useSessionRecovery } from "./hooks/useSessionRecovery";
import { useWorkspaceManager } from "./hooks/useWorkspaceManager";
import { biolangVariant, comparisonTask, comparisonVariants, lineCount } from "./comparison";
import { credentialsForService, isMissing, type CredentialStatus } from "./credentials";
import { fuzzyMatch, highlightSegments } from "./fuzzy";
import { jobLogText, latestJobForFile } from "./jobLogs";
import { languageForPath } from "./language";
import { importProblems } from "./importIssues";
import { ASSIGNMENT_MANIFEST, parseAssignment, taskProgress, isComplete } from "./assignment";
import {
  commandForEvent,
  formatChord,
  resolveBindings,
  type KeybindingMap,
} from "./keybindings";
import { apaReference, bibtex } from "./methods";
import { findOccurrences } from "./occurrences";
import { defaultSearchOptions, MIN_SEARCH_LENGTH, searchPattern, type SearchOptions } from "./searchOptions";
import { setStageRunner } from "./pipelineLens";
import {
  buildOutputExport,
  outputExportOptions,
  type OutputExportFormat,
} from "./outputExport";
import { buildRunBundle, createZip } from "./runBundle";
import type {
  Activity,
  BottomPanel,
  CodeImportResult,
  FileEntry,
  HelpEntry,
  HelpIndex,
  HelpKind,
  JobArtifact,
  OpenFile,
  GitFileStatus,
  JobProvenance,
  Problem,
  ReferenceBuild,
  SearchHit,
  TestRun,
  SomerProfile,
  WorkspaceSnapshot,
} from "./types";
import { viewerForPath } from "./viewers";

const categoryDescriptions: Record<string, string> = {
  api: "External biological data services",
  bio: "Sequences and biological records",
  container: "Lists, maps, and records",
  core: "Language and conversion primitives",
  fs: "Files and datasets",
  kmer: "K-mer analysis",
  matrix: "Dense matrix operations",
  plot: "Scientific visualization",
  runtime: "Runtime-registered functions",
  stats: "Statistics and distributions",
  stream: "Lazy and bounded processing",
  table: "Tabular data operations",
};

const welcomeExamples = [
  {
    id: "sequence-qc",
    name: "Sequence QC",
    detail: "Inspect length, GC content, and reverse complement.",
    fileName: "sequence_qc.bl",
    icon: "sequence",
    source: `let sequence = dna"ATGCGTACGTTAGCGATCGATCG"
println("Length: {seq_len(sequence)}")
println("GC content: {gc_content(sequence)}")
println("Reverse complement: {reverse_complement(sequence)}")
`,
  },
  {
    id: "kmer-table",
    name: "K-mer Table",
    detail: "Generate a structured table that can be sorted and exported.",
    fileName: "kmer_table.bl",
    icon: "table",
    source: `kmer_count(dna"ATCGATCGATCGATCG", 3)
`,
  },
  {
    id: "expression-plot",
    name: "Expression Plot",
    detail: "Run an analysis and open the resulting interactive plot.",
    fileName: "expression_plot.bl",
    icon: "plot",
    source: `plot({
  x: ["BRCA1", "TP53", "EGFR", "KRAS"],
  y: [8.2, 11.4, 6.7, 9.1],
  title: "Relative gene expression",
  xlabel: "Gene",
  ylabel: "Expression"
})
`,
  },
] as const;

/**
 * One runnable starting point on the welcome screen.
 *
 * Structural rather than `typeof welcomeExamples[number]`, because the language
 * comparison synthesizes one at runtime and the `as const` literal union would
 * reject it.
 */
type WelcomeExample = {
  id: string;
  name: string;
  detail: string;
  fileName: string;
  icon: string;
  source: string;
};

type FunctionEntry = {
  name: string;
  signature: string;
  example: string;
  summary: string;
  returnType?: string | null;
};

type FunctionGroup = {
  name: string;
  description: string;
  functions: FunctionEntry[];
};

function groupBuiltinMetadata(builtins: Array<{
  name: string;
  signature: string;
  example?: string | null;
  summary: string;
  category: string;
  returnType?: string | null;
}>): FunctionGroup[] {
  return [...new Set(builtins.map((builtin) => builtin.category))]
    .sort()
    .map((category) => ({
      name: category.replaceAll("_", " ").replace(/\b\w/g, (value) => value.toUpperCase()),
      description: categoryDescriptions[category] ?? "BioLang built-in functions",
      functions: builtins
        .filter((builtin) => builtin.category === category)
        .map((builtin) => ({
          name: builtin.name,
          signature: builtin.signature,
          example: builtin.example ?? builtin.signature,
          summary: builtin.summary,
          returnType: builtin.returnType,
        })),
    }));
}

function externalApiProvider(name: string): string {
  if (name.startsWith("ncbi_")) return "NCBI";
  if (name.startsWith("uniprot_")) return "UniProt";
  if (name.startsWith("ensembl_")) return "Ensembl";
  if (name.startsWith("go_")) return "Gene Ontology";
  if (name.startsWith("kegg_")) return "KEGG";
  if (name.startsWith("pdb_")) return "RCSB PDB";
  return "BioLang runtime";
}

const CodeEditor = lazy(() => import("./components/CodeEditor"));
const TestPane = lazy(() => import("./components/TestPane").then((module) => ({ default: module.TestPane })));
const AssignmentPane = lazy(() => import("./components/AssignmentPane").then((module) => ({ default: module.AssignmentPane })));
const ConsolePane = lazy(() => import("./components/ConsolePane").then((module) => ({ default: module.ConsolePane })));
type ConsoleSubmission = import("./components/ConsolePane").ConsoleSubmission;
const ImportCodeDialog = lazy(() => import("./components/ImportCodeDialog").then((module) => ({ default: module.ImportCodeDialog })));
const HelpDocument = lazy(() => import("./components/HelpDocument"));
const DataPreviewPane = lazy(() => import("./components/DataPreviewPane").then((module) => ({ default: module.DataPreviewPane })));
const NotebookPane = lazy(() => import("./components/NotebookPane").then((module) => ({ default: module.NotebookPane })));
const PipelineViewer = lazy(() => import("./components/PipelineViewer").then((module) => ({ default: module.PipelineViewer })));
const WorkflowPane = lazy(() => import("./components/WorkflowPane").then((module) => ({ default: module.WorkflowPane })));

type EntryPrompt = {
  mode: "directory" | "rename" | "save";
  parent?: string;
  path?: string;
  value: string;
};

type ConfirmationRequest = {
  title: string;
  message: string;
  confirmLabel: string;
  danger?: boolean;
  resolve: (confirmed: boolean) => void;
};

type DocumentSymbolEntry = {
  name: string;
  kind: string;
  line: number;
};

type ContextMenuTarget = {
  kind: "explorer";
  entry: FileEntry;
} | {
  kind: "workspace";
} | {
  kind: "tab";
  path: string;
};

type ContextMenuState = ContextMenuTarget & {
  x: number;
  y: number;
};

type MenuItem = {
  label?: string;
  shortcut?: string;
  disabled?: boolean;
  checked?: boolean;
  separator?: boolean;
  action?: () => void | Promise<void>;
};

function flattenFiles(entries: FileEntry[]): FileEntry[] {
  return entries.flatMap((entry) => (entry.kind === "directory" ? flattenFiles(entry.children) : [entry]));
}

function directoryPaths(entries: FileEntry[]): string[] {
  return entries.flatMap((entry) =>
    entry.kind === "directory" ? [entry.path, ...directoryPaths(entry.children)] : []);
}

function resolveHelpPath(sourcePath: string, href: string) {
  const [linkedPath, hash = ""] = href.split("#", 2);
  if (!linkedPath) return { path: sourcePath, hash };
  const parts = [...sourcePath.split("/").slice(0, -1), ...linkedPath.replaceAll("\\", "/").split("/")];
  const normalized: string[] = [];
  for (const part of parts) {
    if (!part || part === ".") continue;
    if (part === "..") normalized.pop();
    else normalized.push(part);
  }
  return { path: normalized.join("/"), hash };
}

function helpIcon(kind: HelpKind) {
  if (kind === "language") return <BookOpen size={14} />;
  if (kind === "builtin") return <Braces size={14} />;
  if (kind === "tutorial") return <GraduationCap size={14} />;
  return <FileCode2 size={14} />;
}

function IconButton({
  label,
  active,
  disabled,
  onClick,
  children,
  className = "",
}: {
  label: string;
  active?: boolean;
  disabled?: boolean;
  onClick?: () => void;
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <button
      type="button"
      className={`icon-button ${active ? "active" : ""} ${className}`}
      title={label}
      aria-label={label}
      aria-pressed={active}
      disabled={disabled}
      onClick={onClick}
    >
      {children}
    </button>
  );
}

function EmptyState({ icon, title, detail }: { icon: React.ReactNode; title: string; detail: string }) {
  return (
    <div className="empty-state">
      {icon}
      <strong>{title}</strong>
      <span>{detail}</span>
    </div>
  );
}

function storedSetting<T>(key: string, fallback: T): T {
  try {
    const value = window.localStorage.getItem(`biolang.desktop.${key}`);
    return value == null ? fallback : JSON.parse(value) as T;
  } catch {
    return fallback;
  }
}

function useStoredSetting<T>(key: string, fallback: T) {
  const [value, setValue] = useState<T>(() => storedSetting(key, fallback));
  useEffect(() => {
    window.localStorage.setItem(`biolang.desktop.${key}`, JSON.stringify(value));
  }, [key, value]);
  return [value, setValue] as const;
}

function restoredActivePath(files: OpenFile[], candidate: string | undefined): string | undefined {
  return candidate && files.some((file) => file.path === candidate)
    ? candidate
    : files.at(-1)?.path;
}

function isDirtyFile(file: OpenFile): boolean {
  return Boolean(file.untitled) || file.content !== file.savedContent;
}

function isRunnableFile(file: OpenFile | undefined): boolean {
  return Boolean(file && (
    file.path.endsWith(".bl")
    || (file.untitled && file.language === "biolang")
    || file.viewer === "notebook"
    || file.viewer === "workflow"
  ));
}

function isRunnablePath(path: string): boolean {
  const viewer = viewerForPath(path);
  return path.endsWith(".bl") || viewer === "notebook" || viewer === "workflow";
}

/** Why the primary Run control cannot start a job right now, if any. */
function runBlockedReason(options: {
  file: OpenFile | undefined;
  trusted: boolean;
  running: boolean;
}): string | undefined {
  if (!isRunnableFile(options.file)) return "Open a BioLang file (.bl, notebook, or workflow) to run";
  if (options.running) return "A run is already in progress";
  // Untrusted workspaces stay clickable: the handler trusts and runs.
  if (!options.trusted) return "Trust this workspace, then run (click to trust and run)";
  return undefined;
}

function formatElapsed(durationMs: number): string {
  const totalSeconds = Math.max(0, Math.floor(durationMs / 1000));
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return minutes ? `${minutes}:${String(seconds).padStart(2, "0")}` : `${seconds}s`;
}

/** One row in the command palette, whatever the palette is currently listing. */
type PaletteItem = {
  label: string;
  /** Secondary text: a directory for files, a kind and line for symbols. */
  hint?: string;
  icon: ReactNode;
  run: () => void | Promise<void>;
};

type PaletteEntry = PaletteItem & { id: string; score: number; positions: number[] };

type PaletteMode = "command" | "symbol" | "file";

const PALETTE_RECENT_LIMIT = 24;
/** Enough rows to scroll through without rendering a whole large workspace. */
const PALETTE_RESULT_LIMIT = 60;

/**
 * The leading character selects what the palette lists, matching the convention
 * every editor with a palette already uses: `>` commands, `@` symbols, and a
 * bare query for files.
 */
function paletteModeFor(search: string): PaletteMode {
  if (search.startsWith(">")) return "command";
  if (search.startsWith("@")) return "symbol";
  return "file";
}

function directoryOf(path: string): string {
  return path.replaceAll("\\", "/").split("/").slice(0, -1).join("/");
}

function paletteEmptyMessage(mode: PaletteMode, hasWorkspace: boolean): string {
  if (mode === "command") return "No matching commands.";
  if (mode === "symbol") return "No symbols in the active file.";
  return hasWorkspace ? "No matching files." : "Open a folder to search files, or type > for commands.";
}

function documentSymbols(file: OpenFile | undefined): DocumentSymbolEntry[] {
  if (!file?.path.endsWith(".bl")) return [];
  const patterns = [
    { kind: "Function", expression: /^\s*fn\s+([A-Za-z_][A-Za-z0-9_]*)/ },
    { kind: "Binding", expression: /^\s*let\s+([A-Za-z_][A-Za-z0-9_]*)/ },
    { kind: "Type", expression: /^\s*(?:struct|enum|type)\s+([A-Za-z_][A-Za-z0-9_]*)/ },
    { kind: "Pipeline", expression: /^\s*(?:pipeline|stage)\s+([A-Za-z_][A-Za-z0-9_]*)/ },
  ];
  return file.content.split(/\r?\n/).flatMap((line, index) => {
    for (const pattern of patterns) {
      const match = line.match(pattern.expression);
      if (match) return [{ name: match[1], kind: pattern.kind, line: index + 1 }];
    }
    return [];
  });
}

const defaultSomerProfiles: SomerProfile[] = [
  {
    id: "somer-lab",
    name: "SOMER Lab",
    baseUrl: "http://127.0.0.1:8787",
    resourceProfile: "standard",
    connectionMode: "direct",
  },
];

const productEdition = isDesktop ? "Desktop" : "Workbench Web";
const productName = `BioLang ${productEdition}`;
const bottomPanelOrder: BottomPanel[] = ["assignment", "problems", "output", "tests", "console", "terminal", "jobs"];
const defaultBottomPanels: BottomPanel[] = ["problems", "output", "console", "terminal"];

type NoticeKind = "info" | "error";
type Notice = { id: number; message: string; kind: NoticeKind };

export function App() {
  const [notice, setNotice] = useState<Notice>();
  const [stickyNotices, setStickyNotices] = useState<Notice[]>([]);
  const [startupPhase, setStartupPhase] = useState<"workspace" | "session" | "ready">("workspace");
  const [startupSlow, setStartupSlow] = useState(false);
  const remoteRunAcknowledged = useRef(false);
  const showNotice = useCallback((message: string, kind?: NoticeKind) => {
    const resolved: NoticeKind = kind
      ?? (/cannot |failed|error:|exception|denied|unavailable|trust this workspace before/i.test(message)
        ? "error"
        : "info");
    const entry: Notice = { id: Date.now() + Math.random(), message, kind: resolved };
    setNotice(entry);
    if (resolved === "error") {
      setStickyNotices((current) => [entry, ...current.filter((item) => item.message !== message)].slice(0, 5));
    }
    window.setTimeout(
      () => setNotice((current) => current?.id === entry.id ? undefined : current),
      resolved === "error" ? 8_000 : 2_800,
    );
  }, []);
  const pwa = usePwa(showNotice);
  const {
    workspace,
    environment,
    packages,
    gitStatus,
    recentWorkspaces,
    workspaceTrusted,
    setPackages,
    initialize: initializeWorkspace,
    activate: commitWorkspace,
    select: chooseWorkspace,
    openRecent: chooseRecentWorkspace,
    close: closeManagedWorkspace,
    refresh: refreshManagedWorkspace,
    refreshGit: refreshGitStatus,
    trust: trustWorkspace,
  } = useWorkspaceManager(showNotice);
  const [openFiles, setOpenFiles] = useState<OpenFile[]>([]);
  const [activePath, setActivePath] = useState<string>();
  /** Secondary side of a horizontal editor split (paths from openFiles). */
  const [splitOpen, setSplitOpen] = useState(false);
  const [secondaryTabs, setSecondaryTabs] = useState<string[]>([]);
  const [secondaryActive, setSecondaryActive] = useState<string>();
  const [focusedGroup, setFocusedGroup] = useState<"primary" | "secondary">("primary");
  const [splitPercent, setSplitPercent] = useStoredSetting("editorSplitPercent", 50);
  const [secondaryPipelineView, setSecondaryPipelineView] = useState(false);
  const [activity, setActivity] = useState<Activity>("explorer");
  const [collapsedTreePaths, setCollapsedTreePaths] = useState<Set<string>>(() => new Set());
  const [bottomPanel, setBottomPanel] = useState<BottomPanel>("output");
  const [bottomVisible, setBottomVisible] = useStoredSetting("bottomVisible", false);
  const [visibleBottomPanels, setVisibleBottomPanels] = useStoredSetting<BottomPanel[]>(
    "visibleBottomPanels",
    defaultBottomPanels,
  );
  const [panelChromeSimplified, setPanelChromeSimplified] = useStoredSetting("panelChromeSimplified", false);
  const [jobsTabActivated, setJobsTabActivated] = useStoredSetting("jobsTabActivated", false);
  useEffect(() => {
    if (panelChromeSimplified) return;
    setVisibleBottomPanels((panels) => panels.filter(
      (panel) => panel !== "assignment" && panel !== "tests" && panel !== "jobs",
    ));
    setPanelChromeSimplified(true);
  }, [panelChromeSimplified, setPanelChromeSimplified, setVisibleBottomPanels]);

  const [terminalMounted, setTerminalMounted] = useState(false);
  const [problems, setProblems] = useState<Problem[]>([]);
  const [packageBusy, setPackageBusy] = useState(false);
  const [search, setSearch] = useState("");
  const [searchHits, setSearchHits] = useState<SearchHit[]>([]);
  const [searchBusy, setSearchBusy] = useState(false);
  const [searchOptions, setSearchOptions] = useStoredSetting<SearchOptions>("searchOptions", defaultSearchOptions);
  const [replaceText, setReplaceText] = useState("");
  const [replaceOpen, setReplaceOpen] = useState(false);
  const [apiSearch, setApiSearch] = useState("");
  const [apiScope, setApiScope] = useState<"external" | "all">("external");
  const [functionGroups, setFunctionGroups] = useState<FunctionGroup[]>([]);
  const [selectedApi, setSelectedApi] = useState<FunctionEntry>();
  const [helpIndex, setHelpIndex] = useState<HelpIndex>();
  const [helpSearch, setHelpSearch] = useState("");
  const [helpSection, setHelpSection] = useState<HelpKind | "all">("all");
  const [selectedHelpId, setSelectedHelpId] = useState<string>();
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [paletteSearch, setPaletteSearch] = useState("");
  const [consoleSubmission, setConsoleSubmission] = useState<ConsoleSubmission>();
  const [referenceResults, setReferenceResults] = useState<{ name: string; hits: SearchHit[] }>();
  const [paletteIndex, setPaletteIndex] = useState(0);
  const [paletteRecent, setPaletteRecent] = useStoredSetting<string[]>("paletteRecent", []);
  const [openMenu, setOpenMenu] = useState<string>();
  const [contextMenu, setContextMenu] = useState<ContextMenuState>();
  const [entryPrompt, setEntryPrompt] = useState<EntryPrompt>();
  const [confirmation, setConfirmation] = useState<ConfirmationRequest>();
  const [codeImport, setCodeImport] = useState<CodeImportResult>();
  const [importUrlOpen, setImportUrlOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [settingsSection, setSettingsSection] = useState<SettingsSection>("editor");
  const openSettings = useCallback((section: SettingsSection = "editor") => {
    setSettingsSection(section);
    setSettingsOpen(true);
  }, []);
  const [shortcutsOpen, setShortcutsOpen] = useState(false);
  const [somerProfiles, setSomerProfiles] = useStoredSetting<SomerProfile[]>(
    "somerProfiles",
    defaultSomerProfiles,
  );
  const [executionTarget, setExecutionTarget] = useStoredSetting("executionTarget", "local");
  const [somerTokens, setSomerTokens] = useState<Record<string, string>>({});
  const [aboutOpen, setAboutOpen] = useState(false);
  const [formatOnSave, setFormatOnSave] = useStoredSetting("formatOnSave", true);
  const [showInlineResults, setShowInlineResults] = useStoredSetting("showInlineResults", true);
  const [keybindingOverrides, setKeybindingOverrides] = useStoredSetting<KeybindingMap>("keybindings", {});
  const keybindings = useMemo(() => resolveBindings(keybindingOverrides), [keybindingOverrides]);
  const shortcutLabel = useCallback((id: keyof typeof keybindings) => formatChord(keybindings[id]), [keybindings]);
  const [credentialStatuses, setCredentialStatuses] = useState<CredentialStatus[]>([]);
  const [testRun, setTestRun] = useState<TestRun>();
  const [commitMessage, setCommitMessage] = useState("");
  const [referenceBuilds, setReferenceBuilds] = useState<ReferenceBuild[]>([]);
  const [comparisonLanguage, setComparisonLanguage] = useState<"biolang" | "python" | "r">("biolang");
  const [learnerGuideDismissed, setLearnerGuideDismissed] = useStoredSetting("learnerGuideDismissed", false);
  const [minimap, setMinimap] = useStoredSetting("minimap", true);
  const [wordWrap, setWordWrap] = useStoredSetting("wordWrap", false);
  const [pipelineView, setPipelineView] = useState(false);
  const [fontSize, setFontSize] = useStoredSetting("fontSize", 13);
  const [tabSize, setTabSize] = useStoredSetting("tabSize", 2);
  const [experienceMode, setExperienceMode] = useStoredSetting<"learner" | "expert">(
    "experienceMode",
    "expert",
  );
  const [editorTheme, setEditorTheme] = useStoredSetting<"biolang-dark" | "biolang-light" | "vs-dark" | "hc-black">(
    "editorTheme",
    "biolang-dark",
  );
  const [sidebarWidth, setSidebarWidth] = useStoredSetting("sidebarWidth", 250);
  const [bottomPanelHeight, setBottomPanelHeight] = useStoredSetting("bottomPanelHeight", 246);
  const [outputLocation, setOutputLocation] = useStoredSetting<OutputLocation>("outputLocation", "bottom");
  const [outputPanelWidth, setOutputPanelWidth] = useStoredSetting("outputPanelWidth", 440);
  const [outputRightVisible, setOutputRightVisible] = useStoredSetting("outputRightVisible", true);
  const [outputEditorOpen, setOutputEditorOpen] = useState(false);
  const [outputEditorActive, setOutputEditorActive] = useState(false);
  const [outputDragging, setOutputDragging] = useState(false);
  const [outputDragTarget, setOutputDragTarget] = useState<OutputLocation>();
  const [outputExportFormat, setOutputExportFormat] = useState<OutputExportFormat>("log");
  const [outputRunByFile, setOutputRunByFile] = useState<Record<string, string>>({});
  const [compareOutputRunId, setCompareOutputRunId] = useState<string>();
  const [inspectorWidth, setInspectorWidth] = useStoredSetting("inspectorWidth", 286);
  const [panelMaximized, setPanelMaximized] = useState(false);
  const [jobClock, setJobClock] = useState(() => Date.now());
  const contextMenuRef = useRef<HTMLDivElement>(null);
  const paletteListRef = useRef<HTMLDivElement>(null);
  const outputDockCleanupRef = useRef<() => void>();
  const initialized = useRef(false);
  const untitledCounter = useRef(1);
  const pendingNavigation = useRef<{ path: string; line: number; column: number }>();
  /** Path to run once the workspace is trusted (welcome examples / trust-and-run). */
  const pendingRun = useRef<{ path: string; cellIndex?: number }>();
  const visibleBottomPanelSet = useMemo(
    () => new Set(visibleBottomPanels.filter((panel) => bottomPanelOrder.includes(panel))),
    [visibleBottomPanels],
  );
  /**
   * The assignment in this workspace, if there is one.
   *
   * Read from the open file when the student has it open, and from disk
   * otherwise, so the Tasks panel appears without them having to find and open
   * the manifest first.
   */
  const [assignmentSource, setAssignmentSource] = useState<string>();
  useEffect(() => {
    if (!workspace) {
      setAssignmentSource(undefined);
      return;
    }
    let disposed = false;
    void bridge.readFile(ASSIGNMENT_MANIFEST)
      .then((text) => { if (!disposed) setAssignmentSource(text); })
      .catch(() => { if (!disposed) setAssignmentSource(undefined); });
    return () => { disposed = true; };
  }, [workspace]);

  const assignment = useMemo(
    () => (assignmentSource ? parseAssignment(assignmentSource) : undefined),
    [assignmentSource],
  );
  const assignmentProgress = useMemo(
    () => (assignment ? taskProgress(assignment, testRun?.results ?? []) : []),
    [assignment, testRun],
  );

  const availableBottomPanels = useMemo(
    () => bottomPanelOrder.filter(
      (panel) => (panel === "assignment" ? Boolean(assignment) : visibleBottomPanelSet.has(panel))
        && (panel !== "output" || outputLocation === "bottom")
        // Learners use the Output run selector for history; a second Jobs tab is noise.
        && (experienceMode === "expert" || panel !== "jobs")
    ),
    [assignment, experienceMode, outputLocation, visibleBottomPanelSet],
  );

  const showOutput = useCallback(() => {
    // Learner mode always docks Output at the bottom so results are not lost in
    // a right column or editor tab the guide never mentions.
    if (experienceMode === "learner" || outputLocation === "bottom") {
      setVisibleBottomPanels((current) => current.includes("output") ? current : [...current, "output"]);
      setOutputLocation("bottom");
      setOutputRightVisible(false);
      setOutputEditorOpen(false);
      setOutputEditorActive(false);
      setBottomPanel("output");
      setBottomVisible(true);
      return;
    }
    if (outputLocation === "right") {
      setOutputRightVisible(true);
      return;
    }
    if (outputLocation === "editor") {
      setOutputEditorOpen(true);
      setOutputEditorActive(true);
      return;
    }
    if (!visibleBottomPanelSet.has("output")) return;
    setBottomPanel("output");
    setBottomVisible(true);
  }, [experienceMode, outputLocation, setBottomVisible, setOutputRightVisible, setVisibleBottomPanels, visibleBottomPanelSet]);

  useEffect(() => {
    if (!bottomVisible || availableBottomPanels.includes(bottomPanel)) return;
    const nextPanel = availableBottomPanels[0];
    if (nextPanel) {
      setBottomPanel(nextPanel);
    } else {
      setBottomVisible(false);
      setPanelMaximized(false);
    }
  }, [availableBottomPanels, bottomPanel, bottomVisible, setBottomVisible]);

  const confirmAction = useCallback((request: Omit<ConfirmationRequest, "resolve">) =>
    new Promise<boolean>((resolve) => setConfirmation({ ...request, resolve })), []);
  const settleConfirmation = useCallback((confirmed: boolean) => {
    setConfirmation((current) => {
      current?.resolve(confirmed);
      return undefined;
    });
  }, []);

  const primaryFile = openFiles.find((file) => file.path === activePath);
  const secondaryFile = openFiles.find((file) => file.path === secondaryActive);
  const focusedPath = splitOpen && focusedGroup === "secondary" ? secondaryActive : activePath;
  const activeFile = openFiles.find((file) => file.path === focusedPath) ?? primaryFile;
  const activeSymbols = useMemo(() => documentSymbols(activeFile), [activeFile]);
  const sequenceStats = useMemo(() => {
    const sequence = activeFile?.preview?.sequence?.toUpperCase();
    if (!sequence) return undefined;
    const gc = [...sequence].filter((base) => base === "G" || base === "C").length;
    const n = [...sequence].filter((base) => base === "N").length;
    return {
      length: sequence.length,
      gcPercent: sequence.length ? (gc / sequence.length) * 100 : 0,
      n,
    };
  }, [activeFile?.preview?.sequence]);
  const {
    state: lspState,
    editorRef,
    beforeMount,
    editorMounted,
    openDocument,
    closeDocument,
    queueChange: queueLspChange,
    stop: stopLsp,
    notebookCellMounted,
    notebookCellChanged,
    notebookCellUnmounted,
  } = useLspManager({ workspace, trusted: workspaceTrusted, openFiles, setProblems });
  useSessionRecovery(workspace, openFiles, activePath);
  const allFiles = useMemo(() => flattenFiles(workspace?.entries ?? []), [workspace]);

  /**
   * Conversion markers left by `bl import`, as problems.
   *
   * Kept separate from `problems` rather than merged into it, because the
   * language server replaces its diagnostics per path on every publish and
   * would wipe these on the next keystroke.
   */
  const importIssues = useMemo(
    () => openFiles
      .filter((file) => file.path.endsWith(".bl") && !file.preview)
      .flatMap((file) => importProblems(file.path, file.content)),
    [openFiles],
  );
  const allProblems = useMemo(
    () => [...problems, ...importIssues],
    [importIssues, problems],
  );
  const allDirectoryPaths = useMemo(() => directoryPaths(workspace?.entries ?? []), [workspace]);
  const gitByPath = useMemo(
    () => new Map(gitStatus.files.map((status) => [status.path, status])),
    [gitStatus.files],
  );
  const toggleTreeDirectory = useCallback((path: string) => {
    setCollapsedTreePaths((current) => {
      const next = new Set(current);
      if (next.has(path)) next.delete(path);
      else next.add(path);
      return next;
    });
  }, []);
  const expandAllTreeDirectories = useCallback(() => setCollapsedTreePaths(new Set()), []);
  const collapseAllTreeDirectories = useCallback(
    () => setCollapsedTreePaths(new Set(allDirectoryPaths)),
    [allDirectoryPaths],
  );

  useEffect(() => {
    setCollapsedTreePaths(new Set());
    setTerminalMounted(false);
  }, [workspace?.root]);
  const deferredHelpSearch = useDeferredValue(helpSearch.trim().toLowerCase());
  const filteredHelp = useMemo(() => {
    const entries = helpIndex?.entries ?? [];
    return entries.filter((entry) => {
      if (helpSection !== "all" && entry.kind !== helpSection) return false;
      if (!deferredHelpSearch) return true;
      return `${entry.keywords} ${entry.summary} ${entry.body}`.toLowerCase().includes(deferredHelpSearch);
    });
  }, [deferredHelpSearch, helpIndex, helpSection]);
  const selectedHelp = filteredHelp.find((entry) => entry.id === selectedHelpId) ?? filteredHelp[0];
  const explorerContextEntry = contextMenu?.kind === "explorer" ? contextMenu.entry : undefined;
  const workspaceContext = contextMenu?.kind === "workspace";
  const tabContextPath = contextMenu?.kind === "tab" ? contextMenu.path : undefined;
  const tabContextFile = openFiles.find((file) => file.path === tabContextPath);

  const {
    notebookCellOutputs,
    invalidateNotebookCell,
    jobs,
    runningJob,
    selectedJob,
    connectionState,
    runFile,
    runNotebookCell,
    executeFile,
    rerunJob,
    stopActive,
    testSomerConnection,
    syncSomerHistory,
    selectJob,
    abortRemotePolling,
    clearJobLog,
    pinJob,
    renameJob,
    deleteJob,
    readJobArtifact,
    readJobArtifactPreview,
    saveJobArtifact,
    readResultPage,
    recordDesktopTask,
  } = useJobManager({
    environment,
    packages,
    workspaceTrusted,
    somerProfiles,
    somerTokens,
    executionTarget,
    openFiles,
    setOpenFiles,
    setActivePath,
    setBottomPanel,
    setBottomVisible,
    showOutput,
    showNotice,
  });
  useEffect(() => {
    if (!jobs.length || jobsTabActivated || experienceMode === "learner") return;
    setVisibleBottomPanels((panels) => panels.includes("jobs") ? panels : [...panels, "jobs"]);
    setJobsTabActivated(true);
  }, [experienceMode, jobs.length, jobsTabActivated, setJobsTabActivated, setVisibleBottomPanels]);

  useEffect(() => {
    if (experienceMode !== "learner" || outputLocation === "bottom") return;
    setOutputLocation("bottom");
    setOutputRightVisible(false);
    setOutputEditorOpen(false);
    setOutputEditorActive(false);
  }, [experienceMode, outputLocation]);

  useEffect(() => {
    if (experienceMode === "learner" && activity === "jobs") setActivity("explorer");
  }, [activity, experienceMode]);
  const outputRuns = useMemo(
    () => activeFile ? jobs.filter((job) => job.file === activeFile.path) : [],
    [activeFile, jobs],
  );
  const activeOutputJob = useMemo(() => {
    if (!activeFile) return undefined;
    const selectedId = outputRunByFile[activeFile.path];
    return outputRuns.find((job) => job.id === selectedId)
      ?? latestJobForFile(outputRuns, activeFile.path);
  }, [activeFile, outputRunByFile, outputRuns]);
  useEffect(() => {
    if (
      activeFile?.preview
      && outputLocation === "bottom"
      && bottomPanel === "output"
      && !activeOutputJob
    ) {
      setBottomVisible(false);
      setPanelMaximized(false);
    }
  }, [activeFile?.path, activeFile?.preview, activeOutputJob, bottomPanel, outputLocation, setBottomVisible]);
  const compareOutputJob = useMemo(
    () => outputRuns.find((job) => job.id === compareOutputRunId && job.id !== activeOutputJob?.id),
    [activeOutputJob?.id, compareOutputRunId, outputRuns],
  );
  useEffect(() => {
    if (compareOutputRunId && !compareOutputJob) setCompareOutputRunId(undefined);
  }, [compareOutputJob, compareOutputRunId]);
  const activeOutput = useMemo(
    () => jobLogText(activeOutputJob?.log),
    [activeOutputJob?.log],
  );
  const activeOutputExportOptions = useMemo(
    () => outputExportOptions(activeOutputJob?.log),
    [activeOutputJob?.log],
  );
  useEffect(() => {
    if (activeOutputExportOptions.some((option) => option.format === outputExportFormat)) return;
    setOutputExportFormat(activeOutputExportOptions[0]?.format ?? "log");
  }, [activeOutputExportOptions, outputExportFormat]);
  useEffect(() => {
    if (!runningJob) return;
    setJobClock(Date.now());
    const timer = window.setInterval(() => setJobClock(Date.now()), 1_000);
    return () => window.clearInterval(timer);
  }, [runningJob?.id]);

  const queueRunWhenTrusted = useCallback((path: string, cellIndex?: number) => {
    pendingRun.current = { path, cellIndex };
  }, []);

  const executionTargetLabel = useMemo(() => {
    if (executionTarget === "local") return isDesktop ? "Local" : "Browser WASM";
    return somerProfiles.find((profile) => profile.id === executionTarget)?.name ?? "Remote";
  }, [executionTarget, somerProfiles]);

  const runFileWithTrust = useCallback(async (file: OpenFile | undefined, cellIndex?: number) => {
    if (!file || !isRunnableFile(file) || runningJob) return;
    if (executionTarget !== "local" && !remoteRunAcknowledged.current) {
      const confirmed = await confirmAction({
        title: "Run on remote target?",
        message: `This run will be submitted to ${executionTargetLabel}. Confirm the target before continuing.`,
        confirmLabel: `Run on ${executionTargetLabel}`,
      });
      if (!confirmed) return;
      remoteRunAcknowledged.current = true;
    }
    const wasTrusted = workspaceTrusted;
    if (!workspaceTrusted || (isDesktop && executionTarget === "local")) {
      const trusted = await trustWorkspace(true);
      if (!trusted) return;
    }
    if (!wasTrusted) {
      queueRunWhenTrusted(file.path, cellIndex);
      showNotice("Workspace trusted — starting run");
      return;
    }
    if (cellIndex == null) await runFile(file);
    else await runNotebookCell(file, cellIndex);
  }, [
    confirmAction,
    executionTarget,
    executionTargetLabel,
    queueRunWhenTrusted,
    runFile,
    runNotebookCell,
    runningJob,
    showNotice,
    trustWorkspace,
    workspaceTrusted,
  ]);

  /**
   * Primary Run control: when the only blocker is trust, trust the folder and
   * queue the run so learners are not stuck on a disabled play button.
   */
  const handleRunClick = useCallback(async () => {
    await runFileWithTrust(activeFile);
  }, [activeFile, runFileWithTrust]);

  useEffect(() => {
    const pending = pendingRun.current;
    if (!pending || !workspaceTrusted || runningJob) return;
    const file = openFiles.find((candidate) => candidate.path === pending.path);
    if (!file) return;
    pendingRun.current = undefined;
    if (pending.cellIndex == null) void runFile(file);
    else void runNotebookCell(file, pending.cellIndex);
  }, [openFiles, runFile, runNotebookCell, runningJob, workspaceTrusted]);

  const runButtonLabel = useMemo(() => {
    const reason = runBlockedReason({
      file: activeFile,
      trusted: workspaceTrusted,
      running: Boolean(runningJob),
    });
    if (!reason) return "Run active BioLang file";
    if (!workspaceTrusted && isRunnableFile(activeFile) && !runningJob) {
      return "Trust workspace and run";
    }
    return reason;
  }, [activeFile, runningJob, workspaceTrusted]);

  const runButtonDisabled = Boolean(runningJob) || !isRunnableFile(activeFile);

  const runPath = useCallback(async (path: string) => {
    setContextMenu(undefined);
    try {
      let file = openFiles.find((candidate) => candidate.path === path);
      if (!file) {
        const content = await bridge.readFile(path);
        file = {
          path,
          name: path.split("/").pop() ?? path,
          content,
          savedContent: content,
          language: languageForPath(path),
          viewer: viewerForPath(path),
        };
        setOpenFiles((files) => [...files, file!]);
        if (path.endsWith(".bl")) await openDocument(path, content);
      }
      setActivePath(path);
      setOutputEditorActive(false);
      await runFileWithTrust(file);
    } catch (error) {
      showNotice(String(error));
    }
  }, [openDocument, openFiles, runFileWithTrust, showNotice]);

  const somerProfileIds = useMemo(
    () => somerProfiles.map((profile) => profile.id).join("\0"),
    [somerProfiles],
  );

  useEffect(() => {
    let disposed = false;
    void Promise.all(somerProfiles.map(async (profile) => ({
      id: profile.id,
      secret: await bridge.getSomerSecret(profile.id),
    }))).then((stored) => {
      if (disposed) return;
      setSomerTokens((current) => {
        const next = { ...current };
        for (const item of stored) {
          if (item.secret) next[item.id] = item.secret;
        }
        return next;
      });
    }).catch(() => undefined);
    return () => {
      disposed = true;
    };
  }, [somerProfileIds]); // Profile details do not affect the credential lookup key.

  const saveSomerCredential = useCallback(async (profile: SomerProfile) => {
    const secret = somerTokens[profile.id]?.trim();
    if (!secret) {
      showNotice("Enter a bearer token before saving it");
      return;
    }
    try {
      await bridge.setSomerSecret(profile.id, secret);
      showNotice(
        isDesktop
          ? `Saved ${profile.name} token in the operating-system credential store`
          : `Saved ${profile.name} token for this browser session`,
      );
    } catch (error) {
      showNotice(String(error));
    }
  }, [showNotice, somerTokens]);

  const forgetSomerCredential = useCallback(async (profile: SomerProfile) => {
    try {
      await bridge.deleteSomerSecret(profile.id);
      setSomerTokens((current) => {
        const next = { ...current };
        delete next[profile.id];
        return next;
      });
      showNotice(`Removed ${profile.name} token`);
    } catch (error) {
      showNotice(String(error));
    }
  }, [showNotice]);

  useEffect(() => {
    if (activity !== "apis" || functionGroups.length) return;
    void import("./generated/builtin-metadata.json")
      .then((module) => {
        const groups = groupBuiltinMetadata(module.default.builtins);
        setFunctionGroups(groups);
        setSelectedApi(
          groups.find((group) => group.name === "Api")?.functions.find((fn) => fn.name === "ncbi_gene")
            ?? groups[0]?.functions[0],
        );
      })
      .catch((error) => showNotice(`Cannot load BioLang metadata: ${String(error)}`));
  }, [activity, functionGroups.length, showNotice]);

  const activatePathInFocusedGroup = useCallback((path: string) => {
    setOutputEditorActive(false);
    setPipelineView(false);
    setSecondaryPipelineView(false);
    if (splitOpen && focusedGroup === "secondary") {
      setSecondaryTabs((tabs) => (tabs.includes(path) ? tabs : [...tabs, path]));
      setSecondaryActive(path);
      return;
    }
    setActivePath(path);
    setFocusedGroup("primary");
  }, [focusedGroup, splitOpen]);

  const openFile = useCallback(
    async (path: string) => {
      const existing = openFiles.find((file) => file.path === path);
      if (existing) {
        activatePathInFocusedGroup(path);
        return;
      }
      try {
        const entry = allFiles.find((candidate) => candidate.path === path);
        const viewer = viewerForPath(path, entry?.size);
        if (viewer === "data") {
          const preview = await bridge.previewFile(path);
          const file: OpenFile = {
            path,
            name: path.split("/").pop() ?? path,
            content: "",
            savedContent: "",
            language: "preview",
            preview,
            viewer,
          };
          setOpenFiles((files) => [...files, file]);
          activatePathInFocusedGroup(path);
          return;
        }
        const content = await bridge.readFile(path);
        const file: OpenFile = {
          path,
          name: path.split("/").pop() ?? path,
          content,
          savedContent: content,
          language: languageForPath(path),
          viewer,
        };
        setOpenFiles((files) => [...files, file]);
        activatePathInFocusedGroup(path);
        await openDocument(path, content);
      } catch (error) {
        showNotice(String(error));
      }
    },
    [activatePathInFocusedGroup, allFiles, openDocument, openFiles, showNotice],
  );

  const openSplitRight = useCallback((path?: string) => {
    const target = path ?? activePath;
    if (!target) {
      showNotice("Open a file before splitting the editor");
      return;
    }
    setSplitOpen(true);
    setSecondaryTabs((tabs) => (tabs.includes(target) ? tabs : [...tabs, target]));
    setSecondaryActive(target);
    setFocusedGroup("secondary");
    setOutputEditorActive(false);
  }, [activePath, showNotice]);

  const closeSplit = useCallback(() => {
    setSplitOpen(false);
    setSecondaryTabs([]);
    setSecondaryActive(undefined);
    setFocusedGroup("primary");
    setSecondaryPipelineView(false);
  }, []);

  const closeSecondaryTab = useCallback((path: string) => {
    setSecondaryTabs((tabs) => {
      const next = tabs.filter((candidate) => candidate !== path);
      if (!next.length) {
        setSplitOpen(false);
        setSecondaryActive(undefined);
        setFocusedGroup("primary");
      } else {
        setSecondaryActive((current) => current === path ? next.at(-1) : current);
      }
      return next;
    });
  }, []);

  useEffect(() => {
    if (initialized.current) return;
    initialized.current = true;
    const slowTimer = window.setTimeout(() => setStartupSlow(true), 8_000);
    void initializeWorkspace()
      .then((nextWorkspace) => {
        setStartupPhase((current) => current === "ready" ? current : "session");
        if (nextWorkspace) {
          const recovery = loadRecoverySession(nextWorkspace.root);
          const files = recovery.files;
          if (files.length) {
            setOpenFiles(files);
            setActivePath(restoredActivePath(files, recovery.activePath));
          }
        }
      })
      .catch((error) => showNotice(`Workspace restore failed: ${String(error)}`))
      .finally(() => {
        window.clearTimeout(slowTimer);
        setStartupPhase("ready");
      });
  }, [initializeWorkspace, showNotice]);

  useEffect(() => {
    if (activity !== "help" || helpIndex) return;
    let disposed = false;
    void import("./generated/help-index.json")
      .then((module) => {
        if (!disposed) setHelpIndex(module.default as HelpIndex);
      })
      .catch((error) => showNotice(`Cannot load BioLang help: ${String(error)}`));
    return () => {
      disposed = true;
    };
  }, [activity, helpIndex, showNotice]);

  useLayoutEffect(() => {
    const menu = contextMenuRef.current;
    if (!menu || !contextMenu) return;
    const bounds = menu.getBoundingClientRect();
    const x = Math.max(4, Math.min(contextMenu.x, window.innerWidth - bounds.width - 4));
    const y = Math.max(4, Math.min(contextMenu.y, window.innerHeight - bounds.height - 4));
    if (x !== contextMenu.x || y !== contextMenu.y) {
      setContextMenu((current) => current ? { ...current, x, y } : current);
      return;
    }
    menu.querySelector<HTMLButtonElement>('[role="menuitem"]:not(:disabled)')?.focus();
  }, [contextMenu]);

  useEffect(() => {
    if (!workspace || search.trim().length < 2) {
      setSearchHits([]);
      setSearchBusy(false);
      return;
    }
    let disposed = false;
    setSearchBusy(true);
    const timer = window.setTimeout(() => {
      void bridge.searchWorkspace(search, searchOptions)
        .then((hits) => {
          if (!disposed) setSearchHits(hits);
        })
        .catch((error) => {
          if (!disposed) showNotice(String(error));
        })
        .finally(() => {
          if (!disposed) setSearchBusy(false);
        });
    }, 250);
    return () => {
      disposed = true;
      window.clearTimeout(timer);
    };
  }, [search, searchOptions, showNotice, workspace]);

  const newUntitledFile = useCallback((preferredDirectory?: string) => {
    let number = untitledCounter.current;
    while (openFiles.some((file) => file.name === `Untitled-${number}.bl`)) number += 1;
    untitledCounter.current = number + 1;
    const name = `Untitled-${number}.bl`;
    const path = `__untitled__/${Date.now()}-${number}`;
    const file: OpenFile = {
      path,
      name,
      content: "",
      savedContent: "",
      language: "biolang",
      viewer: "editor",
      untitled: true,
      preferredDirectory,
    };
    setOpenFiles((files) => [...files, file]);
    setActivePath(path);
    setActivity("explorer");
    setPipelineView(false);
  }, [openFiles]);

  const promptUntitledSave = useCallback((file: OpenFile) => {
    setEntryPrompt({
      mode: "save",
      path: file.path,
      parent: file.preferredDirectory,
      value: file.name,
    });
  }, []);

  /**
   * Run the registered formatter over the active document and return the
   * result, or undefined when nothing formatted it.
   *
   * The model is read back after the action rather than trusting the React
   * copy, because the edit lands in Monaco first and the state update that
   * mirrors it has not necessarily been applied yet.
   */
  const formatActiveDocument = useCallback(async () => {
    const editor = editorRef.current;
    const action = editor?.getAction("editor.action.formatDocument");
    if (!editor || !action) return undefined;
    try {
      await action.run();
    } catch {
      // A formatter that fails must never block a save.
      return undefined;
    }
    return editor.getModel()?.getValue();
  }, [editorRef]);

  const saveActive = useCallback(async () => {
    if (!activeFile || !isDirtyFile(activeFile)) return;
    if (activeFile.untitled) {
      promptUntitledSave(activeFile);
      return;
    }
    try {
      let content = activeFile.content;
      if (formatOnSave && activeFile.path.endsWith(".bl") && activePath === activeFile.path) {
        content = await formatActiveDocument() ?? content;
      }
      await bridge.writeFile(activeFile.path, content);
      setOpenFiles((files) =>
        files.map((file) =>
          file.path === activeFile.path ? { ...file, savedContent: content } : file,
        ),
      );
      showNotice(`Saved ${activeFile.name}`);
      void refreshGitStatus();
    } catch (error) {
      showNotice(String(error));
    }
  }, [activeFile, activePath, formatActiveDocument, formatOnSave, promptUntitledSave, refreshGitStatus, showNotice]);

  const saveActiveAs = useCallback(async () => {
    if (!activeFile || activeFile.viewer === "data") return;
    if (activeFile.untitled) {
      promptUntitledSave(activeFile);
      return;
    }
    try {
      const previousPath = activeFile.path;
      const nextPath = await bridge.saveFileAs(previousPath, activeFile.content);
      if (!nextPath) return;
      const nextViewer = viewerForPath(nextPath, activeFile.content.length);
      const nextFile: OpenFile = {
        ...activeFile,
        path: nextPath,
        name: nextPath.split("/").pop() ?? nextPath,
        savedContent: activeFile.content,
        language: languageForPath(nextPath),
        viewer: nextViewer === "data" ? "editor" : nextViewer,
      };
      closeDocument(previousPath);
      if (nextPath !== previousPath) closeDocument(nextPath);
      setOpenFiles((files) => {
        const withoutExistingTarget = files.filter(
          (file) => file.path !== nextPath || file.path === previousPath,
        );
        return withoutExistingTarget.map((file) =>
          file.path === previousPath ? nextFile : file,
        );
      });
      setActivePath(nextPath);
      await openDocument(nextPath, nextFile.content);
      await refreshManagedWorkspace();
      void refreshGitStatus();
      showNotice(`Saved as ${nextFile.name}`);
    } catch (error) {
      showNotice(String(error));
    }
  }, [
    activeFile,
    closeDocument,
    openDocument,
    promptUntitledSave,
    refreshGitStatus,
    refreshManagedWorkspace,
    showNotice,
  ]);

  const closeFile = useCallback(
    async (path: string) => {
      const file = openFiles.find((candidate) => candidate.path === path);
      if (file && isDirtyFile(file) && !await confirmAction({
        title: "Discard unsaved changes?",
        message: `Close ${file.name} without saving its changes?`,
        confirmLabel: "Discard and close",
        danger: true,
      })) return;
      const next = openFiles.filter((candidate) => candidate.path !== path);
      setOpenFiles(next);
      closeDocument(path);
      if (activePath === path) setActivePath(next.at(-1)?.path);
      setSecondaryTabs((tabs) => {
        const remaining = tabs.filter((candidate) => candidate !== path);
        if (!remaining.length && splitOpen) {
          setSplitOpen(false);
          setSecondaryActive(undefined);
          setFocusedGroup("primary");
        } else {
          setSecondaryActive((current) => current === path ? remaining.at(-1) : current);
        }
        return remaining;
      });
    },
    [activePath, closeDocument, confirmAction, openFiles, splitOpen],
  );

  const activateWorkspace = useCallback(async (next: WorkspaceSnapshot) => {
    abortRemotePolling();
    await bridge.closeConsole().catch(() => undefined);
    stopLsp(openFiles);
    const recovery = loadRecoverySession(next.root);
    const files = recovery.files;
    await commitWorkspace(next);
    setOpenFiles(files);
    setActivePath(restoredActivePath(files, recovery.activePath));
    setProblems([]);
  }, [abortRemotePolling, commitWorkspace, openFiles, stopLsp]);

  const selectWorkspace = useCallback(async () => {
    try {
      const next = await chooseWorkspace();
      if (next) await activateWorkspace(next);
    } catch (error) {
      showNotice(String(error), "error");
    }
  }, [activateWorkspace, chooseWorkspace, showNotice]);

  /**
   * Open a welcome example, optionally trusting the workspace and running it.
   *
   * Desktop used to leave the file open with Run disabled until the user found
   * the trust banner. Teaching flows pass `run: true` so the first click is a
   * complete Folder → Trust → Run loop.
   */
  const openWelcomeExample = useCallback(async (
    example: WelcomeExample,
    options: { run?: boolean } = { run: true },
  ) => {
    try {
      let root = workspace?.root;
      let shouldRun = Boolean(options.run);
      if (!workspace) {
        const next = await chooseWorkspace();
        if (!next) return;
        await activateWorkspace(next);
        root = next.root;
      }
      if (shouldRun && isDesktop && root) {
        shouldRun = await trustWorkspace(true, root);
      }
      const path = `__untitled__/example-${example.id}-${Date.now()}`;
      const file: OpenFile = {
        path,
        name: example.fileName,
        content: example.source,
        savedContent: "",
        language: "biolang",
        viewer: "editor",
        untitled: true,
      };
      setOpenFiles((files) => [...files, file]);
      setActivePath(path);
      setActivity("explorer");
      setPipelineView(false);
      // Queue after state updates; the trust+openFiles effect starts the job.
      if (shouldRun) queueRunWhenTrusted(path);
    } catch (error) {
      showNotice(String(error));
    }
  }, [activateWorkspace, chooseWorkspace, queueRunWhenTrusted, showNotice, trustWorkspace, workspace]);

  /**
   * Open the BioLang side of the comparison as a runnable file.
   *
   * The point of the panel is that the code on screen is the code that runs, so
   * this uses the same source it displays rather than a tidied-up variant.
   */
  const openComparisonExample = useCallback(async () => {
    await openWelcomeExample({
      id: "language-comparison",
      name: "Language comparison",
      detail: comparisonTask,
      fileName: "compare_fasta_gc.bl",
      icon: "sequence",
      source: biolangVariant().source,
    }, { run: true });
  }, [openWelcomeExample]);

  /**
   * One-click teaching entry: open the built-in tutorial workspace (browser demo
   * project, or a starter analysis on Desktop), switch to Learner mode, and land
   * on a runnable file.
   */
  const openTutorialProject = useCallback(async () => {
    try {
      setExperienceMode("learner");
      setLearnerGuideDismissed(false);
      if (!isDesktop) {
        const next = workspace ?? await chooseWorkspace();
        if (!next) return;
        if (!workspace) await activateWorkspace(next);
        await openFile("analysis.bl");
        showNotice("Tutorial project open — press Ctrl+Enter to run analysis.bl");
        return;
      }
      // Desktop has no packaged sample tree yet; reuse the trust-and-run starter
      // so demos still complete without hunting for a folder of examples.
      await openWelcomeExample(welcomeExamples[0], { run: true });
      showNotice("Starter tutorial running — Explorer shows your workspace files");
    } catch (error) {
      showNotice(String(error), "error");
    }
  }, [
    activateWorkspace,
    chooseWorkspace,
    openFile,
    openWelcomeExample,
    setExperienceMode,
    setLearnerGuideDismissed,
    showNotice,
    workspace,
  ]);

  /**
   * Follow an example-pack deep link: `?pack=rosalind-armory&problem=SUBO`.
   *
   * Browser only — on Desktop a pack belongs in a real folder on disk, not in
   * the in-memory demo workspace. Runs once on mount; the query string is left
   * in place so the link stays shareable and reloadable.
   */
  const openPackLink = useCallback(async (packId: string, problemId?: string) => {
    try {
      const catalog = await fetchPackIndex();
      const entry = catalog.find((candidate) => candidate.id === packId);
      if (!entry) {
        showNotice(`No example pack named "${packId}"`, "error");
        return;
      }

      // The shared sample data goes in whatever the link was for, so an
      // example that reads data/counts.csv works here the same as it does in
      // the playground. 61 KiB, installed once.
      await installDataFilesIntoWorkspace();

      // Already here at this version: open it rather than fetching the whole
      // pack again. Every one of these links opens its own tab, so without this
      // reading a second problem re-downloads all of them.
      let verified = true;
      let target: string | undefined;
      if ((await installedPackVersion(packId)) === entry.version) {
        const manifest = await installedPackManifest(packId);
        target = manifest && problemId
          ? problemPathFromManifest(packId, manifest, problemId)
          : undefined;
        const next = await chooseWorkspace();
        if (next) await activateWorkspace(next);
      } else {
        showNotice(`Downloading ${entry.name}…`);
        const fetched = await fetchPackBundle(entry);
        verified = fetched.verified;
        await installPackIntoWorkspace(fetched.bundle);

        const next = await chooseWorkspace();
        if (next) await activateWorkspace(next);

        target = problemId ? problemPath(fetched.bundle, problemId) : undefined;
      }
      if (problemId && !target) {
        // The pack is installed either way, but stop here: showing the summary
        // afterwards would replace this toast and hide the broken link.
        showNotice(`${entry.name} has no problem "${problemId}"`, "error");
        return;
      }
      if (target) await openFile(target);

      const { label } = packSummary(entry);
      // Say plainly when the download could not be checked rather than letting
      // an unverified pack look identical to a verified one.
      showNotice(verified ? `${entry.name}: ${label}` : `${entry.name}: ${label} (checksum not verified)`);
    } catch (error) {
      showNotice(String(error), "error");
    }
  }, [activateWorkspace, chooseWorkspace, openFile, showNotice]);

  const packLinkFollowed = useRef(false);

  useEffect(() => {
    if (isDesktop || packLinkFollowed.current) return;
    const { pack, problem } = parsePackLink(window.location.search);
    if (!pack) return;
    // StrictMode mounts twice in development; without this the pack downloads
    // twice on every page load.
    packLinkFollowed.current = true;
    void openPackLink(pack, problem);
    // Deliberately mount-only: re-running on callback identity would re-download
    // the pack on every render that changes one of them.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const openRecentWorkspace = useCallback(async (path: string) => {
    try {
      await activateWorkspace(await chooseRecentWorkspace(path));
    } catch (error) {
      showNotice(String(error));
    }
  }, [activateWorkspace, chooseRecentWorkspace, showNotice]);

  /**
   * Re-read every open, unmodified file from disk.
   *
   * A workspace replace edits files behind the editor's back. Without this the
   * open tabs would still show the old text and the next save would put it
   * straight back. Files with unsaved edits are left alone — silently throwing
   * away someone's work to win a race is worse than a stale tab.
   */
  const reloadOpenFiles = useCallback(async () => {
    const reloaded = await Promise.all(openFiles.map(async (file) => {
      if (file.untitled || file.viewer === "data" || isDirtyFile(file)) return file;
      try {
        const content = await bridge.readFile(file.path);
        return content === file.content ? file : { ...file, content, savedContent: content };
      } catch {
        return file;
      }
    }));
    setOpenFiles(reloaded);
  }, [openFiles]);

  /**
   * Paths whose on-disk text differs from the last saved buffer, and the disk
   * snapshot we have already decided to ignore for the current editor session.
   */
  const [diskChangedPaths, setDiskChangedPaths] = useState<Record<string, string>>({});
  const ignoredDiskContent = useRef<Record<string, string>>({});

  useEffect(() => {
    if (!workspace) {
      setDiskChangedPaths({});
      return;
    }
    let cancelled = false;
    const poll = async () => {
      const next: Record<string, string> = {};
      await Promise.all(openFiles.map(async (file) => {
        if (file.untitled || file.viewer === "data") return;
        try {
          const disk = await bridge.readFile(file.path);
          if (disk === file.savedContent) {
            delete ignoredDiskContent.current[file.path];
            return;
          }
          if (ignoredDiskContent.current[file.path] === disk) return;
          next[file.path] = disk;
        } catch {
          // Missing files are handled by save/open errors, not this banner.
        }
      }));
      if (!cancelled) setDiskChangedPaths(next);
    };
    void poll();
    const timer = window.setInterval(() => void poll(), 4_000);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [openFiles, workspace]);

  const reloadActiveFromDisk = useCallback(async () => {
    if (!activeFile || activeFile.untitled || activeFile.viewer === "data") return;
    try {
      const content = diskChangedPaths[activeFile.path] ?? await bridge.readFile(activeFile.path);
      delete ignoredDiskContent.current[activeFile.path];
      setOpenFiles((files) => files.map((file) =>
        file.path === activeFile.path ? { ...file, content, savedContent: content } : file));
      setDiskChangedPaths((current) => {
        const next = { ...current };
        delete next[activeFile.path];
        return next;
      });
      if (activeFile.path.endsWith(".bl")) await openDocument(activeFile.path, content);
      showNotice(`Reloaded ${activeFile.name} from disk`);
    } catch (error) {
      showNotice(String(error), "error");
    }
  }, [activeFile, diskChangedPaths, openDocument, showNotice]);

  const keepActiveDespiteDisk = useCallback(() => {
    if (!activeFile) return;
    const disk = diskChangedPaths[activeFile.path];
    if (disk != null) ignoredDiskContent.current[activeFile.path] = disk;
    setDiskChangedPaths((current) => {
      const next = { ...current };
      delete next[activeFile.path];
      return next;
    });
  }, [activeFile, diskChangedPaths]);

  const openSearchHit = useCallback(async (hit: SearchHit) => {
    pendingNavigation.current = { path: hit.path, line: hit.line, column: hit.column };
    await openFile(hit.path);
  }, [openFile]);

  useEffect(() => {
    const target = pendingNavigation.current;
    if (!target || target.path !== activePath) return;
    const frame = window.requestAnimationFrame(() => {
      const editor = editorRef.current;
      if (!editor) return;
      editor.setPosition({ lineNumber: target.line, column: target.column });
      editor.revealLineInCenter(target.line);
      editor.focus();
      pendingNavigation.current = undefined;
    });
    return () => window.cancelAnimationFrame(frame);
  }, [activePath, activeFile?.content]);

  const closeWorkspace = useCallback(async () => {
    if (openFiles.some(isDirtyFile)
      && !await confirmAction({
        title: "Close workspace?",
        message: "The workspace contains unsaved editor changes. Closing it will discard them.",
        confirmLabel: "Discard and close",
        danger: true,
      })) return;
    abortRemotePolling();
    await bridge.closeConsole().catch(() => undefined);
    await closeManagedWorkspace();
    stopLsp(openFiles);
    setOpenFiles([]);
    setActivePath(undefined);
    setProblems([]);
  }, [abortRemotePolling, closeManagedWorkspace, confirmAction, openFiles, stopLsp]);

  const refreshWorkspace = useCallback(async () => {
    await refreshManagedWorkspace();
  }, [refreshManagedWorkspace]);

  const importWorkspaceFiles = useCallback(async () => {
    try {
      const imported = await bridge.importFiles();
      if (!imported.length) return;
      await refreshWorkspace();
      showNotice(`Imported ${imported.length} file${imported.length === 1 ? "" : "s"} into data`);
      await openFile(imported[0]);
    } catch (error) {
      showNotice(String(error));
    }
  }, [openFile, refreshWorkspace, showNotice]);

  const importCodeSource = useCallback(async () => {
    if (!workspace) {
      showNotice("Open a workspace before importing code");
      return;
    }
    try {
      const imported = await bridge.importCode();
      if (imported) setCodeImport(imported);
    } catch (error) {
      showNotice(String(error));
    }
  }, [showNotice, workspace]);

  const importCodeFromUrl = useCallback(async (url: string) => {
    if (!workspace) {
      showNotice("Open a workspace before importing code");
      return;
    }
    try {
      const imported = await bridge.importCodeUrl(url);
      setImportUrlOpen(false);
      setCodeImport(imported);
    } catch (error) {
      showNotice(String(error));
    }
  }, [showNotice, workspace]);

  const saveCodeImport = useCallback(async (request: ImportSaveRequest) => {
    if (!codeImport) return;
    try {
      await bridge.createEntry(request.path, "file");
      await bridge.writeFile(request.path, request.content);
      await refreshWorkspace();
      setCodeImport(undefined);
      setActivity("explorer");
      await openFile(request.path);
      showNotice(request.validationCurrent && request.validation.valid
        ? `Imported and validated ${codeImport.sourceName}`
        : `Imported ${codeImport.sourceName} as a draft`);
    } catch (error) {
      showNotice(String(error));
    }
  }, [codeImport, openFile, refreshWorkspace, showNotice]);

  const exportDataPreview = useCallback(async (path: string, format: string) => {
    try {
      const destination = await bridge.exportPreview(path, format);
      if (destination) showNotice(`Exported preview to ${destination}`);
    } catch (error) {
      showNotice(String(error));
    }
  }, [showNotice]);

  const exportOutput = useCallback(async () => {
    if (!activeOutputJob?.log.length) {
      showNotice("There is no output to save");
      return;
    }
    try {
      const baseName = activeFile?.name.replace(/\.[^.]+$/, "") || "biolang";
      const option = activeOutputExportOptions.find(
        (candidate) => candidate.format === outputExportFormat,
      ) ?? activeOutputExportOptions[0];
      const content = buildOutputExport(
        activeOutputJob.log,
        option?.format ?? "log",
        activeOutputJob,
      );
      const destination = await bridge.exportText(
        `${baseName}-output.${option?.extension ?? "log"}`,
        content,
      );
      if (destination) showNotice(`Saved output to ${destination}`);
    } catch (error) {
      showNotice(String(error));
    }
  }, [
    activeFile?.name,
    activeOutputExportOptions,
    activeOutputJob,
    outputExportFormat,
    showNotice,
  ]);

  const selectOutputRun = useCallback((jobId: string) => {
    if (!activeFile || !jobId) return;
    setOutputRunByFile((current) => ({ ...current, [activeFile.path]: jobId }));
  }, [activeFile]);

  const exportRunBundle = useCallback(async () => {
    if (!activeOutputJob) return;
    try {
      const artifacts = new Map<string, Uint8Array>();
      const skipped: string[] = [];
      for (const artifact of activeOutputJob.artifacts ?? []) {
        try {
          artifacts.set(artifact.name, await readJobArtifact(activeOutputJob, artifact));
        } catch {
          skipped.push(artifact.name);
        }
      }
      const bundle = buildRunBundle(activeOutputJob, artifacts);
      const destination = await bridge.exportBinary(bundle.name, bundle.bytes);
      if (destination) showNotice(skipped.length
        ? `Saved run bundle to ${destination}; ${skipped.length} artifact${skipped.length === 1 ? "" : "s"} could not be included`
        : `Saved run bundle to ${destination}`);
    } catch (error) {
      showNotice(String(error));
    }
  }, [activeOutputJob, readJobArtifact, showNotice]);

  const deleteOutputRun = useCallback(async () => {
    if (!activeOutputJob) return;
    if (!await confirmAction({
      title: "Delete run history?",
      message: `Delete the recorded output for ${activeOutputJob.displayName ?? activeOutputJob.file}? This does not delete the source file.`,
      confirmLabel: "Delete run",
      danger: true,
    })) return;
    deleteJob(activeOutputJob.id);
  }, [activeOutputJob, confirmAction, deleteJob]);

  const openOutputDiagnostic = useCallback(async (path: string, line: number, column: number) => {
    const normalized = path.replaceAll("\\", "/");
    const workspaceRoot = workspace?.root.replaceAll("\\", "/").replace(/\/$/, "");
    const target = workspaceRoot && !/^(?:[A-Za-z]:\/|\/)/.test(normalized)
      ? `${workspaceRoot}/${normalized.replace(/^\.\//, "")}`
      : normalized;
    pendingNavigation.current = { path: target, line, column };
    try {
      await openFile(target);
    } catch (error) {
      showNotice(`Cannot open diagnostic source: ${String(error)}`);
    }
  }, [openFile, showNotice, workspace?.root]);

  const detachOutput = useCallback(async () => {
    if (!activeOutputJob) {
      showNotice("Run a script before detaching Output");
      return;
    }
    const query = `?detachedOutput=1&jobId=${encodeURIComponent(activeOutputJob.id)}`;
    if (!isDesktop) {
      window.open(`${window.location.origin}${window.location.pathname}${query}`, "_blank", "popup,width=980,height=720");
      return;
    }
    try {
      const { WebviewWindow } = await import("@tauri-apps/api/webviewWindow");
      const label = `output-${Date.now()}`;
      const detached = new WebviewWindow(label, {
        url: query,
        title: `BioLang Output - ${activeOutputJob.displayName ?? activeOutputJob.file}`,
        width: 980,
        height: 720,
        resizable: true,
      });
      detached.once("tauri://error", (event) => showNotice(`Cannot detach Output: ${String(event.payload)}`));
    } catch (error) {
      showNotice(`Cannot detach Output: ${String(error)}`);
    }
  }, [activeOutputJob, showNotice]);

  const moveOutput = useCallback((location: OutputLocation) => {
    setPanelMaximized(false);
    setOutputLocation(location);
    if (location === "bottom") {
      setVisibleBottomPanels((current) => current.includes("output") ? current : [...current, "output"]);
      setOutputRightVisible(false);
      setOutputEditorOpen(false);
      setOutputEditorActive(false);
      setBottomPanel("output");
      setBottomVisible(true);
      return;
    }
    if (bottomPanel === "output") {
      setBottomPanel("problems");
    }
    if (location === "right") {
      setOutputRightVisible(true);
      setOutputEditorOpen(false);
      setOutputEditorActive(false);
      return;
    }
    setOutputRightVisible(false);
    setOutputEditorOpen(true);
    setOutputEditorActive(true);
    setActivity("explorer");
  }, [
    bottomPanel,
    setBottomVisible,
    setOutputLocation,
    setOutputRightVisible,
    setVisibleBottomPanels,
  ]);

  const startOutputDockDrag = useCallback((event: React.PointerEvent<HTMLButtonElement>) => {
    if (event.button !== 0) return;
    event.preventDefault();
    outputDockCleanupRef.current?.();
    const handle = event.currentTarget;
    const pointerId = event.pointerId;
    const startX = event.clientX;
    const startY = event.clientY;
    let dragging = false;

    const locationAt = (x: number, y: number): OutputLocation | undefined => {
      const target = document.elementFromPoint(x, y)
        ?.closest<HTMLElement>("[data-output-dock-location]");
      const location = target?.dataset.outputDockLocation;
      return location === "bottom" || location === "right" || location === "editor"
        ? location
        : undefined;
    };
    const cleanup = () => {
      window.removeEventListener("pointermove", onMove, true);
      window.removeEventListener("pointerup", onUp, true);
      window.removeEventListener("pointercancel", onCancel, true);
      if (handle.hasPointerCapture(pointerId)) handle.releasePointerCapture(pointerId);
      outputDockCleanupRef.current = undefined;
      setOutputDragging(false);
      setOutputDragTarget(undefined);
    };
    const onMove = (moveEvent: PointerEvent) => {
      if (moveEvent.pointerId !== pointerId) return;
      if (!dragging && Math.hypot(moveEvent.clientX - startX, moveEvent.clientY - startY) < 6) return;
      dragging = true;
      setOutputDragging(true);
      setOutputDragTarget(locationAt(moveEvent.clientX, moveEvent.clientY));
    };
    const onUp = (upEvent: PointerEvent) => {
      if (upEvent.pointerId !== pointerId) return;
      const location = dragging ? locationAt(upEvent.clientX, upEvent.clientY) : undefined;
      cleanup();
      if (location) moveOutput(location);
    };
    const onCancel = (cancelEvent: PointerEvent) => {
      if (cancelEvent.pointerId === pointerId) cleanup();
    };

    outputDockCleanupRef.current = cleanup;
    try {
      handle.setPointerCapture(pointerId);
    } catch {
      // Window listeners still provide drag tracking when pointer capture is unavailable.
    }
    window.addEventListener("pointermove", onMove, true);
    window.addEventListener("pointerup", onUp, true);
    window.addEventListener("pointercancel", onCancel, true);
  }, [moveOutput]);

  useEffect(() => () => outputDockCleanupRef.current?.(), []);

  const closeOutput = useCallback(() => {
    if (outputLocation === "right") {
      setOutputRightVisible(false);
    } else if (outputLocation === "editor") {
      setOutputEditorOpen(false);
      setOutputEditorActive(false);
    } else {
      setBottomVisible(false);
    }
  }, [outputLocation, setBottomVisible, setOutputRightVisible]);

  const saveAll = useCallback(async () => {
    const dirty = openFiles.filter((file) => !file.untitled && isDirtyFile(file));
    const untitled = openFiles.find((file) => file.untitled);
    try {
      await Promise.all(dirty.map((file) => bridge.writeFile(file.path, file.content)));
      setOpenFiles((files) => files.map((file) =>
        dirty.some((candidate) => candidate.path === file.path)
          ? { ...file, savedContent: file.content }
          : file));
      if (dirty.length) showNotice(`Saved ${dirty.length} file${dirty.length === 1 ? "" : "s"}`);
      if (dirty.length) void refreshGitStatus();
      if (untitled) {
        setActivePath(untitled.path);
        promptUntitledSave(untitled);
      }
    } catch (error) {
      showNotice(String(error));
    }
  }, [openFiles, promptUntitledSave, refreshGitStatus, showNotice]);

  const submitEntryPrompt = useCallback(async () => {
    if (!entryPrompt || !workspace) return;
    const value = entryPrompt.value.trim();
    if (!value) return;
    try {
      if (entryPrompt.mode === "save") {
        const file = openFiles.find((candidate) => candidate.path === entryPrompt.path);
        if (!file?.untitled) throw new Error("The untitled editor is no longer open");
        const path = entryPrompt.parent ? `${entryPrompt.parent}/${value}` : value;
        await bridge.createEntry(path, "file");
        await bridge.writeFile(path, file.content);
        const viewer = viewerForPath(path, file.content.length);
        const saved: OpenFile = {
          ...file,
          path,
          name: path.split("/").pop() ?? path,
          savedContent: file.content,
          language: languageForPath(path),
          viewer: viewer === "data" ? "editor" : viewer,
          untitled: false,
          preferredDirectory: undefined,
        };
        closeDocument(file.path);
        setOpenFiles((files) => files.map((candidate) =>
          candidate.path === file.path ? saved : candidate));
        setActivePath(path);
        await openDocument(path, saved.content);
        showNotice(`Saved ${saved.name}`);
        void refreshGitStatus();
      } else if (entryPrompt.mode === "rename") {
        if (!entryPrompt.path) throw new Error("No file or folder selected for rename");
        const oldPath = entryPrompt.path;
        const affected = openFiles.filter((file) =>
          file.path === oldPath || file.path.startsWith(`${oldPath}/`));
        const nextPath = await bridge.renameEntry(oldPath, value);
        for (const file of affected) closeDocument(file.path);
        const renamed = affected.map((file) => {
          const path = file.path === oldPath ? nextPath : `${nextPath}${file.path.slice(oldPath.length)}`;
          return { ...file, path, name: path.split("/").pop() ?? path, language: languageForPath(path) };
        });
        setOpenFiles((files) => files.map((file) => {
          if (file.path !== oldPath && !file.path.startsWith(`${oldPath}/`)) return file;
          const path = file.path === oldPath ? nextPath : `${nextPath}${file.path.slice(oldPath.length)}`;
          return { ...file, path, name: path.split("/").pop() ?? path, language: languageForPath(path) };
        }));
        for (const file of renamed) await openDocument(file.path, file.content);
        if (activePath === oldPath || activePath?.startsWith(`${oldPath}/`)) {
          setActivePath(activePath === oldPath ? nextPath : `${nextPath}${activePath.slice(oldPath.length)}`);
        }
      } else {
        const path = entryPrompt.parent ? `${entryPrompt.parent}/${value}` : value;
        await bridge.createEntry(path, "directory");
      }
      setEntryPrompt(undefined);
      await refreshWorkspace();
    } catch (error) {
      showNotice(String(error));
    }
  }, [activePath, closeDocument, entryPrompt, openFiles, openDocument, refreshGitStatus, refreshWorkspace, showNotice, workspace]);

  const deleteWorkspaceEntry = useCallback(async (entry: FileEntry) => {
    if (!await confirmAction({
      title: `Delete ${entry.kind}?`,
      message: `Delete ${entry.name}${entry.kind === "directory" ? " and all of its contents" : ""}? This cannot be undone.`,
      confirmLabel: "Delete",
      danger: true,
    })) return;
    try {
      await bridge.deleteEntry(entry.path);
      const removed = openFiles.filter((file) => file.path === entry.path || file.path.startsWith(`${entry.path}/`));
      for (const file of removed) closeDocument(file.path);
      const nextFiles = openFiles.filter((file) => file.path !== entry.path && !file.path.startsWith(`${entry.path}/`));
      setOpenFiles(nextFiles);
      if (activePath === entry.path || activePath?.startsWith(`${entry.path}/`)) setActivePath(nextFiles.at(-1)?.path);
      setContextMenu(undefined);
      await refreshWorkspace();
    } catch (error) {
      showNotice(String(error));
    }
  }, [activePath, closeDocument, confirmAction, openFiles, refreshWorkspace, showNotice]);

  const repathOpenFiles = useCallback((from: string, to: string) => {
    setOpenFiles((files) => files.map((file) => {
      if (file.path === from) {
        return { ...file, path: to, name: to.split("/").pop() ?? to, language: languageForPath(to) };
      }
      if (file.path.startsWith(`${from}/`)) {
        const next = `${to}${file.path.slice(from.length)}`;
        return { ...file, path: next, name: next.split("/").pop() ?? next, language: languageForPath(next) };
      }
      return file;
    }));
    setActivePath((current) => {
      if (!current) return current;
      if (current === from) return to;
      if (current.startsWith(`${from}/`)) return `${to}${current.slice(from.length)}`;
      return current;
    });
  }, []);

  const moveWorkspaceEntry = useCallback(async (sourcePath: string, destinationDirectory: string) => {
    try {
      const nextPath = await bridge.moveEntry(sourcePath, destinationDirectory);
      repathOpenFiles(sourcePath, nextPath);
      await refreshWorkspace();
      void refreshGitStatus();
      showNotice(`Moved to ${nextPath}`);
    } catch (error) {
      showNotice(String(error), "error");
    }
  }, [refreshGitStatus, refreshWorkspace, repathOpenFiles, showNotice]);

  const importDroppedFiles = useCallback(async (destinationDirectory: string, files: FileList | File[]) => {
    const list = Array.from(files);
    if (!list.length) return;
    const targetDir = destinationDirectory || "data";
    const imported: string[] = [];
    try {
      for (const file of list) {
        const safeName = file.name.replace(/[\\/]/g, "_");
        let relative = targetDir ? `${targetDir}/${safeName}` : safeName;
        let attempt = 1;
        // Avoid clobbering existing names by suffixing before write.
        while (allFiles.some((entry) => entry.path === relative)) {
          attempt += 1;
          const dot = safeName.lastIndexOf(".");
          const stem = dot > 0 ? safeName.slice(0, dot) : safeName;
          const extension = dot > 0 ? safeName.slice(dot) : "";
          relative = targetDir
            ? `${targetDir}/${stem}-${attempt}${extension}`
            : `${stem}-${attempt}${extension}`;
        }
        const bytes = new Uint8Array(await file.arrayBuffer());
        imported.push(await bridge.writeNewFile(relative, bytes));
      }
      await refreshWorkspace();
      void refreshGitStatus();
      showNotice(`Imported ${imported.length} file${imported.length === 1 ? "" : "s"} into ${targetDir || "workspace root"}`);
      if (imported[0]) await openFile(imported[0]);
    } catch (error) {
      showNotice(String(error), "error");
    }
  }, [allFiles, openFile, refreshGitStatus, refreshWorkspace, showNotice]);

  const closeTabGroup = useCallback(async (path: string, mode: "others" | "all") => {
    const closing = mode === "all"
      ? openFiles
      : openFiles.filter((file) => file.path !== path);
    if (closing.some(isDirtyFile)
      && !await confirmAction({
        title: "Discard unsaved changes?",
        message: `Close ${closing.length} editor${closing.length === 1 ? "" : "s"} and discard unsaved changes?`,
        confirmLabel: "Discard and close",
        danger: true,
      })) return;
    const closingPaths = new Set(closing.map((file) => file.path));
    for (const file of closing) closeDocument(file.path);
    const remaining = openFiles.filter((file) => !closingPaths.has(file.path));
    setOpenFiles(remaining);
    if (activePath && closingPaths.has(activePath)) setActivePath(remaining.at(-1)?.path);
    setContextMenu(undefined);
  }, [activePath, closeDocument, confirmAction, openFiles]);

  const copyWorkspacePath = useCallback(async (path: string) => {
    try {
      await bridge.copyText(path);
      showNotice(`Copied ${path}`);
    } catch {
      showNotice("Clipboard access is unavailable");
    } finally {
      setContextMenu(undefined);
    }
  }, [showNotice]);

  const showContextMenu = useCallback((
    menu: ContextMenuTarget,
    x: number,
    y: number,
  ) => {
    const width = 205;
    const estimatedHeight = menu.kind === "explorer" && menu.entry.kind === "directory"
      ? 238
      : menu.kind === "workspace"
        ? 188
        : 205;
    setContextMenu({
      ...menu,
      x: Math.max(4, Math.min(x, window.innerWidth - width - 4)),
      y: Math.max(4, Math.min(y, window.innerHeight - estimatedHeight - 4)),
    } as ContextMenuState);
  }, []);

  const navigateContextMenu = (event: React.KeyboardEvent<HTMLDivElement>) => {
    if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) return;
    const items = [...event.currentTarget.querySelectorAll<HTMLButtonElement>('[role="menuitem"]:not(:disabled)')];
    if (!items.length) return;
    event.preventDefault();
    const current = items.indexOf(document.activeElement as HTMLButtonElement);
    if (event.key === "Home") items[0].focus();
    else if (event.key === "End") items.at(-1)?.focus();
    else {
      const delta = event.key === "ArrowDown" ? 1 : -1;
      items[(current + delta + items.length) % items.length].focus();
    }
  };

  const editorCommand = useCallback((command: string) => {
    editorRef.current?.trigger("menu", command, null);
    editorRef.current?.focus();
  }, []);

  const copyEditorSelection = useCallback(async () => {
    const editor = editorRef.current;
    const model = editor?.getModel();
    const selections = editor?.getSelections();
    if (!editor || !model || !selections?.length) return;
    const text = selections.map((selection) =>
      selection.isEmpty()
        ? model.getLineContent(selection.startLineNumber)
        : model.getValueInRange(selection)).join(model.getEOL());
    try {
      await bridge.copyText(text);
    } catch (error) {
      showNotice(String(error));
    } finally {
      editor.focus();
    }
  }, [showNotice]);

  const goToSymbol = useCallback((symbol: DocumentSymbolEntry) => {
    setPipelineView(false);
    pendingNavigation.current = {
      path: activePath ?? "",
      line: symbol.line,
      column: 1,
    };
    window.requestAnimationFrame(() => {
      const editor = editorRef.current;
      if (!editor) return;
      editor.setPosition({ lineNumber: symbol.line, column: 1 });
      editor.revealLineInCenter(symbol.line);
      editor.focus();
      pendingNavigation.current = undefined;
    });
  }, [activePath]);

  const startPaneResize = useCallback((
    pane: "sidebar" | "panel" | "inspector" | "output",
    event: React.PointerEvent<HTMLDivElement>,
  ) => {
    event.preventDefault();
    const startX = event.clientX;
    const startY = event.clientY;
    const startSidebar = sidebarWidth;
    const startPanel = bottomPanelHeight;
    const startOutputPanel = outputPanelWidth;
    const startInspector = inspectorWidth;
    document.body.classList.add("resizing-pane");
    const onMove = (moveEvent: PointerEvent) => {
      if (pane === "sidebar") {
        setSidebarWidth(Math.max(180, Math.min(520, startSidebar + moveEvent.clientX - startX)));
      } else if (pane === "inspector") {
        setInspectorWidth(Math.max(220, Math.min(620, startInspector + startX - moveEvent.clientX)));
      } else if (pane === "output") {
        if (window.innerWidth <= 800) {
          setBottomPanelHeight(Math.max(120, Math.min(window.innerHeight - 180, startPanel + startY - moveEvent.clientY)));
        } else {
          setOutputPanelWidth(Math.max(300, Math.min(window.innerWidth - 480, startOutputPanel + startX - moveEvent.clientX)));
        }
      } else {
        setPanelMaximized(false);
        setBottomPanelHeight(Math.max(120, Math.min(window.innerHeight - 180, startPanel + startY - moveEvent.clientY)));
      }
    };
    const onUp = () => {
      document.body.classList.remove("resizing-pane");
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
  }, [
    bottomPanelHeight,
    inspectorWidth,
    outputPanelWidth,
    setBottomPanelHeight,
    setInspectorWidth,
    setOutputPanelWidth,
    setSidebarWidth,
    sidebarWidth,
  ]);

  const updateContentFor = useCallback((path: string, content = "") => {
    setOpenFiles((files) =>
      files.map((file) => (file.path === path ? { ...file, content } : file)),
    );
    if (path.endsWith(".bl")) queueLspChange(path, content);
  }, [queueLspChange]);

  const updateContent = useCallback((content = "") => {
    if (!focusedPath) return;
    updateContentFor(focusedPath, content);
  }, [focusedPath, updateContentFor]);

  /**
   * Push the selection — or the line the cursor is on — to the console.
   *
   * Everyone arriving from RStudio or a Jupyter notebook reaches for this within
   * the first minute, and without it the only way to try one line was to comment
   * out the rest of the file or retype it into the console by hand.
   */
  const sendSelectionToConsole = useCallback(() => {
    const editor = editorRef.current;
    const model = editor?.getModel();
    const selection = editor?.getSelection();
    if (!editor || !model || !selection) return;
    if (!workspace) {
      showNotice("Open a workspace before sending code to the BioLang Console");
      return;
    }
    if (!workspaceTrusted) {
      showNotice("Trust this workspace before sending code to the BioLang Console");
      return;
    }

    const source = selection.isEmpty()
      ? model.getLineContent(selection.startLineNumber)
      : model.getValueInRange(selection);
    if (!source.trim()) return;

    setBottomPanel("console");
    setBottomVisible(true);
    setConsoleSubmission({ id: Date.now(), source });

    // Stepping through a script line by line is the whole point, so an empty
    // selection advances past the line it just sent.
    if (selection.isEmpty() && selection.startLineNumber < model.getLineCount()) {
      const next = selection.startLineNumber + 1;
      editor.setPosition({ lineNumber: next, column: model.getLineMaxColumn(next) });
      editor.revealLineInCenterIfOutsideViewport(next);
    }
  }, [editorRef, showNotice, workspace, workspaceTrusted]);

  /**
   * List every use of the symbol under the cursor in the Search sidebar.
   *
   * Monaco's standalone build registers the reference provider plumbing but
   * ships no peek widget, so `Find All References` there is a command that
   * quietly does nothing. A result list in the sidebar is both visible and more
   * useful: it survives navigation instead of closing the moment you click.
   */
  const findReferences = useCallback(() => {
    const editor = editorRef.current;
    const model = editor?.getModel();
    const position = editor?.getPosition();
    if (!editor || !model || !position || !activeFile) return;
    const word = model.getWordAtPosition(position)?.word;
    if (!word) {
      showNotice("Put the cursor on a symbol to find its references");
      return;
    }
    const lines = Array.from(
      { length: model.getLineCount() },
      (_, index) => model.getLineContent(index + 1),
    );
    const hits = findOccurrences(lines, word).map((found) => ({
      path: activeFile.path,
      line: found.line,
      column: found.column,
      preview: found.preview,
    }));
    if (!hits.length) {
      showNotice(`No references to ${word}`);
      return;
    }
    setReferenceResults({ name: word, hits });
    setActivity("search");
  }, [activeFile, editorRef, showNotice]);

  /**
   * Annotate each line whose `print`/`println` produced a value with what it
   * produced, as dim text after the code.
   *
   * Reading a run meant scrolling a linear log and matching each line of output
   * back to the statement that wrote it by eye. Putting the value beside the
   * code is the notebook affordance without leaving a plain `.bl` file.
   */
  const inlineTrace = activeFile && !activeFile.untitled
    && outputRunByFile[activeFile.path] === undefined
    ? latestJobForFile(jobs, activeFile.path)?.trace
    : jobs.find((job) => job.id === outputRunByFile[activeFile?.path ?? ""])?.trace;

  useEffect(() => {
    const editor = editorRef.current;
    const model = editor?.getModel();
    if (!editor || !model) return;
    if (!inlineTrace?.length || !showInlineResults) {
      const cleared = editor.createDecorationsCollection([]);
      return () => cleared.clear();
    }

    // A line printed inside a loop yields one entry per iteration; the last one
    // is the state the run finished in, which is what the annotation should say.
    const byLine = new Map<number, string>();
    for (const entry of inlineTrace) byLine.set(entry.line, entry.text);

    const collection = editor.createDecorationsCollection(
      [...byLine.entries()]
        .filter(([line]) => line >= 1 && line <= model.getLineCount())
        .map(([line, text]) => ({
          range: {
            startLineNumber: line,
            startColumn: model.getLineMaxColumn(line),
            endLineNumber: line,
            endColumn: model.getLineMaxColumn(line),
          },
          options: {
            after: { content: `  ${text}`, inlineClassName: "inline-run-result" },
            showIfCollapsed: true,
          },
        })),
    );
    return () => collection.clear();
  }, [editorRef, inlineTrace, showInlineResults]);

  /** Evaluate a pipeline stage in the console, from a CodeLens click. */
  const runStageSource = useCallback((source: string, label: string) => {
    if (!workspace) {
      showNotice("Open a workspace before running a stage");
      return;
    }
    if (!workspaceTrusted) {
      showNotice("Trust this workspace before running a stage");
      return;
    }
    setBottomPanel("console");
    setBottomVisible(true);
    setConsoleSubmission({ id: Date.now(), source });
    showNotice(`Running ${label}`);
  }, [showNotice, workspace, workspaceTrusted]);

  useEffect(() => {
    setStageRunner(runStageSource);
    return () => setStageRunner(undefined);
  }, [runStageSource]);

  useEffect(() => {
    // Capture phase, and on its own: Monaco binds Shift+F12 to its peek action
    // and calls stopPropagation, so a listener on the bubble path never runs.
    const onKeyDownCapture = (event: KeyboardEvent) => {
      if (event.key !== "F12" || !event.shiftKey || event.ctrlKey || event.altKey) return;
      event.preventDefault();
      event.stopPropagation();
      findReferences();
    };
    window.addEventListener("keydown", onKeyDownCapture, true);
    return () => window.removeEventListener("keydown", onKeyDownCapture, true);
  }, [findReferences]);

  /**
   * Credentials the selected API needs that are not available.
   *
   * Keyed off the function-name prefix, which is how the API browser already
   * groups these — `clinvar_*` and `geo_*` both go through NCBI E-utilities.
   */
  const apiCredentialNotices = useMemo(() => {
    const name = selectedApi?.name ?? "";
    const service = name.split("_")[0];
    const byPrefix = credentialsForService(service);
    const nested = ["clinvar", "geo", "ncbi"].includes(service)
      ? credentialsForService("ncbi")
      : [];
    const relevant = [...new Set([...byPrefix, ...nested])];
    return relevant.filter((credential) => isMissing(credential, credentialStatuses));
  }, [credentialStatuses, selectedApi]);

  const stageFiles = useCallback(async (paths: string[]) => {
    try {
      await bridge.gitStage(paths);
      await refreshGitStatus();
    } catch (error) {
      showNotice(String(error));
    }
  }, [refreshGitStatus, showNotice]);

  const unstageFiles = useCallback(async (paths: string[]) => {
    try {
      await bridge.gitUnstage(paths);
      await refreshGitStatus();
    } catch (error) {
      showNotice(String(error));
    }
  }, [refreshGitStatus, showNotice]);

  const commitStaged = useCallback(async () => {
    try {
      const output = await bridge.gitCommit(commitMessage);
      // Git prints the short stat; its first line names the commit.
      showNotice(output.split("\n").find((line) => line.trim()) ?? "Committed");
      setCommitMessage("");
      await refreshGitStatus();
    } catch (error) {
      showNotice(String(error));
    }
  }, [commitMessage, refreshGitStatus, showNotice]);

  /**
   * Show a file's diff in an editor tab.
   *
   * A read-only untitled buffer rather than a bespoke diff viewer: Monaco
   * already colours unified diffs, and the tab participates in the normal
   * editor lifecycle so it can be closed and reopened like anything else.
   */
  const openGitDiff = useCallback(async (path: string, staged: boolean) => {
    try {
      const diff = await bridge.gitDiff(path, staged);
      if (!diff.trim()) {
        showNotice(`No ${staged ? "staged " : ""}changes in ${path}`);
        return;
      }
      const name = `${path.split("/").pop()} (${staged ? "staged" : "working tree"}).diff`;
      const virtualPath = `__untitled__/${name}`;
      setOpenFiles((files) => {
        const existing = files.find((file) => file.path === virtualPath);
        const next: OpenFile = {
          path: virtualPath,
          name,
          content: diff,
          savedContent: diff,
          language: "plaintext",
          untitled: true,
        };
        return existing
          ? files.map((file) => file.path === virtualPath ? next : file)
          : [...files, next];
      });
      setActivePath(virtualPath);
      setOutputEditorActive(false);
    } catch (error) {
      showNotice(String(error));
    }
  }, [showNotice]);

  /**
   * Put back the parts of a recorded run that can be put back.
   *
   * Package pins and the script snapshot are restorable. Input data and the
   * interpreter version are not, and the report says so rather than leaving
   * someone believing the environment matches when it does not.
   */
  const restoreRunEnvironment = useCallback(async (
    provenance: JobProvenance,
    restoreSource: boolean,
  ) => {
    try {
      const outcome = await bridge.restoreRunEnvironment(provenance, restoreSource);
      showNotice(outcome);
      await reloadOpenFiles();
      return outcome;
    } catch (error) {
      showNotice(String(error));
      throw error;
    }
  }, [reloadOpenFiles, showNotice]);



  /**
   * Package the student's work for hand-in.
   *
   * There is no server, so a submission is a file: their source, the manifest,
   * the check results, and provenance proving it ran on their machine. Whatever
   * the course already uses to collect work will take a zip.
   */
  const exportSubmission = useCallback(async () => {
    if (!assignment) return;
    const sources = openFiles.filter((file) => file.path.endsWith(".bl") && !file.untitled);
    const summary = assignmentProgress
      .map((task) => `${task.passed ? "PASS" : task.missing ? "SKIP" : "FAIL"}  ${task.name}`)
      .join("\n");
    const entries = [
      {
        name: "SUBMISSION.txt",
        content: [
          `Assignment: ${assignment.title}`,
          `Exported: ${new Date().toISOString()}`,
          `Complete: ${isComplete(assignmentProgress) ? "yes" : "no"}`,
          "",
          summary,
        ].join("\n"),
      },
      { name: ASSIGNMENT_MANIFEST, content: assignmentSource ?? "" },
      ...sources.map((file) => ({ name: `source/${file.path}`, content: file.content })),
    ];
    try {
      const bytes = createZip(entries);
      const destination = await bridge.exportBinary(
        `submission-${assignment.title.replace(/\W+/g, "-").toLowerCase()}.zip`,
        bytes,
        "application/zip",
      );
      if (destination) showNotice(`Saved your submission to ${destination}`);
    } catch (error) {
      showNotice(String(error));
    }
  }, [assignment, assignmentProgress, assignmentSource, openFiles, showNotice]);

  const runWorkspaceTests = useCallback(async (path?: string) => {
    setTestRun({ status: "running", results: [], passed: 0, failed: 0 });
    setVisibleBottomPanels((panels) => panels.includes("tests") ? panels : [...panels, "tests"]);
    setBottomPanel("tests");
    setBottomVisible(true);
    try {
      const summary = await bridge.runWorkspaceTests(path);
      setTestRun({ status: "finished", ...summary });
    } catch (error) {
      setTestRun({
        status: "failed",
        results: [],
        passed: 0,
        failed: 0,
        error: String(error),
      });
    }
  }, [setBottomVisible, setVisibleBottomPanels]);

  const refreshReferenceBuilds = useCallback(() => {
    void bridge.listReferenceBuilds()
      .then(setReferenceBuilds)
      .catch(() => setReferenceBuilds([]));
  }, []);

  useEffect(refreshReferenceBuilds, [refreshReferenceBuilds]);

  const saveReferenceBuild = useCallback(async (name: string, assets: Record<string, string>) => {
    try {
      await bridge.saveReferenceBuild(name, assets);
      showNotice(`Reference build ${name} saved`);
      refreshReferenceBuilds();
    } catch (error) {
      showNotice(String(error));
    }
  }, [refreshReferenceBuilds, showNotice]);

  const deleteReferenceBuild = useCallback(async (name: string) => {
    try {
      await bridge.deleteReferenceBuild(name);
      refreshReferenceBuilds();
    } catch (error) {
      showNotice(String(error));
    }
  }, [refreshReferenceBuilds, showNotice]);

  const refreshCredentials = useCallback(() => {
    void bridge.listCredentials()
      .then(setCredentialStatuses)
      .catch(() => setCredentialStatuses([]));
  }, []);

  useEffect(refreshCredentials, [refreshCredentials]);

  const saveCredentialValue = useCallback(async (name: string, value: string) => {
    try {
      await bridge.setCredential(name, value);
      // Already-running processes captured the old environment, so a key saved
      // now only reaches the next run. Saying so beats a silent 401.
      showNotice(`${name} saved. Restart the console or rerun to use it.`);
      refreshCredentials();
    } catch (error) {
      showNotice(String(error));
    }
  }, [refreshCredentials, showNotice]);

  const forgetCredentialValue = useCallback(async (name: string) => {
    try {
      await bridge.deleteCredential(name);
      showNotice(`${name} removed`);
      refreshCredentials();
    } catch (error) {
      showNotice(String(error));
    }
  }, [refreshCredentials, showNotice]);

  const openPalette = useCallback((prefix: string) => {
    setPaletteSearch(prefix);
    setPaletteIndex(0);
    setPaletteOpen(true);
  }, []);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      if (target?.closest(".keybinding-capture")) return;
      // Keep ordinary typing in form fields; still allow workbench chords and F1.
      // Monaco uses a textarea.inputarea inside .monaco-editor — those must pass.
      if (
        target?.closest("input, textarea, select")
        && !target.closest(".monaco-editor")
        && !(event.ctrlKey || event.metaKey || event.altKey || event.key === "F1" || event.key === "Escape")
      ) {
        return;
      }

      if (event.key === "Escape") {
        outputDockCleanupRef.current?.();
        setPanelMaximized(false);
        setPaletteOpen(false);
        setOpenMenu(undefined);
        setContextMenu(undefined);
        setEntryPrompt(undefined);
        setSettingsOpen(false);
        setAboutOpen(false);
        setShortcutsOpen(false);
        setCodeImport(undefined);
        setImportUrlOpen(false);
        settleConfirmation(false);
        return;
      }

      const command = commandForEvent(event, keybindings);
      if (!command) return;

      // Run-in-console must not steal Ctrl+Enter from the console itself.
      if (command.id === "run" && (event.target as HTMLElement | null)?.closest(".console-pane")) {
        return;
      }

      event.preventDefault();
      switch (command.id) {
        case "help":
          setHelpSection("all");
          setActivity("help");
          break;
        case "saveAs":
          void saveActiveAs();
          break;
        case "save":
          void saveActive();
          break;
        case "newFile":
          if (workspace) newUntitledFile();
          break;
        case "closeEditor":
          if (activePath) void closeFile(activePath);
          break;
        case "run":
          void handleRunClick();
          break;
        case "sendToConsole":
          sendSelectionToConsole();
          break;
        case "commandPalette":
          openPalette(">");
          break;
        case "goToFile":
          openPalette("");
          break;
        case "togglePanel":
          setBottomVisible((visible) => !visible);
          break;
        case "explorer":
          setActivity("explorer");
          break;
        case "search":
          setActivity("search");
          break;
        case "scm":
          setActivity("scm");
          break;
        case "runTests":
          void runWorkspaceTests();
          break;
        case "goToSymbol":
          openPalette("@");
          break;
        case "settings":
          openSettings();
          break;
        case "splitEditor":
          openSplitRight();
          break;
        case "wordWrap":
          setWordWrap((value) => !value);
          break;
        case "console":
          if (!workspace) showNotice("Open a workspace before starting the BioLang Console");
          else if (!workspaceTrusted) showNotice("Trust this workspace before starting the BioLang Console");
          else {
            setBottomPanel("console");
            setBottomVisible(true);
          }
          break;
        case "terminal":
          if (!workspace) showNotice("Open a workspace before starting a terminal");
          else if (!workspaceTrusted) showNotice("Trust this workspace before starting a terminal");
          else {
            setTerminalMounted(true);
            setBottomPanel("terminal");
            setBottomVisible(true);
          }
          break;
        default:
          break;
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [
    activePath,
    closeFile,
    handleRunClick,
    keybindings,
    newUntitledFile,
    openPalette,
    openSettings,
    openSplitRight,
    runWorkspaceTests,
    sendSelectionToConsole,
    saveActive,
    saveActiveAs,
    settleConfirmation,
    showNotice,
    workspace,
    workspaceTrusted,
  ]);

  const insertApiExample = () => {
    if (!activeFile?.path.endsWith(".bl") || !selectedApi) return;
    const editor = editorRef.current;
    if (!editor) return;
    const selection = editor.getSelection();
    if (!selection) return;
    editor.executeEdits("api-browser", [{ range: selection, text: selectedApi.example, forceMoveMarkers: true }]);
    editor.focus();
  };

  const testApiExample = useCallback(async () => {
    if (!selectedApi || runningJob) return;
    if (!workspace) {
      showNotice("Open a workspace before testing an external API");
      return;
    }
    if (!workspaceTrusted) {
      showNotice("Trust this workspace before testing an external API");
      return;
    }
    const expression = selectedApi.example.replace(/\s+#.*$/, "").trim();
    if (!expression) {
      showNotice(`No runnable example is available for ${selectedApi.name}`);
      return;
    }
    const content = `# ${externalApiProvider(selectedApi.name)} API test\n\n`
      + "```biolang\n"
      + `let response = ${expression}\n`
      + "println(json_pretty(json_stringify(response)))\n"
      + "```\n";
    const file: OpenFile = {
      path: `API test/${selectedApi.name}`,
      name: `${selectedApi.name} API test`,
      content,
      savedContent: content,
      language: "markdown",
      viewer: "notebook",
    };
    const execution = executeFile(file, executionTarget);
    setVisibleBottomPanels((panels) => panels.includes("jobs") ? panels : [...panels, "jobs"]);
    setBottomPanel("jobs");
    setBottomVisible(true);
    await execution;
  }, [
    executeFile,
    executionTarget,
    runningJob,
    selectedApi,
    setBottomVisible,
    setVisibleBottomPanels,
    showNotice,
    workspace,
    workspaceTrusted,
  ]);

  const openHelp = (section: HelpKind | "all" = "all") => {
    setOutputEditorActive(false);
    if (bottomPanel === "output" && !activeOutputJob) {
      setPanelMaximized(false);
      setBottomVisible(false);
    }
    setHelpSearch("");
    setHelpSection(section);
    setSelectedHelpId(undefined);
    setActivity("help");
  };

  const navigateHelpLink = useCallback((href: string) => {
    if (/^https?:\/\//i.test(href)) {
      void bridge.openExternal(href).catch((error) => showNotice(String(error)));
      return;
    }
    if (!selectedHelp?.sourcePath) {
      showNotice("This offline help link has no source location");
      return;
    }
    const target = resolveHelpPath(selectedHelp.sourcePath, href);
    const entry = helpIndex?.entries.find((candidate) => candidate.sourcePath === target.path);
    if (!entry) {
      if (href.startsWith("#")) {
        document.getElementById(`help-${target.hash}`)?.scrollIntoView({ block: "start" });
      } else {
        showNotice("The linked document is not included in offline Help");
      }
      return;
    }
    setHelpSearch("");
    setHelpSection(entry.kind);
    setSelectedHelpId(entry.id);
    if (target.hash) {
      window.setTimeout(() => {
        document.getElementById(`help-${target.hash}`)?.scrollIntoView({ block: "start" });
      });
    }
  }, [helpIndex?.entries, selectedHelp?.sourcePath, showNotice]);

  const insertHelpExample = (text: string) => {
    if (!activeFile?.path.endsWith(".bl")) return;
    const separator = activeFile.content.endsWith("\n") ? "\n" : "\n\n";
    const nextContent = `${activeFile.content}${separator}${text.trim()}\n`;
    setOpenFiles((files) => files.map((file) => file.path === activeFile.path ? { ...file, content: nextContent } : file));
    queueLspChange(activeFile.path, nextContent, true);
    setActivity("explorer");
    showNotice(`Added help example to ${activeFile.name}`);
  };

  const openHelpSource = (path: string) => {
    if (!allFiles.some((entry) => entry.path === path)) return;
    setActivity("explorer");
    void openFile(path);
  };

  const installPackages = async () => {
    if (!workspaceTrusted) {
      showNotice("Trust this workspace before installing packages");
      return;
    }
    setPackageBusy(true);
    setBottomVisible(true);
    try {
      const result = await bridge.installPackages();
      recordDesktopTask("Package installation", result, "succeeded");
      setPackages(await bridge.packages());
    } catch (error) {
      recordDesktopTask("Package installation", `${String(error)}\n`, "failed");
    } finally {
      setPackageBusy(false);
    }
  };

  const setBottomPanelTabVisible = (panel: BottomPanel, visible: boolean) => {
    const next = visible
      ? visibleBottomPanels.includes(panel) ? visibleBottomPanels : [...visibleBottomPanels, panel]
      : visibleBottomPanels.filter((candidate) => candidate !== panel);
    setVisibleBottomPanels(next);
    if (visible || bottomPanel !== panel) return;
    const nextPanel = bottomPanelOrder.find(
      (candidate) => next.includes(candidate) && (candidate !== "output" || outputLocation === "bottom"),
    );
    if (nextPanel) {
      setBottomPanel(nextPanel);
    } else {
      setPanelMaximized(false);
      setBottomVisible(false);
    }
  };

  const openPanel = (panel: BottomPanel) => {
    if (panel === "output") {
      moveOutput(outputLocation);
      return;
    }
    if (panel === "terminal" && !isDesktop) {
      showNotice("A native terminal is available in BioLang Desktop; use the BioLang Console for browser WASM");
      return;
    }
    if (panel === "terminal" || panel === "console") {
      if (!workspace) {
        showNotice(`Open a workspace before starting the ${panel === "console" ? "BioLang Console" : "terminal"}`);
        return;
      }
      if (!workspaceTrusted) {
        showNotice(`Trust this workspace before starting the ${panel === "console" ? "BioLang Console" : "terminal"}`);
        return;
      }
    }
    setBottomPanelTabVisible(panel, true);
    if (panel === "terminal") setTerminalMounted(true);
    setBottomPanel(panel);
    setBottomVisible(true);
  };

  const menuModels: Record<string, MenuItem[]> = {
    File: [
      { label: "New File", shortcut: shortcutLabel("newFile"), disabled: !workspace, action: () => newUntitledFile() },
      { label: "New Folder...", disabled: !workspace, action: () => setEntryPrompt({ mode: "directory", value: "" }) },
      { separator: true },
      { label: "Open Folder...", shortcut: "Ctrl+K Ctrl+O", action: selectWorkspace },
      { label: "Close Folder", disabled: !workspace, action: closeWorkspace },
      { separator: true },
      { label: "Import Script from File...", disabled: !workspace, action: importCodeSource },
      { label: "Import Script from URL...", disabled: !workspace, action: () => setImportUrlOpen(true) },
      { label: "Import Data...", disabled: !workspace, action: importWorkspaceFiles },
      { separator: true },
      { label: "Save", shortcut: shortcutLabel("save"), disabled: !activeFile || !isDirtyFile(activeFile), action: saveActive },
      { label: "Save As...", shortcut: shortcutLabel("saveAs"), disabled: !activeFile || activeFile.viewer === "data", action: saveActiveAs },
      { label: "Save All", shortcut: "Ctrl+K S", disabled: !openFiles.some(isDirtyFile), action: saveAll },
      { separator: true },
      { label: "Close Editor", shortcut: shortcutLabel("closeEditor"), disabled: !activePath, action: () => activePath && closeFile(activePath) },
    ],
    Edit: [
      { label: "Undo", shortcut: "Ctrl+Z", disabled: !activeFile, action: () => editorCommand("undo") },
      { label: "Redo", shortcut: "Ctrl+Y", disabled: !activeFile, action: () => editorCommand("redo") },
      { separator: true },
      { label: "Cut", shortcut: "Ctrl+X", disabled: !activeFile, action: () => editorCommand("editor.action.clipboardCutAction") },
      { label: "Copy", shortcut: "Ctrl+C", disabled: !activeFile, action: copyEditorSelection },
      { label: "Paste", shortcut: "Ctrl+V", disabled: !activeFile, action: () => editorCommand("editor.action.clipboardPasteAction") },
      { separator: true },
      { label: "Find", shortcut: "Ctrl+F", disabled: !activeFile, action: () => editorCommand("actions.find") },
      { label: "Replace", shortcut: "Ctrl+H", disabled: !activeFile, action: () => editorCommand("editor.action.startFindReplaceAction") },
      { label: "Go to Line...", shortcut: "Ctrl+G", disabled: !activeFile, action: () => editorCommand("editor.action.gotoLine") },
      { separator: true },
      { label: "Format Document", shortcut: "Shift+Alt+F", disabled: !activeFile?.path.endsWith(".bl"), action: () => editorCommand("editor.action.formatDocument") },
      { label: "Format on Save", checked: formatOnSave, action: () => setFormatOnSave((value) => !value) },
      { label: "Inline Run Results", checked: showInlineResults, action: () => setShowInlineResults((value) => !value) },
      { separator: true },
      { label: "Rename Symbol", shortcut: "F2", disabled: !activeFile?.path.endsWith(".bl"), action: () => editorCommand("editor.action.rename") },
      { label: "Find All References", shortcut: "Shift+F12", disabled: !activeFile?.path.endsWith(".bl"), action: findReferences },
      { label: "Quick Fix...", shortcut: "Ctrl+.", disabled: !activeFile?.path.endsWith(".bl"), action: () => editorCommand("editor.action.quickFix") },
    ],
    Selection: [
      { label: "Select All", shortcut: "Ctrl+A", disabled: !activeFile, action: () => editorCommand("editor.action.selectAll") },
      { label: "Expand Selection", shortcut: "Shift+Alt+Right", disabled: !activeFile, action: () => editorCommand("editor.action.smartSelect.expand") },
      { label: "Shrink Selection", shortcut: "Shift+Alt+Left", disabled: !activeFile, action: () => editorCommand("editor.action.smartSelect.shrink") },
      { separator: true },
      { label: "Add Cursor Above", shortcut: "Ctrl+Alt+Up", disabled: !activeFile, action: () => editorCommand("editor.action.insertCursorAbove") },
      { label: "Add Cursor Below", shortcut: "Ctrl+Alt+Down", disabled: !activeFile, action: () => editorCommand("editor.action.insertCursorBelow") },
    ],
    View: [
      { label: "Command Palette...", shortcut: shortcutLabel("commandPalette"), action: () => openPalette(">") },
      { label: "Go to File...", shortcut: shortcutLabel("goToFile"), action: () => openPalette("") },
      { label: "Go to Symbol...", shortcut: shortcutLabel("goToSymbol"), disabled: !activeFile, action: () => openPalette("@") },
      { separator: true },
      { label: "Split Editor Right", shortcut: shortcutLabel("splitEditor"), disabled: !activePath, action: () => openSplitRight() },
      { label: "Close Split", disabled: !splitOpen, action: closeSplit },
      { separator: true },
      { label: "Explorer", shortcut: shortcutLabel("explorer"), checked: activity === "explorer", action: () => setActivity("explorer") },
      { label: "Expand All Explorer Folders", disabled: !workspace, action: expandAllTreeDirectories },
      { label: "Collapse All Explorer Folders", disabled: !workspace, action: collapseAllTreeDirectories },
      { label: "Search", shortcut: shortcutLabel("search"), checked: activity === "search", action: () => setActivity("search") },
      { label: "Source Control", shortcut: shortcutLabel("scm"), checked: activity === "scm", action: () => setActivity("scm") },
      { label: "Packages", checked: activity === "packages", action: () => setActivity("packages") },
      { label: "Bio APIs", checked: activity === "apis", action: () => setActivity("apis") },
      { label: "Jobs", checked: activity === "jobs", action: () => setActivity("jobs") },
      { label: "Help Center", shortcut: shortcutLabel("help"), checked: activity === "help", action: () => openHelp() },
      { separator: true },
      { label: "Bottom Panel", shortcut: shortcutLabel("togglePanel"), checked: bottomVisible, action: () => setBottomVisible((visible) => !visible) },
      { label: "Problems Tab", checked: visibleBottomPanelSet.has("problems"), action: () => setBottomPanelTabVisible("problems", !visibleBottomPanelSet.has("problems")) },
      { label: "Output Tab", checked: visibleBottomPanelSet.has("output"), action: () => setBottomPanelTabVisible("output", !visibleBottomPanelSet.has("output")) },
      { label: "Tests Tab", checked: visibleBottomPanelSet.has("tests"), action: () => setBottomPanelTabVisible("tests", !visibleBottomPanelSet.has("tests")) },
      { label: "Console Tab", checked: visibleBottomPanelSet.has("console"), action: () => setBottomPanelTabVisible("console", !visibleBottomPanelSet.has("console")) },
      { label: "Terminal Tab", checked: visibleBottomPanelSet.has("terminal"), action: () => setBottomPanelTabVisible("terminal", !visibleBottomPanelSet.has("terminal")) },
      { label: "Jobs Tab", checked: visibleBottomPanelSet.has("jobs"), action: () => setBottomPanelTabVisible("jobs", !visibleBottomPanelSet.has("jobs")) },
      { separator: true },
      { label: "Output at Bottom", checked: outputLocation === "bottom", action: () => moveOutput("bottom") },
      { label: "Output at Right", checked: outputLocation === "right", action: () => moveOutput("right") },
      { label: "Output in Editor", checked: outputLocation === "editor", action: () => moveOutput("editor") },
      { label: "Word Wrap", shortcut: shortcutLabel("wordWrap"), checked: wordWrap, action: () => setWordWrap((value) => !value) },
      { label: "Minimap", checked: minimap, action: () => setMinimap((value) => !value) },
      { separator: true },
      { label: "Learner Mode", checked: experienceMode === "learner", action: () => setExperienceMode("learner") },
      { label: "Show Getting Started Guide", disabled: experienceMode !== "learner", action: () => setLearnerGuideDismissed(false) },
      { label: "Expert Mode", checked: experienceMode === "expert", action: () => setExperienceMode("expert") },
      { label: "Settings", shortcut: shortcutLabel("settings"), action: () => openSettings() },
    ],
    Run: [
      { label: "Run Active File", shortcut: shortcutLabel("run"), disabled: !isRunnableFile(activeFile) || Boolean(runningJob), action: handleRunClick },
      { label: "Run Tests", shortcut: shortcutLabel("runTests"), disabled: !isDesktop || !workspaceTrusted, action: () => void runWorkspaceTests() },
      { label: "Send Selection to Console", shortcut: shortcutLabel("sendToConsole"), disabled: !workspaceTrusted || !activeFile?.path.endsWith(".bl"), action: sendSelectionToConsole },
      { label: "Stop", shortcut: "Shift+F5", disabled: !runningJob, action: stopActive },
      { separator: true },
      { label: "Show Output", action: () => openPanel("output") },
      { label: "Show Jobs", action: () => openPanel("jobs") },
      { label: "BioLang Console", shortcut: shortcutLabel("console"), disabled: !workspace || !workspaceTrusted, action: () => openPanel("console") },
      { label: "New Terminal", shortcut: shortcutLabel("terminal"), disabled: !isDesktop || !workspace || !workspaceTrusted, action: () => openPanel("terminal") },
    ],
    Help: [
      { label: "Help Center", shortcut: shortcutLabel("help"), action: () => openHelp() },
      { label: "Keyboard Shortcuts", action: () => setShortcutsOpen(true) },
      { separator: true },
      { label: "Language Guide", action: () => openHelp("language") },
      { label: "Builtin Reference", action: () => openHelp("builtin") },
      { label: "Tutorials", action: () => openHelp("tutorial") },
      { label: "Examples", action: () => openHelp("example") },
      { label: "Biological APIs", action: () => setActivity("apis") },
      { separator: true },
      { label: `About ${productName}`, action: () => setAboutOpen(true) },
    ],
  };

  const paletteCommands: PaletteItem[] = [
    { label: "File: New File", icon: <FilePlus2 size={15} />, run: () => newUntitledFile() },
    { label: "File: Open Folder", icon: <FolderOpen size={15} />, run: selectWorkspace },
    { label: "File: Import Script from File", icon: <FileInput size={15} />, run: importCodeSource },
    { label: "File: Import Script from URL", icon: <Globe2 size={15} />, run: () => setImportUrlOpen(true) },
    { label: "File: Import Data", icon: <Upload size={15} />, run: importWorkspaceFiles },
    { label: "File: Save", icon: <Save size={15} />, run: saveActive },
    { label: "File: Save As", icon: <Save size={15} />, run: saveActiveAs },
    { label: "File: Save All", icon: <Save size={15} />, run: saveAll },
    { label: "BioLang: Run Active File", icon: <Play size={15} />, run: handleRunClick },
    { label: "BioLang: Run Tests", icon: <FlaskConical size={15} />, run: () => void runWorkspaceTests() },
    { label: "BioLang: Run Tests in This File", icon: <FlaskConical size={15} />, run: () => void runWorkspaceTests(activeFile?.path) },
    { label: "BioLang: Format Document", icon: <Braces size={15} />, run: () => editorCommand("editor.action.formatDocument") },
    { label: "BioLang: Rename Symbol", icon: <Braces size={15} />, run: () => editorCommand("editor.action.rename") },
    { label: "BioLang: Find All References", icon: <Search size={15} />, run: findReferences },
    { label: "BioLang: Quick Fix", icon: <Zap size={15} />, run: () => editorCommand("editor.action.quickFix") },
    { label: "BioLang: Send Selection to Console", icon: <Braces size={15} />, run: sendSelectionToConsole },
    { label: "BioLang: Stop Running Job", icon: <CircleStop size={15} />, run: stopActive },
    { label: "View: Explorer", icon: <Files size={15} />, run: () => setActivity("explorer") },
    { label: "Explorer: Expand All Folders", icon: <ChevronsDown size={15} />, run: expandAllTreeDirectories },
    { label: "Explorer: Collapse All Folders", icon: <ChevronsUp size={15} />, run: collapseAllTreeDirectories },
    { label: "View: Search", icon: <Search size={15} />, run: () => setActivity("search") },
    { label: "View: Source Control", icon: <GitBranch size={15} />, run: () => setActivity("scm") },
    { label: "View: Packages", icon: <Blocks size={15} />, run: () => setActivity("packages") },
    { label: "View: Bio APIs", icon: <Globe2 size={15} />, run: () => setActivity("apis") },
    { label: "View: Learner Mode", icon: <GraduationCap size={15} />, run: () => setExperienceMode("learner") },
    { label: "View: Expert Mode", icon: <Zap size={15} />, run: () => setExperienceMode("expert") },
    { label: "Help: Open Help Center", icon: <Library size={15} />, run: () => openHelp() },
    { label: "Help: Language Guide", icon: <BookOpen size={15} />, run: () => openHelp("language") },
    { label: "Help: Builtin Reference", icon: <Braces size={15} />, run: () => openHelp("builtin") },
    { label: "Help: Tutorials", icon: <GraduationCap size={15} />, run: () => openHelp("tutorial") },
    { label: "Help: Examples", icon: <FileCode2 size={15} />, run: () => openHelp("example") },
    { label: "View: Terminal", icon: <TerminalSquare size={15} />, run: () => openPanel("terminal") },
    { label: "View: BioLang Console", icon: <Braces size={15} />, run: () => openPanel("console") },
    { label: "View: Split Editor Right", icon: <Columns2 size={15} />, run: () => openSplitRight() },
    { label: "View: Close Split", icon: <Columns2 size={15} />, run: closeSplit },
    { label: "View: Move Output to Bottom", icon: <PanelBottom size={15} />, run: () => moveOutput("bottom") },
    { label: "View: Move Output to Right", icon: <PanelRight size={15} />, run: () => moveOutput("right") },
    { label: "View: Open Output in Editor", icon: <FileText size={15} />, run: () => moveOutput("editor") },
    { label: "Output: Export", icon: <Download size={15} />, run: () => void exportOutput() },
    { label: "View: Toggle Bottom Panel", icon: <PanelBottom size={15} />, run: () => setBottomVisible((value) => !value) },
    { label: "Preferences: Settings", icon: <Settings size={15} />, run: () => openSettings() },
    { label: "Help: Keyboard Shortcuts", icon: <Command size={15} />, run: () => setShortcutsOpen(true) },
    ...recentWorkspaces.map((path) => ({
      label: `Recent Workspace: ${path}`,
      icon: <Folder size={15} />,
      run: () => openRecentWorkspace(path),
    })),
  ];

  // Grouping by file turns a flat list of 200 lines into something you can
  // scan; a rename across a project usually touches a handful of files with
  // many hits each.
  const groupedSearchHits = [...searchHits.reduce((groups, hit) => {
    groups.set(hit.path, [...(groups.get(hit.path) ?? []), hit]);
    return groups;
  }, new Map<string, SearchHit[]>())];

  const searchStatus = searchBusy
    ? "Searching..."
    : search.trim().length < MIN_SEARCH_LENGTH
      ? `Enter at least ${MIN_SEARCH_LENGTH} characters`
      : searchOptions.regex && !searchPattern(search.trim(), searchOptions)
        ? "Incomplete regular expression"
        : `${searchHits.length}${searchHits.length === 200 ? "+" : ""} results in ${groupedSearchHits.length} file${groupedSearchHits.length === 1 ? "" : "s"}`;

  /**
   * Rewrite every match in the workspace, after confirming.
   *
   * Confirmation is not optional here: this edits files that are not open, so
   * there is nothing on screen to undo and no editor history to fall back on.
   */
  const replaceAllMatches = async () => {
    const files = groupedSearchHits.length;
    const confirmed = await confirmAction({
      title: "Replace across the workspace",
      message: `Replace ${searchHits.length} match${searchHits.length === 1 ? "" : "es"} in ${files} file${files === 1 ? "" : "s"} with "${replaceText}"? Files that are not open cannot be undone from the editor.`,
      confirmLabel: "Replace All",
      danger: true,
    });
    if (!confirmed) return;
    try {
      const changed = await bridge.replaceInWorkspace(search.trim(), replaceText, searchOptions);
      showNotice(`Replaced in ${changed} file${changed === 1 ? "" : "s"}`);
      await reloadOpenFiles();
      const hits = await bridge.searchWorkspace(search, searchOptions);
      setSearchHits(hits);
    } catch (error) {
      showNotice(String(error));
    }
  };

  const paletteSymbols: PaletteItem[] = activeSymbols.map((symbol) => ({
    label: symbol.name,
    hint: `${symbol.kind} · line ${symbol.line}`,
    icon: <Braces size={15} />,
    run: () => goToSymbol(symbol),
  }));

  // Quick open indexes the whole workspace, not just the editors already open,
  // because the point of Ctrl+P is reaching a file you have not opened yet.
  const paletteFiles: PaletteItem[] = [
    ...openFiles.map((file) => ({
      label: file.name,
      hint: file.untitled ? "unsaved" : directoryOf(file.path),
      icon: fileIcon(file.untitled ? file.name : file.path),
      run: () => {
        setOutputEditorActive(false);
        setActivity("explorer");
        setActivePath(file.path);
      },
    })),
    ...allFiles
      .filter((entry) => !openFiles.some((file) => file.path === entry.path))
      .map((entry) => ({
        label: entry.name,
        hint: directoryOf(entry.path),
        icon: fileIcon(entry.path),
        run: () => void openFile(entry.path),
      })),
  ];

  const paletteMode = paletteModeFor(paletteSearch);
  const paletteQuery = paletteMode === "file" ? paletteSearch : paletteSearch.slice(1);
  const paletteSource = paletteMode === "command"
    ? paletteCommands
    : paletteMode === "symbol"
      ? paletteSymbols
      : paletteFiles;

  const commands = paletteSource
    .map((item, order) => {
      const id = `${item.label} ${item.hint ?? ""}`;
      const direct = fuzzyMatch(paletteQuery, item.label);
      // Fall back to the full path so "src/kmer" still finds a file whose name
      // alone does not contain the query.
      const qualified = direct || !item.hint
        ? undefined
        : fuzzyMatch(paletteQuery, `${item.hint}/${item.label}`);
      const match = direct ?? (qualified ? { ...qualified, positions: [] } : undefined);
      if (!match) return undefined;
      const recency = paletteRecent.indexOf(id);
      return {
        ...item,
        id,
        positions: match.positions,
        score: match.score
          - (direct ? 0 : 30)
          - order / 1000
          + (recency < 0 ? 0 : (PALETTE_RECENT_LIMIT - recency) * 4),
      };
    })
    .filter((item): item is PaletteEntry => Boolean(item))
    .sort((left, right) => right.score - left.score)
    .slice(0, PALETTE_RESULT_LIMIT);

  const runPaletteItem = (item: PaletteEntry) => {
    setPaletteOpen(false);
    setPaletteRecent((recent) => [item.id, ...recent.filter((entry) => entry !== item.id)]
      .slice(0, PALETTE_RECENT_LIMIT));
    void item.run();
  };

  const onPaletteKeyDown = (event: React.KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      if (!commands.length) return;
      const delta = event.key === "ArrowDown" ? 1 : -1;
      setPaletteIndex((index) => (index + delta + commands.length) % commands.length);
    } else if (event.key === "Home") {
      event.preventDefault();
      setPaletteIndex(0);
    } else if (event.key === "End") {
      event.preventDefault();
      setPaletteIndex(Math.max(0, commands.length - 1));
    } else if (event.key === "Enter") {
      event.preventDefault();
      const selected = commands[paletteIndex];
      if (selected) runPaletteItem(selected);
    }
  };

  const paletteCount = commands.length;
  useEffect(() => {
    setPaletteIndex((index) => (index < paletteCount ? index : 0));
  }, [paletteCount, paletteSearch]);

  useEffect(() => {
    if (!paletteOpen) return;
    paletteListRef.current
      ?.querySelector(`#palette-option-${paletteIndex}`)
      ?.scrollIntoView({ block: "nearest" });
  }, [paletteIndex, paletteOpen]);

  const renderSidebar = () => {
    if (activity === "help") {
      const sections: Array<{ id: HelpKind | "all"; label: string }> = [
        { id: "all", label: "All" },
        { id: "language", label: "Language" },
        { id: "builtin", label: "Built-ins" },
        { id: "tutorial", label: "Tutorials" },
        { id: "example", label: "Examples" },
      ];
      const visibleEntries = filteredHelp.slice(0, 250);
      const totalEntries = helpIndex ? Object.values(helpIndex.counts).reduce((sum, count) => sum + count, 0) : 0;
      return (
        <div className="help-sidebar">
          <div className="sidebar-title"><span>Help</span><span className="help-total">{totalEntries || "..."}</span></div>
          <div className="search-field"><Search size={14} /><input autoFocus value={helpSearch} onChange={(event) => setHelpSearch(event.target.value)} placeholder="Search all BioLang help" /></div>
          <div className="help-filters" role="tablist" aria-label="Help sections">
            {sections.map((section) => (
              <button type="button" role="tab" aria-selected={helpSection === section.id} className={helpSection === section.id ? "active" : ""} key={section.id} onClick={() => { setHelpSection(section.id); setSelectedHelpId(undefined); }}>
                {section.label}
              </button>
            ))}
          </div>
          <div className="result-count">{helpIndex ? `${filteredHelp.length} entries` : "Loading offline reference..."}</div>
          <div className="help-results">
            {visibleEntries.map((entry) => (
              <button type="button" className={`help-result ${selectedHelp?.id === entry.id ? "selected" : ""}`} key={entry.id} onClick={() => setSelectedHelpId(entry.id)}>
                <span className={`help-result-icon ${entry.kind}`}>{helpIcon(entry.kind)}</span>
                <span><strong>{entry.title}</strong><small>{entry.category} / {entry.collection}</small></span>
              </button>
            ))}
            {helpIndex && filteredHelp.length === 0 && <EmptyState icon={<Search size={21} />} title="No help results" detail="Try a builtin, concept, format, or workflow" />}
            {filteredHelp.length > visibleEntries.length && <div className="sidebar-note">Refine the search to view the remaining {filteredHelp.length - visibleEntries.length} entries.</div>}
          </div>
        </div>
      );
    }
    if (activity === "explorer") {
      return (
        <>
          <div className="sidebar-title">
            <span>Explorer</span>
            <div className="sidebar-actions">
              <IconButton label="New file" disabled={!workspace} onClick={() => newUntitledFile()}><FilePlus2 size={14} /></IconButton>
              <IconButton label="New folder" disabled={!workspace} onClick={() => setEntryPrompt({ mode: "directory", value: "" })}><FolderPlus size={14} /></IconButton>
              <IconButton label="Refresh Explorer" disabled={!workspace} onClick={refreshWorkspace}><RefreshCw size={14} /></IconButton>
              <IconButton label="Import script from local file" disabled={!workspace} onClick={importCodeSource}><FileInput size={14} /></IconButton>
              <IconButton label="Import data" disabled={!workspace} onClick={importWorkspaceFiles}><Upload size={14} /></IconButton>
              <IconButton label={isDesktop ? "Open folder" : "Open browser workspace"} onClick={selectWorkspace}><FolderOpen size={14} /></IconButton>
            </div>
          </div>
          {workspace ? <>
            <div className="workspace-heading" onContextMenu={(event) => {
              event.preventDefault();
              showContextMenu({ kind: "workspace" }, event.clientX, event.clientY);
            }}>
              <ChevronDown size={13} />
              <span>{workspace.name}</span>
              <div className="workspace-tree-actions">
                <IconButton label="Expand all folders" onClick={expandAllTreeDirectories}><ChevronsDown size={13} /></IconButton>
                <IconButton label="Collapse all folders" onClick={collapseAllTreeDirectories}><ChevronsUp size={13} /></IconButton>
              </div>
            </div>
            <FileTree
              entries={workspace.entries}
              activePath={activePath}
              gitByPath={gitByPath}
              collapsedPaths={collapsedTreePaths}
              onToggleDirectory={toggleTreeDirectory}
              onOpen={openFile}
              onContext={(entry, x, y) => showContextMenu({ kind: "explorer", entry }, x, y)}
              onMove={moveWorkspaceEntry}
              onImportFiles={importDroppedFiles}
            />
            {workspace.truncated && <div className="sidebar-note">File list truncated</div>}
            <div className="outline-section">
              <div className="section-heading"><ChevronDown size={13} /> Outline</div>
              {activeSymbols.length ? activeSymbols.map((symbol) => (
                <button type="button" className="outline-item" key={`${symbol.kind}-${symbol.name}-${symbol.line}`} onClick={() => goToSymbol(symbol)}>
                  <Braces size={14} />
                  <span>{symbol.name}<small>{symbol.kind} | line {symbol.line}</small></span>
                </button>
              )) : <div className="outline-item"><Braces size={14} /> No symbols</div>}
            </div>
          </> : <div className="sidebar-empty">
            <FolderOpen size={24} />
            <span>No workspace open</span>
            <button type="button" className="command-button primary" onClick={selectWorkspace}>{isDesktop ? "Open Folder" : "Open Browser Workspace"}</button>
          </div>}
        </>
      );
    }
    if (activity === "search") {
      return (
        <>
          <div className="sidebar-title"><span>Search</span></div>
          {referenceResults && <div className="reference-results">
            <header>
              <span>{referenceResults.hits.length} reference{referenceResults.hits.length === 1 ? "" : "s"} to <code>{referenceResults.name}</code></span>
              <IconButton label="Clear references" onClick={() => setReferenceResults(undefined)}><X size={12} /></IconButton>
            </header>
            {referenceResults.hits.map((hit, index) => (
              <button className="search-result content-hit" type="button" key={`${hit.line}-${hit.column}-${index}`} onClick={() => void openSearchHit(hit)}>
                {fileIcon(hit.path)}<span>{hit.preview || "(blank line)"}<small>{hit.path}:{hit.line}:{hit.column}</small></span>
              </button>
            ))}
          </div>}
          {!workspace ? <div className="sidebar-empty"><Search size={23} /><span>Open a folder to search</span></div> : <>
          <div className="search-field">
            <IconButton label={replaceOpen ? "Hide replace" : "Show replace"} onClick={() => setReplaceOpen((value) => !value)}>
              {replaceOpen ? <ChevronDown size={13} /> : <ChevronRight size={13} />}
            </IconButton>
            <Search size={14} />
            <input autoFocus value={search} onChange={(event) => setSearch(event.target.value)} placeholder="Search file contents" />
            <span className="search-toggles">
              <button type="button" aria-label="Match case" aria-pressed={searchOptions.caseSensitive} className={searchOptions.caseSensitive ? "active" : ""} title="Match case" onClick={() => setSearchOptions((value) => ({ ...value, caseSensitive: !value.caseSensitive }))}>Aa</button>
              <button type="button" aria-label="Match whole word" aria-pressed={searchOptions.wholeWord} className={searchOptions.wholeWord ? "active" : ""} title="Match whole word" onClick={() => setSearchOptions((value) => ({ ...value, wholeWord: !value.wholeWord }))}>ab</button>
              <button type="button" aria-label="Use regular expression" aria-pressed={searchOptions.regex} className={searchOptions.regex ? "active" : ""} title="Use regular expression" onClick={() => setSearchOptions((value) => ({ ...value, regex: !value.regex }))}>.*</button>
            </span>
          </div>
          {replaceOpen && <div className="search-field replace-field">
            <Replace size={14} />
            <input aria-label="Replace with" value={replaceText} onChange={(event) => setReplaceText(event.target.value)} placeholder="Replace with" />
            <button type="button" className="replace-all" disabled={!searchHits.length || searchBusy} onClick={() => void replaceAllMatches()}>Replace All</button>
          </div>}
          <div className="result-count">{searchStatus}</div>
          {groupedSearchHits.map(([path, hits]) => (
            <div className="search-file-group" key={path}>
              <div className="search-file-heading">{fileIcon(path)}<span>{path}</span><small>{hits.length}</small></div>
              {hits.map((hit, index) => (
                <button className="search-result content-hit" type="button" key={`${hit.line}-${hit.column}-${index}`} onClick={() => void openSearchHit(hit)}>
                  <span>{hit.preview || "(blank line)"}<small>{hit.line}:{hit.column}</small></span>
                </button>
              ))}
            </div>
          ))}</>}
        </>
      );
    }
    if (activity === "scm") {
      const staged = gitStatus.files.filter((file) => file.indexStatus.trim() && file.indexStatus !== "?");
      const changed = gitStatus.files.filter((file) => !file.indexStatus.trim() || file.indexStatus === "?");
      const row = (file: GitFileStatus, group: "staged" | "changed") => (
        <div className="scm-row" key={`${group}-${file.path}`}>
          <button type="button" className="scm-open" onClick={() => void openGitDiff(file.path, group === "staged")}>
            {fileIcon(file.path)}
            <span>{file.path}</span>
            <code>{(group === "staged" ? file.indexStatus : file.worktreeStatus || file.indexStatus).trim() || "M"}</code>
          </button>
          <IconButton
            label={group === "staged" ? `Unstage ${file.path}` : `Stage ${file.path}`}
            onClick={() => void (group === "staged" ? unstageFiles([file.path]) : stageFiles([file.path]))}
          >{group === "staged" ? <Minus size={13} /> : <Plus size={13} />}</IconButton>
        </div>
      );

      return (
        <>
          <div className="sidebar-title">
            <span>Source Control</span>
            <IconButton label="Refresh Git status" onClick={() => void refreshGitStatus()}><RefreshCw size={13} /></IconButton>
          </div>
          {!gitStatus.available
            ? <div className="sidebar-empty"><GitBranch size={23} /><span>{workspace ? "This folder is not a Git repository" : "Open a folder to use Source Control"}</span></div>
            : <>
              <div className="scm-branch"><GitBranch size={12} />{gitStatus.branch ?? "detached"}</div>
              <div className="scm-commit">
                <textarea
                  aria-label="Commit message"
                  placeholder="Message (what changed, and why)"
                  rows={2}
                  value={commitMessage}
                  onChange={(event) => setCommitMessage(event.target.value)}
                />
                <button
                  type="button"
                  className="command-button primary"
                  disabled={!isDesktop || !workspaceTrusted || !commitMessage.trim() || !staged.length}
                  onClick={() => void commitStaged()}
                ><Check size={14} />Commit {staged.length || ""}</button>
              </div>
              {!gitStatus.files.length && <div className="sidebar-empty"><Check size={23} /><span>No changes</span></div>}
              {staged.length > 0 && <>
                <div className="sidebar-subtitle">
                  <span>Staged ({staged.length})</span>
                  <button type="button" onClick={() => void unstageFiles(staged.map((file) => file.path))}>Unstage all</button>
                </div>
                {staged.map((file) => row(file, "staged"))}
              </>}
              {changed.length > 0 && <>
                <div className="sidebar-subtitle">
                  <span>Changes ({changed.length})</span>
                  <button type="button" onClick={() => void stageFiles(changed.map((file) => file.path))}>Stage all</button>
                </div>
                {changed.map((file) => row(file, "changed"))}
              </>}
            </>}
        </>
      );
    }
    if (activity === "packages") {
      return (
        <>
          <div className="sidebar-title">
            <span>Packages</span>
            <IconButton label="Install dependencies" disabled={!isDesktop || packageBusy || !workspaceTrusted} onClick={installPackages}>
              <RefreshCw size={14} className={packageBusy ? "spin" : ""} />
            </IconButton>
          </div>
          {!workspace ? <div className="sidebar-empty"><Package size={23} /><span>Open a package workspace</span></div> : <>
          <div className="sidebar-subtitle">biolang.toml</div>
          {packages.length === 0 ? (
            <EmptyState icon={<Package size={22} />} title="No dependencies" detail="Add packages to biolang.toml" />
          ) : packages.map((pkg) => (
            <div className="package-row" key={pkg.name}>
              <div className={`package-status ${pkg.installed ? "installed" : "missing"}`}>{pkg.installed ? <Check size={12} /> : <AlertCircle size={12} />}</div>
              <div><strong>{pkg.name}</strong><span>{pkg.version ?? pkg.source}</span></div>
            </div>
          ))}
          <button type="button" className="command-button" onClick={installPackages} disabled={!isDesktop || packageBusy || !workspaceTrusted}>
            {packageBusy ? <LoaderCircle size={14} className="spin" /> : <Package size={14} />}
            Install dependencies
          </button>
          {!isDesktop && <div className="sidebar-note">Install packages with Desktop or on the selected SOMER runtime.</div>}
          </>}
        </>
      );
    }
    if (activity === "apis") {
      const scopedGroups = apiScope === "external"
        ? functionGroups.filter((group) => group.name === "Api")
        : functionGroups;
      const groups = scopedGroups.map((group) => ({
        ...group,
        functions: group.functions.filter((fn) => `${group.name} ${fn.name} ${fn.signature}`.toLowerCase().includes(apiSearch.toLowerCase())),
      })).filter((group) => group.functions.length);
      const visibleCount = groups.reduce((count, group) => count + group.functions.length, 0);
      return (
        <>
          <div className="sidebar-title"><span>API Browser</span><small>{visibleCount || "..."}</small></div>
          <div className="api-scope-switch" role="group" aria-label="API browser scope">
            <button type="button" className={apiScope === "external" ? "active" : ""} onClick={() => {
              setApiScope("external");
              if (externalApiProvider(selectedApi?.name ?? "") === "BioLang runtime") {
                setSelectedApi(functionGroups.find((group) => group.name === "Api")?.functions[0]);
              }
            }}>External DBs</button>
            <button type="button" className={apiScope === "all" ? "active" : ""} onClick={() => setApiScope("all")}>All builtins</button>
          </div>
          <div className="search-field"><Search size={14} /><input value={apiSearch} onChange={(event) => setApiSearch(event.target.value)} placeholder={apiScope === "external" ? "Search external databases" : "Search functions"} /></div>
          {groups.map((group) => (
            <div className="api-group" key={group.name}>
              <div className="api-group-title"><Database size={14} /><span><strong>{group.name}</strong><small>{group.description}</small></span></div>
              {group.functions.map((fn) => (
                <button key={fn.name} type="button" className={fn.name === selectedApi?.name ? "selected" : ""} onClick={() => setSelectedApi(fn)}>
                  <Zap size={12} /><span>{fn.name}</span>
                </button>
              ))}
            </div>
          ))}
        </>
      );
    }
    return (
      <>
        <div className="sidebar-title"><span>Jobs</span></div>
        {jobs.length === 0 ? (
          <EmptyState icon={<FlaskConical size={22} />} title="No jobs yet" detail="Run a BioLang file to begin" />
        ) : jobs.length > 60 ? (
          <VirtualList
            count={jobs.length}
            itemHeight={44}
            height={Math.min(520, jobs.length * 44)}
            className="jobs-virtual"
            renderItem={(index) => {
              const job = jobs[index]!;
              return (
                <button type="button" className="job-sidebar-row" onClick={() => { openPanel("jobs"); void selectJob(job); }}>
                  <span className={`job-dot ${job.status}`} />
                  <span>{job.file.split("/").pop()}<small>{job.backend} | {job.status}</small></span>
                  <time>{job.durationMs ? `${(job.durationMs / 1000).toFixed(1)}s` : ""}</time>
                </button>
              );
            }}
          />
        ) : jobs.map((job) => (
          <button type="button" className="job-sidebar-row" key={job.id} onClick={() => { openPanel("jobs"); void selectJob(job); }}>
            <span className={`job-dot ${job.status}`} />
            <span>{job.file.split("/").pop()}<small>{job.backend} | {job.status}</small></span>
            <time>{job.durationMs ? `${(job.durationMs / 1000).toFixed(1)}s` : ""}</time>
          </button>
        ))}
      </>
    );
  };

  const outputPaneProps = {
    runs: outputRuns,
    job: activeOutputJob,
    compareJob: compareOutputJob,
    fileName: activeFile?.name,
    elapsed: activeOutputJob ? formatElapsed(jobClock - activeOutputJob.startedAt) : "0s",
    simplified: experienceMode === "learner",
    exportFormat: outputExportFormat,
    exportOptions: activeOutputExportOptions,
    onSelectJob: selectOutputRun,
    onCompareJob: setCompareOutputRunId,
    onExportFormat: setOutputExportFormat,
    onExport: () => void exportOutput(),
    onExportBundle: () => void exportRunBundle(),
    onClear: () => {
      if (activeOutputJob) clearJobLog(activeOutputJob.id);
    },
    onMove: moveOutput,
    onDockPointerDown: startOutputDockDrag,
    onClose: closeOutput,
    onPin: () => {
      if (activeOutputJob) pinJob(activeOutputJob.id, !activeOutputJob.pinned);
    },
    onRename: (name: string) => {
      if (activeOutputJob) renameJob(activeOutputJob.id, name);
    },
    onDelete: () => void deleteOutputRun(),
    onRerun: () => {
      if (activeOutputJob) void rerunJob(activeOutputJob);
    },
    onDetach: () => void detachOutput(),
    onOpenDiagnostic: (path: string, line: number, column: number) => {
      void openOutputDiagnostic(path, line, column);
    },
    onReadArtifactPreview: async (artifact: JobArtifact, length?: number) => {
      if (!activeOutputJob) throw new Error("No run selected");
      return readJobArtifactPreview(activeOutputJob, artifact, length);
    },
    onSaveArtifact: (artifact: JobArtifact) => {
      if (!activeOutputJob) return;
      void saveJobArtifact(activeOutputJob, artifact)
        .then((destination) => {
          if (destination) showNotice(`Saved artifact to ${destination}`);
        })
        .catch((error) => showNotice(String(error)));
    },
    onReadResultPage: async (resultIndex: number, request: import("./types").ResultPageRequest) => {
      if (!activeOutputJob) throw new Error("No run selected");
      return readResultPage(activeOutputJob, resultIndex, request);
    },
    onCompareEnvironment: (provenance: JobProvenance) => bridge.compareRunEnvironment(provenance),
    onRestoreEnvironment: restoreRunEnvironment,
    onCopyText: (text: string) => {
      void bridge.copyText(text);
      showNotice("Copied to the clipboard");
    },
  };
  const bottomPanelShown = bottomVisible && Boolean(workspace);

  if (startupPhase !== "ready") {
    return (
      <div className="startup-screen" role="status" aria-live="polite">
        <div className="startup-content">
          <span className="startup-mark" aria-hidden="true"><Dna size={24} /></span>
          <strong>BioLang {productEdition}</strong>
          <div
            className="startup-progress"
            role="progressbar"
            aria-label={`Restoring BioLang ${productEdition}`}
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={startupPhase === "session" ? 82 : 48}
          >
            <span className={startupPhase === "session" ? "session" : ""} />
          </div>
          <span>{startupPhase === "workspace" ? "Opening workspace..." : "Restoring editor session..."}</span>
          {startupSlow && (
            <button type="button" onClick={() => setStartupPhase("ready")}>
              Continue without waiting
            </button>
          )}
        </div>
      </div>
    );
  }

  return (
    <div className={`app-shell ${experienceMode}-mode ${editorTheme === "biolang-light" ? "light-theme" : ""}`}>
      <header className="titlebar">
        <div className="title-navigation">
          <div className="brand"><span className="brand-mark"><Dna size={17} /></span><strong>BioLang</strong><span>{productEdition}</span></div>
          <nav className="app-menu" aria-label="Application menu">
            {Object.entries(menuModels)
              .filter(([name]) => experienceMode === "expert" || ["File", "Run", "Help"].includes(name))
              .map(([name, items]) => (
                <div className="menu-root" key={name}>
                  <button type="button" className={openMenu === name ? "active" : ""} onClick={() => setOpenMenu((current) => current === name ? undefined : name)}>{name}</button>
                  {openMenu === name && <div className="menu-popup" role="menu">
                    {items.map((item, index) => item.separator
                      ? <div className="menu-separator" key={`separator-${index}`} />
                      : <button type="button" role="menuitem" disabled={item.disabled} key={item.label} onClick={() => { setOpenMenu(undefined); void item.action?.(); }}>
                        <span className="menu-check">{item.checked ? <Check size={12} /> : null}</span>
                        <span>{item.label}</span>
                        {item.shortcut && <kbd>{item.shortcut}</kbd>}
                      </button>)}
                  </div>}
                </div>
              ))}
          </nav>
          <div className="menu-root compact-app-menu">
            <button
              type="button"
              className={openMenu === "Menu" ? "active" : ""}
              aria-label="Application menu"
              onClick={() => setOpenMenu((current) => current === "Menu" ? undefined : "Menu")}
            >Menu</button>
            {openMenu === "Menu" && <div className="menu-popup compact-menu-popup" role="menu">
              {Object.entries(menuModels)
                .filter(([name]) => experienceMode === "expert" || ["File", "Run", "Help"].includes(name))
                .flatMap(([name, items]) => [
                  <div className="menu-section-label" key={`section-${name}`}>{name}</div>,
                  ...items.map((item, index) => item.separator
                    ? <div className="menu-separator" key={`${name}-sep-${index}`} />
                    : <button type="button" role="menuitem" disabled={item.disabled} key={`${name}-${item.label}`} onClick={() => { setOpenMenu(undefined); void item.action?.(); }}>
                      <span className="menu-check">{item.checked ? <Check size={12} /> : null}</span>
                      <span>{item.label}</span>
                      {item.shortcut && <kbd>{item.shortcut}</kbd>}
                    </button>),
                ])}
            </div>}
          </div>
        </div>
        <button className="command-center" type="button" aria-label="Command center" onClick={() => openPalette("")}><Search size={13} /><span>{experienceMode === "learner" ? "Search files, help, and commands" : workspace?.name ?? "Open a workspace"}</span><kbd>Ctrl P</kbd></button>
        <div className="title-actions">
          <div className="experience-switch" role="group" aria-label="Interface mode">
            <button type="button" className={experienceMode === "learner" ? "active" : ""} aria-pressed={experienceMode === "learner"} onClick={() => setExperienceMode("learner")}>Learner</button>
            <button type="button" className={experienceMode === "expert" ? "active" : ""} aria-pressed={experienceMode === "expert"} onClick={() => setExperienceMode("expert")}>Expert</button>
          </div>
          <IconButton label="Open workspace" onClick={selectWorkspace}><FolderOpen size={15} /></IconButton>
          <IconButton label="Save file" onClick={saveActive} disabled={!activeFile || !isDirtyFile(activeFile)}><Save size={15} /></IconButton>
          {pwa.canInstall && <IconButton label="Install BioLang Workbench Web" onClick={() => void pwa.install()}><Download size={15} /></IconButton>}
          <IconButton label="Toggle bottom panel" active={bottomPanelShown} disabled={!workspace} onClick={() => setBottomVisible((value) => !value)}><PanelBottom size={16} /></IconButton>
        </div>
      </header>

      <div
        className={`workbench ${bottomPanelShown ? "panel-open" : ""} ${bottomPanelShown && panelMaximized ? "panel-maximized" : ""} ${workspace && outputLocation === "right" && outputRightVisible ? "output-dock-right" : ""}`}
        style={{
          "--sidebar-width": `${sidebarWidth}px`,
          "--bottom-panel-height": `${bottomPanelHeight}px`,
          "--output-panel-width": `${outputPanelWidth}px`,
          "--inspector-width": `${inspectorWidth}px`,
        } as React.CSSProperties}
      >
        <aside className="activity-bar" aria-label="Primary navigation">
          <div>
            <IconButton label="Explorer" active={activity === "explorer"} onClick={() => setActivity("explorer")}><Files size={21} /><span>Explorer</span></IconButton>
            <IconButton label="Search" active={activity === "search"} onClick={() => setActivity("search")}><Search size={21} /><span>Search</span></IconButton>
            {experienceMode === "expert" && <IconButton label="Source Control" active={activity === "scm"} onClick={() => setActivity("scm")}><GitBranch size={21} />{gitStatus.files.length > 0 && <b className="activity-badge">{gitStatus.files.length}</b>}<span>Source Control</span></IconButton>}
            {experienceMode === "expert" && <IconButton label="Packages" active={activity === "packages"} onClick={() => setActivity("packages")}><Blocks size={21} /><span>Packages</span></IconButton>}
            {experienceMode === "expert" && <IconButton label="Bio APIs" active={activity === "apis"} onClick={() => setActivity("apis")}><Globe2 size={21} /><span>Bio APIs</span></IconButton>}
            {experienceMode === "expert" && <IconButton label="Jobs" active={activity === "jobs"} onClick={() => setActivity("jobs")}><FlaskConical size={21} /><span>Jobs</span></IconButton>}
            <IconButton label="Help Center" active={activity === "help"} onClick={() => openHelp()}><BookOpen size={21} /><span>Help</span></IconButton>
          </div>
          <div><IconButton label="Settings" onClick={() => openSettings()}><Settings size={21} /><span>Settings</span></IconButton></div>
        </aside>

        <aside className="sidebar">{renderSidebar()}</aside>
        <div
          className="pane-resizer sidebar-resizer"
          role="separator"
          aria-label="Resize sidebar"
          aria-orientation="vertical"
          onPointerDown={(event) => startPaneResize("sidebar", event)}
        />

        <main className={`editor-workspace ${workspace && !workspaceTrusted ? "restricted" : ""} ${splitOpen ? "split-open" : ""}`}>
          <div className="editor-tabs">
            {activity === "help" && !outputEditorActive ? (
              <div className="editor-tab active help-tab"><BookOpen size={14} /><span>{selectedHelp?.title ?? "BioLang Help"}</span></div>
            ) : openFiles.map((file) => (
              <div
                className={`editor-tab ${file.path === activePath && !outputEditorActive ? "active" : ""}`}
                key={file.path}
                onContextMenu={(event) => {
                  event.preventDefault();
                  setActivePath(file.path);
                  showContextMenu({ kind: "tab", path: file.path }, event.clientX, event.clientY);
                }}
              >
                <button type="button" className="editor-tab-main" onClick={() => {
                  setOutputEditorActive(false);
                  setFocusedGroup("primary");
                  setActivePath(file.path);
                }}>
                  {fileIcon(file.untitled ? file.name : file.path)}<span>{file.name}</span>{isDirtyFile(file) && <i />}
                </button>
                <button type="button" className="tab-close" aria-label={`Close ${file.name}`} onClick={() => void closeFile(file.path)}><X size={13} /></button>
              </div>
            ))}
            {outputLocation === "editor" && outputEditorOpen && (
              <div className={`editor-tab output-editor-tab ${outputEditorActive ? "active" : ""}`}>
                <button type="button" className="editor-tab-main" onClick={() => setOutputEditorActive(true)}>
                  <FileText size={14} /><span>Output</span>
                </button>
                <button type="button" className="tab-close" aria-label="Close Output" onClick={closeOutput}><X size={13} /></button>
              </div>
            )}
            {openFiles.length > 5 && activity !== "help" && (
              <details className="tab-overflow">
                <summary aria-label={`${openFiles.length} open editors`} title="Open editors">
                  <ChevronsDown size={13} />
                  <span>{openFiles.length}</span>
                </summary>
                <div className="tab-overflow-menu" role="menu">
                  {openFiles.map((file) => (
                    <div
                      className={`tab-overflow-row ${file.path === activePath && !outputEditorActive ? "active" : ""}`}
                      key={`overflow-${file.path}`}
                    >
                      <button
                        type="button"
                        role="menuitem"
                        className="tab-overflow-open"
                        onClick={(event) => {
                          setOutputEditorActive(false);
                          setActivePath(file.path);
                          const details = event.currentTarget.closest("details");
                          if (details) details.open = false;
                        }}
                      >
                        {fileIcon(file.untitled ? file.name : file.path)}
                        <span>{file.name}</span>
                        {isDirtyFile(file) && <i className="tab-overflow-dirty" />}
                      </button>
                      <button
                        type="button"
                        className="tab-overflow-close"
                        aria-label={`Close ${file.name}`}
                        onClick={() => void closeFile(file.path)}
                      ><X size={12} /></button>
                    </div>
                  ))}
                </div>
              </details>
            )}
            {((activity !== "help" && !outputEditorActive && isRunnableFile(activeFile)) || Boolean(runningJob)) && <div className="editor-run-actions">
              {experienceMode === "expert" && (
                <IconButton
                  label={splitOpen ? "Close editor split" : "Split editor right"}
                  active={splitOpen}
                  disabled={!activePath && !splitOpen}
                  onClick={() => splitOpen ? closeSplit() : openSplitRight()}
                ><Columns2 size={15} /></IconButton>
              )}
              {activity !== "help" && !outputEditorActive && isRunnableFile(activeFile) && (
                <>
                  <label className="run-target-chip" title="Execution target">
                    <span className="sr-only">Execution target</span>
                    {executionTarget === "local" ? <HardDrive size={12} /> : <Server size={12} />}
                    <select
                      aria-label="Execution target for Run"
                      value={executionTarget}
                      onChange={(event) => {
                        setExecutionTarget(event.target.value);
                        remoteRunAcknowledged.current = false;
                      }}
                    >
                      <option value="local">{isDesktop ? "Local" : "Browser WASM"}</option>
                      {somerProfiles.map((profile) => (
                        <option value={profile.id} key={profile.id}>{profile.name}</option>
                      ))}
                    </select>
                  </label>
                  <IconButton
                    label={runButtonLabel}
                    onClick={() => void handleRunClick()}
                    disabled={runButtonDisabled}
                    className="run"
                  ><Play size={16} fill="currentColor" /></IconButton>
                </>
              )}
              {runningJob && <IconButton label="Stop running job" onClick={stopActive}><CircleStop size={16} /></IconButton>}
            </div>}
          </div>
          {workspace && !workspaceTrusted && <div className="trust-banner">
            <AlertCircle size={15} />
            <span><strong>Restricted mode</strong> Editing is enabled. Execution, terminals, packages, and language services are disabled.</span>
            <button type="button" onClick={() => void trustWorkspace(true)}>Trust Workspace</button>
          </div>}
          {activeFile && diskChangedPaths[activeFile.path] != null && (
            <div className="disk-change-banner" role="status">
              <RefreshCw size={14} />
              <span>
                <strong>{activeFile.name} changed on disk</strong>
                {isDirtyFile(activeFile)
                  ? " You have unsaved edits in the editor."
                  : " The file was modified outside BioLang."}
              </span>
              <button type="button" className="primary" onClick={() => void reloadActiveFromDisk()}>
                {isDirtyFile(activeFile) ? "Reload (discard edits)" : "Reload"}
              </button>
              <button type="button" onClick={keepActiveDespiteDisk}>Keep editing</button>
            </div>
          )}
          {outputLocation === "editor" && outputEditorActive ? (
            <OutputPane
              {...outputPaneProps}
              location="editor"
            />
          ) : activity === "help" ? (
            <ErrorBoundary label="BioLang Help">
              <Suspense fallback={<div className="editor-loading">Loading BioLang help...</div>}>
                <HelpDocument
                  key={selectedHelp?.id}
                  entry={selectedHelp}
                  canOpenSource={Boolean(selectedHelp?.sourcePath && allFiles.some((entry) => entry.path === selectedHelp.sourcePath))}
                  canInsert={Boolean(activeFile?.path.endsWith(".bl"))}
                  onOpenSource={openHelpSource}
                  onInsert={insertHelpExample}
                  onNavigate={navigateHelpLink}
                />
              </Suspense>
            </ErrorBoundary>
          ) : splitOpen ? (
            <div
              className="editor-split-body"
              style={{ "--editor-split-percent": `${splitPercent}%` } as React.CSSProperties}
            >
              <div
                className={`editor-group primary ${focusedGroup === "primary" ? "focused" : ""}`}
                onMouseDown={() => setFocusedGroup("primary")}
              >
                <EditorSurface
                  file={primaryFile}
                  workspaceName={workspace?.name}
                  group="primary"
                  pipelineView={pipelineView}
                  onPipelineView={setPipelineView}
                  editorTheme={editorTheme}
                  fontSize={fontSize}
                  tabSize={tabSize}
                  wordWrap={wordWrap}
                  minimap={minimap}
                  beforeMount={beforeMount}
                  onMount={editorMounted}
                  onChange={(value) => primaryFile && updateContentFor(primaryFile.path, value ?? "")}
                  output={activeOutput}
                  cellOutputs={primaryFile ? notebookCellOutputs[primaryFile.path] : undefined}
                  running={primaryFile ? runningJob?.file === primaryFile.path : false}
                  onRun={() => runFileWithTrust(primaryFile)}
                  onRunCell={(cellIndex) => runFileWithTrust(primaryFile, cellIndex)}
                  onStop={stopActive}
                  onCellMount={notebookCellMounted as never}
                  onCellChange={notebookCellChanged as never}
                  onCellUnmount={notebookCellUnmounted as never}
                  onInvalidateCell={(cellIndex) => primaryFile && invalidateNotebookCell(primaryFile.path, cellIndex)}
                  onExportPreview={exportDataPreview}
                  empty={workspace ? (
                    <div className="workspace-welcome compact editor-group-empty">
                      <span>Select a primary tab</span>
                    </div>
                  ) : undefined}
                />
              </div>
              <div
                className="pane-resizer editor-split-resizer"
                role="separator"
                aria-label="Resize editor split"
                aria-orientation="vertical"
                onPointerDown={(event) => {
                  event.preventDefault();
                  const startX = event.clientX;
                  const start = splitPercent;
                  const target = (event.currentTarget.parentElement as HTMLElement | null);
                  const onMove = (moveEvent: PointerEvent) => {
                    if (!target) return;
                    const rect = target.getBoundingClientRect();
                    const next = ((startX - rect.left + (moveEvent.clientX - startX)) / rect.width) * 100;
                    // keep start-based delta for stability
                    const delta = ((moveEvent.clientX - startX) / rect.width) * 100;
                    setSplitPercent(Math.min(75, Math.max(25, start + delta)));
                  };
                  const onUp = () => {
                    window.removeEventListener("pointermove", onMove);
                    window.removeEventListener("pointerup", onUp);
                  };
                  window.addEventListener("pointermove", onMove);
                  window.addEventListener("pointerup", onUp);
                }}
              />
              <div
                className={`editor-group secondary ${focusedGroup === "secondary" ? "focused" : ""}`}
                onMouseDown={() => setFocusedGroup("secondary")}
              >
                <div className="editor-tabs secondary-tabs">
                  {secondaryTabs.map((path) => {
                    const file = openFiles.find((candidate) => candidate.path === path);
                    if (!file) return null;
                    return (
                      <div className={`editor-tab ${path === secondaryActive ? "active" : ""}`} key={`sec-${path}`}>
                        <button
                          type="button"
                          className="editor-tab-main"
                          onClick={() => {
                            setFocusedGroup("secondary");
                            setSecondaryActive(path);
                            setOutputEditorActive(false);
                          }}
                        >
                          {fileIcon(file.untitled ? file.name : file.path)}
                          <span>{file.name}</span>
                          {isDirtyFile(file) && <i />}
                        </button>
                        <button type="button" className="tab-close" aria-label={`Close ${file.name} in split`} onClick={() => closeSecondaryTab(path)}><X size={13} /></button>
                      </div>
                    );
                  })}
                  <div className="editor-run-actions">
                    <IconButton label="Close editor split" onClick={closeSplit}><X size={14} /></IconButton>
                  </div>
                </div>
                <EditorSurface
                  file={secondaryFile}
                  workspaceName={workspace?.name}
                  group="secondary"
                  pipelineView={secondaryPipelineView}
                  onPipelineView={setSecondaryPipelineView}
                  editorTheme={editorTheme}
                  fontSize={fontSize}
                  tabSize={tabSize}
                  wordWrap={wordWrap}
                  minimap={minimap}
                  beforeMount={beforeMount}
                  onChange={(value) => secondaryFile && updateContentFor(secondaryFile.path, value ?? "")}
                  output={secondaryFile ? jobLogText(latestJobForFile(jobs, secondaryFile.path)?.log) : ""}
                  cellOutputs={secondaryFile ? notebookCellOutputs[secondaryFile.path] : undefined}
                  running={secondaryFile ? runningJob?.file === secondaryFile.path : false}
                  onRun={() => runFileWithTrust(secondaryFile)}
                  onRunCell={(cellIndex) => runFileWithTrust(secondaryFile, cellIndex)}
                  onStop={stopActive}
                  onCellMount={notebookCellMounted as never}
                  onCellChange={notebookCellChanged as never}
                  onCellUnmount={notebookCellUnmounted as never}
                  onInvalidateCell={(cellIndex) => secondaryFile && invalidateNotebookCell(secondaryFile.path, cellIndex)}
                  onExportPreview={exportDataPreview}
                />
              </div>
            </div>
          ) : primaryFile || activeFile ? (
            <EditorSurface
              file={activeFile}
              workspaceName={workspace?.name}
              group="primary"
              pipelineView={pipelineView}
              onPipelineView={setPipelineView}
              editorTheme={editorTheme}
              fontSize={fontSize}
              tabSize={tabSize}
              wordWrap={wordWrap}
              minimap={minimap}
              beforeMount={beforeMount}
              onMount={editorMounted}
              onChange={updateContent}
              output={activeOutput}
              cellOutputs={activeFile ? notebookCellOutputs[activeFile.path] : undefined}
              running={activeFile ? runningJob?.file === activeFile.path : false}
              onRun={() => runFileWithTrust(activeFile)}
              onRunCell={(cellIndex) => runFileWithTrust(activeFile, cellIndex)}
              onStop={stopActive}
              onCellMount={notebookCellMounted as never}
              onCellChange={notebookCellChanged as never}
              onCellUnmount={notebookCellUnmounted as never}
              onInvalidateCell={(cellIndex) => activeFile && invalidateNotebookCell(activeFile.path, cellIndex)}
              onExportPreview={exportDataPreview}
            />
          ) : workspace ? (
            <div className="workspace-welcome compact">
              <Dna size={31} />
              <h1>{workspace.name}</h1>
              <p>{workspace.root}</p>
              <span>Open a file from the Explorer, or start something new</span>
              <div className="welcome-empty-actions">
                <button type="button" className="command-button primary" onClick={() => newUntitledFile()}><FilePlus2 size={15} />New BioLang file</button>
                <button type="button" className="command-button" onClick={() => void importWorkspaceFiles()}><Upload size={15} />Import data</button>
                <button type="button" className="command-button" onClick={() => openPalette("")}><Search size={15} />Go to file</button>
              </div>
              <section className="welcome-examples compact-examples" aria-label="BioLang examples">
                <header><strong>Or run a starter analysis</strong><span>Trusts the workspace and runs</span></header>
                <div>
                  {welcomeExamples.slice(0, 2).map((example) => <button type="button" key={example.id} onClick={() => void openWelcomeExample(example, { run: true })}>
                    <span className="welcome-example-icon">
                      {example.icon === "sequence" ? <Dna size={16} /> : example.icon === "table" ? <Braces size={16} /> : <FlaskConical size={16} />}
                    </span>
                    <span><strong>{example.name}</strong><small>{example.detail}</small></span>
                    <Play size={14} />
                  </button>)}
                </div>
              </section>
            </div>
          ) : (
            <div className="workspace-welcome">
              <span className="welcome-mark"><Dna size={32} /></span>
              <h1>{productName}</h1>
              <p>{isDesktop ? "Open a local folder to start a BioLang workspace." : "Open the browser workspace to edit locally, run with WebAssembly, or connect to SOMER."}</p>
              <div className="welcome-empty-actions">
                <button type="button" className="command-button primary" onClick={() => void openTutorialProject()}>
                  <GraduationCap size={15} />{isDesktop ? "Start tutorial" : "Open tutorial project"}
                </button>
                <button type="button" className="command-button" onClick={selectWorkspace}>
                  <FolderOpen size={15} />{isDesktop ? "Open Folder" : "Open Browser Workspace"}
                </button>
                {pwa.canInstall && <button type="button" className="command-button" onClick={() => void pwa.install()}><Download size={15} />Install App</button>}
              </div>
              {recentWorkspaces.length > 0 && <div className="recent-workspaces">
                <span>Recent</span>
                {recentWorkspaces.slice(0, 5).map((path) => <button type="button" key={path} onClick={() => void openRecentWorkspace(path)}><Folder size={13} /><span>{path.split(/[\\/]/).pop() || path}<small>{path}</small></span></button>)}
              </div>}
              <section className="welcome-examples" aria-label="BioLang examples">
                <header><strong>Start with an analysis</strong><span>{isDesktop ? "Opens a folder, trusts it, and runs" : "Opens and runs an unsaved BioLang file"}</span></header>
                <div>
                  {welcomeExamples.map((example) => <button type="button" key={example.id} onClick={() => void openWelcomeExample(example, { run: true })}>
                    <span className="welcome-example-icon">
                      {example.icon === "sequence" ? <Dna size={16} /> : example.icon === "table" ? <Braces size={16} /> : <FlaskConical size={16} />}
                    </span>
                    <span><strong>{example.name}</strong><small>{example.detail}</small></span>
                    <Play size={14} />
                  </button>)}
                </div>
              </section>
              <details className="welcome-comparison" aria-label="BioLang compared with Python and R">
                <summary>
                  <span><strong>Compare BioLang with Python and R</strong><small>{comparisonTask}</small></span>
                  <ChevronRight size={14} />
                </summary>
                <div className="comparison-tabs" role="tablist" aria-label="Language">
                  {comparisonVariants.map((variant) => (
                    <button
                      type="button"
                      role="tab"
                      key={variant.id}
                      aria-selected={comparisonLanguage === variant.id}
                      className={comparisonLanguage === variant.id ? "active" : ""}
                      onClick={() => setComparisonLanguage(variant.id)}
                    >{variant.label}</button>
                  ))}
                </div>
                {comparisonVariants
                  .filter((variant) => variant.id === comparisonLanguage)
                  .map((variant) => (
                    <div className="comparison-body" key={variant.id} role="tabpanel">
                      <pre>{variant.source.trimEnd()}</pre>
                      <footer>
                        <span>{lineCount(variant.source)} lines</span>
                        <span className={variant.id === "biolang" ? "comparison-clean" : ""}>
                          {variant.dependencies}
                        </span>
                        {variant.id === "biolang" && (
                          <button type="button" onClick={() => void openComparisonExample()}>
                            <Play size={12} />Run it
                          </button>
                        )}
                      </footer>
                    </div>
                  ))}
              </details>
              <div className="welcome-actions">
                <button type="button" onClick={() => setAboutOpen(true)}><BookOpen size={14} />About BioLang</button>
                <button type="button" onClick={() => openSettings()}><Settings size={14} />Settings</button>
              </div>
            </div>
          )}
          {workspace && experienceMode === "learner" && !learnerGuideDismissed && (
            <LearnerGuide
              progress={{
                hasWorkspace: Boolean(workspace),
                needsTrust: isDesktop && Boolean(workspace) && !workspaceTrusted,
                hasOpenFile: Boolean(activeFile),
                hasRun: jobs.length > 0,
                hasReadOutput: bottomVisible && bottomPanel === "output" && jobs.length > 0,
                problemCount: allProblems.length,
              }}
              onDismiss={() => setLearnerGuideDismissed(true)}
            />
          )}
        </main>

        {activity === "apis" && selectedApi && (
          <aside className="inspector-panel">
            <div className="inspector-heading"><BookOpen size={15} /><span>API reference</span></div>
            <div className="api-detail">
              <span className="api-service">{externalApiProvider(selectedApi.name)}</span>
              <h2>{selectedApi.name}</h2>
              <code>{selectedApi.signature}</code>
              <p>{selectedApi.summary}</p>
              {selectedApi.returnType && <span className="api-return">Returns {selectedApi.returnType}</span>}
              {/* Without this, running an example that needs a key produced a
                  rate-limit or auth error with nothing explaining the cause. */}
              {apiCredentialNotices.map((credential) => (
                <div className={`api-credential ${credential.required ? "required" : "advisory"}`} key={credential.name}>
                  <KeyRound size={13} />
                  <span>
                    <strong>{credential.required ? `${credential.label} key required` : `${credential.label} key recommended`}</strong>
                    <small>{credential.detail}</small>
                  </span>
                  <button type="button" onClick={() => openSettings("credentials")}>Add key</button>
                </div>
              ))}
              <div className="api-code"><span>Example</span><pre>{selectedApi.example}</pre></div>
              <button type="button" className="command-button primary" onClick={() => void testApiExample()} disabled={Boolean(runningJob) || !workspaceTrusted}>{runningJob ? <LoaderCircle size={14} className="spin" /> : <Play size={14} />}Run example</button>
              <button type="button" className="command-button primary" onClick={insertApiExample} disabled={!activeFile?.path.endsWith(".bl")}><FileCode2 size={14} />Insert into editor</button>
            </div>
          </aside>
        )}
        {activity === "apis" && selectedApi && <div
          className="pane-resizer inspector-resizer"
          role="separator"
          aria-label="Resize API inspector"
          aria-orientation="vertical"
          onPointerDown={(event) => startPaneResize("inspector", event)}
        />}

        {workspace && outputLocation === "right" && outputRightVisible && (
          <>
            <div
              className="pane-resizer output-resizer"
              role="separator"
              aria-label="Resize Output"
              aria-orientation="vertical"
              onPointerDown={(event) => startPaneResize("output", event)}
            />
            <aside className="output-side-panel">
              <OutputPane
                {...outputPaneProps}
                location="right"
              />
            </aside>
          </>
        )}

        {workspace && (bottomPanelShown || terminalMounted) && (
          <>
          {bottomPanelShown && <div
              className="pane-resizer panel-resizer"
              role="separator"
              aria-label="Resize bottom panel"
              aria-orientation="horizontal"
              onPointerDown={(event) => startPaneResize("panel", event)}
            />}
          <section className="bottom-panel" hidden={!bottomPanelShown}>
            <div className="panel-tabs">
              {availableBottomPanels.map((panel) => (
                <button type="button" key={panel} className={bottomPanel === panel ? "active" : ""} onClick={() => {
                  if (panel === "terminal") setTerminalMounted(true);
                  setBottomPanel(panel);
                }}>
                  {panel}{panel === "problems" && allProblems.length > 0 && <span className="count">{allProblems.length}</span>}
                </button>
              ))}
              <div className="panel-tools">
                {bottomPanel === "jobs" && <IconButton label="Sync SOMER history" onClick={() => void syncSomerHistory()}><RefreshCw size={13} /></IconButton>}
                <IconButton label={panelMaximized ? "Restore panel" : "Maximize panel"} active={panelMaximized} onClick={() => setPanelMaximized((value) => !value)}><ChevronDown size={14} className={panelMaximized ? "panel-restore-icon" : ""} /></IconButton>
                <IconButton label="Close panel" onClick={() => { setPanelMaximized(false); setBottomVisible(false); }}><X size={14} /></IconButton>
              </div>
            </div>
            <div className={`panel-content ${bottomPanel === "terminal" ? "terminal-panel-content" : ""}`}>
              {bottomPanel === "output" && <OutputPane
                {...outputPaneProps}
                location="bottom"
              />}
              {bottomPanel === "assignment" && assignment && <ErrorBoundary label="Assignment"><Suspense fallback={<div className="panel-loading"><LoaderCircle size={14} className="spin" />Loading tasks...</div>}><AssignmentPane
                assignment={assignment}
                progress={assignmentProgress}
                running={testRun?.status === "running"}
                canRun={isDesktop && Boolean(workspace) && workspaceTrusted}
                onRun={() => void runWorkspaceTests()}
                onSubmit={() => void exportSubmission()}
              /></Suspense></ErrorBoundary>}
              {bottomPanel === "tests" && <ErrorBoundary label="Tests"><Suspense fallback={<div className="panel-loading"><LoaderCircle size={14} className="spin" />Loading tests...</div>}><TestPane
                run={testRun}
                activeFile={activeFile?.path.endsWith(".bl") && !activeFile.untitled ? activeFile.path : undefined}
                canRun={isDesktop && Boolean(workspace) && workspaceTrusted}
                onRun={(path) => void runWorkspaceTests(path)}
                onOpenFailure={(file) => void openFile(file)}
              /></Suspense></ErrorBoundary>}
              {bottomPanel === "console" && workspace && workspaceTrusted && <ErrorBoundary label="BioLang Console"><Suspense fallback={<div className="panel-loading"><LoaderCircle size={14} className="spin" />Loading console...</div>}><ConsolePane
                workspaceRoot={workspace.root}
                editorTheme={editorTheme}
                fontSize={fontSize}
                tabSize={tabSize}
                beforeMount={beforeMount}
                onDocumentMount={notebookCellMounted}
                onDocumentChange={notebookCellChanged}
                onDocumentUnmount={notebookCellUnmounted}
                showNotice={showNotice}
                submission={consoleSubmission}
              /></Suspense></ErrorBoundary>}
              {terminalMounted && workspaceTrusted && (
                <div className="terminal-panel-surface" hidden={bottomPanel !== "terminal"}>
                  <TerminalManager />
                </div>
              )}
              {bottomPanel === "problems" && (allProblems.length ? allProblems.map((problem, index) => <button type="button" className={`problem-row severity-${problem.severity}`} key={`${problem.path}-${index}`} onClick={() => void openSearchHit({ path: problem.path, line: problem.line, column: problem.column, preview: problem.message })}>{problem.severity === 1 ? <AlertCircle size={14} /> : problem.severity === 2 ? <AlertTriangle size={14} /> : <Info size={14} />}<span>{problem.message}<small>{problem.path}:{problem.line}:{problem.column}</small></span></button>) : <EmptyState icon={<Check size={21} />} title="No problems detected" detail="Diagnostics and anything bl import left to port will appear here" />)}
              {bottomPanel === "jobs" && (jobs.length ? <div className="jobs-view"><div className="jobs-table">
                <div className="jobs-header"><span>Status</span><span>File</span><span>Target</span><span>Job</span><span>Duration</span><span /></div>
                {jobs.map((job) => <div className="jobs-row" key={job.id}>
                  <span><i className={`job-dot ${job.status}`} />{job.status}</span>
                  <span>{job.file}</span>
                  <span>{job.backend}</span>
                  <span title={job.id}>{job.remoteId ? job.remoteId.slice(0, 8) : job.id.replace("local:", "#")}</span>
                  <span>{job.durationMs ? `${(job.durationMs / 1000).toFixed(2)}s` : job.status === "running" ? "Running" : job.status === "staging" ? "Waiting for inputs" : "-"}</span>
                  <span className="job-actions"><IconButton label={`View logs for ${job.file}`} onClick={() => void selectJob(job)}><FileSearch size={12} /></IconButton><IconButton label={`Rerun ${job.file}`} disabled={Boolean(runningJob)} onClick={() => void rerunJob(job)}><RefreshCw size={12} /></IconButton></span>
                </div>)}
              </div>{selectedJob && <section className="job-log-view"><header><strong>{selectedJob.file}</strong><span>{selectedJob.backend} | {selectedJob.status}</span></header>{selectedJob.status === "running" && <div className="active-job-progress compact" role="status"><LoaderCircle size={13} className="spin" /><span><strong>Running</strong><small>Elapsed {formatElapsed(jobClock - selectedJob.startedAt)}</small></span><div className="progress-track" aria-hidden="true"><i /></div></div>}<JobLog chunks={selectedJob.log} /></section>}</div> : <EmptyState icon={<FlaskConical size={21} />} title="No jobs yet" detail="Runs are recorded in this workspace" />)}
            </div>
          </section>
          </>
        )}
      </div>

      <footer className="statusbar">
        <div><span className="remote-indicator">{executionTarget === "local" ? <HardDrive size={13} /> : <Server size={13} />}</span><label className="execution-target"><span className="sr-only">Execution target</span><select aria-label="Execution target" value={executionTarget} onChange={(event) => setExecutionTarget(event.target.value)}><option value="local">{isDesktop ? "Local" : "Browser WASM"}</option>{somerProfiles.map((profile) => <option value={profile.id} key={profile.id}>{profile.name}</option>)}</select><ChevronDown size={11} /></label>{!isDesktop && <span className={pwa.online ? "web-online" : "web-offline"}>{pwa.online ? "Online" : "Offline"}</span>}{gitStatus.available && <span title={`${gitStatus.files.length} changed files`}>{gitStatus.branch ?? "Git"}{gitStatus.files.length ? `*${gitStatus.files.length}` : ""}</span>}<span className={`status-health ${lspState}`} /> <span>BioLang {environment?.blVersion?.replace(/^bl\s*/i, "") ?? "detecting"}</span><span>{allProblems.length ? <><AlertCircle size={12} /> {allProblems.length}</> : <><Check size={12} /> 0 problems</>}</span></div>
        <div>
          {sequenceStats && <><span>{sequenceStats.length.toLocaleString()} bases</span><span>GC {sequenceStats.gcPercent.toFixed(1)}%</span><span>N {sequenceStats.n.toLocaleString()}</span></>}
          <span>{activeFile?.language === "biolang" ? "BioLang" : activeFile?.language ?? "Plain Text"}</span><span>UTF-8</span><span>Spaces: {tabSize}</span><span>{environment?.platform}</span>
        </div>
      </footer>

      {outputDragging && (
        <div className="output-dock-overlay">
          {([
            ["editor", <FileText size={23} />, "Editor"],
            ["right", <PanelRight size={23} />, "Right"],
            ["bottom", <PanelBottom size={23} />, "Bottom"],
          ] as Array<[OutputLocation, React.ReactNode, string]>).map(([location, icon, label]) => (
            <div
              key={location}
              className={`output-dock-target ${location} ${outputDragTarget === location ? "active" : ""}`}
              aria-label={`Dock Output in ${location}`}
              data-output-dock-location={location}
            >
              {icon}
              <strong>{label}</strong>
              <span>{location === "editor" ? "Open as an editor tab" : `Dock Output at the ${location}`}</span>
            </div>
          ))}
        </div>
      )}

      {openMenu && <button type="button" className="menu-dismiss" aria-label="Close menu" onClick={() => setOpenMenu(undefined)} />}

      {contextMenu && <>
        <button type="button" className="context-dismiss" aria-label="Close context menu" onClick={() => setContextMenu(undefined)} />
        <div
          className="context-menu"
          ref={contextMenuRef}
          role="menu"
          aria-label={explorerContextEntry
            ? `${explorerContextEntry.name} actions`
            : workspaceContext
              ? `${workspace?.name} workspace actions`
              : "Editor tab actions"}
          style={{ left: contextMenu.x, top: contextMenu.y }}
          onKeyDown={navigateContextMenu}
        >
          {explorerContextEntry ? <>
            {explorerContextEntry.kind === "file" && <>
              <button type="button" role="menuitem" onClick={() => { setContextMenu(undefined); void openFile(explorerContextEntry.path); }}><FileText size={13} />Open</button>
              {isRunnablePath(explorerContextEntry.path) && <button type="button" role="menuitem" disabled={!workspaceTrusted || Boolean(runningJob)} onClick={() => void runPath(explorerContextEntry.path)}><Play size={13} />Run</button>}
              <div className="menu-separator" />
            </>}
            {explorerContextEntry.kind === "directory" && <>
            <button type="button" role="menuitem" onClick={() => { newUntitledFile(explorerContextEntry.path); setContextMenu(undefined); }}><FilePlus2 size={13} />New File</button>
            <button type="button" role="menuitem" onClick={() => { setEntryPrompt({ mode: "directory", parent: explorerContextEntry.path, value: "" }); setContextMenu(undefined); }}><FolderPlus size={13} />New Folder...</button>
            <div className="menu-separator" />
          </>}
          <button type="button" role="menuitem" onClick={() => { setEntryPrompt({ mode: "rename", path: explorerContextEntry.path, value: explorerContextEntry.name }); setContextMenu(undefined); }}><Pencil size={13} />Rename...</button>
          <button type="button" role="menuitem" onClick={() => void bridge.duplicateEntry(explorerContextEntry.path).then(async (path) => {
            setContextMenu(undefined);
            await refreshWorkspace();
            if (explorerContextEntry.kind === "file") await openFile(path);
          }).catch((error) => showNotice(String(error)))}><Copy size={13} />Duplicate</button>
          <button type="button" role="menuitem" onClick={() => void copyWorkspacePath(explorerContextEntry.path)}><Copy size={13} />Copy Relative Path</button>
          <button type="button" role="menuitem" onClick={() => void bridge.revealEntry(explorerContextEntry.path).then(() => setContextMenu(undefined)).catch((error) => showNotice(String(error)))}><FolderOpen size={13} />Reveal in File Manager</button>
          <div className="menu-separator" />
          <button type="button" role="menuitem" className="danger" onClick={() => void deleteWorkspaceEntry(explorerContextEntry)}><Trash2 size={13} />Delete</button>
          </> : workspaceContext && workspace ? <>
            <button type="button" role="menuitem" onClick={() => { newUntitledFile(); setContextMenu(undefined); }}><FilePlus2 size={13} />New File</button>
            <button type="button" role="menuitem" onClick={() => { setEntryPrompt({ mode: "directory", value: "" }); setContextMenu(undefined); }}><FolderPlus size={13} />New Folder...</button>
            <button type="button" role="menuitem" onClick={() => { setContextMenu(undefined); void refreshWorkspace(); }}><RefreshCw size={13} />Refresh</button>
            <button type="button" role="menuitem" onClick={() => { setContextMenu(undefined); void importWorkspaceFiles(); }}><Upload size={13} />Import Data...</button>
            <div className="menu-separator" />
            <button type="button" role="menuitem" onClick={() => void copyWorkspacePath(workspace.root)}><Copy size={13} />Copy Workspace Path</button>
            <button type="button" role="menuitem" onClick={() => void bridge.revealEntry("").then(() => setContextMenu(undefined)).catch((error) => showNotice(String(error)))}><FolderOpen size={13} />Reveal in File Manager</button>
          </> : tabContextPath ? <>
            {isRunnableFile(tabContextFile) && <>
              <button type="button" role="menuitem" disabled={!workspaceTrusted || Boolean(runningJob)} onClick={() => void runPath(tabContextPath)}><Play size={13} />Run</button>
              <div className="menu-separator" />
            </>}
            <button type="button" role="menuitem" onClick={() => { openSplitRight(tabContextPath); setContextMenu(undefined); }}><Columns2 size={13} />Split Right</button>
            <button type="button" role="menuitem" onClick={() => { setContextMenu(undefined); void closeFile(tabContextPath); }}><X size={13} />Close</button>
            <button type="button" role="menuitem" disabled={openFiles.length <= 1} onClick={() => void closeTabGroup(tabContextPath, "others")}><Files size={13} />Close Others</button>
            <button type="button" role="menuitem" onClick={() => void closeTabGroup(tabContextPath, "all")}><X size={13} />Close All</button>
            <div className="menu-separator" />
            <button type="button" role="menuitem" disabled={tabContextFile?.untitled} onClick={() => void copyWorkspacePath(tabContextPath)}><Copy size={13} />Copy Relative Path</button>
            <button type="button" role="menuitem" disabled={tabContextFile?.untitled} onClick={() => void bridge.revealEntry(tabContextPath).then(() => setContextMenu(undefined)).catch((error) => showNotice(String(error)))}><FolderOpen size={13} />Reveal in File Manager</button>
          </> : null}
        </div>
      </>}

      {entryPrompt && <div className="dialog-backdrop" onMouseDown={() => setEntryPrompt(undefined)}>
        <form className="prompt-dialog" onMouseDown={(event) => event.stopPropagation()} onSubmit={(event) => { event.preventDefault(); void submitEntryPrompt(); }}>
          <div className="dialog-heading">
            <span>{entryPrompt.mode === "rename" ? "Rename" : entryPrompt.mode === "directory" ? "New Folder" : "Save Untitled File"}</span>
            <IconButton label="Close" onClick={() => setEntryPrompt(undefined)}><X size={14} /></IconButton>
          </div>
          <label htmlFor="entry-name">Name</label>
          <input id="entry-name" autoFocus value={entryPrompt.value} onChange={(event) => setEntryPrompt({ ...entryPrompt, value: event.target.value })} onFocus={(event) => event.currentTarget.select()} />
          <div className="dialog-actions">
            <button type="button" onClick={() => setEntryPrompt(undefined)}>Cancel</button>
            <button type="submit" className="primary" disabled={!entryPrompt.value.trim()}>{entryPrompt.mode === "rename" ? "Rename" : entryPrompt.mode === "save" ? "Save" : "Create"}</button>
          </div>
        </form>
      </div>}

      <SettingsDialog
        open={settingsOpen}
        onClose={() => setSettingsOpen(false)}
        initialSection={settingsSection}
        fontSize={fontSize}
        setFontSize={setFontSize}
        tabSize={tabSize}
        setTabSize={setTabSize}
        experienceMode={experienceMode}
        setExperienceMode={setExperienceMode}
        editorTheme={editorTheme}
        setEditorTheme={setEditorTheme}
        wordWrap={wordWrap}
        setWordWrap={setWordWrap}
        minimap={minimap}
        setMinimap={setMinimap}
        formatOnSave={formatOnSave}
        setFormatOnSave={setFormatOnSave}
        showInlineResults={showInlineResults}
        setShowInlineResults={setShowInlineResults}
        credentialStatuses={credentialStatuses}
        referenceBuilds={referenceBuilds}
        onSaveReferenceBuild={saveReferenceBuild}
        onDeleteReferenceBuild={deleteReferenceBuild}
        onSaveCredentialValue={saveCredentialValue}
        onForgetCredentialValue={forgetCredentialValue}
        bottomVisible={bottomVisible}
        setBottomVisible={setBottomVisible}
        hasWorkspace={Boolean(workspace)}
        workspaceTrusted={workspaceTrusted}
        onToggleTrust={() => void trustWorkspace(!workspaceTrusted)}
        somerProfiles={somerProfiles}
        setSomerProfiles={setSomerProfiles}
        executionTarget={executionTarget}
        setExecutionTarget={setExecutionTarget}
        somerTokens={somerTokens}
        setSomerTokens={setSomerTokens}
        connectionState={connectionState}
        onSaveCredential={saveSomerCredential}
        onForgetCredential={forgetSomerCredential}
        onTestConnection={testSomerConnection}
        keybindingOverrides={keybindingOverrides}
        setKeybindingOverrides={setKeybindingOverrides}
      />

      {aboutOpen && <div className="dialog-backdrop" onMouseDown={() => setAboutOpen(false)}>
        <section className="about-dialog" onMouseDown={(event) => event.stopPropagation()} aria-label={`About ${productName}`}>
          <div className="dialog-heading"><span>About</span><IconButton label="Close" onClick={() => setAboutOpen(false)}><X size={14} /></IconButton></div>
          <span className="welcome-mark"><Dna size={27} /></span>
          <h2>{productName}</h2>
          <p>{isDesktop ? "Local-first development environment for BioLang projects." : "Installable browser workspace with BioLang WebAssembly and remote SOMER execution."}</p>
          <dl><div><dt>{productEdition}</dt><dd>0.1.0</dd></div><div><dt>BioLang</dt><dd>{environment?.blVersion ?? "Not detected"}</dd></div><div><dt>Platform</dt><dd>{environment?.platform} {environment?.architecture}</dd></div></dl>
          <div className="about-citation">
            <strong>Citing BioLang</strong>
            <p>{apaReference()}</p>
            <div>
              <button type="button" onClick={() => { void bridge.copyText(apaReference()); showNotice("Copied the reference"); }}>Copy reference</button>
              <button type="button" onClick={() => { void bridge.copyText(bibtex()); showNotice("Copied BibTeX"); }}>Copy BibTeX</button>
            </div>
          </div>
        </section>
      </div>}

      {shortcutsOpen && <div className="dialog-backdrop" onMouseDown={() => setShortcutsOpen(false)}>
        <section className="shortcuts-dialog" onMouseDown={(event) => event.stopPropagation()} aria-label="Keyboard shortcuts">
          <div className="dialog-heading">
            <span>Keyboard Shortcuts</span>
            <div className="dialog-heading-actions">
              <button type="button" className="setting-command" onClick={() => { setShortcutsOpen(false); openSettings("keyboard"); }}>Edit</button>
              <IconButton label="Close" onClick={() => setShortcutsOpen(false)}><X size={14} /></IconButton>
            </div>
          </div>
          <table>
            <tbody>
              {[
                ["Help Center", shortcutLabel("help")],
                ["Command Palette", shortcutLabel("commandPalette")],
                ["Go to File", shortcutLabel("goToFile")],
                ["Go to Symbol", shortcutLabel("goToSymbol")],
                ["Settings", shortcutLabel("settings")],
                ["Save", shortcutLabel("save")],
                ["Save As", shortcutLabel("saveAs")],
                ["New File", shortcutLabel("newFile")],
                ["Close Editor", shortcutLabel("closeEditor")],
                ["Run Active File", shortcutLabel("run")],
                ["Send Selection to Console", shortcutLabel("sendToConsole")],
                ["Run Tests", shortcutLabel("runTests")],
                ["Source Control", shortcutLabel("scm")],
                ["Format Document", "Shift+Alt+F"],
                ["Rename Symbol", "F2"],
                ["Find All References", "Shift+F12"],
                ["Quick Fix", "Ctrl+."],
                ["Split Editor Right", shortcutLabel("splitEditor")],
                ["Toggle Panel", shortcutLabel("togglePanel")],
                ["Terminal", shortcutLabel("terminal")],
                ["BioLang Console", shortcutLabel("console")],
                ["Explorer", shortcutLabel("explorer")],
                ["Workspace Search", shortcutLabel("search")],
                ["Word Wrap", shortcutLabel("wordWrap")],
              ].map(([label, shortcut]) => <tr key={label}><th>{label}</th><td><kbd>{shortcut}</kbd></td></tr>)}
            </tbody>
          </table>
        </section>
      </div>}

      {importUrlOpen && (
        <ImportUrlDialog
          onClose={() => setImportUrlOpen(false)}
          onImport={importCodeFromUrl}
        />
      )}

      {codeImport && (
        <Suspense fallback={<div className="dialog-backdrop"><div className="dialog-loading"><LoaderCircle size={14} className="spin" />Loading importer...</div></div>}><ImportCodeDialog
          result={codeImport}
          directories={allDirectoryPaths}
          onClose={() => setCodeImport(undefined)}
          onValidate={bridge.validateImportCode}
          onSave={saveCodeImport}
        /></Suspense>
      )}

      {confirmation && <div className="dialog-backdrop" onMouseDown={() => settleConfirmation(false)}>
        <section className="confirmation-dialog" role="alertdialog" aria-modal="true" aria-labelledby="confirmation-title" onMouseDown={(event) => event.stopPropagation()}>
          <div className="dialog-heading">
            <span id="confirmation-title">{confirmation.title}</span>
            <IconButton label="Close" onClick={() => settleConfirmation(false)}><X size={14} /></IconButton>
          </div>
          <p>{confirmation.message}</p>
          <div className="dialog-actions">
            <button type="button" autoFocus onClick={() => settleConfirmation(false)}>Cancel</button>
            <button type="button" className={confirmation.danger ? "danger" : "primary"} onClick={() => settleConfirmation(true)}>{confirmation.confirmLabel}</button>
          </div>
        </section>
      </div>}

      {paletteOpen && (
        <div className="palette-backdrop" onMouseDown={() => setPaletteOpen(false)}>
          <div className="command-palette" onMouseDown={(event) => event.stopPropagation()}>
            <div className="palette-input">
              <Command size={16} />
              <input
                autoFocus
                role="combobox"
                aria-expanded
                aria-controls="palette-results"
                aria-activedescendant={commands[paletteIndex] ? `palette-option-${paletteIndex}` : undefined}
                aria-label={paletteMode === "command" ? "Run a command" : paletteMode === "symbol" ? "Go to symbol" : "Go to file"}
                value={paletteSearch}
                onChange={(event) => setPaletteSearch(event.target.value)}
                onKeyDown={onPaletteKeyDown}
                placeholder="Search files, > for commands, @ for symbols"
              />
            </div>
            <div className="palette-results" id="palette-results" role="listbox" ref={paletteListRef}>
              {commands.length === 0 ? (
                <p className="palette-empty">{paletteEmptyMessage(paletteMode, Boolean(workspace))}</p>
              ) : commands.map((command, index) => (
                <button
                  type="button"
                  id={`palette-option-${index}`}
                  role="option"
                  aria-selected={index === paletteIndex}
                  className={index === paletteIndex ? "selected" : undefined}
                  key={command.id}
                  onMouseMove={() => setPaletteIndex(index)}
                  onClick={() => runPaletteItem(command)}
                >
                  {command.icon}
                  <span>{highlightSegments(command.label, command.positions).map((segment, part) => (
                    segment.matched ? <mark key={part}>{segment.text}</mark> : <span key={part}>{segment.text}</span>
                  ))}</span>
                  {command.hint && <small>{command.hint}</small>}
                </button>
              ))}
            </div>
            <div className="palette-footer">
              <span><kbd>&uarr;</kbd><kbd>&darr;</kbd> to navigate</span>
              <span><kbd>Enter</kbd> to select</span>
              <span><kbd>Esc</kbd> to dismiss</span>
            </div>
          </div>
        </div>
      )}
      {notice && <div className={`toast toast-${notice.kind}`} role="status">{notice.message}</div>}
      {stickyNotices.length > 0 && (
        <div className="notice-stack" aria-label="Recent errors">
          {stickyNotices.map((entry) => (
            <div className="notice-stack-item" key={entry.id} role="alert">
              <AlertCircle size={13} />
              <span>{entry.message}</span>
              <button
                type="button"
                aria-label="Dismiss error"
                onClick={() => setStickyNotices((current) => current.filter((item) => item.id !== entry.id))}
              ><X size={12} /></button>
            </div>
          ))}
          {stickyNotices.length > 1 && (
            <button type="button" className="notice-stack-clear" onClick={() => setStickyNotices([])}>
              Clear all
            </button>
          )}
        </div>
      )}
    </div>
  );
}
