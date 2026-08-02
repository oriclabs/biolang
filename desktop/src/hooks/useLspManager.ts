import type { Monaco, OnMount } from "@monaco-editor/react";
import { useCallback, useEffect, useRef, useState } from "react";
import type * as MonacoEditor from "monaco-editor";
import { registerBrowserIntelligence } from "../browserIntelligence";
import { isDesktop } from "../bridge";
import { registerBioLang } from "../language";
import { BioLangLspClient } from "../lsp";
import { replaceProblemsForPath } from "../lspProtocol";
import type { OpenFile, Problem, WorkspaceSnapshot } from "../types";

type LspState = "off" | "starting" | "ready" | "error";

export const notebookVirtualRoot = "__notebook__/";

function isBioLangDocument(path: string) {
  return path.endsWith(".bl")
    || path.startsWith("__untitled__/")
    || path.startsWith(notebookVirtualRoot);
}

export function useLspManager({
  workspace,
  trusted,
  openFiles,
  setProblems,
}: {
  workspace: WorkspaceSnapshot | undefined;
  trusted: boolean;
  openFiles: OpenFile[];
  setProblems: React.Dispatch<React.SetStateAction<Problem[]>>;
}) {
  const [state, setState] = useState<LspState>("off");
  const monacoRef = useRef<Monaco>();
  const editorRef = useRef<MonacoEditor.editor.IStandaloneCodeEditor>();
  const clientRef = useRef(new BioLangLspClient());
  const versionsRef = useRef(new Map<string, number>());
  const changeTimersRef = useRef(new Map<string, number>());
  const openedRef = useRef(new Set<string>());
  const notebookDocumentsRef = useRef(new Map<string, string>());
  const providerDisposablesRef = useRef<MonacoEditor.IDisposable[]>([]);
  const browserProviderDisposablesRef = useRef<MonacoEditor.IDisposable[]>([]);
  const startPromiseRef = useRef<Promise<boolean>>();
  const lifecycleRef = useRef(0);

  const clearDocument = useCallback((path: string) => {
    const timer = changeTimersRef.current.get(path);
    if (timer != null) window.clearTimeout(timer);
    changeTimersRef.current.delete(path);
    versionsRef.current.delete(path);
  }, []);

  const resetTracking = useCallback(() => {
    for (const timer of changeTimersRef.current.values()) window.clearTimeout(timer);
    changeTimersRef.current.clear();
    versionsRef.current.clear();
  }, []);

  const queueChange = useCallback((path: string, content: string, immediate = false) => {
    if (!clientRef.current.isReady() || !openedRef.current.has(path)) return;
    const version = (versionsRef.current.get(path) ?? 1) + 1;
    versionsRef.current.set(path, version);
    const previous = changeTimersRef.current.get(path);
    if (previous != null) window.clearTimeout(previous);
    changeTimersRef.current.delete(path);
    if (immediate) {
      void clientRef.current.change(path, content, version);
      return;
    }
    const timer = window.setTimeout(() => {
      changeTimersRef.current.delete(path);
      void clientRef.current.change(path, content, version);
    }, 250);
    changeTimersRef.current.set(path, timer);
  }, []);

  const openDocument = useCallback(async (path: string, content: string) => {
    if (!isBioLangDocument(path) || !clientRef.current.isReady()) return;
    if (openedRef.current.has(path)) {
      queueChange(path, content, true);
      return;
    }
    await clientRef.current.open(path, content);
    openedRef.current.add(path);
    versionsRef.current.set(path, 1);
  }, [queueChange]);

  const closeDocument = useCallback((path: string) => {
    if (openedRef.current.has(path)) {
      void clientRef.current.close(path);
      openedRef.current.delete(path);
    }
    clearDocument(path);
  }, [clearDocument]);

  const disposeProviders = useCallback(() => {
    for (const disposable of providerDisposablesRef.current) disposable.dispose();
    providerDisposablesRef.current = [];
  }, []);

  const stop = useCallback((files: OpenFile[] = []) => {
    lifecycleRef.current += 1;
    for (const file of files) closeDocument(file.path);
    clientRef.current.stop();
    disposeProviders();
    openedRef.current.clear();
    resetTracking();
    startPromiseRef.current = undefined;
    setState("off");
  }, [closeDocument, disposeProviders, resetTracking]);

  const beforeMount = useCallback((monaco: Monaco) => {
    registerBioLang(monaco);
    monacoRef.current = monaco;
  }, []);

  const ensureBrowserIntelligence = useCallback((monaco: Monaco) => {
    if (isDesktop || browserProviderDisposablesRef.current.length) return;
    browserProviderDisposablesRef.current = registerBrowserIntelligence(monaco);
    setState("ready");
  }, []);

  const startClient = useCallback(async (monaco: Monaco) => {
    monacoRef.current = monaco;
    if (clientRef.current.isReady()) return true;
    if (startPromiseRef.current) return startPromiseRef.current;
    if (!workspace || !trusted || !isDesktop) return false;

    const lifecycle = lifecycleRef.current;
    setState("starting");
    const startPromise = (async () => {
      try {
        const started = await clientRef.current.start(workspace.root, monaco, (path, nextProblems) => {
          if (path.startsWith(notebookVirtualRoot)) return;
          setProblems((current) => replaceProblemsForPath(current, path, nextProblems));
        });
        if (!started || lifecycle !== lifecycleRef.current) {
          if (lifecycle === lifecycleRef.current) setState("error");
          return false;
        }
        disposeProviders();
        providerDisposablesRef.current = [
          monaco.languages.registerCompletionItemProvider("biolang", clientRef.current.completionProvider()),
          monaco.languages.registerHoverProvider("biolang", clientRef.current.hoverProvider()),
          monaco.languages.registerSignatureHelpProvider("biolang", clientRef.current.signatureHelpProvider()),
          monaco.languages.registerDefinitionProvider("biolang", clientRef.current.definitionProvider()),
          monaco.languages.registerReferenceProvider("biolang", clientRef.current.referenceProvider()),
          monaco.languages.registerRenameProvider("biolang", clientRef.current.renameProvider()),
          monaco.languages.registerDocumentFormattingEditProvider("biolang", clientRef.current.documentFormattingProvider()),
          monaco.languages.registerCodeActionProvider("biolang", clientRef.current.codeActionProvider()),
        ];
        setState("ready");
        for (const file of openFiles) await openDocument(file.path, file.content);
        for (const [path, content] of notebookDocumentsRef.current) {
          await openDocument(path, content);
        }
        return true;
      } catch {
        if (lifecycle === lifecycleRef.current) setState("error");
        return false;
      } finally {
        if (lifecycle === lifecycleRef.current) startPromiseRef.current = undefined;
      }
    })();
    startPromiseRef.current = startPromise;
    return startPromise;
  }, [disposeProviders, openDocument, openFiles, setProblems, trusted, workspace]);

  const editorMounted: OnMount = useCallback(async (editor, monaco) => {
    editorRef.current = editor;
    ensureBrowserIntelligence(monaco);
    editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.Space, () => {
      void editor.getAction("editor.action.triggerSuggest")?.run();
    });
    await startClient(monaco);
  }, [ensureBrowserIntelligence, startClient]);

  const notebookCellMounted = useCallback(async (path: string, content: string, monaco: Monaco) => {
    notebookDocumentsRef.current.set(path, content);
    ensureBrowserIntelligence(monaco);
    const started = await startClient(monaco);
    if (started) await openDocument(path, content);
  }, [ensureBrowserIntelligence, openDocument, startClient]);

  const notebookCellChanged = useCallback((path: string, content: string) => {
    notebookDocumentsRef.current.set(path, content);
    queueChange(path, content);
  }, [queueChange]);

  const notebookCellUnmounted = useCallback((path: string) => {
    notebookDocumentsRef.current.delete(path);
    closeDocument(path);
  }, [closeDocument]);

  useEffect(() => {
    if (!trusted || !monacoRef.current || clientRef.current.isReady()) return;
    void startClient(monacoRef.current);
  }, [startClient, trusted]);

  useEffect(() => {
    return () => {
      lifecycleRef.current += 1;
      for (const timer of changeTimersRef.current.values()) window.clearTimeout(timer);
      changeTimersRef.current.clear();
      disposeProviders();
      clientRef.current.stop();
    };
  }, [disposeProviders]);

  return {
    state,
    editorRef,
    beforeMount,
    editorMounted,
    openDocument,
    closeDocument,
    clearDocument,
    queueChange,
    stop,
    notebookCellMounted,
    notebookCellChanged,
    notebookCellUnmounted,
  };
}
