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
import type { FileEntry, GitFileStatus } from "../types";

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

export function FileTree({
  entries,
  activePath,
  onOpen,
  onContext,
  gitByPath,
  collapsedPaths,
  onToggleDirectory,
  level = 0,
}: {
  entries: FileEntry[];
  activePath?: string;
  onOpen: (path: string) => void;
  onContext: (entry: FileEntry, x: number, y: number) => void;
  gitByPath: Map<string, GitFileStatus>;
  collapsedPaths: ReadonlySet<string>;
  onToggleDirectory: (path: string) => void;
  level?: number;
}) {
  return <div className="file-tree">
    {entries.map((entry) => {
      if (entry.kind === "directory") {
        const isCollapsed = collapsedPaths.has(entry.path);
        const changed = [...gitByPath.keys()].some((path) => path.startsWith(`${entry.path}/`));
        return <div key={entry.path}>
          <button
            type="button"
            className="tree-row directory"
            data-path={entry.path}
            style={{ paddingLeft: 8 + level * 13 }}
            onClick={() => onToggleDirectory(entry.path)}
            onContextMenu={(event) => {
              event.preventDefault();
              onContext(entry, event.clientX, event.clientY);
            }}
          >
            {isCollapsed ? <ChevronRight size={13} /> : <ChevronDown size={13} />}
            {isCollapsed ? <Folder size={15} /> : <FolderOpen size={15} />}
            <span>{entry.name}</span>
            {changed && <i className="git-directory-dot" title="Contains Git changes" />}
          </button>
          {!isCollapsed && <FileTree
            entries={entry.children}
            activePath={activePath}
            onOpen={onOpen}
            onContext={onContext}
            gitByPath={gitByPath}
            collapsedPaths={collapsedPaths}
            onToggleDirectory={onToggleDirectory}
            level={level + 1}
          />}
        </div>;
      }
      const git = gitByPath.get(entry.path);
      const code = git
        ? git.indexStatus === "?" || git.worktreeStatus === "?"
          ? "U"
          : git.worktreeStatus.trim() || git.indexStatus.trim()
        : "";
      return <button
        type="button"
        className={`tree-row ${entry.path === activePath ? "selected" : ""}`}
        data-path={entry.path}
        style={{ paddingLeft: 25 + level * 13 }}
        key={entry.path}
        onClick={() => onOpen(entry.path)}
        onContextMenu={(event) => {
          event.preventDefault();
          onContext(entry, event.clientX, event.clientY);
        }}
      >
        {fileIcon(entry.path)}
        <span>{entry.name}</span>
        {code && <i className={`git-status git-${code.toLowerCase()}`} title={`Git: ${git?.indexStatus}${git?.worktreeStatus}`}>{code}</i>}
      </button>;
    })}
  </div>;
}
