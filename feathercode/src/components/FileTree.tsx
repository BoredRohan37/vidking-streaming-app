import { useEffect, useState } from "react";
import { useStore } from "../state";
import { activeWs, wsRoot } from "../api";
import type { FileEntry } from "../types";
import { ConfirmDialog, InputDialog } from "./Dialogs";

interface MenuState {
  x: number;
  y: number;
  entry: FileEntry;
}
interface DialogState {
  type: "newfile" | "newdir" | "rename" | "rootfile" | "rootdir";
  parent: string;
  initial?: string;
}

function TreeNode({ entry, depth }: { entry: FileEntry; depth: number }) {
  const expanded = useStore((s) => !!s.expanded[entry.path]);
  const children = useStore((s) => s.childrenMap[entry.path]);
  const activePath = useStore((s) => s.activePath);
  const toggleDir = useStore((s) => s.toggleDir);
  const openFile = useStore((s) => s.openFile);
  const [menu, setMenu] = useState<MenuState | null>(null);
  const [dialog, setDialog] = useState<DialogState | null>(null);
  const [confirmDel, setConfirmDel] = useState(false);

  useEffect(() => {
    if (!menu) return;
    const close = () => setMenu(null);
    window.addEventListener("click", close);
    return () => window.removeEventListener("click", close);
  }, [menu]);

  const title = (d: DialogState) => {
    switch (d.type) {
      case "newfile": return `New file in ${entry.name}`;
      case "newdir": return `New folder in ${entry.name}`;
      case "rootfile": return "New file in workspace root";
      case "rootdir": return "New folder in workspace root";
      default: return `Rename ${entry.name}`;
    }
  };
  void title;

  return (
    <>
      <div
        className={"tree-item " + (entry.isDir ? "dir" : "file") + (activePath === entry.path ? " selected" : "")}
        style={{ paddingLeft: 10 + depth * 14 }}
        onClick={() => (entry.isDir ? toggleDir(entry.path) : openFile(entry.path))}
        onContextMenu={(e) => {
          e.preventDefault();
          e.stopPropagation();
          setMenu({ x: e.clientX, y: e.clientY, entry });
        }}
        title={entry.path}
      >
        <span className="chev">{entry.isDir ? (expanded ? "▾" : "▸") : ""}</span>
        <span className="ficon">{entry.isDir ? "▣" : "▪"}</span>
        <span>{entry.name}</span>
      </div>
      {entry.isDir && expanded && children?.map((c) => <TreeNode key={c.path} entry={c} depth={depth + 1} />)}

      {menu && (
        <div className="ctx-menu" style={{ left: menu.x, top: menu.y }} onClick={(e) => e.stopPropagation()}>
          {entry.isDir ? (
            <>
              <div className="ctx-item" onClick={() => { setMenu(null); setDialog({ type: "newfile", parent: entry.path }); }}>
                New File…
              </div>
              <div className="ctx-item" onClick={() => { setMenu(null); setDialog({ type: "newdir", parent: entry.path }); }}>
                New Folder…
              </div>
            </>
          ) : null}
          <div
            className="ctx-item"
            onClick={() => {
              setMenu(null);
              useStore.getState().addContext(entry.path);
              useStore.getState().showToast("Added to AI context: " + entry.name);
            }}
          >
            Add to AI context
          </div>
          <div className="ctx-item" onClick={() => { setMenu(null); setDialog({ type: "rename", parent: entry.path, initial: entry.name }); }}>
            Rename…
          </div>
          <div className="ctx-item danger" onClick={() => { setMenu(null); setConfirmDel(true); }}>
            Delete…
          </div>
        </div>
      )}

      {dialog && (
        <InputDialog
          title={dialog.type === "rename" ? `Rename ${entry.name}` : `New ${dialog.type.endsWith("dir") ? "folder" : "file"}${dialog.type.startsWith("root") ? " (workspace root)" : " in " + entry.name}`}
          initial={dialog.initial ?? ""}
          placeholder={dialog.type === "rename" ? "New name" : "Name"}
          onCancel={() => setDialog(null)}
          onOk={(name) => {
            const n = name.trim();
            setDialog(null);
            if (!n) return;
            const s = useStore.getState();
            if (dialog.type === "newfile") s.createEntry(dialog.parent, n, false);
            else if (dialog.type === "newdir") s.createEntry(dialog.parent, n, true);
            else s.renameEntry(entry.path, n);
          }}
        />
      )}

      {confirmDel && (
        <ConfirmDialog
          message={`Delete "${entry.name}"? This cannot be undone.`}
          onCancel={() => setConfirmDel(false)}
          onOk={() => {
            setConfirmDel(false);
            useStore.getState().deleteEntry(entry.path);
          }}
        />
      )}
    </>
  );
}

export default function FileTreePanel() {
  const tree = useStore((s) => s.tree);
  const ws = activeWs();
  const root = ws ? wsRoot(ws) : "/";
  const [dialog, setDialog] = useState<DialogState | null>(null);

  return (
    <div className="sidebar">
      <div className="sidebar-head">
        <span>Explorer</span>
        <span className="actions">
          <button className="icon-btn" title="New file in workspace root" onClick={() => setDialog({ type: "rootfile", parent: root })}>
            ＋
          </button>
          <button className="icon-btn" title="Refresh" onClick={() => useStore.getState().refreshTree()}>
            ⟳
          </button>
        </span>
      </div>
      <div className="tree">
        {tree.length === 0 ? (
          <div className="tree-empty">
            {ws
              ? "Empty folder.\n\nUse ＋ to create a file, or add content on disk and hit ⟳."
              : "No workspace selected.\n\nAdd one with “+ Workspace”: a local folder, an SSH host, or a git repo."}
          </div>
        ) : (
          tree.map((e) => <TreeNode key={e.path} entry={e} depth={0} />)
        )}
      </div>
      {dialog && (
        <InputDialog
          title={dialog.type === "rootdir" ? "New folder (workspace root)" : "New file (workspace root)"}
          initial=""
          placeholder="Name"
          onCancel={() => setDialog(null)}
          onOk={(name) => {
            const n = name.trim();
            setDialog(null);
            if (!n) return;
            useStore.getState().createEntry(root, n, dialog.type === "rootdir");
          }}
        />
      )}
    </div>
  );
}
