import Editor from "@monaco-editor/react";
import { useStore, uid } from "../state";
import { langFor } from "../utils";
import * as monaco from "monaco-editor";

export default function EditorArea() {
  const tabs = useStore((s) => s.tabs);
  const activePath = useStore((s) => s.activePath);
  const setTabContent = useStore((s) => s.setTabContent);
  const active = tabs.find((t) => t.path === activePath);

  return (
    <div className="editor-area">
      <div className="tabs">
        {tabs.map((t) => (
          <div
            key={t.path}
            className={"tab" + (t.path === activePath ? " active" : "")}
            onClick={() => useStore.getState().setActive(t.path)}
            title={t.path}
          >
            <span>{t.name}{t.dirty ? " •" : ""}</span>
            <span
              className="tab-close"
              onClick={(e) => {
                e.stopPropagation();
                useStore.getState().closeTab(t.path);
              }}
            >
              ×
            </span>
          </div>
        ))}
        {!tabs.length && <div className="tabs-empty">Open a file from the explorer to start editing</div>}
      </div>
      <div className="editor-host">
        {active && (
          <Editor
            key={active.path}
            path={active.path}
            language={langFor(active.path)}
            theme="feather"
            value={active.content}
            onChange={(v) => setTabContent(active.path, v ?? "")}
            onMount={(editor, m) => {
              useStore.setState({ editor });
              // Ctrl+K: inline AI edit of selection
              editor.addCommand(m.KeyMod.CtrlCmd | m.KeyCode.KeyK, () => {
                const s = useStore.getState();
                const sel = editor.getSelection();
                const model = editor.getModel();
                if (!sel || sel.isEmpty() || !model) {
                  s.showToast("Select some code first, then press Ctrl+K");
                  return;
                }
                useStore.setState({
                  inline: {
                    reqId: uid(),
                    sel: model.getValueInRange(sel),
                    path: s.activePath ?? "",
                    range: {
                      startLineNumber: sel.startLineNumber,
                      startColumn: sel.startColumn,
                      endLineNumber: sel.endLineNumber,
                      endColumn: sel.endColumn,
                    },
                    input: "",
                    result: "",
                    busy: false,
                    done: false,
                  },
                });
              });
            }}
            options={{
              fontSize: 13,
              fontFamily: "'JetBrains Mono','Fira Code','DejaVu Sans Mono',monospace",
              fontLigatures: true,
              minimap: { enabled: false },
              scrollBeyondLastLine: false,
              smoothScrolling: true,
              cursorBlinking: "smooth",
              renderWhitespace: "selection",
              tabSize: 2,
            }}
          />
        )}
        {!tabs.length && (
          <div
            style={{
              position: "absolute", inset: 0, display: "flex", alignItems: "center", justifyContent: "center",
              color: "var(--muted)", flexDirection: "column", gap: 8, pointerEvents: "none",
            }}
          >
            <div style={{ fontSize: 15, color: "var(--accent)" }}>FeatherCode</div>
            <div style={{ fontSize: 12 }}>Ctrl+K — edit selection with AI · Ctrl+S — save · Ctrl+` — terminal</div>
          </div>
        )}
      </div>
    </div>
  );
}

// keep the monaco namespace referenced for typings above
void monaco;
