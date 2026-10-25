import { invoke } from "@tauri-apps/api/core";
import type { FileEntry, Workspace } from "./types";
import { useStore } from "./state";

export function uid(): string {
  return typeof crypto !== "undefined" && crypto.randomUUID
    ? crypto.randomUUID()
    : Math.random().toString(36).slice(2) + Date.now().toString(36);
}

export function activeWs(): Workspace | null {
  const { workspaces, activeWsId } = useStore.getState();
  return workspaces.find((w) => w.id === activeWsId) ?? null;
}

export function isSsh(ws: Workspace | null): boolean {
  return ws?.kind === "ssh";
}

export function wsRoot(ws: Workspace | null): string {
  if (!ws) return "/";
  if (isSsh(ws)) return ws.sshRoot || "/";
  return ws.localPath || "/";
}

// Workspace-aware file operations: route to local fs or remote sftp
export const listDir = (path: string): Promise<FileEntry[]> => {
  const ws = activeWs();
  return ws && isSsh(ws) ? invoke("ssh_list", { workspaceId: ws.id, path }) : invoke("fs_list", { path });
};

export const readFile = (path: string): Promise<string> => {
  const ws = activeWs();
  return ws && isSsh(ws) ? invoke("ssh_read", { workspaceId: ws.id, path }) : invoke("fs_read", { path });
};

export const writeFile = (path: string, content: string): Promise<void> => {
  const ws = activeWs();
  return ws && isSsh(ws) ? invoke("ssh_write", { workspaceId: ws.id, path, content }) : invoke("fs_write", { path, content });
};

export const createEntry = (path: string, isDir: boolean): Promise<void> => {
  const ws = activeWs();
  return ws && isSsh(ws) ? invoke("ssh_create", { workspaceId: ws.id, path, isDir }) : invoke("fs_create", { path, isDir });
};

export const deleteEntry = (path: string): Promise<void> => {
  const ws = activeWs();
  return ws && isSsh(ws) ? invoke("ssh_delete", { workspaceId: ws.id, path }) : invoke("fs_delete", { path });
};

export const renameEntry = (from: string, to: string): Promise<void> => {
  const ws = activeWs();
  return ws && isSsh(ws) ? invoke("ssh_rename", { workspaceId: ws.id, from, to }) : invoke("fs_rename", { from, to });
};
