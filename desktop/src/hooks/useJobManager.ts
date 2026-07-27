import { SomerClient, type Job as SomerJob } from "@somer/client";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { bridge, onJobFinished, onJobOutput } from "../bridge";
import { appendJobLog, normalizeJobLog, remoteJobLog } from "../jobLogs";
import { languageForPath } from "../language";
import {
  createNotebookRunPlan,
  NotebookOutputRouter,
  type NotebookOutputChunk,
  type NotebookRunPlan,
} from "../notebooks";
import { snapshotDelta } from "../somerOutput";
import type {
  BottomPanel,
  EnvironmentInfo,
  Job,
  NotebookCellOutput,
  OpenFile,
  SomerProfile,
} from "../types";
import { viewerForPath } from "../viewers";
import { parseWorkflow, workflowToBioLang } from "../workflows";

const ansiPattern = new RegExp("\\u001b\\[[0-9;]*m", "g");

interface LocalJobContext {
  path: string;
  router?: NotebookOutputRouter;
  cellIndexes?: number[];
}

function storedJobs(): Job[] {
  try {
    const raw = window.localStorage.getItem("biolang.desktop.jobs");
    const jobs = raw ? JSON.parse(raw) as Array<Omit<Job, "log"> & { log?: unknown }> : [];
    return Array.isArray(jobs)
      ? jobs.map((job): Job => ({
          ...job,
          status: job.status === "running" ? "disconnected" : job.status,
          log: normalizeJobLog(job.log),
        }))
      : [];
  } catch {
    return [];
  }
}

export function useJobManager({
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
}: {
  environment: EnvironmentInfo | undefined;
  workspaceTrusted: boolean;
  somerProfiles: SomerProfile[];
  somerTokens: Record<string, string>;
  executionTarget: string;
  openFiles: OpenFile[];
  setOpenFiles: React.Dispatch<React.SetStateAction<OpenFile[]>>;
  setActivePath: React.Dispatch<React.SetStateAction<string | undefined>>;
  setBottomPanel: React.Dispatch<React.SetStateAction<BottomPanel>>;
  setBottomVisible: React.Dispatch<React.SetStateAction<boolean>>;
  showNotice: (message: string) => void;
}) {
  const [notebookCellOutputs, setNotebookCellOutputs] = useState<
    Record<string, Record<number, NotebookCellOutput>>
  >({});
  const [jobs, setJobs] = useState<Job[]>(storedJobs);
  const [selectedJobId, setSelectedJobId] = useState<string>();
  const [connectionState, setConnectionState] = useState<Record<string, string>>({});
  const localJobContexts = useRef(new Map<number, LocalJobContext>());
  const remotePollControllers = useRef(new Map<string, AbortController>());
  const workspaceGeneration = useRef(0);
  const newestJobId = jobs[0]?.id;
  const previousNewestJobId = useRef<string>();
  const runningJob = useMemo(() => jobs.find((job) => job.status === "running"), [jobs]);
  const selectedJob = useMemo(
    () => jobs.find((job) => job.id === selectedJobId),
    [jobs, selectedJobId],
  );

  const profileBaseUrl = useCallback(async (profile: SomerProfile) => {
    if (profile.connectionMode === "proxy") {
      if (!profile.proxyUrl?.trim()) throw new Error(`Enter a proxy URL for ${profile.name}`);
      return profile.proxyUrl.trim();
    }
    if (profile.connectionMode !== "ssh") return profile.baseUrl;
    if (!workspaceTrusted) throw new Error("Trust this workspace before starting an SSH tunnel");
    const remote = new URL(profile.baseUrl);
    if (remote.protocol !== "http:") {
      throw new Error("SSH tunnel profiles currently require an http SOMER service URL");
    }
    return bridge.startSomerTunnel({
      id: profile.id,
      sshHost: profile.sshHost?.trim() ?? "",
      sshUser: profile.sshUser?.trim() ?? "",
      sshPort: profile.sshPort ?? 22,
      remoteHost: remote.hostname,
      remotePort: Number(remote.port || "80"),
      identityFile: profile.sshIdentityFile?.trim() || undefined,
    });
  }, [workspaceTrusted]);

  const abortRemotePolling = useCallback(() => {
    workspaceGeneration.current += 1;
    for (const controller of remotePollControllers.current.values()) controller.abort();
    remotePollControllers.current.clear();
    setJobs((current) => current.map((job) =>
      job.remoteId && job.status === "running" ? { ...job, status: "disconnected" } : job));
  }, []);

  const clearJobLog = useCallback((jobId: string) => {
    setJobs((current) => current.map((job) =>
      job.id === jobId ? { ...job, log: [] } : job));
  }, []);

  const recordDesktopTask = useCallback((
    label: string,
    text: string,
    status: "succeeded" | "failed",
  ) => {
    const id = `desktop:${Date.now()}:${Math.random().toString(36).slice(2)}`;
    const job: Job = {
      id,
      file: label,
      status,
      startedAt: Date.now(),
      durationMs: 0,
      backend: "Desktop",
      targetId: "local",
      log: text ? [{ stream: status === "failed" ? "stderr" : "stdout", text }] : [],
    };
    setJobs((current) => [job, ...current]);
    setSelectedJobId(id);
    setBottomPanel("jobs");
    setBottomVisible(true);
  }, [setBottomPanel, setBottomVisible]);

  useEffect(() => {
    if (newestJobId && newestJobId !== previousNewestJobId.current) {
      setSelectedJobId(newestJobId);
    }
    previousNewestJobId.current = newestJobId;
  }, [newestJobId]);

  const appendNotebookChunks = useCallback((path: string, chunks: NotebookOutputChunk[]) => {
    if (!chunks.length) return;
    setNotebookCellOutputs((current) => {
      const outputs = { ...(current[path] ?? {}) };
      for (const chunk of chunks) {
        const previous = outputs[chunk.cellIndex] ?? { text: "", status: "running" as const };
        outputs[chunk.cellIndex] = { ...previous, text: previous.text + chunk.data };
      }
      return { ...current, [path]: outputs };
    });
  }, []);

  const beginNotebookRun = useCallback((path: string, plan: NotebookRunPlan) => {
    setNotebookCellOutputs((current) => {
      const outputs = { ...(current[path] ?? {}) };
      for (const cellIndex of plan.reportedCellIndexes) {
        outputs[cellIndex] = { text: "", status: "running" };
      }
      return { ...current, [path]: outputs };
    });
  }, []);

  const finishNotebookRun = useCallback((
    path: string,
    cellIndexes: number[],
    status: NotebookCellOutput["status"],
  ) => {
    setNotebookCellOutputs((current) => {
      const outputs = { ...(current[path] ?? {}) };
      for (const cellIndex of cellIndexes) {
        const previous = outputs[cellIndex] ?? { text: "", status };
        outputs[cellIndex] = { ...previous, status };
      }
      return { ...current, [path]: outputs };
    });
  }, []);

  const invalidateNotebookCell = useCallback((path: string, cellIndex: number) => {
    setNotebookCellOutputs((current) => {
      const previous = current[path]?.[cellIndex];
      if (!previous || previous.stale) return current;
      return {
        ...current,
        [path]: {
          ...current[path],
          [cellIndex]: { ...previous, stale: true },
        },
      };
    });
  }, []);

  useEffect(() => {
    window.localStorage.setItem("biolang.desktop.jobs", JSON.stringify(jobs.slice(0, 100)));
  }, [jobs]);

  useEffect(() => {
    let disposed = false;
    let unlistenOutput: () => void = () => undefined;
    let unlistenFinished: () => void = () => undefined;
    void onJobOutput((event) => {
      const data = event.data.replace(ansiPattern, "");
      const context = localJobContexts.current.get(event.jobId);
      const routed = context?.router
        ? event.stream === "stdout"
          ? context.router.stdout(data)
          : context.router.stderr(data)
        : { visible: data, chunks: [] };
      if (context) {
        appendNotebookChunks(context.path, routed.chunks);
        setJobs((current) => current.map((job) =>
          job.id === `local:${event.jobId}`
            ? { ...job, log: appendJobLog(job.log, event.stream, routed.visible) }
            : job));
      }
    }).then((dispose) => {
      if (disposed) dispose();
      else unlistenOutput = dispose;
    });
    void onJobFinished((event) => {
      const context = localJobContexts.current.get(event.jobId);
      if (context?.router) {
        const routed = context.router.flush();
        if (routed.visible) {
          setJobs((current) => current.map((job) =>
            job.id === `local:${event.jobId}`
              ? { ...job, log: appendJobLog(job.log, "stdout", routed.visible) }
              : job));
        }
        appendNotebookChunks(context.path, routed.chunks);
      }
      if (context?.cellIndexes) {
        finishNotebookRun(
          context.path,
          context.cellIndexes,
          event.exitCode === 0 ? "succeeded" : event.exitCode == null ? "cancelled" : "failed",
        );
      }
      setJobs((current) =>
        current.map((job) =>
          job.id === `local:${event.jobId}`
            ? (() => {
                const status = event.exitCode === 0
                  ? "succeeded"
                  : event.exitCode == null
                    ? "cancelled"
                    : "failed";
                const message = `\n${event.exitCode === 0
                  ? "Process completed"
                  : event.exitCode == null
                    ? "Process cancelled"
                    : `Process exited with code ${event.exitCode}`} in ${(event.durationMs / 1000).toFixed(2)}s.\n`;
                return {
                  ...job,
                  status,
                  exitCode: event.exitCode,
                  durationMs: event.durationMs,
                  log: appendJobLog(
                    job.log,
                    event.exitCode === 0 ? "success" : event.exitCode == null ? "system" : "stderr",
                    message,
                  ),
                };
              })()
            : job,
        ),
      );
      localJobContexts.current.delete(event.jobId);
    }).then((dispose) => {
      if (disposed) dispose();
      else unlistenFinished = dispose;
    });
    return () => {
      disposed = true;
      unlistenOutput();
      unlistenFinished();
    };
  }, [appendNotebookChunks, finishNotebookRun]);

  useEffect(() => () => {
    workspaceGeneration.current += 1;
    for (const controller of remotePollControllers.current.values()) controller.abort();
    remotePollControllers.current.clear();
  }, []);

  const executeFile = useCallback(async (
    file: OpenFile,
    targetId: string,
    selectedCellIndex?: number,
  ) => {
    const notebook = file.viewer === "notebook";
    const workflow = file.viewer === "workflow";
    if ((!file.path.endsWith(".bl") && !notebook && !workflow) || runningJob) return;
    if (!workspaceTrusted) {
      showNotice("Trust this workspace before executing code");
      return;
    }
    let notebookPlan: NotebookRunPlan | undefined;
    try {
      notebookPlan = notebook
        ? createNotebookRunPlan(
            file.content,
            `${Date.now()}_${Math.random().toString(36).slice(2)}`,
            selectedCellIndex,
          )
        : undefined;
      if (notebookPlan && !notebookPlan.reportedCellIndexes.length) {
        showNotice(selectedCellIndex == null ? "This notebook has no runnable cells" : "This cell is skipped");
        return;
      }
      if (notebookPlan) beginNotebookRun(file.path, notebookPlan);
      setBottomPanel("output");
      setBottomVisible(true);
      if (targetId === "local") {
        if (file.content !== file.savedContent) {
          await bridge.writeFile(file.path, file.content);
          setOpenFiles((files) => files.map((candidate) =>
            candidate.path === file.path ? { ...candidate, savedContent: candidate.content } : candidate));
        }
        const id = notebookPlan
          ? await bridge.runNotebookSource(notebookPlan.notebookSource)
          : workflow
            ? await bridge.runWorkflow(file.path)
            : await bridge.runFile(file.path);
        localJobContexts.current.set(id, {
          path: file.path,
          router: notebookPlan
            ? new NotebookOutputRouter(
                notebookPlan.markerPrefix,
                notebookPlan.cellIndexes,
                notebookPlan.hiddenOutputCellIndexes,
              )
            : undefined,
          cellIndexes: notebookPlan?.reportedCellIndexes,
        });
        setJobs((current) => [
          {
            id: `local:${id}`,
            file: file.path,
            status: "running",
            startedAt: Date.now(),
            backend: "Local",
            targetId: "local",
            cellIndex: selectedCellIndex,
            log: [],
          },
          ...current,
        ]);
        return;
      }
      const profile = somerProfiles.find((candidate) => candidate.id === targetId);
      if (!profile) throw new Error("The job's SOMER execution target is no longer configured");
      const token = somerTokens[profile.id]?.trim();
      if (!token) throw new Error(`Open Settings and enter a token for ${profile.name}`);
      const client = new SomerClient({ baseUrl: await profileBaseUrl(profile), token });
      const remoteSource = workflow
        ? workflowToBioLang(parseWorkflow(file.content))
        : notebookPlan
          ? notebookPlan.scriptSource
          : file.content;
      const submitted = await client.submitJob({
        name: file.name,
        entrypoint: workflow || notebook ? `${file.name}.generated.bl` : file.name,
        source: remoteSource,
        runtimeVersion: environment?.blVersion?.replace(/^bl\s*/i, ""),
        resources: { profile: profile.resourceProfile },
      });
      const id = `somer:${submitted.id}`;
      const startedAt = Date.now();
      const pollingGeneration = workspaceGeneration.current;
      const pollingController = new AbortController();
      remotePollControllers.current.set(id, pollingController);
      setJobs((current) => [
        {
          id,
          remoteId: submitted.id,
          file: file.path,
          status: "running",
          startedAt,
          backend: profile.name,
          targetId: profile.id,
          cellIndex: selectedCellIndex,
          log: [{ stream: "system", text: `Submitted ${file.name} to ${profile.name}.\n` }],
        },
        ...current,
      ]);
      let stdoutCursor = 0;
      let stderrCursor = 0;
      let terminalRecorded = false;
      const outputRouter = notebookPlan
        ? new NotebookOutputRouter(
            notebookPlan.markerPrefix,
            notebookPlan.cellIndexes,
            notebookPlan.hiddenOutputCellIndexes,
          )
        : undefined;
      void client.waitForJob(submitted.id, (remote: SomerJob) => {
        if (pollingController.signal.aborted || pollingGeneration !== workspaceGeneration.current) return;
        const stdoutSnapshot = snapshotDelta(remote.stdout, remote.stdoutOffset, stdoutCursor);
        const stderrSnapshot = snapshotDelta(remote.stderr, remote.stderrOffset, stderrCursor);
        const stdout = stdoutSnapshot.data.replace(ansiPattern, "");
        const stderr = stderrSnapshot.data.replace(ansiPattern, "");
        stdoutCursor = stdoutSnapshot.cursor;
        stderrCursor = stderrSnapshot.cursor;
        const routedStdout = outputRouter?.stdout(stdout) ?? { visible: stdout, chunks: [] };
        const routedStderr = outputRouter?.stderr(stderr) ?? { visible: stderr, chunks: [] };
        if (stdout || stderr) {
          appendNotebookChunks(file.path, [...routedStdout.chunks, ...routedStderr.chunks]);
        }
        const terminal = remote.status === "succeeded"
          || remote.status === "failed"
          || remote.status === "cancelled";
        const firstTerminal = terminal && !terminalRecorded;
        if (firstTerminal) terminalRecorded = true;
        const flushed = firstTerminal && outputRouter
          ? outputRouter.flush()
          : { visible: "", chunks: [] };
        if (firstTerminal) appendNotebookChunks(file.path, flushed.chunks);
        const desktopStatus: Job["status"] = terminal
          ? remote.status as Job["status"]
          : "running";
        setJobs((current) => current.map((job) => {
          if (job.id !== id) return job;
          let log = appendJobLog(job.log, "stdout", routedStdout.visible);
          log = appendJobLog(log, "stderr", routedStderr.visible);
          log = appendJobLog(log, "stdout", flushed.visible);
          if (firstTerminal) {
            log = appendJobLog(
              log,
              remote.status === "succeeded" ? "success" : remote.status === "failed" ? "stderr" : "system",
              `\nRemote job ${remote.status} on ${profile.name}.\n`,
            );
          }
          return {
            ...job,
            status: desktopStatus,
            exitCode: remote.exitCode,
            durationMs: terminal ? Date.now() - startedAt : undefined,
            log,
          };
        }));
        if (firstTerminal) {
          if (notebookPlan) {
            finishNotebookRun(file.path, notebookPlan.reportedCellIndexes, remote.status as NotebookCellOutput["status"]);
          }
          remotePollControllers.current.delete(id);
        }
      }, pollingController.signal).catch((error) => {
        remotePollControllers.current.delete(id);
        if (pollingController.signal.aborted || pollingGeneration !== workspaceGeneration.current) return;
        setJobs((current) => current.map((job) =>
          job.id === id ? {
            ...job,
            status: "failed",
            durationMs: Date.now() - startedAt,
            log: appendJobLog(job.log, "stderr", `\nSOMER connection failed: ${String(error)}\n`),
          } : job));
        if (notebookPlan) finishNotebookRun(file.path, notebookPlan.reportedCellIndexes, "failed");
      });
    } catch (error) {
      if (notebookPlan) finishNotebookRun(file.path, notebookPlan.reportedCellIndexes, "failed");
      const id = `desktop:${Date.now()}:${Math.random().toString(36).slice(2)}`;
      setJobs((current) => [{
        id,
        file: file.path,
        status: "failed",
        startedAt: Date.now(),
        durationMs: 0,
        backend: targetId === "local"
          ? "Local"
          : somerProfiles.find((profile) => profile.id === targetId)?.name ?? "SOMER",
        targetId,
        cellIndex: selectedCellIndex,
        log: [{ stream: "stderr", text: `${String(error)}\n` }],
      }, ...current]);
      setSelectedJobId(id);
      setBottomVisible(true);
    }
  }, [
    environment?.blVersion,
    appendNotebookChunks,
    beginNotebookRun,
    finishNotebookRun,
    profileBaseUrl,
    runningJob,
    setBottomPanel,
    setBottomVisible,
    setOpenFiles,
    showNotice,
    somerProfiles,
    somerTokens,
    workspaceTrusted,
  ]);

  const runFile = useCallback(async (file: OpenFile | undefined) => {
    if (file) await executeFile(file, executionTarget);
  }, [executeFile, executionTarget]);

  const runNotebookCell = useCallback(async (file: OpenFile, cellIndex: number) => {
    await executeFile(file, executionTarget, cellIndex);
  }, [executeFile, executionTarget]);

  const rerunJob = useCallback(async (job: Job) => {
    if (runningJob) return;
    try {
      let file = openFiles.find((candidate) => candidate.path === job.file);
      if (!file) {
        const content = await bridge.readFile(job.file);
        file = {
          path: job.file,
          name: job.file.split("/").pop() ?? job.file,
          content,
          savedContent: content,
          language: languageForPath(job.file),
          viewer: viewerForPath(job.file),
        };
        setOpenFiles((files) => [...files, file!]);
      }
      setActivePath(file.path);
      const targetId = job.targetId
        ?? (job.backend === "Local"
          ? "local"
          : somerProfiles.find((profile) => profile.name === job.backend)?.id);
      if (!targetId) throw new Error("The original execution target is no longer configured");
      await executeFile(file, targetId);
    } catch (error) {
      showNotice(String(error));
    }
  }, [executeFile, openFiles, runningJob, setActivePath, setOpenFiles, showNotice, somerProfiles]);

  const stopActive = useCallback(async () => {
    if (!runningJob) return;
    try {
      if (runningJob.remoteId) {
        const profile = somerProfiles.find((candidate) => candidate.id === runningJob.targetId);
        const token = profile && somerTokens[profile.id];
        if (!profile || !token) throw new Error("SOMER connection credentials are unavailable");
        await new SomerClient({ baseUrl: await profileBaseUrl(profile), token }).cancelJob(runningJob.remoteId);
      } else {
        await bridge.stopJob(Number(runningJob.id.replace("local:", "")));
      }
    } catch (error) {
      showNotice(String(error));
    }
  }, [profileBaseUrl, runningJob, showNotice, somerProfiles, somerTokens]);

  const testSomerConnection = useCallback(async (profile: SomerProfile) => {
    const token = somerTokens[profile.id]?.trim();
    if (!token) {
      setConnectionState((current) => ({ ...current, [profile.id]: "Token required" }));
      return;
    }
    setConnectionState((current) => ({ ...current, [profile.id]: "Connecting..." }));
    try {
      const client = new SomerClient({ baseUrl: await profileBaseUrl(profile), token });
      const [service, user] = await Promise.all([client.serviceInfo(), client.me()]);
      setConnectionState((current) => ({
        ...current,
        [profile.id]: `${service.name} ${service.version} as ${user.displayName}`,
      }));
    } catch (error) {
      setConnectionState((current) => ({ ...current, [profile.id]: String(error) }));
    }
  }, [profileBaseUrl, somerTokens]);

  const syncSomerHistory = useCallback(async () => {
    const configured = somerProfiles.filter((profile) => somerTokens[profile.id]?.trim());
    if (!configured.length) {
      showNotice("Add a SOMER credential before syncing remote history");
      return;
    }
    try {
      const imported = (await Promise.all(configured.map(async (profile) => {
        const token = somerTokens[profile.id].trim();
        const client = new SomerClient({ baseUrl: await profileBaseUrl(profile), token });
        const remoteJobs = await client.listJobs();
        return remoteJobs.map((remote): Job => {
          const startedAt = Date.parse(remote.startedAt ?? remote.createdAt);
          const finishedAt = remote.finishedAt ? Date.parse(remote.finishedAt) : undefined;
          return {
            id: `somer:${remote.id}`,
            remoteId: remote.id,
            file: remote.entrypoint || remote.name,
            status: remote.status === "queued" ? "running" : remote.status,
            startedAt: Number.isFinite(startedAt) ? startedAt : Date.now(),
            durationMs: finishedAt && Number.isFinite(startedAt) ? Math.max(0, finishedAt - startedAt) : undefined,
            exitCode: remote.exitCode,
            backend: profile.name,
            targetId: profile.id,
            log: remoteJobLog(remote.stdout, remote.stderr),
          };
        });
      }))).flat();
      setJobs((current) => {
        const importedIds = new Set(imported.map((job) => job.id));
        return [...imported, ...current.filter((job) => !importedIds.has(job.id))]
          .sort((left, right) => right.startedAt - left.startedAt)
          .slice(0, 100);
      });
      showNotice(`Synced ${imported.length} SOMER job${imported.length === 1 ? "" : "s"}`);
    } catch (error) {
      showNotice(`Cannot sync SOMER history: ${String(error)}`);
    }
  }, [profileBaseUrl, showNotice, somerProfiles, somerTokens]);

  const selectJob = useCallback(async (job: Job) => {
    setSelectedJobId(job.id);
    if (!job.remoteId || !job.targetId) return;
    const profile = somerProfiles.find((candidate) => candidate.id === job.targetId);
    const token = profile && somerTokens[profile.id]?.trim();
    if (!profile || !token) return;
    try {
      const remote = await new SomerClient({
        baseUrl: await profileBaseUrl(profile),
        token,
      }).getJob(job.remoteId);
      setJobs((current) => current.map((candidate) => candidate.id === job.id ? {
        ...candidate,
        status: remote.status === "queued" ? "running" : remote.status,
        exitCode: remote.exitCode,
        log: remoteJobLog(remote.stdout, remote.stderr),
      } : candidate));
    } catch (error) {
      showNotice(`Cannot refresh SOMER job: ${String(error)}`);
    }
  }, [profileBaseUrl, showNotice, somerProfiles, somerTokens]);

  return {
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
  };
}
