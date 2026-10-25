export type ProviderKind = "glm" | "openai" | "anthropic" | "openai_compat";

export interface Provider {
  id: string;
  name: string;
  kind: ProviderKind;
  baseUrl: string;
  apiKey: string;
  model: string;
}

export interface Workspace {
  id: string;
  name: string;
  kind: "local" | "ssh";
  localPath: string | null;
  sshHost: string | null;
  sshPort: number | null;
  sshUser: string | null;
  sshAuth: "password" | "key" | null;
  sshSecret: string | null;
  sshRoot: string | null;
}

export interface FileEntry {
  name: string;
  path: string;
  isDir: boolean;
  size: number;
}

export interface Tab {
  path: string;
  name: string;
  content: string;
  dirty: boolean;
}

export interface ChatMsg {
  id: string;
  role: "user" | "assistant";
  content: string;
  reasoning?: string;
  error?: boolean;
  done?: boolean;
}

export interface RangeSel {
  startLineNumber: number;
  startColumn: number;
  endLineNumber: number;
  endColumn: number;
}

export interface InlineState {
  reqId: string;
  sel: string;
  path: string;
  range: RangeSel;
  input: string;
  result: string;
  busy: boolean;
  done: boolean;
}

export interface DiffRequest {
  code: string;
  target: string | null;
}
