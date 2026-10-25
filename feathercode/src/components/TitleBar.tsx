import { useStore } from "../state";
import { activeWs, wsRoot } from "../api";

function FeatherIcon() {
  return (
    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M20.24 12.24a6 6 0 0 0-8.49-8.49L5 10.5V19h8.5z" />
      <line x1="16" y1="8" x2="2" y2="22" />
      <line x1="17.5" y1="15" x2="9" y2="15" />
    </svg>
  );
}

export default function TitleBar() {
  const { workspaces, activeWsId, setActiveWs, set } = useStore();
  const ws = activeWs();

  return (
    <div className="titlebar">
      <div className="brand">
        <FeatherIcon /> FeatherCode
      </div>
      <select
        className="ws-select"
        value={activeWsId ?? ""}
        onChange={(e) => setActiveWs(e.target.value)}
        title="Active workspace"
      >
        <option value="">No workspace</option>
        {workspaces.map((w) => (
          <option key={w.id} value={w.id}>
            {w.kind === "ssh" ? "⇅ " : "▤ "}
            {w.name}
          </option>
        ))}
      </select>
      {ws && (
        <span className="hint" style={{ whiteSpace: "nowrap" }}>
          {ws.kind === "ssh" ? `${ws.sshUser}@${ws.sshHost}:${wsRoot(ws)}` : ws.localPath}
        </span>
      )}
      <div className="spacer" />
      <button className="ghost" onClick={() => set({ wsModalOpen: true })}>
        + Workspace
      </button>
      <button className="ghost" onClick={() => set({ settingsOpen: true })}>
        ⚙ Settings
      </button>
    </div>
  );
}
