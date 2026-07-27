import type { Monaco } from "@monaco-editor/react";

export const biolangKeywords = [
  "let",
  "const",
  "fn",
  "if",
  "else",
  "for",
  "in",
  "while",
  "break",
  "continue",
  "return",
  "match",
  "import",
  "as",
  "true",
  "false",
  "nil",
  "and",
  "or",
  "not",
  "try",
  "catch",
  "pipeline",
  "stage",
  "assert",
  "yield",
  "enum",
  "struct",
  "trait",
  "impl",
];

export function registerBioLang(monaco: Monaco): void {
  if (monaco.languages.getLanguages().some((language) => language.id === "biolang")) return;

  monaco.languages.register({ id: "biolang", extensions: [".bl"], aliases: ["BioLang", "bl"] });
  monaco.languages.setLanguageConfiguration("biolang", {
    comments: { lineComment: "#" },
    brackets: [
      ["{", "}"],
      ["[", "]"],
      ["(", ")"],
    ],
    autoClosingPairs: [
      { open: "{", close: "}" },
      { open: "[", close: "]" },
      { open: "(", close: ")" },
      { open: '"', close: '"' },
    ],
    surroundingPairs: [
      { open: "{", close: "}" },
      { open: "[", close: "]" },
      { open: "(", close: ")" },
      { open: '"', close: '"' },
    ],
    indentationRules: {
      increaseIndentPattern: /\{[^}]*$/,
      decreaseIndentPattern: /^\s*\}/,
    },
  });

  monaco.languages.setMonarchTokensProvider("biolang", {
    keywords: biolangKeywords,
    typeKeywords: ["DNA", "RNA", "Protein", "Table", "List", "Record", "Interval", "Variant"],
    tokenizer: {
      root: [
        [/#.*$/, "comment"],
        [/\b(?:dna|rna|protein)"/, { token: "type.identifier", next: "@bioString" }],
        [/[a-zA-Z_][\w]*/, { cases: { "@keywords": "keyword", "@typeKeywords": "type", "@default": "identifier" } }],
        [/\d*\.\d+([eE][+-]?\d+)?/, "number.float"],
        [/0[xX][0-9a-fA-F]+/, "number.hex"],
        [/\d+/, "number"],
        [/"/, { token: "string.quote", next: "@string" }],
        [/[{}()[\]]/, "@brackets"],
        [/\|>|=>|->|==|!=|<=|>=|\+\+|\?\?|[+\-*\/%=<>!]/, "operator"],
        [/[,:;.]/, "delimiter"],
      ],
      string: [
        [/[^\\"]+/, "string"],
        [/\\./, "string.escape"],
        [/"/, { token: "string.quote", next: "@pop" }],
      ],
      bioString: [
        [/[ACGTUNRYSWKMBDHVacgtunryswkmbdhv-]+/, "string.bio"],
        [/"/, { token: "string.quote", next: "@pop" }],
      ],
    },
  });

  monaco.editor.defineTheme("biolang-dark", {
    base: "vs-dark",
    inherit: true,
    rules: [
      { token: "comment", foreground: "66717F", fontStyle: "italic" },
      { token: "keyword", foreground: "C792EA" },
      { token: "type", foreground: "65C7B4" },
      { token: "type.identifier", foreground: "65C7B4", fontStyle: "bold" },
      { token: "string", foreground: "C3E88D" },
      { token: "string.bio", foreground: "7FDBCA" },
      { token: "number", foreground: "F2B86B" },
      { token: "operator", foreground: "89B4FA" },
    ],
    colors: {
      "editor.background": "#15181d",
      "editor.foreground": "#d6dae1",
      "editorLineNumber.foreground": "#515966",
      "editorLineNumber.activeForeground": "#aeb6c2",
      "editorCursor.foreground": "#62d0bd",
      "editor.selectionBackground": "#294f4b",
      "editor.inactiveSelectionBackground": "#253a3a",
      "editor.lineHighlightBackground": "#1a1e24",
      "editorIndentGuide.background1": "#292e36",
      "editorIndentGuide.activeBackground1": "#444c58",
    },
  });
}

export function languageForPath(path: string): string {
  const extension = path.split(".").pop()?.toLowerCase();
  if (extension === "bl") return "biolang";
  if (extension === "toml") return "ini";
  if (extension === "md") return "markdown";
  if (extension === "json") return "json";
  if (extension === "csv" || extension === "tsv") return "plaintext";
  return "plaintext";
}
