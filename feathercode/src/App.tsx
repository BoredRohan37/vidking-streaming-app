import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { useStore } from "./state";
import TitleBar from "./components/TitleBar";
import ActivityBar from "./components/ActivityBar";
import FileTreePanel from "./components/FileTree";
import EditorArea from "./components/EditorArea";
import TerminalPanel from "./components/TerminalPanel";
import ChatPanel from "./components/ChatPanel";
import SettingsModal from "./components/SettingsModal";
import WorkspacesModal from "./components/WorkspacesModal";
import DiffModal from "./components/DiffModal";
import InlinePrompt from "./components/InlinePrompt";

export default function App() {
  const { loadAll, sidebarOpen, chatOpen, settingsOpen, wsModalOpen, toast, saveFile, set } =
    useStore();

  // initial load
  useEffect(() => {
    loadAll();
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // global keyboard shortcuts
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.ctrlKey && !e.altKey && !e.metaKey) {
        const k = e.key.toLowerCase();
        if (k === "s") {
          e.preventDefault();
          const p = useStore.getState().activePath;
          if (p) saveFile(p);
        } else if (k === "b") {
          e.preventDefault();
          useStore.setState({ sidebarOpen: !useStore.getState().sidebarOpen });
        } else if (e.key === "`") {
          e.preventDefault();
          useStore.setState({ terminalOpen: !useStore.getState().terminalOpen });
        }
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [saveFile]);

  // AI streaming events (chat + inline prompt share these)
  useEffect(() => {
    const offs = [
      listen<{ requestId: string; delta: string; reasoning: boolean }>("ai-chunk", ({ payload }) => {
        const s = useStore.getState();
        if (s.streamingReqId && payload.requestId === s.streamingReqId) {
          const m = s.chat.find((x) => x.id === s.streamingMsgId);
          if (!m) return;
          if (payload.reasoning) {
            s.updateChat(m.id, { reasoning: (m.reasoning || "") + payload.delta });
          } else {
            s.updateChat(m.id, { content: m.content + payload.delta });
          }
        } else if (s.inline && payload.requestId === s.inline.reqId && !payload.reasoning) {
          useStore.setState({ inline: { ...s.inline, result: s.inline.result + payload.delta } });
        }
      }),
      listen<{ requestId: string }>("ai-done", ({ payload }) => {
        const s = useStore.getState();
        if (s.streamingReqId && payload.requestId === s.streamingReqId) {
          s.updateChat(s.streamingMsgId, { done: true });
          useStore.setState({ streaming: false, streamingReqId: "" });
        } else if (s.inline && payload.requestId === s.inline.reqId) {
          useStore.setState({ inline: { ...s.inline, done: true, busy: false } });
        }
      }),
      listen<{ requestId: string; error: string }>("ai-error", ({ payload }) => {
        const s = useStore.getState();
        if (s.streamingReqId && payload.requestId === s.streamingReqId) {
          const m = s.chat.find((x) => x.id === s.streamingMsgId);
          s.updateChat(s.streamingMsgId, {
            content: (m?.content || "") + `\n\n**Error:** ${payload.error}`,
            error: true,
            done: true,
          });
          useStore.setState({ streaming: false, streamingReqId: "" });
        } else if (s.inline && payload.requestId === s.inline.reqId) {
          useStore.setState({
            inline: { ...s.inline, result: "Error: " + payload.error, done: true, busy: false },
          });
        }
      }),
    ];
    return () => {
      offs.forEach((p) => p.then((f) => f()));
    };
  }, []);

  return (
    <div className="app">
      <TitleBar />
      <div className="main">
        <ActivityBar />
        {sidebarOpen && <FileTreePanel />}
        <div className="center">
          <EditorArea />
          <TerminalPanel />
        </div>
        {chatOpen && <ChatPanel />}
      </div>

      {settingsOpen && <SettingsModal onClose={() => set({ settingsOpen: false })} />}
      {wsModalOpen && <WorkspacesModal onClose={() => set({ wsModalOpen: false })} />}
      <DiffModal />
      <InlinePrompt />
      {toast && <div className="toast">{toast}</div>}
    </div>
  );
}
