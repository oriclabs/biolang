import {
  Blocks,
  Braces,
  ChevronDown,
  ChevronRight,
  Dna,
  File,
  FileCode2,
  FileJson,
  FileText,
  Folder,
  FolderOpen,
} from "lucide-react";
import { useMemo, useRef, useState, type DragEvent } from "react";
import type { FileEntry, GitFileStatus } from "../types";
import { VirtualList } from "./VirtualList";

export function fileIcon(path: string) {
  const extension = path.split(".").pop()?.toLowerCase();
  if (extension === "bl" || extension === "bln" || path.toLowerCase().endsWith(".bl.md")) {
    return <FileCode2 size={15} className="file-icon code" />;
  }
  if (extension === "blflow") return <Blocks size={15} className="file-icon bio" />;
  if (extension === "json") return <FileJson size={15} className="file-icon json" />;
  if (extension === "toml") return <Braces size={15} className="file-icon config" />;
  if (["fasta", "fa", "fna", "faa", "fastq", "fq", "vcf", "bed", "gff", "gff3", "gtf", "sam", "nwk", "newick", "tree", "pdb", "ent", "cif", "mmcif"].includes(extension ?? "")) {
    return <Dna size={15} className="file-icon bio" />;
  }
  if (extension === "md") return <FileText size={15} className="file-icon text" />;
  return <File size={15} className="file-icon" />;
}

type FlatRow = {
  entry: FileEntry;
  level: number;
};

const ROW_HEIGHT = 24;
const VIRTUALIZE_AFTER = 80;

/** Flatten expanded folders into visible rows (also used by unit tests). */
export function flattenVisible(
  entries: FileEntry[],
  collapsedPaths: ReadonlySet<string>,
  level = 0,
): FlatRow[] {
  const rows: FlatRow[] = [];
  for (const entry of entries) {
    rows.push({ entry, level });
    if (entry.kind === "directory" && !collapsedPaths.has(entry.path)) {
      rows.push(...flattenVisible(entry.children, collapsedPaths, level + 1));
    }
  }
  return rows;
}

function parentDirectory(path: string): string {
  const normalized = path.replaceAll("\\", "/");
  const index = normalized.lastIndexOf("/");
  return index >= 0 ? normalized.slice(0, index) : "";
}

function isSelfOrDescendant(source: string, candidate: string): boolean {
  return candidate === source || candidate.startsWith(`${source}/`);
}

export function FileTree({
  entries,
  activePath,
  onOpen,
  onContext,
  gitByPath,
  collapsedPaths,
  onToggleDirectory,
  onMove,
  onImportFiles,
}: {
  entries: FileEntry[];
  activePath?: string;
  onOpen: (path: string) => void;
  onContext: (entry: FileEntry, x: number, y: number) => void;
  gitByPath: Map<string, GitFileStatus>;
  collapsedPaths: ReadonlySet<string>;
  onToggleDirectory: (path: string) => void;
  /** Move a workspace entry into a directory ("" = workspace root). */
  onMove?: (sourcePath: string, destinationDirectory: string) => void | Promise<void>;
  /** Import OS files dropped onto a directory (or root when empty). */
  onImportFiles?: (destinationDirectory: string, files: FileList | File[]) => void | Promise<void>;
}) {
  const rows = useMemo(
    () => flattenVisible(entries, collapsedPaths),
    [collapsedPaths, entries],
  );
  const [dropTarget, setDropTarget] = useState<string | null>(null);
  const dragSource = useRef<string>();

  const clearDrop = () => setDropTarget(null);

  const acceptMove = (destinationDirectory: string, sourcePath: string) => {
    if (!onMove) return false;
    if (isSelfOrDescendant(sourcePath, destinationDirectory)) return false;
    if (parentDirectory(sourcePath) === destinationDirectory) return false;
    return true;
  };

  const onRowDragStart = (event: DragEvent, path: string) => {
    if (!onMove) return;
    dragSource.current = path;
    event.dataTransfer.setData("application/x-biolang-path", path);
    event.dataTransfer.setData("text/plain", path);
    event.dataTransfer.effectAllowed = "move";
  };

  const onRowDragOver = (event: DragEvent, destinationDirectory: string) => {
    const types = [...event.dataTransfer.types];
    const source = dragSource.current
      ?? (types.includes("application/x-biolang-path") ? dragSource.current : undefined);
    const hasOsFiles = types.includes("Files");
    if (source && acceptMove(destinationDirectory, source)) {
      event.preventDefault();
      event.dataTransfer.dropEffect = "move";
      setDropTarget(destinationDirectory);
      return;
    }
    if (hasOsFiles && onImportFiles) {
      event.preventDefault();
      event.dataTransfer.dropEffect = "copy";
      setDropTarget(destinationDirectory);
    }
  };

  const onRowDrop = (event: DragEvent, destinationDirectory: string) => {
    event.preventDefault();
    event.stopPropagation();
    clearDrop();
    const osFiles = event.dataTransfer.files;
    if (osFiles?.length && onImportFiles) {
      void onImportFiles(destinationDirectory, osFiles);
      dragSource.current = undefined;
      return;
    }
    const source = event.dataTransfer.getData("application/x-biolang-path")
      || event.dataTransfer.getData("text/plain")
      || dragSource.current;
    dragSource.current = undefined;
    if (!source || !onMove || !acceptMove(destinationDirectory, source)) return;
    void onMove(source, destinationDirectory);
  };

  const renderRow = (row: FlatRow) => {
    const { entry, level } = row;
    if (entry.kind === "directory") {
      const isCollapsed = collapsedPaths.has(entry.path);
      const changed = [...gitByPath.keys()].some((path) => path.startsWith(`${entry.path}/`));
      const isDrop = dropTarget === entry.path;
      return (
        <button
          type="button"
          className={`tree-row directory${isDrop ? " drop-target" : ""}`}
          data-path={entry.path}
          draggable={Boolean(onMove)}
          style={{ paddingLeft: 8 + level * 13 }}
          onClick={() => onToggleDirectory(entry.path)}
          onContextMenu={(event) => {
            event.preventDefault();
            onContext(entry, event.clientX, event.clientY);
          }}
          onDragStart={(event) => onRowDragStart(event, entry.path)}
          onDragEnd={clearDrop}
          onDragOver={(event) => onRowDragOver(event, entry.path)}
          onDragLeave={(event) => {
            if (event.currentTarget.contains(event.relatedTarget as Node)) return;
            if (dropTarget === entry.path) clearDrop();
          }}
          onDrop={(event) => onRowDrop(event, entry.path)}
        >
          {isCollapsed ? <ChevronRight size={13} /> : <ChevronDown size={13} />}
          {isCollapsed ? <Folder size={15} /> : <FolderOpen size={15} />}
          <span>{entry.name}</span>
          {changed && <i className="git-directory-dot" title="Contains Git changes" />}
        </button>
      );
    }

    const git = gitByPath.get(entry.path);
    const code = git
      ? git.indexStatus === "?" || git.worktreeStatus === "?"
        ? "U"
        : git.worktreeStatus.trim() || git.indexStatus.trim()
      : "";
    return (
      <button
        type="button"
        className={`tree-row ${entry.path === activePath ? "selected" : ""}`}
        data-path={entry.path}
        draggable={Boolean(onMove)}
        style={{ paddingLeft: 25 + level * 13 }}
        onClick={() => onOpen(entry.path)}
        onContextMenu={(event) => {
          event.preventDefault();
          onContext(entry, event.clientX, event.clientY);
        }}
        onDragStart={(event) => onRowDragStart(event, entry.path)}
        onDragEnd={clearDrop}
        onDragOver={(event) => onRowDragOver(event, parentDirectory(entry.path))}
        onDrop={(event) => onRowDrop(event, parentDirectory(entry.path))}
      >
        {fileIcon(entry.path)}
        <span>{entry.name}</span>
        {code && <i className={`git-status git-${code.toLowerCase()}`} title={`Git: ${git?.indexStatus}${git?.worktreeStatus}`}>{code}</i>}
      </button>
    );
  };

  const rootDropActive = dropTarget === "";

  return (
    <div
      className={`file-tree${rootDropActive ? " drop-target-root" : ""}`}
      onDragOver={(event) => onRowDragOver(event, "")}
      onDragLeave={(event) => {
        if (event.currentTarget.contains(event.relatedTarget as Node)) return;
        if (dropTarget === "") clearDrop();
      }}
      onDrop={(event) => onRowDrop(event, "")}
    >
      {rows.length > VIRTUALIZE_AFTER ? (
        <VirtualList
          count={rows.length}
          itemHeight={ROW_HEIGHT}
          height={Math.min(480, Math.max(160, rows.length * ROW_HEIGHT))}
          renderItem={(index) => renderRow(rows[index]!)}
          className="file-tree-virtual"
        />
      ) : rows.map((row) => (
        <div key={row.entry.path}>{renderRow(row)}</div>
      ))}
    </div>
  );
}
