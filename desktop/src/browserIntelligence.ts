import type { Monaco } from "@monaco-editor/react";
import type * as MonacoEditor from "monaco-editor";
import builtinMetadata from "./generated/builtin-metadata.json";
import packageMetadata from "./generated/package-metadata.json";
import { aliasDocumentation, matchingAliases } from "./aliases";
import { formatBrowserSource } from "./browserRuntime";
import { identifierOccurrences } from "./occurrences";
import { biolangKeywords } from "./language";
import { callSnippet, parametersFromSignature, scaffoldSnippets } from "./snippets";

type ApiFunction = {
  name: string;
  signature: string;
  summary: string;
  returnType?: string;
  fields: string[];
  sourcePath: string;
  line: number;
};

type ApiPackage = {
  name: string;
  version: string;
  description: string;
  exports: ApiFunction[];
};

type BrowserSymbol = {
  name: string;
  kind: "function" | "variable" | "module" | "field";
  detail: string;
  documentation?: string;
  fields?: string[];
  line: number;
  column: number;
};

type BrowserAnalysis = {
  symbols: Map<string, BrowserSymbol>;
  modules: Map<string, ApiPackage>;
};

const packages = new Map(
  (packageMetadata.packages as ApiPackage[]).map((entry) => [entry.name, entry]),
);
const builtins = new Map(
  builtinMetadata.builtins.map((entry) => [entry.name, entry]),
);

function sourcePosition(source: string, offset: number) {
  const before = source.slice(0, offset);
  const lines = before.split(/\r?\n/);
  return { line: lines.length, column: lines.at(-1)!.length + 1 };
}

function fieldsInRecord(source: string) {
  return [...source.matchAll(/(?:^|[,{\n])\s*([A-Za-z_]\w*)\s*:/g)]
    .map((match) => match[1]);
}

/**
 * The analysis of each model, keyed by the version it was computed from.
 *
 * `analyze` makes six full-document regex passes, and seven providers call it —
 * completion on every keystroke, signature help on every `(` and `,`. That was
 * dozens of whole-file scans per character typed in a long pipeline. A model's
 * version id changes on every edit, so this is exact rather than a heuristic:
 * the cache can never serve a stale analysis.
 */
const analysisCache = new WeakMap<
  MonacoEditor.editor.ITextModel,
  { version: number; analysis: BrowserAnalysis }
>();

function analyzeModel(model: MonacoEditor.editor.ITextModel): BrowserAnalysis {
  const version = model.getVersionId();
  const cached = analysisCache.get(model);
  if (cached?.version === version) return cached.analysis;
  const analysis = analyze(model.getValue());
  analysisCache.set(model, { version, analysis });
  return analysis;
}

function analyze(source: string): BrowserAnalysis {
  const symbols = new Map<string, BrowserSymbol>();
  const modules = new Map<string, ApiPackage>();

  for (const match of source.matchAll(/import\s+"([^"]+)"\s+as\s+([A-Za-z_]\w*)/g)) {
    const packageName = match[1].replace(/^pkg\//, "").split("/")[0];
    const api = packages.get(packageName);
    if (!api) continue;
    modules.set(match[2], api);
    const position = sourcePosition(source, match.index);
    symbols.set(match[2], {
      name: match[2],
      kind: "module",
      detail: `${api.name} ${api.version}`,
      documentation: api.description,
      line: position.line,
      column: position.column,
    });
  }

  for (const match of source.matchAll(/\bfn\s+([A-Za-z_]\w*)\s*\(([^)]*)\)/g)) {
    const position = sourcePosition(source, match.index);
    symbols.set(match[1], {
      name: match[1],
      kind: "function",
      detail: `${match[1]}(${match[2].replace(/\s+/g, " ").trim()})`,
      line: position.line,
      column: position.column,
    });
  }

  for (const match of source.matchAll(/\b(?:let|const)\s+([A-Za-z_]\w*)\s*=\s*\{([^}]*)\}/gs)) {
    const position = sourcePosition(source, match.index);
    const fields = fieldsInRecord(match[2]);
    symbols.set(match[1], {
      name: match[1],
      kind: "variable",
      detail: fields.length ? `Record{${fields.join(", ")}}` : "Record",
      fields,
      line: position.line,
      column: position.column,
    });
  }

  for (const match of source.matchAll(
    /\b(?:let|const)\s+([A-Za-z_]\w*)\s*=\s*([A-Za-z_]\w*)\.([A-Za-z_]\w*)\s*\(([^)]*)\)/g,
  )) {
    const api = modules.get(match[2]);
    const fn = api?.exports.find((entry) => entry.name === match[3]);
    if (!fn) continue;
    const firstArgument = match[4].split(",")[0]?.trim();
    const inherited = firstArgument ? symbols.get(firstArgument)?.fields ?? [] : [];
    const fields = [...new Set([...inherited, ...fn.fields])];
    const position = sourcePosition(source, match.index);
    symbols.set(match[1], {
      name: match[1],
      kind: "variable",
      detail: fields.length
        ? `Record{${fields.join(", ")}}`
        : fn.returnType || "Any",
      documentation: `Result of ${match[2]}.${fn.name}.`,
      fields,
      line: position.line,
      column: position.column,
    });
  }

  for (const match of source.matchAll(
    /\b(?:let|const)\s+([A-Za-z_]\w*)\s*=\s*(true|false|nil|-?\d+(?:\.\d+)?|"(?:\\.|[^"])*")/g,
  )) {
    if (symbols.has(match[1])) continue;
    const value = match[2];
    const type = value.startsWith('"')
      ? "Str"
      : value === "true" || value === "false"
        ? "Bool"
        : value === "nil"
          ? "Nil"
          : value.includes(".") ? "Float" : "Int";
    const position = sourcePosition(source, match.index);
    symbols.set(match[1], {
      name: match[1],
      kind: "variable",
      detail: `${type} = ${value}`,
      line: position.line,
      column: position.column,
    });
  }

  return { symbols, modules };
}

function memberContext(model: MonacoEditor.editor.ITextModel, position: MonacoEditor.Position) {
  const before = model.getValueInRange({
    startLineNumber: position.lineNumber,
    startColumn: 1,
    endLineNumber: position.lineNumber,
    endColumn: position.column,
  });
  return before.match(/([A-Za-z_]\w*)\.[A-Za-z_]*$/)?.[1];
}

function wordContext(model: MonacoEditor.editor.ITextModel, position: MonacoEditor.Position) {
  const word = model.getWordAtPosition(position);
  if (!word) return undefined;
  const before = model.getValueInRange({
    startLineNumber: position.lineNumber,
    startColumn: 1,
    endLineNumber: position.lineNumber,
    endColumn: word.startColumn,
  });
  return {
    word: word.word,
    qualifier: before.match(/([A-Za-z_]\w*)\.$/)?.[1],
  };
}

function callContext(model: MonacoEditor.editor.ITextModel, position: MonacoEditor.Position) {
  const source = model.getValue();
  const offset = model.getOffsetAt(position);
  const before = source.slice(0, offset);
  let depth = 0;
  let open = -1;
  for (let index = before.length - 1; index >= 0; index -= 1) {
    const character = before[index];
    if (")]}".includes(character)) depth += 1;
    else if ("([{".includes(character)) {
      if (character === "(" && depth === 0) {
        open = index;
        break;
      }
      depth = Math.max(0, depth - 1);
    }
  }
  if (open < 0) return undefined;
  const callee = before.slice(0, open).match(/([A-Za-z_]\w*(?:\.[A-Za-z_]\w*)?)\s*$/)?.[1];
  if (!callee) return undefined;
  let nesting = 0;
  let activeParameter = 0;
  for (const character of before.slice(open + 1)) {
    if ("([{".includes(character)) nesting += 1;
    else if (")]}".includes(character)) nesting = Math.max(0, nesting - 1);
    else if (character === "," && nesting === 0) activeParameter += 1;
  }
  return { callee, activeParameter };
}

function packageFunction(analysis: BrowserAnalysis, qualifier: string, name: string) {
  return analysis.modules.get(qualifier)?.exports.find((entry) => entry.name === name);
}

function completionKind(monaco: Monaco, kind: BrowserSymbol["kind"]) {
  if (kind === "function") return monaco.languages.CompletionItemKind.Function;
  if (kind === "module") return monaco.languages.CompletionItemKind.Module;
  if (kind === "field") return monaco.languages.CompletionItemKind.Field;
  return monaco.languages.CompletionItemKind.Variable;
}

/** Levenshtein distance, for did-you-mean suggestions. */
function editDistance(left: string, right: string): number {
  let previous = Array.from({ length: right.length + 1 }, (_, index) => index);
  for (let i = 0; i < left.length; i += 1) {
    const current = [i + 1];
    for (let j = 0; j < right.length; j += 1) {
      current[j + 1] = Math.min(
        previous[j] + (left[i] === right[j] ? 0 : 1),
        previous[j + 1] + 1,
        current[j] + 1,
      );
    }
    previous = current;
  }
  return previous[right.length];
}

function spellingSuggestions(analysis: BrowserAnalysis, word: string): string[] {
  const allowance = word.length <= 3 ? 1 : word.length <= 7 ? 2 : 3;
  const candidates = [...analysis.symbols.keys(), ...builtins.keys()];
  return [...new Set(candidates)]
    .filter((candidate) => candidate !== word)
    .map((candidate) => [editDistance(word, candidate), candidate] as const)
    .filter(([distance]) => distance <= allowance)
    .sort((left, right) => left[0] - right[0] || left[1].localeCompare(right[1]))
    .slice(0, 3)
    .map(([, candidate]) => candidate);
}

/** Packages that export a function by this name, for the missing-import fix. */
function packagesExporting(name: string): string[] {
  return [...packages.values()]
    .filter((api) => api.exports.some((entry) => entry.name === name))
    .map((api) => api.name)
    .slice(0, 3);
}

/** A short, valid identifier to import a package under. */
function defaultAlias(packageName: string): string {
  const cleaned = packageName.replace(/[^A-Za-z0-9_]/g, "_");
  return /^[A-Za-z_]/.test(cleaned) ? cleaned : `_${cleaned}`;
}

export function registerBrowserIntelligence(monaco: Monaco) {
  const completion = monaco.languages.registerCompletionItemProvider("biolang", {
    triggerCharacters: [".", ":", "|"],
    provideCompletionItems(model, position) {
      const analysis = analyzeModel(model);
      const target = memberContext(model, position);
      const word = model.getWordUntilPosition(position);
      const range = {
        startLineNumber: position.lineNumber,
        endLineNumber: position.lineNumber,
        startColumn: word.startColumn,
        endColumn: position.column,
      };
      if (target) {
        const module = analysis.modules.get(target);
        if (module) {
          return {
            suggestions: module.exports
              .filter((entry) => !entry.name.startsWith("_"))
              .map((entry) => ({
                label: entry.name,
                kind: monaco.languages.CompletionItemKind.Function,
                detail: entry.signature,
                documentation: { value: entry.summary },
                insertText: callSnippet(entry.name, parametersFromSignature(entry.signature)),
                insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                range,
              })),
          };
        }
        const variable = analysis.symbols.get(target);
        return {
          suggestions: (variable?.fields ?? [])
            .filter((field) => !field.startsWith("_"))
            .map((field) => ({
              label: field,
              kind: monaco.languages.CompletionItemKind.Field,
              detail: `field of ${target}`,
              insertText: field,
              range,
            })),
        };
      }

      const suggestions: MonacoEditor.languages.CompletionItem[] = [
        ...biolangKeywords.map((keyword) => ({
          label: keyword,
          kind: monaco.languages.CompletionItemKind.Keyword,
          detail: "BioLang keyword",
          insertText: keyword,
          range,
          sortText: `2_${keyword}`,
        })),
        ...builtinMetadata.builtins.map((entry) => ({
          label: entry.name,
          kind: monaco.languages.CompletionItemKind.Function,
          detail: entry.signature,
          documentation: { value: entry.summary },
          insertText: callSnippet(entry.name, entry.parameters),
          insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
          range,
          sortText: `1_${entry.name}`,
        })),
        // Someone arriving from dplyr or pandas types the spelling they know.
        // Ranked above everything else because when one of these matches, it is
        // exactly what the author meant.
        ...matchingAliases(word.word).map((alias) => ({
          label: alias.foreign,
          kind: monaco.languages.CompletionItemKind.Reference,
          detail: `${alias.origin} → ${alias.biolang}`,
          documentation: { value: aliasDocumentation(alias) },
          insertText: alias.biolang,
          range,
          sortText: `0_alias_${alias.foreign}`,
        })),
        ...scaffoldSnippets.map((snippet) => ({
          label: snippet.label,
          kind: monaco.languages.CompletionItemKind.Snippet,
          detail: snippet.detail,
          documentation: { value: snippet.documentation },
          insertText: snippet.body,
          insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
          range,
          // Ahead of the bare keyword of the same name: someone typing "for"
          // almost always wants the loop, not the token.
          sortText: `1_${snippet.label}`,
        })),
        ...[...analysis.symbols.values()].map((symbol) => ({
          label: symbol.name,
          kind: completionKind(monaco, symbol.kind),
          detail: symbol.detail,
          documentation: symbol.documentation ? { value: symbol.documentation } : undefined,
          // Functions declared in this file get argument tabstops too; their
          // detail is already the `name(a, b)` shape parsed out of `fn`.
          insertText: symbol.kind === "function"
            ? callSnippet(symbol.name, parametersFromSignature(symbol.detail))
            : symbol.name,
          insertTextRules: symbol.kind === "function"
            ? monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet
            : undefined,
          range,
          sortText: `0_${symbol.name}`,
        })),
      ];
      return { suggestions };
    },
  });

  const hover = monaco.languages.registerHoverProvider("biolang", {
    provideHover(model, position) {
      const context = wordContext(model, position);
      if (!context) return null;
      const analysis = analyzeModel(model);
      if (context.qualifier) {
        const fn = packageFunction(analysis, context.qualifier, context.word);
        if (fn) {
          return {
            contents: [
              { value: `\`\`\`biolang\n${fn.signature}\n\`\`\`` },
              { value: fn.summary },
            ],
          };
        }
        const variable = analysis.symbols.get(context.qualifier);
        if (variable?.fields?.includes(context.word)) {
          return { contents: [{ value: `**${context.word}**: field of \`${context.qualifier}\`` }] };
        }
      }
      const symbol = analysis.symbols.get(context.word);
      if (symbol) {
        return {
          contents: [
            { value: `\`\`\`biolang\n${symbol.name}: ${symbol.detail}\n\`\`\`` },
            ...(symbol.documentation ? [{ value: symbol.documentation }] : []),
          ],
        };
      }
      const builtin = builtins.get(context.word);
      return builtin ? {
        contents: [
          { value: `\`\`\`biolang\n${builtin.signature}\n\`\`\`` },
          { value: builtin.summary },
        ],
      } : null;
    },
  });

  const signature = monaco.languages.registerSignatureHelpProvider("biolang", {
    signatureHelpTriggerCharacters: ["(", ","],
    signatureHelpRetriggerCharacters: [","],
    provideSignatureHelp(model, position) {
      const context = callContext(model, position);
      if (!context) return null;
      const analysis = analyzeModel(model);
      const [qualifier, name] = context.callee.includes(".")
        ? context.callee.split(".", 2)
        : [undefined, context.callee];
      const fn = qualifier
        ? packageFunction(analysis, qualifier, name)
        : builtins.get(name);
      const local = !fn ? analysis.symbols.get(name) : undefined;
      const label = fn?.signature || local?.detail;
      if (!label) return null;
      const parameters = label
        .slice(label.indexOf("(") + 1, label.indexOf(")"))
        .split(",")
        .map((parameter) => parameter.trim())
        .filter(Boolean)
        .map((parameter) => ({ label: parameter }));
      return {
        value: {
          signatures: [{
            label,
            documentation: fn?.summary || local?.documentation,
            parameters,
            activeParameter: Math.min(context.activeParameter, Math.max(0, parameters.length - 1)),
          }],
          activeSignature: 0,
          activeParameter: context.activeParameter,
        },
        dispose: () => undefined,
      };
    },
  });

  const definition = monaco.languages.registerDefinitionProvider("biolang", {
    provideDefinition(model, position) {
      const context = wordContext(model, position);
      if (!context || context.qualifier) return null;
      const symbol = analyzeModel(model).symbols.get(context.word);
      if (!symbol) return null;
      return {
        uri: model.uri,
        range: {
          startLineNumber: symbol.line,
          startColumn: symbol.column,
          endLineNumber: symbol.line,
          endColumn: symbol.column + symbol.name.length,
        },
      };
    },
  });

  const formatting = monaco.languages.registerDocumentFormattingEditProvider("biolang", {
    async provideDocumentFormattingEdits(model, options) {
      const source = model.getValue();
      const formatted = await formatBrowserSource(source, options.tabSize);
      if (formatted === source) return [];
      return [{ range: model.getFullModelRange(), text: formatted }];
    },
  });

  const references = monaco.languages.registerReferenceProvider("biolang", {
    provideReferences(model, position) {
      const word = model.getWordAtPosition(position)?.word;
      if (!word) return [];
      return identifierOccurrences(model, word).map((range) => ({ uri: model.uri, range }));
    },
  });

  const rename = monaco.languages.registerRenameProvider("biolang", {
    provideRenameEdits(model, position, newName) {
      const word = model.getWordAtPosition(position)?.word;
      const symbol = word ? analyzeModel(model).symbols.get(word) : undefined;
      if (!word || !symbol) {
        return { edits: [], rejectReason: "This symbol is not defined in this file." };
      }
      return {
        edits: identifierOccurrences(model, word).map((range) => ({
          resource: model.uri,
          versionId: undefined,
          textEdit: { range, text: newName },
        })),
      };
    },
    resolveRenameLocation(model, position) {
      const word = model.getWordAtPosition(position);
      const symbol = word ? analyzeModel(model).symbols.get(word.word) : undefined;
      const range = new monaco.Range(
        position.lineNumber,
        word?.startColumn ?? position.column,
        position.lineNumber,
        word?.endColumn ?? position.column,
      );
      // Builtins and package exports live in files this edit cannot reach.
      if (!word || !symbol) {
        return { range, text: word?.word ?? "", rejectReason: "This symbol is not defined in this file." };
      }
      return { range, text: word.word };
    },
  });

  const codeActions = monaco.languages.registerCodeActionProvider("biolang", {
    provideCodeActions(model, range) {
      const word = model.getWordAtPosition(range.getStartPosition());
      if (!word) return { actions: [], dispose: () => undefined };
      const analysis = analyzeModel(model);
      const actions: MonacoEditor.languages.CodeAction[] = [];
      const wordRange = new monaco.Range(
        range.startLineNumber,
        word.startColumn,
        range.startLineNumber,
        word.endColumn,
      );

      // An identifier that resolves to nothing is the only safe place to offer
      // a rewrite; anything else risks "fixing" working code.
      const known = analysis.symbols.has(word.word)
        || builtins.has(word.word)
        || biolangKeywords.includes(word.word);
      if (!known) {
        for (const suggestion of spellingSuggestions(analysis, word.word)) {
          actions.push({
            title: `Change to \`${suggestion}\``,
            kind: "quickfix",
            edit: {
              edits: [{
                resource: model.uri,
                versionId: undefined,
                textEdit: { range: wordRange, text: suggestion },
              }],
            },
          });
        }
        for (const packageName of packagesExporting(word.word)) {
          const alias = defaultAlias(packageName);
          actions.push({
            title: `Import "${packageName}" as ${alias}`,
            kind: "quickfix",
            edit: {
              edits: [{
                resource: model.uri,
                versionId: undefined,
                textEdit: {
                  range: new monaco.Range(1, 1, 1, 1),
                  text: `import "${packageName}" as ${alias}\n`,
                },
              }, {
                resource: model.uri,
                versionId: undefined,
                textEdit: { range: wordRange, text: `${alias}.${word.word}` },
              }],
            },
          });
        }
      }
      return { actions, dispose: () => undefined };
    },
  });

  return [completion, hover, signature, definition, formatting, references, rename, codeActions];
}
