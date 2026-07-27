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

  const activate = useCallback(async (next: WorkspaceSnapshot) => {
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
  }, [remember]);

  const initialize = useCallback(async () => {
    const [next, nextEnvironment] = await Promise.all([bridge.workspace(), bridge.environment()]);
    setEnvironment(nextEnvironment);
    if (next) {
      setWorkspace(next);
      const [nextPackages, nextGitStatus] = await Promise.all([bridge.packages(), bridge.gitStatus()]);
      setPackages(nextPackages);
      setGitStatus(nextGitStatus);
      remember(next.root);
    }
    return next ?? undefined;
  }, [remember]);

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

  const trust = useCallback((trusted: boolean) => {
    if (!workspace) return;
    setTrustedWorkspaces((current) => trusted
      ? [...new Set([...current, workspace.root])]
      : current.filter((path) => path !== workspace.root));
  }, [workspace]);

  useEffect(() => {
    window.localStorage.setItem("biolang.desktop.recentWorkspaces", JSON.stringify(recentWorkspaces));
    window.localStorage.setItem("biolang.desktop.trustedWorkspaces", JSON.stringify(trustedWorkspaces));
  }, [recentWorkspaces, trustedWorkspaces]);

  useEffect(() => {
    if (!workspace || !isDesktop) return;
    void bridge.setWorkspaceTrust(workspace.root, workspaceTrusted)
      .catch((error) => showNotice(String(error)));
  }, [showNotice, workspace, workspaceTrusted]);

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
