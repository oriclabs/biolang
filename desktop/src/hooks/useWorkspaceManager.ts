import { useCallback, useEffect, useState } from "react";
import { bridge, isDesktop } from "../bridge";
import type { EnvironmentInfo, GitStatusSnapshot, PackageInfo, WorkspaceSnapshot } from "../types";

function storedPaths(key: string): string[] {
  try {
    const raw = window.localStorage.getItem(`biolang.desktop.${key}`);
    const paths = raw ? JSON.parse(raw) as string[] : [];
    return Array.isArray(paths) ? paths.filter((path) => typeof path === "string") : [];
  } catch {
    return [];
  }
}

export function useWorkspaceManager(showNotice: (message: string) => void) {
  const [workspace, setWorkspace] = useState<WorkspaceSnapshot>();
  const [environment, setEnvironment] = useState<EnvironmentInfo>();
  const [packages, setPackages] = useState<PackageInfo[]>([]);
  const [gitStatus, setGitStatus] = useState<GitStatusSnapshot>({ available: false, files: [] });
  const [recentWorkspaces, setRecentWorkspaces] = useState<string[]>(() => storedPaths("recentWorkspaces"));
  const [trustedWorkspaces, setTrustedWorkspaces] = useState<string[]>(() => storedPaths("trustedWorkspaces"));
  const workspaceTrusted = !workspace || !isDesktop || trustedWorkspaces.includes(workspace.root);

  const remember = useCallback((root: string) => {
    setRecentWorkspaces((current) => [root, ...current.filter((path) => path !== root)].slice(0, 8));
  }, []);

  const restoreBackendTrust = useCallback(async (root: string) => {
    if (!isDesktop || !trustedWorkspaces.includes(root)) return;
    try {
      await bridge.setWorkspaceTrust(root, true);
    } catch (error) {
      setTrustedWorkspaces((current) => current.filter((path) => path !== root));
      showNotice(`Could not restore workspace trust: ${String(error)}`);
    }
  }, [showNotice, trustedWorkspaces]);

  const activate = useCallback(async (next: WorkspaceSnapshot) => {
    await restoreBackendTrust(next.root);
    setWorkspace(next);
    const [nextPackages, nextEnvironment, nextGitStatus] = await Promise.all([
      bridge.packages(),
      bridge.environment(),
      bridge.gitStatus(),
    ]);
    setPackages(nextPackages);
    setEnvironment(nextEnvironment);
    setGitStatus(nextGitStatus);
    remember(next.root);
  }, [remember, restoreBackendTrust]);

  const initialize = useCallback(async () => {
    const [next, nextEnvironment] = await Promise.all([bridge.workspace(), bridge.environment()]);
    setEnvironment(nextEnvironment);
    if (next) {
      await restoreBackendTrust(next.root);
      setWorkspace(next);
      const [nextPackages, nextGitStatus] = await Promise.all([bridge.packages(), bridge.gitStatus()]);
      setPackages(nextPackages);
      setGitStatus(nextGitStatus);
      remember(next.root);
    }
    return next ?? undefined;
  }, [remember, restoreBackendTrust]);

  const select = useCallback(() => bridge.selectWorkspace(), []);

  const openRecent = useCallback(async (path: string) => {
    try {
      return await bridge.openWorkspace(path);
    } catch (error) {
      setRecentWorkspaces((current) => current.filter((candidate) => candidate !== path));
      throw error;
    }
  }, []);

  const close = useCallback(async () => {
    await bridge.closeWorkspace();
    setWorkspace(undefined);
    setPackages([]);
    setGitStatus({ available: false, files: [] });
  }, []);

  const refresh = useCallback(async () => {
    const next = await bridge.workspace();
    setWorkspace(next ?? undefined);
    if (next) {
      const [nextPackages, nextGitStatus] = await Promise.all([bridge.packages(), bridge.gitStatus()]);
      setPackages(nextPackages);
      setGitStatus(nextGitStatus);
    } else {
      setPackages([]);
      setGitStatus({ available: false, files: [] });
    }
    return next ?? undefined;
  }, []);

  const refreshGit = useCallback(async () => {
    const next = await bridge.gitStatus();
    setGitStatus(next);
    return next;
  }, []);

  /**
   * Mark a workspace trusted (or revoke trust).
   *
   * `root` is optional so callers that just finished opening a folder can trust
   * it before React has re-rendered with the new workspace state — needed for
   * welcome-example "open and run" on Desktop.
   */
  const trust = useCallback(async (trusted: boolean, root?: string): Promise<boolean> => {
    const target = root ?? workspace?.root;
    if (!target) return false;
    if (isDesktop) {
      try {
        await bridge.setWorkspaceTrust(target, trusted);
      } catch (error) {
        showNotice(String(error));
        return false;
      }
    }
    setTrustedWorkspaces((current) => trusted
      ? [...new Set([...current, target])]
      : current.filter((path) => path !== target));
    return true;
  }, [showNotice, workspace?.root]);

  useEffect(() => {
    window.localStorage.setItem("biolang.desktop.recentWorkspaces", JSON.stringify(recentWorkspaces));
    window.localStorage.setItem("biolang.desktop.trustedWorkspaces", JSON.stringify(trustedWorkspaces));
  }, [recentWorkspaces, trustedWorkspaces]);

  return {
    workspace,
    environment,
    packages,
    gitStatus,
    recentWorkspaces,
    trustedWorkspaces,
    workspaceTrusted,
    setWorkspace,
    setEnvironment,
    setPackages,
    setRecentWorkspaces,
    setTrustedWorkspaces,
    initialize,
    activate,
    select,
    openRecent,
    close,
    refresh,
    refreshGit,
    trust,
  };
}
