import { useEffect, useState } from "react";
import { DiffEditor } from "@monaco-editor/react";
import { useStore } from "../state";
import { readFile, writeFile } from "../api";
import { langFor, resolveTarget } from "../utils";

export default function DiffModal() {
  const diffRequest = useStore((s) => s.diffRequest);
  const tabs = useStore((s) => s.tabs);
  const [target, setTarget] = useState("");
  const [original, setOriginal] = useState("");
  const [applying, setApplying] = useState(false);

  useEffect(() => {
    if (diffRequest) {
      const fallback = diffRequest.target || useStore.getState().activePath || "";
      setTarget(fallback);
    }
  }, [diffRequest]);

  useEffect(() => {
    if (!target) {
      setOriginal("");
      return;
    }
    readFile(target)
      .then(setOriginal)
      .catch(() => setOriginal(""));
  }, [target]);

  if (!diffRequest) return null;
  const close = () => useStore.setState({ diffRequest: null });

  const apply = async () => {
    if (!target) return;
    setApplying(true);
    try {
      await writeFile(target, diffRequest.code);
      // refresh open tab if present
      const existing = useStore.getState().tabs.find((t) => t.path === target);
      if (existing) {
        useStore.setState({
          tabs: useStore.getState().tabs.map((t) =>
            t.path === target ? { ...t, content: diffRequest.code, dirty: false } : t,
          ),
          activePath: target,
        });
      }
      useStore.getState().showToast("Applied changes to " + target.split("/").pop());
      close();
    } catch (e) {
      useStore.getState().showToast("Apply failed: " + String(e));
    }
    setApplying(false);
  };

  return (
    <div className="modal-backdrop" onMouseDown={close}>
      <div className="modal diff-modal" onMouseDown={(e) => e.stopPropagation()}>
        <div className="modal-title">AI change review</div>
        <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
          <input
            style={{ flex: 1 }}
            value={target}
            placeholder="/absolute/path/to/file or path/relative/to/workspace/root"
            onChange={(e) => setTarget(e.target.value)}
          />
          {tabs.some((t) => t.path === target) && <span className="hint">open in editor</span>}
        </div>
        <div className="diff-editor-host">
          <DiffEditor
            original={original}
            modified={diffRequest.code}
            language={langFor(target)}
            theme="feather"
            options={{
              readOnly: true,
              renderSideBySide: true,
              minimap: { enabled: false },
              fontSize: 12.5,
              scrollBeyondLastLine: false,
            }}
          />
        </div>
        <div className="modal-actions">
          <span className="hint" style={{ marginRight: "auto" }}>
            Original ← current file · Modified ← AI proposal
          </span>
          <button onClick={close}>Cancel</button>
          <button className="primary" onClick={apply} disabled={!target || applying}>
            {applying ? "Applying…" : "Apply"}
          </button>
        </div>
      </div>
    </div>
  );
}
