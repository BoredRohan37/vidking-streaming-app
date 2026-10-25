import type { ProviderKind, RangeSel, Workspace } from "./types";
import { useStore } from "./state";

const EXT_LANG: Record<string, string> = {
  ts: "typescript", tsx: "typescript", js: "javascript", jsx: "javascript", mjs: "javascript", cjs: "javascript",
  json: "json", css: "css", scss: "scss", less: "less", html: "html", htm: "html", vue: "html", svelte: "html",
  md: "markdown", py: "python", rs: "rust", go: "go", c: "c", h: "c", cpp: "cpp", cc: "cpp", cxx: "cpp", hpp: "cpp",
  java: "java", kt: "kotlin", kts: "kotlin", sh: "shell", bash: "shell", zsh: "shell", fish: "shell",
  yml: "yaml", yaml: "yaml", toml: "ini", ini: "ini", conf: "ini", sql: "sql", php: "php", rb: "ruby",
  xml: "xml", svg: "xml", lua: "lua", dart: "dart", swift: "swift", cs: "csharp", csx: "csharp",
  dockerfile: "dockerfile", makefile: "makefile", graphql: "graphql", proto: "proto",
};

export function langFor(path: string): string {
  const base = path.split("/").pop() || path;
  if (/^dockerfile$/i.test(base)) return "dockerfile";
  if (/^makefile$/i.test(base)) return "makefile";
  const ext = base.includes(".") ? base.split(".").pop()!.toLowerCase() : "";
  return EXT_LANG[ext] || "plaintext";
}

/** A monaco code-fence "language" tag is treated as a file path if it looks like one */
export function looksLikePath(lang: string): boolean {
  if (!lang) return false;
  if (/\s/.test(lang)) return false;
  return lang.includes("/") || lang.includes(".");
}

export function stripFences(text: string): string {
  const m = text.match(/```[\w./-]*\n?([\s\S]*?)```/);
  const inner = m ? m[1] : text;
  return inner.trim();
}

/** Resolve a possibly-relative path against the workspace root */
export function resolveTarget(path: string): string {
  if (!path) return path;
  if (path.startsWith("/") || path.startsWith("~")) return path;
  const { workspaces, activeWsId } = useStore.getState();
  const ws = workspaces.find((w) => w.id === activeWsId);
  const root = ws ? (ws.kind === "ssh" ? ws.sshRoot || "/" : ws.localPath || "/") : "/";
  return root.replace(/\/$/, "") + "/" + path;
}

export function buildSystemPrompt(ws: Workspace | null, files: { path: string; content: string }[]): string {
  let s =
    "You are FeatherCode, a concise AI coding assistant embedded in a lightweight code editor for Linux.\n";
  if (ws) {
    const desc =
      ws.kind === "ssh"
        ? `remote SSH workspace ${ws.sshUser}@${ws.sshHost}:${ws.sshRoot}`
        : `local folder ${ws.localPath}`;
    s += `The user's active workspace is "${ws.name}" (${desc}).\n`;
  }
  if (files.length) {
    s += "\nContext files provided by the user:\n";
    for (const f of files) {
      s += `\n--- File: ${f.path} ---\n${f.content}\n--- end of ${f.path} ---\n`;
    }
  }
  s +=
    "\nGuidelines:\n" +
    "- Keep explanations short and useful.\n" +
    "- When proposing changes to a file, output the FULL updated file content in a fenced code block whose info string is the file path (e.g. ```src/app.ts).\n" +
    "- Use markdown code fences with a language tag for all other code snippets.";
  return s;
}

export const DEFAULT_BASE: Record<ProviderKind, string> = {
  glm: "https://api.z.ai/api/paas/v4",
  openai: "https://api.openai.com/v1",
  anthropic: "https://api.anthropic.com",
  openai_compat: "",
};

export const MODEL_HINT: Record<ProviderKind, string> = {
  glm: "glm-4.6",
  openai: "gpt-4o",
  anthropic: "claude-sonnet-4-5",
  openai_compat: "model-name",
};

export function formatRange(r: RangeSel): string {
  return `${r.startLineNumber}:${r.startColumn}-${r.endLineNumber}:${r.endColumn}`;
}
