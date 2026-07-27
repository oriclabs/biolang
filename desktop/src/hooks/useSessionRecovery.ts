import { useEffect } from "react";
import type { OpenFile, WorkspaceSnapshot } from "../types";

interface RecoverySession {
  files: OpenFile[];
  activePath?: string;
}

const MAX_RECOVERY_FILES = 20;
const MAX_FILE_BYTES = 2_000_000;
const MAX_SESSION_BYTES = 4_000_000;

function recoveryKey(root: string) {
  return `biolang.desktop.recovery.${root}`;
}

export function loadRecoverySession(root: string): RecoverySession {
  try {
    const raw = window.localStorage.getItem(recoveryKey(root));
    const stored = raw ? JSON.parse(raw) as RecoverySession : { files: [] };
    const files = Array.isArray(stored.files)
      ? stored.files
          .filter((file) => file && !file.preview && typeof file.content === "string")
          .filter((file) => file.content.length <= MAX_FILE_BYTES)
          .slice(0, MAX_RECOVERY_FILES)
      : [];
    return {
      files,
      activePath: files.some((file) => file.path === stored.activePath)
        ? stored.activePath
        : files.at(-1)?.path,
    };
  } catch {
    return { files: [] };
  }
}

export function useSessionRecovery(
  workspace: WorkspaceSnapshot | undefined,
  openFiles: OpenFile[],
  activePath: string | undefined,
) {
  useEffect(() => {
    if (!workspace) return;
    const files = openFiles
      .filter((file) => !file.preview)
      .filter((file) => file.content.length <= MAX_FILE_BYTES)
      .slice(0, MAX_RECOVERY_FILES);
    if (files.reduce((sum, file) => sum + file.content.length, 0) > MAX_SESSION_BYTES) return;

    const timer = window.setTimeout(() => {
      window.localStorage.setItem(
        recoveryKey(workspace.root),
        JSON.stringify({ files, activePath } satisfies RecoverySession),
      );
    }, 300);
    return () => window.clearTimeout(timer);
  }, [activePath, openFiles, workspace]);
}
