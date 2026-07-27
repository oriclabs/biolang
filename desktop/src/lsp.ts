import type { Monaco } from "@monaco-editor/react";
import type * as MonacoEditor from "monaco-editor";
import { bridge, onLspMessage } from "./bridge";
import {
  completionReplacementRange,
  diagnosticMarkerRange,
  pathToFileUri,
  startLspListening,
  type LspRange,
} from "./lspProtocol";
import type { Problem } from "./types";

type JsonObject = Record<string, unknown>;
type PendingRequest = {
  resolve: (value: unknown) => void;
  reject: (reason: Error) => void;
  timer: number;
};

export class BioLangLspClient {
  private nextId = 1;
  private pending = new Map<number, PendingRequest>();
  private ready = false;
  private unlisten?: () => void;
  private monaco?: Monaco;
  private problemsChanged?: (path: string, problems: Problem[]) => void;

  async start(root: string, monaco: Monaco, problemsChanged: (path: string, problems: Problem[]) => void) {
    this.stop();
    this.monaco = monaco;
    this.problemsChanged = problemsChanged;
    try {
      const listener = await startLspListening(
        () => onLspMessage((message) => this.receive(message as JsonObject)),
        () => bridge.startLsp(),
      );
      if (!listener.started) return false;
      this.unlisten = listener.dispose;

      await this.request("initialize", {
        processId: null,
        rootUri: pathToFileUri(root),
        capabilities: {
          textDocument: {
            synchronization: { didSave: true },
            completion: { completionItem: { documentationFormat: ["markdown", "plaintext"] } },
            hover: { contentFormat: ["markdown", "plaintext"] },
          },
        },
        clientInfo: { name: "BioLang Desktop", version: "0.1.0" },
      });
      await this.notify("initialized", {});
      this.ready = true;
      return true;
    } catch (error) {
      this.stop();
      throw error;
    }
  }

  stop() {
    this.unlisten?.();
    this.unlisten = undefined;
    for (const pending of this.pending.values()) {
      window.clearTimeout(pending.timer);
      pending.reject(new Error("Language service stopped"));
    }
    this.pending.clear();
    this.ready = false;
  }

  isReady() {
    return this.ready;
  }

  async open(path: string, content: string) {
    if (!this.ready) return;
    await this.notify("textDocument/didOpen", {
      textDocument: { uri: this.documentUri(path), languageId: "biolang", version: 1, text: content },
    });
  }

  async change(path: string, content: string, version: number) {
    if (!this.ready) return;
    await this.notify("textDocument/didChange", {
      textDocument: { uri: this.documentUri(path), version },
      contentChanges: [{ text: content }],
    });
  }

  async close(path: string) {
    if (!this.ready) return;
    await this.notify("textDocument/didClose", { textDocument: { uri: this.documentUri(path) } });
  }

  completionProvider(): MonacoEditor.languages.CompletionItemProvider {
    return {
      triggerCharacters: [".", ":", "|"],
      provideCompletionItems: async (model, position) => {
        if (!this.ready) return { suggestions: [] };
        const result = (await this.request("textDocument/completion", {
          textDocument: { uri: this.documentUri(this.pathFromModel(model)) },
          position: { line: position.lineNumber - 1, character: position.column - 1 },
        })) as JsonObject[] | { items?: JsonObject[] } | null;
        const items = Array.isArray(result) ? result : result?.items ?? [];
        const word = model.getWordUntilPosition(position);
        return {
          suggestions: items.map((item) => ({
            label: String(item.label ?? ""),
            kind: this.completionKind(Number(item.kind ?? 1)),
            detail: item.detail ? String(item.detail) : undefined,
            insertText: String(item.insertText ?? item.label ?? ""),
            range: completionReplacementRange(position.lineNumber, position.column, word.startColumn),
          })),
        };
      },
    };
  }

  hoverProvider(): MonacoEditor.languages.HoverProvider {
    return {
      provideHover: async (model, position) => {
        if (!this.ready) return null;
        const result = (await this.request("textDocument/hover", {
          textDocument: { uri: this.documentUri(this.pathFromModel(model)) },
          position: { line: position.lineNumber - 1, character: position.column - 1 },
        })) as JsonObject | null;
        if (!result?.contents) return null;
        const contents = result.contents as JsonObject | string | Array<JsonObject | string>;
        const values = Array.isArray(contents) ? contents : [contents];
        return {
          contents: values.map((entry) => ({
            value: typeof entry === "string" ? entry : String(entry.value ?? entry.language ?? ""),
          })),
        };
      },
    };
  }

  private async request(method: string, params: unknown): Promise<unknown> {
    const id = this.nextId++;
    const response = new Promise<unknown>((resolve, reject) => {
      const timer = window.setTimeout(() => {
        this.pending.delete(id);
        reject(new Error(`Language service timed out: ${method}`));
      }, 8_000);
      this.pending.set(id, { resolve, reject, timer });
    });
    await bridge.sendLsp({ jsonrpc: "2.0", id, method, params });
    return response;
  }

  private notify(method: string, params: unknown) {
    return bridge.sendLsp({ jsonrpc: "2.0", method, params });
  }

  private receive(message: JsonObject) {
    if (typeof message.id === "number" && !message.method) {
      const pending = this.pending.get(message.id);
      if (!pending) return;
      window.clearTimeout(pending.timer);
      this.pending.delete(message.id);
      if (message.error) pending.reject(new Error(JSON.stringify(message.error)));
      else pending.resolve(message.result);
      return;
    }

    if (message.method === "textDocument/publishDiagnostics") {
      this.publishDiagnostics(message.params as JsonObject);
    }
  }

  private publishDiagnostics(params: JsonObject) {
    if (!this.monaco) return;
    const uri = String(params.uri ?? "");
    const model = this.monaco.editor.getModels().find((candidate) => candidate.uri.toString() === uri);
    const diagnostics = Array.isArray(params.diagnostics) ? (params.diagnostics as JsonObject[]) : [];
    const markers: MonacoEditor.editor.IMarkerData[] = diagnostics.map((diagnostic) => {
      const range = diagnostic.range as LspRange;
      return {
        message: String(diagnostic.message ?? "BioLang diagnostic"),
        severity: this.markerSeverity(Number(diagnostic.severity ?? 1)),
        ...diagnosticMarkerRange(range),
      };
    });
    if (model) this.monaco.editor.setModelMarkers(model, "biolang", markers);

    const path = decodeURIComponent(uri.split("/workspace/")[1] ?? uri);
    this.problemsChanged?.(
      path,
      markers.map((marker) => ({
        path,
        message: marker.message,
        severity: this.problemSeverity(marker.severity),
        line: marker.startLineNumber,
        column: marker.startColumn,
      })),
    );
  }

  private documentUri(path: string) {
    return `file:///workspace/${path.replaceAll("\\", "/").split("/").map(encodeURIComponent).join("/")}`;
  }

  private pathFromModel(model: MonacoEditor.editor.ITextModel) {
    return decodeURIComponent(model.uri.path.replace(/^\/workspace\//, ""));
  }

  private completionKind(kind: number) {
    if (!this.monaco) return 1;
    const mapping: Record<number, MonacoEditor.languages.CompletionItemKind> = {
      3: this.monaco.languages.CompletionItemKind.Function,
      6: this.monaco.languages.CompletionItemKind.Variable,
      9: this.monaco.languages.CompletionItemKind.Module,
      13: this.monaco.languages.CompletionItemKind.Enum,
      14: this.monaco.languages.CompletionItemKind.Keyword,
    };
    return mapping[kind] ?? this.monaco.languages.CompletionItemKind.Text;
  }

  private markerSeverity(severity: number) {
    if (!this.monaco) return 8;
    if (severity === 1) return this.monaco.MarkerSeverity.Error;
    if (severity === 2) return this.monaco.MarkerSeverity.Warning;
    if (severity === 3) return this.monaco.MarkerSeverity.Info;
    return this.monaco.MarkerSeverity.Hint;
  }

  private problemSeverity(markerSeverity: number): 1 | 2 | 3 | 4 {
    if (!this.monaco) return 1;
    if (markerSeverity === this.monaco.MarkerSeverity.Error) return 1;
    if (markerSeverity === this.monaco.MarkerSeverity.Warning) return 2;
    if (markerSeverity === this.monaco.MarkerSeverity.Info) return 3;
    return 4;
  }
}
