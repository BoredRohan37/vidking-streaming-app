import * as monaco from "monaco-editor";
import { loader } from "@monaco-editor/react";
import editorWorker from "monaco-editor/esm/vs/editor/editor.worker?worker";
import jsonWorker from "monaco-editor/esm/vs/language/json/json.worker?worker";
import cssWorker from "monaco-editor/esm/vs/language/css/css.worker?worker";
import htmlWorker from "monaco-editor/esm/vs/language/html/html.worker?worker";
import tsWorker from "monaco-editor/esm/vs/language/typescript/ts.worker?worker";

(self as any).MonacoEnvironment = {
  getWorker(_label: string, label: string) {
    if (label === "json") return new jsonWorker();
    if (label === "css" || label === "scss" || label === "less") return new cssWorker();
    if (label === "html" || label === "handlebars" || label === "razor") return new htmlWorker();
    if (label === "typescript" || label === "javascript") return new tsWorker();
    return new editorWorker();
  },
};

monaco.editor.defineTheme("feather", {
  base: "vs-dark",
  inherit: true,
  rules: [
    { token: "comment", foreground: "6a6a7a", fontStyle: "italic" },
    { token: "keyword", foreground: "7dd3fc" },
    { token: "string", foreground: "a5e0b8" },
    { token: "number", foreground: "f0b37e" },
  ],
  colors: {
    "editor.background": "#1b1c24",
    "editor.foreground": "#d6d6e0",
    "editorLineNumber.foreground": "#4a4a5a",
    "editorLineNumber.activeForeground": "#4fd1c5",
    "editor.selectionBackground": "#2c3e50",
    "editor.lineHighlightBackground": "#20222c",
    "editorCursor.foreground": "#4fd1c5",
    "editorIndentGuide.background1": "#262631",
    "editorGutter.background": "#1b1c24",
    "diffEditor.insertedTextBackground": "#4fd1c520",
    "diffEditor.removedTextBackground": "#f5656520",
  },
});

loader.config({ monaco });
