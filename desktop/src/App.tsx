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
  FolderOpen,
  FolderPlus,
  FlaskConical,
  Globe2,
  HardDrive,
  GraduationCap,
  LoaderCircle,
  Library,
  Info,
  Package,
  PanelBottom,
  Pencil,
  Play,
  Redo2,
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
import { lazy, Suspense, useCallback, useDeferredValue, useEffect, useMemo, useRef, useState } from "react";
import { bridge, isDesktop } from "./bridge";
import { ConsolePane } from "./components/ConsolePane";
import { FileTree, fileIcon } from "./components/FileTree";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { ImportCodeDialog, type ImportSaveRequest } from "./components/ImportCodeDialog";
import { ImportUrlDialog } from "./components/ImportUrlDialog";
import { JobLog } from "./components/JobLog";
import { SettingsDialog } from "./components/SettingsDialog";
import { TerminalManager } from "./components/TerminalManager";
import { useJobManager } from "./hooks/useJobManager";
import { useLspManager } from "./hooks/useLspManager";
import { usePwa } from "./hooks/usePwa";
import { loadRecoverySession, useSessionRecovery } from "./hooks/useSessionRecovery";
import { useWorkspaceManager } from "./hooks/useWorkspaceManager";
import { jobLogText, latestJobForFile } from "./jobLogs";
import { languageForPath } from "./language";
import type {
  Activity,
  BottomPanel,
  CodeImportResult,
  FileEntry,
  HelpEntry,
  HelpIndex,
  HelpKind,
  OpenFile,
  Problem,
  SearchHit,
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

function formatElapsed(durationMs: number): string {
  const totalSeconds = Math.max(0, Math.floor(durationMs / 1000));
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return minutes ? `${minutes}:${String(seconds).padStart(2, "0")}` : `${seconds}s`;
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

const productEdition = isDesktop ? "Desktop" : "Studio Web";
const productName = `BioLang ${productEdition}`;

export function App() {
  const [notice, setNotice] = useState<string>();
  const showNotice = useCallback((message: string) => {
    setNotice(message);
    window.setTimeout(() => setNotice(undefined), 2_800);
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
  const [activity, setActivity] = useState<Activity>("explorer");
  const [collapsedTreePaths, setCollapsedTreePaths] = useState<Set<string>>(() => new Set());
  const [bottomPanel, setBottomPanel] = useState<BottomPanel>("output");
  const [bottomVisible, setBottomVisible] = useStoredSetting("bottomVisible", false);
  const [problems, setProblems] = useState<Problem[]>([]);
  const [packageBusy, setPackageBusy] = useState(false);
  const [search, setSearch] = useState("");
  const [searchHits, setSearchHits] = useState<SearchHit[]>([]);
  const [searchBusy, setSearchBusy] = useState(false);
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
  const [openMenu, setOpenMenu] = useState<string>();
  const [contextMenu, setContextMenu] = useState<ContextMenuState>();
  const [entryPrompt, setEntryPrompt] = useState<EntryPrompt>();
  const [confirmation, setConfirmation] = useState<ConfirmationRequest>();
  const [codeImport, setCodeImport] = useState<CodeImportResult>();
  const [importUrlOpen, setImportUrlOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [shortcutsOpen, setShortcutsOpen] = useState(false);
  const [somerProfiles, setSomerProfiles] = useStoredSetting<SomerProfile[]>(
    "somerProfiles",
    defaultSomerProfiles,
  );
  const [executionTarget, setExecutionTarget] = useStoredSetting("executionTarget", "local");
  const [somerTokens, setSomerTokens] = useState<Record<string, string>>({});
  const [aboutOpen, setAboutOpen] = useState(false);
  const [minimap, setMinimap] = useStoredSetting("minimap", true);
  const [wordWrap, setWordWrap] = useStoredSetting("wordWrap", false);
  const [pipelineView, setPipelineView] = useState(false);
  const [fontSize, setFontSize] = useStoredSetting("fontSize", 13);
  const [tabSize, setTabSize] = useStoredSetting("tabSize", 2);
  const [experienceMode, setExperienceMode] = useStoredSetting<"learner" | "expert">(
    "experienceMode",
    "expert",
  );
  const [editorTheme, setEditorTheme] = useStoredSetting<"biolang-dark" | "vs-dark" | "hc-black">(
    "editorTheme",
    "biolang-dark",
  );
  const [sidebarWidth, setSidebarWidth] = useStoredSetting("sidebarWidth", 250);
  const [bottomPanelHeight, setBottomPanelHeight] = useStoredSetting("bottomPanelHeight", 246);
  const [inspectorWidth, setInspectorWidth] = useStoredSetting("inspectorWidth", 286);
  const [panelMaximized, setPanelMaximized] = useState(false);
  const [jobClock, setJobClock] = useState(() => Date.now());
  const contextMenuRef = useRef<HTMLDivElement>(null);
  const initialized = useRef(false);
  const untitledCounter = useRef(1);
  const pendingNavigation = useRef<{ path: string; line: number; column: number }>();

  const confirmAction = useCallback((request: Omit<ConfirmationRequest, "resolve">) =>
    new Promise<boolean>((resolve) => setConfirmation({ ...request, resolve })), []);
  const settleConfirmation = useCallback((confirmed: boolean) => {
    setConfirmation((current) => {
      current?.resolve(confirmed);
      return undefined;
    });
  }, []);

  const activeFile = openFiles.find((file) => file.path === activePath);
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
    recordDesktopTask,
  } = useJobManager({
    environment,
    workspaceTrusted,
    somerProfiles,
    somerTokens,
    executionTarget,
    openFiles,
    setOpenFiles,
    setActivePath,
    setBottomPanel,
    setBottomVisible,
    showNotice,
  });
  const activeOutputJob = useMemo(
    () => latestJobForFile(jobs, activeFile?.path),
    [activeFile?.path, jobs],
  );
  const activeOutput = useMemo(
    () => jobLogText(activeOutputJob?.log),
    [activeOutputJob?.log],
  );
  useEffect(() => {
    if (!runningJob) return;
    setJobClock(Date.now());
    const timer = window.setInterval(() => setJobClock(Date.now()), 1_000);
    return () => window.clearInterval(timer);
  }, [runningJob?.id]);

  const runActive = useCallback(async () => {
    await runFile(activeFile);
  }, [activeFile, runFile]);

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

  const openFile = useCallback(
    async (path: string) => {
      const existing = openFiles.find((file) => file.path === path);
      if (existing) {
        setActivePath(path);
        setPipelineView(false);
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
          setActivePath(path);
          setPipelineView(false);
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
        setActivePath(path);
        setPipelineView(false);
        await openDocument(path, content);
      } catch (error) {
        showNotice(String(error));
      }
    },
    [allFiles, openDocument, openFiles, showNotice],
  );

  useEffect(() => {
    if (initialized.current) return;
    initialized.current = true;
    void initializeWorkspace()
      .then((nextWorkspace) => {
        if (nextWorkspace) {
          const recovery = loadRecoverySession(nextWorkspace.root);
          const files = recovery.files;
          if (files.length) {
            setOpenFiles(files);
            setActivePath(restoredActivePath(files, recovery.activePath));
          }
        }
      })
      .catch((error) => showNotice(String(error)));
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

  useEffect(() => {
    contextMenuRef.current?.querySelector<HTMLButtonElement>('[role="menuitem"]')?.focus();
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
      void bridge.searchWorkspace(search)
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
  }, [search, showNotice, workspace]);

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

  const saveActive = useCallback(async () => {
    if (!activeFile || !isDirtyFile(activeFile)) return;
    if (activeFile.untitled) {
      promptUntitledSave(activeFile);
      return;
    }
    try {
      await bridge.writeFile(activeFile.path, activeFile.content);
      setOpenFiles((files) =>
        files.map((file) =>
          file.path === activeFile.path ? { ...file, savedContent: file.content } : file,
        ),
      );
      showNotice(`Saved ${activeFile.name}`);
      void refreshGitStatus();
    } catch (error) {
      showNotice(String(error));
    }
  }, [activeFile, promptUntitledSave, refreshGitStatus, showNotice]);

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
    },
    [activePath, closeDocument, confirmAction, openFiles],
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
      showNotice(String(error));
    }
  }, [activateWorkspace, chooseWorkspace, showNotice]);

  const openRecentWorkspace = useCallback(async (path: string) => {
    try {
      await activateWorkspace(await chooseRecentWorkspace(path));
    } catch (error) {
      showNotice(String(error));
    }
  }, [activateWorkspace, chooseRecentWorkspace, showNotice]);

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
    if (!activeOutput) {
      showNotice("There is no output to save");
      return;
    }
    try {
      const baseName = activeFile?.name.replace(/\.[^.]+$/, "") || "biolang";
      const destination = await bridge.exportText(`${baseName}-output.log`, activeOutput);
      if (destination) showNotice(`Saved output to ${destination}`);
    } catch (error) {
      showNotice(String(error));
    }
  }, [activeFile?.name, activeOutput, showNotice]);

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
      await navigator.clipboard.writeText(path);
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
    pane: "sidebar" | "panel" | "inspector",
    event: React.PointerEvent<HTMLDivElement>,
  ) => {
    event.preventDefault();
    const startX = event.clientX;
    const startY = event.clientY;
    const startSidebar = sidebarWidth;
    const startPanel = bottomPanelHeight;
    const startInspector = inspectorWidth;
    document.body.classList.add("resizing-pane");
    const onMove = (moveEvent: PointerEvent) => {
      if (pane === "sidebar") {
        setSidebarWidth(Math.max(180, Math.min(520, startSidebar + moveEvent.clientX - startX)));
      } else if (pane === "inspector") {
        setInspectorWidth(Math.max(220, Math.min(620, startInspector + startX - moveEvent.clientX)));
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
    setBottomPanelHeight,
    setInspectorWidth,
    setSidebarWidth,
    sidebarWidth,
  ]);

  const updateContent = useCallback((content = "") => {
    if (!activePath) return;
    setOpenFiles((files) =>
      files.map((file) => (file.path === activePath ? { ...file, content } : file)),
    );
    if (activePath.endsWith(".bl")) queueLspChange(activePath, content);
  }, [activePath, queueLspChange]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const mod = event.ctrlKey || event.metaKey;
      if (event.key === "F1") {
        event.preventDefault();
        setHelpSection("all");
        setActivity("help");
      } else if (mod && event.shiftKey && event.key.toLowerCase() === "s") {
        event.preventDefault();
        void saveActiveAs();
      } else if (mod && event.key.toLowerCase() === "s") {
        event.preventDefault();
        void saveActive();
      } else if (mod && event.key.toLowerCase() === "n") {
        event.preventDefault();
        if (workspace) newUntitledFile();
      } else if (mod && event.key.toLowerCase() === "w") {
        event.preventDefault();
        if (activePath) void closeFile(activePath);
      } else if (mod && event.key === "Enter") {
        if ((event.target as HTMLElement | null)?.closest(".console-pane")) return;
        event.preventDefault();
        void runActive();
      } else if (mod && event.shiftKey && event.key.toLowerCase() === "p") {
        event.preventDefault();
        setPaletteOpen(true);
      } else if (mod && event.key.toLowerCase() === "j") {
        event.preventDefault();
        setBottomVisible((visible) => !visible);
      } else if (mod && event.shiftKey && event.key.toLowerCase() === "e") {
        event.preventDefault();
        setActivity("explorer");
      } else if (mod && event.shiftKey && event.key.toLowerCase() === "f") {
        event.preventDefault();
        setActivity("search");
      } else if (mod && event.shiftKey && event.key.toLowerCase() === "o") {
        event.preventDefault();
        setPaletteSearch("symbol:");
        setPaletteOpen(true);
      } else if (mod && event.key === ",") {
        event.preventDefault();
        setSettingsOpen(true);
      } else if (event.altKey && event.key.toLowerCase() === "z") {
        event.preventDefault();
        setWordWrap((value) => !value);
      } else if (mod && event.shiftKey && event.code === "Backquote") {
        event.preventDefault();
        if (!workspace) showNotice("Open a workspace before starting the BioLang Console");
        else if (!workspaceTrusted) showNotice("Trust this workspace before starting the BioLang Console");
        else {
          setBottomPanel("console");
          setBottomVisible(true);
        }
      } else if (mod && event.code === "Backquote") {
        event.preventDefault();
        if (!workspace) showNotice("Open a workspace before starting a terminal");
        else if (!workspaceTrusted) showNotice("Trust this workspace before starting a terminal");
        else {
          setBottomPanel("terminal");
          setBottomVisible(true);
        }
      } else if (event.key === "Escape") {
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
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [
    activePath,
    closeFile,
    newUntitledFile,
    runActive,
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
    await executeFile(file, executionTarget);
    setBottomPanel("jobs");
    setBottomVisible(true);
  }, [
    executeFile,
    executionTarget,
    runningJob,
    selectedApi,
    setBottomVisible,
    showNotice,
    workspace,
    workspaceTrusted,
  ]);

  const openHelp = (section: HelpKind | "all" = "all") => {
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

  const openPanel = (panel: BottomPanel) => {
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
    setBottomPanel(panel);
    setBottomVisible(true);
  };

  const menuModels: Record<string, MenuItem[]> = {
    File: [
      { label: "New File", shortcut: "Ctrl+N", disabled: !workspace, action: () => newUntitledFile() },
      { label: "New Folder...", disabled: !workspace, action: () => setEntryPrompt({ mode: "directory", value: "" }) },
      { separator: true },
      { label: "Open Folder...", shortcut: "Ctrl+K Ctrl+O", action: selectWorkspace },
      { label: "Close Folder", disabled: !workspace, action: closeWorkspace },
      { separator: true },
      { label: "Import Script from File...", disabled: !workspace, action: importCodeSource },
      { label: "Import Script from URL...", disabled: !workspace, action: () => setImportUrlOpen(true) },
      { label: "Import Data...", disabled: !workspace, action: importWorkspaceFiles },
      { separator: true },
      { label: "Save", shortcut: "Ctrl+S", disabled: !activeFile || !isDirtyFile(activeFile), action: saveActive },
      { label: "Save As...", shortcut: "Ctrl+Shift+S", disabled: !activeFile || activeFile.viewer === "data", action: saveActiveAs },
      { label: "Save All", shortcut: "Ctrl+K S", disabled: !openFiles.some(isDirtyFile), action: saveAll },
      { separator: true },
      { label: "Close Editor", shortcut: "Ctrl+W", disabled: !activePath, action: () => activePath && closeFile(activePath) },
    ],
    Edit: [
      { label: "Undo", shortcut: "Ctrl+Z", disabled: !activeFile, action: () => editorCommand("undo") },
      { label: "Redo", shortcut: "Ctrl+Y", disabled: !activeFile, action: () => editorCommand("redo") },
      { separator: true },
      { label: "Cut", shortcut: "Ctrl+X", disabled: !activeFile, action: () => editorCommand("editor.action.clipboardCutAction") },
      { label: "Copy", shortcut: "Ctrl+C", disabled: !activeFile, action: () => editorCommand("editor.action.clipboardCopyAction") },
      { label: "Paste", shortcut: "Ctrl+V", disabled: !activeFile, action: () => editorCommand("editor.action.clipboardPasteAction") },
      { separator: true },
      { label: "Find", shortcut: "Ctrl+F", disabled: !activeFile, action: () => editorCommand("actions.find") },
      { label: "Replace", shortcut: "Ctrl+H", disabled: !activeFile, action: () => editorCommand("editor.action.startFindReplaceAction") },
      { label: "Go to Line...", shortcut: "Ctrl+G", disabled: !activeFile, action: () => editorCommand("editor.action.gotoLine") },
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
      { label: "Command Palette...", shortcut: "Ctrl+Shift+P", action: () => setPaletteOpen(true) },
      { separator: true },
      { label: "Explorer", shortcut: "Ctrl+Shift+E", checked: activity === "explorer", action: () => setActivity("explorer") },
      { label: "Expand All Explorer Folders", disabled: !workspace, action: expandAllTreeDirectories },
      { label: "Collapse All Explorer Folders", disabled: !workspace, action: collapseAllTreeDirectories },
      { label: "Search", shortcut: "Ctrl+Shift+F", checked: activity === "search", action: () => setActivity("search") },
      { label: "Packages", checked: activity === "packages", action: () => setActivity("packages") },
      { label: "Bio APIs", checked: activity === "apis", action: () => setActivity("apis") },
      { label: "Jobs", checked: activity === "jobs", action: () => setActivity("jobs") },
      { label: "Help Center", shortcut: "F1", checked: activity === "help", action: () => openHelp() },
      { separator: true },
      { label: "Bottom Panel", shortcut: "Ctrl+J", checked: bottomVisible, action: () => setBottomVisible((visible) => !visible) },
      { label: "Word Wrap", shortcut: "Alt+Z", checked: wordWrap, action: () => setWordWrap((value) => !value) },
      { label: "Minimap", checked: minimap, action: () => setMinimap((value) => !value) },
      { separator: true },
      { label: "Learner Mode", checked: experienceMode === "learner", action: () => setExperienceMode("learner") },
      { label: "Expert Mode", checked: experienceMode === "expert", action: () => setExperienceMode("expert") },
      { label: "Settings", action: () => setSettingsOpen(true) },
    ],
    Run: [
      { label: "Run Active File", shortcut: "Ctrl+Enter", disabled: !workspaceTrusted || (!activeFile?.path.endsWith(".bl") && activeFile?.viewer !== "notebook" && activeFile?.viewer !== "workflow") || Boolean(runningJob), action: runActive },
      { label: "Stop", shortcut: "Shift+F5", disabled: !runningJob, action: stopActive },
      { separator: true },
      { label: "Show Output", action: () => openPanel("output") },
      { label: "Show Jobs", action: () => openPanel("jobs") },
      { label: "BioLang Console", shortcut: "Ctrl+Shift+`", disabled: !workspace || !workspaceTrusted, action: () => openPanel("console") },
      { label: "New Terminal", shortcut: "Ctrl+`", disabled: !isDesktop || !workspace || !workspaceTrusted, action: () => openPanel("terminal") },
    ],
    Help: [
      { label: "Help Center", shortcut: "F1", action: () => openHelp() },
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

  const commands = [
    { label: "File: New File", icon: <FilePlus2 size={15} />, run: () => newUntitledFile() },
    { label: "File: Open Folder", icon: <FolderOpen size={15} />, run: selectWorkspace },
    { label: "File: Import Script from File", icon: <FileInput size={15} />, run: importCodeSource },
    { label: "File: Import Script from URL", icon: <Globe2 size={15} />, run: () => setImportUrlOpen(true) },
    { label: "File: Import Data", icon: <Upload size={15} />, run: importWorkspaceFiles },
    { label: "File: Save", icon: <Save size={15} />, run: saveActive },
    { label: "File: Save As", icon: <Save size={15} />, run: saveActiveAs },
    { label: "File: Save All", icon: <Save size={15} />, run: saveAll },
    { label: "BioLang: Run Active File", icon: <Play size={15} />, run: runActive },
    { label: "BioLang: Stop Running Job", icon: <CircleStop size={15} />, run: stopActive },
    { label: "View: Explorer", icon: <Files size={15} />, run: () => setActivity("explorer") },
    { label: "Explorer: Expand All Folders", icon: <ChevronsDown size={15} />, run: expandAllTreeDirectories },
    { label: "Explorer: Collapse All Folders", icon: <ChevronsUp size={15} />, run: collapseAllTreeDirectories },
    { label: "View: Search", icon: <Search size={15} />, run: () => setActivity("search") },
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
    { label: "View: Toggle Bottom Panel", icon: <PanelBottom size={15} />, run: () => setBottomVisible((value) => !value) },
    { label: "Preferences: Settings", icon: <Settings size={15} />, run: () => setSettingsOpen(true) },
    { label: "Help: Keyboard Shortcuts", icon: <Command size={15} />, run: () => setShortcutsOpen(true) },
    ...activeSymbols.map((symbol) => ({
      label: `Symbol: ${symbol.name} (${symbol.kind}, line ${symbol.line})`,
      icon: <Braces size={15} />,
      run: () => goToSymbol(symbol),
    })),
    ...openFiles.map((file) => ({
      label: `Open Editor: ${file.name} - ${file.path}`,
      icon: fileIcon(file.path),
      run: () => {
        setActivity("explorer");
        setActivePath(file.path);
      },
    })),
    ...recentWorkspaces.map((path) => ({
      label: `Recent Workspace: ${path}`,
      icon: <Folder size={15} />,
      run: () => openRecentWorkspace(path),
    })),
  ].filter((command) => command.label.toLowerCase().includes(paletteSearch.toLowerCase()));

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
          {!workspace ? <div className="sidebar-empty"><Search size={23} /><span>Open a folder to search</span></div> : <>
          <div className="search-field"><Search size={14} /><input autoFocus value={search} onChange={(event) => setSearch(event.target.value)} placeholder="Search file contents" /></div>
          <div className="result-count">{searchBusy ? "Searching..." : search.trim().length < 2 ? "Enter at least 2 characters" : `${searchHits.length}${searchHits.length === 200 ? "+" : ""} results`}</div>
          {searchHits.map((hit, index) => (
            <button className="search-result content-hit" type="button" key={`${hit.path}-${hit.line}-${index}`} onClick={() => void openSearchHit(hit)}>
              {fileIcon(hit.path)}<span>{hit.preview || "(blank line)"}<small>{hit.path}:{hit.line}:{hit.column}</small></span>
            </button>
          ))}</>}
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
        ) : jobs.map((job) => (
          <button type="button" className="job-sidebar-row" key={job.id} onClick={() => { setBottomPanel("jobs"); setBottomVisible(true); void selectJob(job); }}>
            <span className={`job-dot ${job.status}`} />
            <span>{job.file.split("/").pop()}<small>{job.backend} | {job.status}</small></span>
            <time>{job.durationMs ? `${(job.durationMs / 1000).toFixed(1)}s` : ""}</time>
          </button>
        ))}
      </>
    );
  };

  return (
    <div className={`app-shell ${experienceMode}-mode`}>
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
        </div>
        <button className="command-center" type="button" aria-label="Command center" onClick={() => setPaletteOpen(true)}><Search size={13} /><span>{experienceMode === "learner" ? "Search files, help, and commands" : workspace?.name ?? "Open a workspace"}</span><kbd>Ctrl Shift P</kbd></button>
        <div className="title-actions">
          <div className="experience-switch" role="group" aria-label="Interface mode">
            <button type="button" className={experienceMode === "learner" ? "active" : ""} aria-pressed={experienceMode === "learner"} onClick={() => setExperienceMode("learner")}>Learner</button>
            <button type="button" className={experienceMode === "expert" ? "active" : ""} aria-pressed={experienceMode === "expert"} onClick={() => setExperienceMode("expert")}>Expert</button>
          </div>
          <IconButton label="Open workspace" onClick={selectWorkspace}><FolderOpen size={15} /></IconButton>
          <IconButton label="Save file" onClick={saveActive} disabled={!activeFile || !isDirtyFile(activeFile)}><Save size={15} /></IconButton>
          {pwa.canInstall && <IconButton label="Install BioLang Studio Web" onClick={() => void pwa.install()}><Download size={15} /></IconButton>}
          <span className="toolbar-divider" />
          <IconButton label="Run active BioLang file" onClick={runActive} disabled={!workspaceTrusted || (!activeFile?.path.endsWith(".bl") && activeFile?.viewer !== "notebook" && activeFile?.viewer !== "workflow") || Boolean(runningJob)} className="run"><Play size={16} fill="currentColor" /></IconButton>
          <IconButton label="Stop running job" onClick={stopActive} disabled={!runningJob}><CircleStop size={16} /></IconButton>
          <IconButton label="Toggle bottom panel" active={bottomVisible} onClick={() => setBottomVisible((value) => !value)}><PanelBottom size={16} /></IconButton>
        </div>
      </header>

      <div
        className={`workbench ${bottomVisible ? "panel-open" : ""} ${panelMaximized ? "panel-maximized" : ""}`}
        style={{
          "--sidebar-width": `${sidebarWidth}px`,
          "--bottom-panel-height": `${bottomPanelHeight}px`,
          "--inspector-width": `${inspectorWidth}px`,
        } as React.CSSProperties}
      >
        <aside className="activity-bar" aria-label="Primary navigation">
          <div>
            <IconButton label="Explorer" active={activity === "explorer"} onClick={() => setActivity("explorer")}><Files size={21} /><span>Explorer</span></IconButton>
            <IconButton label="Search" active={activity === "search"} onClick={() => setActivity("search")}><Search size={21} /><span>Search</span></IconButton>
            <IconButton label="Packages" active={activity === "packages"} onClick={() => setActivity("packages")}><Blocks size={21} /><span>Packages</span></IconButton>
            <IconButton label="Bio APIs" active={activity === "apis"} onClick={() => setActivity("apis")}><Globe2 size={21} /><span>Bio APIs</span></IconButton>
            <IconButton label="Jobs" active={activity === "jobs"} onClick={() => setActivity("jobs")}><FlaskConical size={21} /><span>Jobs</span></IconButton>
            <IconButton label="Help Center" active={activity === "help"} onClick={() => openHelp()}><BookOpen size={21} /><span>Help</span></IconButton>
          </div>
          <div><IconButton label="Settings" onClick={() => setSettingsOpen(true)}><Settings size={21} /><span>Settings</span></IconButton></div>
        </aside>

        <aside className="sidebar">{renderSidebar()}</aside>
        <div
          className="pane-resizer sidebar-resizer"
          role="separator"
          aria-label="Resize sidebar"
          aria-orientation="vertical"
          onPointerDown={(event) => startPaneResize("sidebar", event)}
        />

        <main className={`editor-workspace ${workspace && !workspaceTrusted ? "restricted" : ""}`}>
          <div className="editor-tabs">
            {activity === "help" ? (
              <div className="editor-tab active help-tab"><BookOpen size={14} /><span>{selectedHelp?.title ?? "BioLang Help"}</span></div>
            ) : openFiles.map((file) => (
              <div
                className={`editor-tab ${file.path === activePath ? "active" : ""}`}
                key={file.path}
                onContextMenu={(event) => {
                  event.preventDefault();
                  setActivePath(file.path);
                  showContextMenu({ kind: "tab", path: file.path }, event.clientX, event.clientY);
                }}
              >
                <button type="button" className="editor-tab-main" onClick={() => setActivePath(file.path)}>
                  {fileIcon(file.untitled ? file.name : file.path)}<span>{file.name}</span>{isDirtyFile(file) && <i />}
                </button>
                <button type="button" className="tab-close" aria-label={`Close ${file.name}`} onClick={() => void closeFile(file.path)}><X size={13} /></button>
              </div>
            ))}
          </div>
          {workspace && !workspaceTrusted && <div className="trust-banner">
            <AlertCircle size={15} />
            <span><strong>Restricted mode</strong> Editing is enabled. Execution, terminals, packages, and language services are disabled.</span>
            <button type="button" onClick={() => trustWorkspace(true)}>Trust Workspace</button>
          </div>}
          {activity === "help" ? (
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
          ) : activeFile?.preview ? (
            <ErrorBoundary label="Data preview">
              <Suspense fallback={<div className="editor-loading">Preparing data preview...</div>}>
                <DataPreviewPane name={activeFile.name} path={activeFile.path} preview={activeFile.preview} onExport={exportDataPreview} />
              </Suspense>
            </ErrorBoundary>
          ) : activeFile?.viewer === "notebook" ? (
            <ErrorBoundary label="Notebook">
              <Suspense fallback={<div className="editor-loading">Preparing notebook...</div>}>
                <NotebookPane
                  name={activeFile.name}
                  path={activeFile.path}
                  content={activeFile.content}
                  output={activeOutput}
                  cellOutputs={notebookCellOutputs[activeFile.path] ?? {}}
                  running={runningJob?.file === activeFile.path}
                  editorTheme={editorTheme}
                  fontSize={fontSize}
                  tabSize={tabSize}
                  wordWrap={wordWrap}
                  beforeMount={beforeMount}
                  onChange={updateContent}
                  onRun={runActive}
                  onRunCell={(cellIndex) => runNotebookCell(activeFile, cellIndex)}
                  onStop={stopActive}
                  onCellMount={notebookCellMounted}
                  onCellChange={notebookCellChanged}
                  onCellUnmount={notebookCellUnmounted}
                  onInvalidateCell={(cellIndex) => invalidateNotebookCell(activeFile.path, cellIndex)}
                />
              </Suspense>
            </ErrorBoundary>
          ) : activeFile?.viewer === "workflow" ? (
            <ErrorBoundary label="Workflow editor">
              <Suspense fallback={<div className="editor-loading">Preparing workflow...</div>}>
                <WorkflowPane
                  content={activeFile.content}
                  running={runningJob?.file === activeFile.path}
                  onChange={updateContent}
                  onRun={runActive}
                  onStop={stopActive}
                />
              </Suspense>
            </ErrorBoundary>
          ) : activeFile ? (
            <>
              <div className="breadcrumbs"><span>{workspace?.name}</span><ChevronRight size={12} />{(activeFile.untitled ? [activeFile.name] : activeFile.path.split("/")).map((part, index, parts) => <span key={`${part}-${index}`}>{index > 0 && <ChevronRight size={12} />}{index === parts.length - 1 && fileIcon(activeFile.untitled ? activeFile.name : activeFile.path)}{part}</span>)}{activeFile.path.endsWith(".bl") && <button type="button" className={pipelineView ? "active" : ""} onClick={() => setPipelineView((value) => !value)}><Blocks size={12} />Pipeline</button>}</div>
              <div className="editor-host">
                {pipelineView ? <ErrorBoundary label="Pipeline viewer"><Suspense fallback={<div className="editor-loading">Inspecting pipeline...</div>}><PipelineViewer source={activeFile.content} onOpenSource={() => setPipelineView(false)} /></Suspense></ErrorBoundary>
                : <ErrorBoundary label="Code editor"><Suspense fallback={<div className="editor-loading">Loading editor...</div>}>
                  <CodeEditor
                    beforeMount={beforeMount}
                    onMount={editorMounted}
                    path={`file:///workspace/${activeFile.path}`}
                    language={activeFile.language}
                    value={activeFile.content}
                    onChange={updateContent}
                    theme={editorTheme}
                    options={{
                      automaticLayout: true,
                      fontFamily: '"Cascadia Code", "SFMono-Regular", Consolas, monospace',
                      fontSize,
                      lineHeight: 21,
                      minimap: { enabled: minimap, scale: 1, showSlider: "mouseover" },
                      scrollBeyondLastLine: false,
                      padding: { top: 14 },
                      renderLineHighlight: "gutter",
                      smoothScrolling: true,
                      tabSize,
                      wordWrap: wordWrap ? "on" : "off",
                      guides: { indentation: true, bracketPairs: true },
                      bracketPairColorization: { enabled: true },
                      quickSuggestions: true,
                      suggest: { showWords: false },
                    }}
                  />
                </Suspense></ErrorBoundary>}
              </div>
            </>
          ) : workspace ? (
            <div className="workspace-welcome compact">
              <Dna size={31} />
              <h1>{workspace.name}</h1>
              <p>{workspace.root}</p>
              <span>Select a file from the Explorer</span>
            </div>
          ) : (
            <div className="workspace-welcome">
              <span className="welcome-mark"><Dna size={32} /></span>
              <h1>{productName}</h1>
              <p>{isDesktop ? "Open a local folder to start a BioLang workspace." : "Open the browser workspace to edit locally, run with WebAssembly, or connect to SOMER."}</p>
              <button type="button" className="command-button primary" onClick={selectWorkspace}><FolderOpen size={15} />{isDesktop ? "Open Folder" : "Open Browser Workspace"}</button>
              {pwa.canInstall && <button type="button" className="command-button" onClick={() => void pwa.install()}><Download size={15} />Install App</button>}
              {recentWorkspaces.length > 0 && <div className="recent-workspaces">
                <span>Recent</span>
                {recentWorkspaces.slice(0, 5).map((path) => <button type="button" key={path} onClick={() => void openRecentWorkspace(path)}><Folder size={13} /><span>{path.split(/[\\/]/).pop() || path}<small>{path}</small></span></button>)}
              </div>}
              <div className="welcome-actions">
                <button type="button" onClick={() => setAboutOpen(true)}><BookOpen size={14} />About BioLang</button>
                <button type="button" onClick={() => setSettingsOpen(true)}><Settings size={14} />Settings</button>
              </div>
            </div>
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

        {bottomVisible && (
          <>
          <div
            className="pane-resizer panel-resizer"
            role="separator"
            aria-label="Resize bottom panel"
            aria-orientation="horizontal"
            onPointerDown={(event) => startPaneResize("panel", event)}
          />
          <section className="bottom-panel">
            <div className="panel-tabs">
              {(["problems", "output", "console", "terminal", "jobs"] as BottomPanel[]).map((panel) => (
                <button type="button" key={panel} className={bottomPanel === panel ? "active" : ""} onClick={() => setBottomPanel(panel)}>
                  {panel}{panel === "problems" && problems.length > 0 && <span className="count">{problems.length}</span>}
                </button>
              ))}
              <div className="panel-tools">
                {bottomPanel === "output" && <IconButton label="Save output" onClick={() => void exportOutput()}><Download size={13} /></IconButton>}
                {bottomPanel === "output" && <IconButton label="Clear output" onClick={() => {
                  if (activeOutputJob) clearJobLog(activeOutputJob.id);
                }}><Trash2 size={13} /></IconButton>}
                {bottomPanel === "jobs" && <IconButton label="Sync SOMER history" onClick={() => void syncSomerHistory()}><RefreshCw size={13} /></IconButton>}
                <IconButton label={panelMaximized ? "Restore panel" : "Maximize panel"} active={panelMaximized} onClick={() => setPanelMaximized((value) => !value)}><ChevronDown size={14} className={panelMaximized ? "panel-restore-icon" : ""} /></IconButton>
                <IconButton label="Close panel" onClick={() => { setPanelMaximized(false); setBottomVisible(false); }}><X size={14} /></IconButton>
              </div>
            </div>
            <div className="panel-content">
              {bottomPanel === "output" && <div className="output-run-view">
                {activeOutputJob && <header><strong>{activeFile?.name ?? activeOutputJob.file}</strong><span>Latest run | {activeOutputJob.backend} | {activeOutputJob.status}</span></header>}
                <div className="output-run-content">
                  {activeOutputJob?.status === "running" && <div className="active-job-progress" role="status" aria-live="polite">
                    <LoaderCircle size={14} className="spin" />
                    <span><strong>Running on {activeOutputJob.backend}</strong><small>Elapsed {formatElapsed(jobClock - activeOutputJob.startedAt)}</small></span>
                    <div className="progress-track" aria-hidden="true"><i /></div>
                  </div>}
                  <JobLog
                    className="output-view"
                    chunks={activeOutputJob?.log}
                    emptyText={activeOutputJob?.status === "running" ? "Waiting for output..." : "No output yet."}
                  />
                </div>
              </div>}
              {bottomPanel === "console" && workspace && workspaceTrusted && <ErrorBoundary label="BioLang Console"><ConsolePane
                workspaceRoot={workspace.root}
                editorTheme={editorTheme}
                fontSize={fontSize}
                tabSize={tabSize}
                beforeMount={beforeMount}
                onDocumentMount={notebookCellMounted}
                onDocumentChange={notebookCellChanged}
                onDocumentUnmount={notebookCellUnmounted}
                showNotice={showNotice}
              /></ErrorBoundary>}
              {bottomPanel === "terminal" && workspaceTrusted && <TerminalManager />}
              {bottomPanel === "problems" && (problems.length ? problems.map((problem, index) => <button type="button" className={`problem-row severity-${problem.severity}`} key={`${problem.path}-${index}`} onClick={() => void openFile(problem.path)}>{problem.severity === 1 ? <AlertCircle size={14} /> : problem.severity === 2 ? <AlertTriangle size={14} /> : <Info size={14} />}<span>{problem.message}<small>{problem.path}:{problem.line}:{problem.column}</small></span></button>) : <EmptyState icon={<Check size={21} />} title="No problems detected" detail="BioLang diagnostics will appear here" />)}
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
        <div><span className="remote-indicator">{executionTarget === "local" ? <HardDrive size={13} /> : <Server size={13} />}</span><label className="execution-target"><span className="sr-only">Execution target</span><select aria-label="Execution target" value={executionTarget} onChange={(event) => setExecutionTarget(event.target.value)}><option value="local">{isDesktop ? "Local" : "Browser WASM"}</option>{somerProfiles.map((profile) => <option value={profile.id} key={profile.id}>{profile.name}</option>)}</select><ChevronDown size={11} /></label>{!isDesktop && <span className={pwa.online ? "web-online" : "web-offline"}>{pwa.online ? "Online" : "Offline"}</span>}{gitStatus.available && <span title={`${gitStatus.files.length} changed files`}>{gitStatus.branch ?? "Git"}{gitStatus.files.length ? `*${gitStatus.files.length}` : ""}</span>}<span className={`status-health ${lspState}`} /> <span>BioLang {environment?.blVersion?.replace(/^bl\s*/i, "") ?? "detecting"}</span><span>{problems.length ? <><AlertCircle size={12} /> {problems.length}</> : <><Check size={12} /> 0 problems</>}</span></div>
        <div>
          {sequenceStats && <><span>{sequenceStats.length.toLocaleString()} bases</span><span>GC {sequenceStats.gcPercent.toFixed(1)}%</span><span>N {sequenceStats.n.toLocaleString()}</span></>}
          <span>{activeFile?.language === "biolang" ? "BioLang" : activeFile?.language ?? "Plain Text"}</span><span>UTF-8</span><span>Spaces: {tabSize}</span><span>{environment?.platform}</span>
        </div>
      </footer>

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
        bottomVisible={bottomVisible}
        setBottomVisible={setBottomVisible}
        hasWorkspace={Boolean(workspace)}
        workspaceTrusted={workspaceTrusted}
        onToggleTrust={() => trustWorkspace(!workspaceTrusted)}
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
      />

      {aboutOpen && <div className="dialog-backdrop" onMouseDown={() => setAboutOpen(false)}>
        <section className="about-dialog" onMouseDown={(event) => event.stopPropagation()} aria-label={`About ${productName}`}>
          <div className="dialog-heading"><span>About</span><IconButton label="Close" onClick={() => setAboutOpen(false)}><X size={14} /></IconButton></div>
          <span className="welcome-mark"><Dna size={27} /></span>
          <h2>{productName}</h2>
          <p>{isDesktop ? "Local-first development environment for BioLang projects." : "Installable browser workspace with BioLang WebAssembly and remote SOMER execution."}</p>
          <dl><div><dt>{productEdition}</dt><dd>0.1.0</dd></div><div><dt>BioLang</dt><dd>{environment?.blVersion ?? "Not detected"}</dd></div><div><dt>Platform</dt><dd>{environment?.platform} {environment?.architecture}</dd></div></dl>
        </section>
      </div>}

      {shortcutsOpen && <div className="dialog-backdrop" onMouseDown={() => setShortcutsOpen(false)}>
        <section className="shortcuts-dialog" onMouseDown={(event) => event.stopPropagation()} aria-label="Keyboard shortcuts">
          <div className="dialog-heading"><span>Keyboard Shortcuts</span><IconButton label="Close" onClick={() => setShortcutsOpen(false)}><X size={14} /></IconButton></div>
          <table>
            <tbody>
              {[
                ["Help Center", "F1"],
                ["Command Palette", "Ctrl+Shift+P"],
                ["Go to Symbol", "Ctrl+Shift+O"],
                ["Settings", "Ctrl+,"],
                ["Save", "Ctrl+S"],
                ["Save As", "Ctrl+Shift+S"],
                ["New File", "Ctrl+N"],
                ["Close Editor", "Ctrl+W"],
                ["Run Active File", "Ctrl+Enter"],
                ["Toggle Panel", "Ctrl+J"],
                ["Terminal", "Ctrl+`"],
                ["BioLang Console", "Ctrl+Shift+`"],
                ["Evaluate Console Input", "Ctrl+Enter"],
                ["Interrupt Console", "Ctrl+C"],
                ["Explorer", "Ctrl+Shift+E"],
                ["Workspace Search", "Ctrl+Shift+F"],
                ["Word Wrap", "Alt+Z"],
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
        <ImportCodeDialog
          result={codeImport}
          directories={allDirectoryPaths}
          onClose={() => setCodeImport(undefined)}
          onValidate={bridge.validateImportCode}
          onSave={saveCodeImport}
        />
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
            <div className="palette-input"><Command size={16} /><input autoFocus value={paletteSearch} onChange={(event) => setPaletteSearch(event.target.value)} placeholder="Type a command" /></div>
            <div className="palette-results">{commands.map((command) => <button type="button" key={command.label} onClick={() => { setPaletteOpen(false); void command.run(); }}>{command.icon}<span>{command.label}</span></button>)}</div>
          </div>
        </div>
      )}
      {notice && <div className="toast">{notice}</div>}
    </div>
  );
}
