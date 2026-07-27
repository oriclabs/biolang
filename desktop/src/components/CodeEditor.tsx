import Editor, { loader } from "@monaco-editor/react";
import * as monaco from "monaco-editor/esm/vs/editor/editor.api";
import editorWorker from "monaco-editor/esm/vs/editor/editor.worker?worker";
import jsonWorker from "monaco-editor/esm/vs/language/json/json.worker?worker";
import "monaco-editor/esm/vs/basic-languages/ini/ini.contribution";
import "monaco-editor/esm/vs/basic-languages/markdown/markdown.contribution";
import "monaco-editor/esm/vs/basic-languages/python/python.contribution";
import "monaco-editor/esm/vs/basic-languages/r/r.contribution";
import "monaco-editor/esm/vs/language/json/monaco.contribution";

self.MonacoEnvironment = {
  getWorker(_moduleId: string, label: string) {
    if (label === "json") return new jsonWorker();
    return new editorWorker();
  },
};

loader.config({ monaco });

export default Editor;
