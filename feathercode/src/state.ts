import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type { ChatMsg, DiffRequest, FileEntry, InlineState, Provider, Tab, Workspace } from "./types";
import {
  createEntry as createRemote,
  deleteEntry as deleteRemote,
  listDir,
  readFile,
  renameEntry as renameRemote,
  uid,
  writeFile,
  wsRoot,
} from "./api";

interface Store {
  providers: Provider[];
  workspaces: Workspace[];
  activeWsId: string | null;
  defaultProviderId: string;

  tabs: Tab[];
  activePath: string | null;
  tree: FileEntry[];
  childrenMap: Record<string, FileEntry[]>;
  expanded: Record<string, boolean>;

  chat: ChatMsg[];
  streaming: boolean;
  streamingReqId: string;
  streamingMsgId: string;
  contextPaths: string[];
  autoIncludeActive: boolean;

  sidebarOpen: boolean;
  chatOpen: boolean;
  terminalOpen: boolean;

  editor: any;
  inline: InlineState | null;
  diffRequest: DiffRequest | null;
  settingsOpen: boolean;
  wsModalOpen: boolean;
  toast: string;

  set(p: Partial<Store>): void;
  showToast(t: string): void;
  loadAll(): Promise<void>;
  setActiveWs(id: string): void;
  refreshTree(): Promise<void>;
  toggleDir(path: string): Promise<void>;
  openFile(path: string): Promise<void>;
  closeTab(path: string): void;
  setActive(path: string): void;
  setTabContent(path: string, content: string): void;
  saveFile(path: string): Promise<void>;
  createEntry(parent: string, name: string, isDir: boolean): Promise<void>;
  deleteEntry(path: string): Promise<void>;
  renameEntry(path: string, newName: string): Promise<void>;
  addContext(path: string): void;
  removeContext(path: string): void;
  appendChat(m: ChatMsg): void;
  updateChat(id: string, patch: Partial<ChatMsg>): void;
}

let toastTimer: ReturnType<typeof setTimeout> | undefined;

export const useStore = create<Store>()((set, get) => ({
  providers: [],
  workspaces: [],
  activeWsId: null,
  defaultProviderId: "",

  tabs: [],
  activePath: null,
  tree: [],
  childrenMap: {},
  expanded: {},

  chat: [],
  streaming: false,
  streamingReqId: "",
  streamingMsgId: "",
  contextPaths: [],
  autoIncludeActive: true,

  sidebarOpen: true,
  chatOpen: true,
  terminalOpen: false,

  editor: null,
  inline: null,
  diffRequest: null,
  settingsOpen: false,
  wsModalOpen: false,
  toast: "",

  set: (p) => set(p),

  showToast: (t) => {
    set({ toast: t });
    if (toastTimer) clearTimeout(toastTimer);
    toastTimer = setTimeout(() => set({ toast: "" }), 3000);
  },

  loadAll: async () => {
    try {
      const providers = await invoke<Provider[]>("db_list_providers");
      const workspaces = await invoke<Workspace[]>("db_list_workspaces");
      let def = await invoke<string | null>("db_get_setting", { key: "defaultProvider" });
      if (!def && providers.length) def = providers[0].id;
      set({
        providers,
        workspaces,
        defaultProviderId: def || "",
        activeWsId: workspaces[0]?.id ?? null,
      });
      await get().refreshTree();
    } catch (e) {
      get().showToast(String(e));
    }
  },

  setActiveWs: (id) => {
    set({ activeWsId: id, tabs: [], activePath: null, childrenMap: {}, expanded: {}, inline: null });
    get().refreshTree();
  },

  refreshTree: async () => {
    const ws = get().workspaces.find((w) => w.id === get().activeWsId);
    if (!ws) {
      set({ tree: [] });
      return;
    }
    try {
      const root = wsRoot(ws);
      const tree = await listDir(root);
      set({ tree, childrenMap: { [root]: tree }, expanded: {} });
    } catch (e) {
      get().showToast(String(e));
      set({ tree: [] });
    }
  },

  toggleDir: async (path) => {
    if (get().expanded[path]) {
      set({ expanded: { ...get().expanded, [path]: false } });
      return;
    }
    try {
      if (!get().childrenMap[path]) {
        const children = await listDir(path);
        set({ childrenMap: { ...get().childrenMap, [path]: children } });
      }
      set({ expanded: { ...get().expanded, [path]: true } });
    } catch (e) {
      get().showToast(String(e));
    }
  },

  openFile: async (path) => {
    const existing = get().tabs.find((t) => t.path === path);
    if (existing) {
      set({ activePath: path });
      return;
    }
    try {
      const content = await readFile(path);
      set({
        tabs: [...get().tabs, { path, name: path.split("/").pop() || path, content, dirty: false }],
        activePath: path,
      });
    } catch (e) {
      get().showToast(String(e));
    }
  },

  closeTab: (path) => {
    const tabs = get().tabs.filter((t) => t.path !== path);
    let activePath = get().activePath;
    if (activePath === path) {
      const idx = get().tabs.findIndex((t) => t.path === path);
      activePath = tabs[Math.min(idx, tabs.length - 1)]?.path ?? null;
    }
    set({ tabs, activePath });
  },

  setActive: (path) => set({ activePath: path }),

  setTabContent: (path, content) => {
    set({ tabs: get().tabs.map((t) => (t.path === path ? { ...t, content, dirty: true } : t)) });
  },

  saveFile: async (path) => {
    const tab = get().tabs.find((t) => t.path === path);
    if (!tab) return;
    try {
      await writeFile(path, tab.content);
      set({ tabs: get().tabs.map((t) => (t.path === path ? { ...t, dirty: false } : t)) });
    } catch (e) {
      get().showToast(String(e));
    }
  },

  createEntry: async (parent, name, isDir) => {
    const path = parent.replace(/\/$/, "") + "/" + name;
    try {
      await createRemote(path, isDir);
      const children = await listDir(parent);
      set({
        childrenMap: { ...get().childrenMap, [parent]: children },
        expanded: { ...get().expanded, [parent]: true },
      });
    } catch (e) {
      get().showToast(String(e));
    }
  },

  deleteEntry: async (path) => {
    try {
      await deleteRemote(path);
      const parent = path.slice(0, path.lastIndexOf("/")) || "/";
      const children = await listDir(parent);
      const tabs = get().tabs.filter((t) => t.path !== path && !t.path.startsWith(path + "/"));
      const ap = get().activePath;
      const gone = ap === path || (!!ap && ap.startsWith(path + "/"));
      set({
        childrenMap: { ...get().childrenMap, [parent]: children },
        tabs,
        activePath: gone ? tabs[0]?.path ?? null : get().activePath,
      });
    } catch (e) {
      get().showToast(String(e));
    }
  },

  renameEntry: async (path, newName) => {
    const parent = path.slice(0, path.lastIndexOf("/")) || "/";
    const to = parent + "/" + newName;
    try {
      await renameRemote(path, to);
      const children = await listDir(parent);
      const tabs = get().tabs.map((t) => (t.path === path ? { ...t, path: to, name: newName } : t));
      set({
        childrenMap: { ...get().childrenMap, [parent]: children },
        tabs,
        activePath: get().activePath === path ? to : get().activePath,
      });
    } catch (e) {
      get().showToast(String(e));
    }
  },

  addContext: (path) => {
    if (!get().contextPaths.includes(path)) set({ contextPaths: [...get().contextPaths, path] });
    if (!get().chatOpen) set({ chatOpen: true });
  },

  removeContext: (path) => set({ contextPaths: get().contextPaths.filter((p) => p !== path) }),

  appendChat: (m) => set({ chat: [...get().chat, m] }),

  updateChat: (id, patch) => {
    set({ chat: get().chat.map((m) => (m.id === id ? { ...m, ...patch } : m)) });
  },
}));

export { uid };
