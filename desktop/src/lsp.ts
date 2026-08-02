import type { Monaco } from "@monaco-editor/react";
import type * as MonacoEditor from "monaco-editor";
import { bridge, onLspMessage } from "./bridge";
import {
  completionReplacementRange,
  diagnosticMarkerRange,
  lspRangeToMonaco,
  pathToFileUri,
  startLspListening,
  type LspRange,
} from "./lspProtocol";
import { aliasDocumentation, matchingAliases } from "./aliases";
import { callSnippet, parametersFromSignature, scaffoldSnippets } from "./snippets";
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
            signatureHelp: {
              signatureInformation: { documentationFormat: ["markdown", "plaintext"] },
            },
            definition: { dynamicRegistration: false },
            references: { dynamicRegistration: false },
            rename: { dynamicRegistration: false, prepareSupport: true },
            formatting: { dynamicRegistration: false },
            codeAction: { dynamicRegistration: false },
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
        if (!this.ready || !this.monaco) return { suggestions: [] };
        const result = (await this.request("textDocument/completion", {
          textDocument: { uri: this.documentUri(this.pathFromModel(model)) },
          position: { line: position.lineNumber - 1, character: position.column - 1 },
        })) as JsonObject[] | { items?: JsonObject[] } | null;
        const items = Array.isArray(result) ? result : result?.items ?? [];
        const word = model.getWordUntilPosition(position);
        const range = completionReplacementRange(position.lineNumber, position.column, word.startColumn);
        return {
          suggestions: [
            ...items.map((item) => this.completionItem(item, range)),
            // Same dplyr and pandas aliases the browser build offers, so the
            // migration path does not depend on which runtime is behind you.
            ...matchingAliases(word.word).map((alias) => ({
              label: alias.foreign,
              kind: this.monaco!.languages.CompletionItemKind.Reference,
              detail: `${alias.origin} → ${alias.biolang}`,
              documentation: { value: aliasDocumentation(alias) },
              insertText: alias.biolang,
              range,
              sortText: `0_alias_${alias.foreign}`,
            })),
            ...scaffoldSnippets.map((snippet) => ({
              label: snippet.label,
              kind: this.monaco!.languages.CompletionItemKind.Snippet,
              detail: snippet.detail,
              documentation: { value: snippet.documentation },
              insertText: snippet.body,
              insertTextRules: this.monaco!.languages.CompletionItemInsertTextRule.InsertAsSnippet,
              range,
              sortText: `1_${snippet.label}`,
            })),
          ],
        };
      },
    };
  }

  /**
   * Turn one LSP completion into a Monaco one, upgrading plain function names
   * into call snippets.
   *
   * The server sends `detail` as the signature but `insertText` as the bare
   * name, so accepting a completion left the author to type the whole argument
   * list themselves. When the server already sends a snippet (`insertTextFormat`
   * 2) its text wins untouched.
   */
  private completionItem(
    item: JsonObject,
    range: MonacoEditor.IRange,
  ): MonacoEditor.languages.CompletionItem {
    const label = String(item.label ?? "");
    const detail = item.detail ? String(item.detail) : undefined;
    const kind = this.completionKind(Number(item.kind ?? 1));
    const serverSnippet = Number(item.insertTextFormat ?? 1) === 2;
    const insertText = String(item.insertText ?? item.label ?? "");
    const callable = !serverSnippet
      && kind === this.monaco?.languages.CompletionItemKind.Function
      && detail?.includes("(");
    return {
      label,
      kind,
      detail,
      documentation: this.markdownValue(item.documentation),
      insertText: callable ? callSnippet(label, parametersFromSignature(detail!)) : insertText,
      insertTextRules: callable || serverSnippet
        ? this.monaco!.languages.CompletionItemInsertTextRule.InsertAsSnippet
        : undefined,
      sortText: item.sortText ? String(item.sortText) : undefined,
      range,
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

  signatureHelpProvider(): MonacoEditor.languages.SignatureHelpProvider {
    return {
      signatureHelpTriggerCharacters: ["(", ","],
      signatureHelpRetriggerCharacters: [","],
      provideSignatureHelp: async (model, position) => {
        if (!this.ready) return null;
        const result = (await this.request("textDocument/signatureHelp", {
          textDocument: { uri: this.documentUri(this.pathFromModel(model)) },
          position: { line: position.lineNumber - 1, character: position.column - 1 },
          context: { triggerKind: 1, isRetrigger: false },
        })) as JsonObject | null;
        const signatures = Array.isArray(result?.signatures)
          ? result.signatures as JsonObject[]
          : [];
        if (!signatures.length) return null;
        return {
          value: {
            signatures: signatures.map((signature) => ({
              label: String(signature.label ?? ""),
              documentation: this.markdownValue(signature.documentation),
              parameters: Array.isArray(signature.parameters)
                ? (signature.parameters as JsonObject[]).map((parameter) => ({
                    label: String(parameter.label ?? ""),
                    documentation: this.markdownValue(parameter.documentation),
                  }))
                : [],
              activeParameter: Number(signature.activeParameter ?? 0),
            })),
            activeSignature: Number(result?.activeSignature ?? 0),
            activeParameter: Number(result?.activeParameter ?? 0),
          },
          dispose: () => undefined,
        };
      },
    };
  }

  definitionProvider(): MonacoEditor.languages.DefinitionProvider {
    return {
      provideDefinition: async (model, position) => {
        if (!this.ready || !this.monaco) return null;
        const result = (await this.request("textDocument/definition", {
          textDocument: { uri: this.documentUri(this.pathFromModel(model)) },
          position: { line: position.lineNumber - 1, character: position.column - 1 },
        })) as JsonObject | JsonObject[] | null;
        const locations = Array.isArray(result) ? result : result ? [result] : [];
        return locations.flatMap((location) => {
          const range = location.range as {
            start?: { line?: number; character?: number };
            end?: { line?: number; character?: number };
          } | undefined;
          if (!range?.start || !range.end) return [];
          return [{
            uri: this.monaco!.Uri.parse(String(location.uri ?? "")),
            range: {
              startLineNumber: Number(range.start.line ?? 0) + 1,
              startColumn: Number(range.start.character ?? 0) + 1,
              endLineNumber: Number(range.end.line ?? 0) + 1,
              endColumn: Number(range.end.character ?? 0) + 1,
            },
          }];
        });
      },
    };
  }

  referenceProvider(): MonacoEditor.languages.ReferenceProvider {
    return {
      provideReferences: async (model, position, context) => {
        if (!this.ready || !this.monaco) return [];
        const result = (await this.request("textDocument/references", {
          textDocument: { uri: this.documentUri(this.pathFromModel(model)) },
          position: { line: position.lineNumber - 1, character: position.column - 1 },
          context: { includeDeclaration: context.includeDeclaration },
        })) as JsonObject[] | null;
        return (result ?? []).flatMap((location) => {
          const range = lspRangeToMonaco(location.range);
          return range ? [{ uri: model.uri, range }] : [];
        });
      },
    };
  }

  renameProvider(): MonacoEditor.languages.RenameProvider {
    return {
      provideRenameEdits: async (model, position, newName) => {
        if (!this.ready || !this.monaco) return null;
        const result = (await this.request("textDocument/rename", {
          textDocument: { uri: this.documentUri(this.pathFromModel(model)) },
          position: { line: position.lineNumber - 1, character: position.column - 1 },
          newName,
        })) as { changes?: Record<string, JsonObject[]> } | null;
        const edits = Object.values(result?.changes ?? {}).flat();
        if (!edits.length) {
          return { edits: [], rejectReason: "Nothing to rename here." };
        }
        return {
          edits: edits.flatMap((edit) => {
            const range = lspRangeToMonaco(edit.range);
            return range
              ? [{ resource: model.uri, versionId: undefined, textEdit: { range, text: String(edit.newText ?? "") } }]
              : [];
          }),
        };
      },
      resolveRenameLocation: async (model, position) => {
        if (!this.ready) return null;
        const result = (await this.request("textDocument/prepareRename", {
          textDocument: { uri: this.documentUri(this.pathFromModel(model)) },
          position: { line: position.lineNumber - 1, character: position.column - 1 },
        })) as JsonObject | null;
        const range = lspRangeToMonaco(result?.range ?? result);
        if (!range) {
          // Builtins and package exports live in files this rename cannot
          // reach, and the server declines them for that reason.
          return { range: new this.monaco!.Range(position.lineNumber, position.column, position.lineNumber, position.column), text: "", rejectReason: "This symbol is not defined in this file." };
        }
        return { range, text: model.getValueInRange(range) };
      },
    };
  }

  documentFormattingProvider(): MonacoEditor.languages.DocumentFormattingEditProvider {
    return {
      provideDocumentFormattingEdits: async (model, options) => {
        if (!this.ready) return [];
        const result = (await this.request("textDocument/formatting", {
          textDocument: { uri: this.documentUri(this.pathFromModel(model)) },
          options: {
            tabSize: options.tabSize,
            insertSpaces: options.insertSpaces,
          },
        })) as JsonObject[] | null;
        return (result ?? []).flatMap((edit) => {
          const range = lspRangeToMonaco(edit.range);
          return range ? [{ range, text: String(edit.newText ?? "") }] : [];
        });
      },
    };
  }

  codeActionProvider(): MonacoEditor.languages.CodeActionProvider {
    return {
      provideCodeActions: async (model, range, context) => {
        if (!this.ready) return { actions: [], dispose: () => undefined };
        const result = (await this.request("textDocument/codeAction", {
          textDocument: { uri: this.documentUri(this.pathFromModel(model)) },
          range: {
            start: { line: range.startLineNumber - 1, character: range.startColumn - 1 },
            end: { line: range.endLineNumber - 1, character: range.endColumn - 1 },
          },
          context: { diagnostics: [], triggerKind: context.trigger },
        })) as JsonObject[] | null;
        const actions = (result ?? []).flatMap((action) => {
          const changes = (action.edit as { changes?: Record<string, JsonObject[]> } | undefined)?.changes ?? {};
          const edits = Object.values(changes).flat().flatMap((edit) => {
            const editRange = lspRangeToMonaco(edit.range);
            return editRange
              ? [{ resource: model.uri, versionId: undefined, textEdit: { range: editRange, text: String(edit.newText ?? "") } }]
              : [];
          });
          if (!edits.length) return [];
          return [{
            title: String(action.title ?? "BioLang fix"),
            kind: String(action.kind ?? "quickfix"),
            edit: { edits },
          }];
        });
        return { actions, dispose: () => undefined };
      },
    };
  }

  private markdownValue(value: unknown): MonacoEditor.IMarkdownString | string | undefined {
    if (typeof value === "string") return value;
    if (value && typeof value === "object") {
      return { value: String((value as JsonObject).value ?? "") };
    }
    return undefined;
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
