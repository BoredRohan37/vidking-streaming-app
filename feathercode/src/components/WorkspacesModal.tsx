import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import { homeDir, basename } from "@tauri-apps/api/path";
import { useStore, uid } from "../state";
import type { Workspace } from "../types";

type Kind = "local" | "ssh" | "git";

export default function WorkspacesModal({ onClose }: { onClose: () => void }) {
  const workspaces = useStore((s) => s.workspaces);
  const [kind, setKind] = useState<Kind>("local");

  // local
  const [localPath, setLocalPath] = useState("");
  // ssh
  const [sshHost, setSshHost] = useState("");
  const [sshPort, setSshPort] = useState("22");
  const [sshUser, setSshUser] = useState("");
  const [sshAuth, setSshAuth] = useState<"password" | "key">("password");
  const [sshSecret, setSshSecret] = useState("");
  const [sshRoot, setSshRoot] = useState("");
  // git
  const [gitUrl, setGitUrl] = useState("");
  const [gitDest, setGitDest] = useState("");

  const [busy, setBusy] = useState("");
  const [testResult, setTestResult] = useState("");

  const refreshWorkspaces = async () => {
    const ws = await invoke<Workspace[]>("db_list_workspaces");
    useStore.setState({ workspaces: ws });
  };

  const pickFolder = async () => {
    const dir = await openFileDialog({ directory: true, multiple: false });
    if (typeof dir === "string") {
      setLocalPath(dir);
      setSshRoot("");
    }
  };

  const browseKey = async () => {
    const f = await openFileDialog({ multiple: false });
    if (typeof f === "string") setSshSecret(f);
  };

  const buildSshWorkspace = (): Workspace => ({
    id: uid(),
    name: `${sshUser}@${sshHost}`,
    kind: "ssh",
    localPath: null,
    sshHost,
    sshPort: parseInt(sshPort || "22", 10) || 22,
    sshUser,
    sshAuth,
    sshSecret,
    sshRoot: sshRoot.trim() || `/home/${sshUser}`,
  });

  const testSsh = async () => {
    if (!sshHost.trim() || !sshUser.trim() || !sshSecret.trim()) {
      setTestResult("Host, user and credentials are required.");
      return;
    }
    setBusy("Testing SSH connection…");
    setTestResult("");
    try {
      const res = await invoke<string>("ssh_test", { workspace: buildSshWorkspace() });
      setTestResult("✓ " + res);
    } catch (e) {
      setTestResult("✗ " + String(e));
    }
    setBusy("");
  };

  const addWorkspace = async (ws: Workspace) => {
    await invoke("db_save_workspace", { workspace: ws });
    await refreshWorkspaces();
    useStore.getState().setActiveWs(ws.id);
    onClose();
  };

  const addLocal = async () => {
    if (!localPath) {
      useStore.getState().showToast("Choose a folder first");
      return;
    }
    let name = localPath.split("/").filter(Boolean).pop() || localPath;
    try {
      name = (await basename(localPath)) || name;
    } catch {
      /* keep fallback */
    }
    await addWorkspace({
      id: uid(),
      name,
      kind: "local",
      localPath,
      sshHost: null, sshPort: null, sshUser: null, sshAuth: null, sshSecret: null, sshRoot: null,
    });
  };

  const addSsh = async () => {
    if (!sshHost.trim() || !sshUser.trim() || !sshSecret.trim()) {
      useStore.getState().showToast("Host, user and credentials are required");
      return;
    }
    await addWorkspace(buildSshWorkspace());
  };

  const addGit = async () => {
    if (!gitUrl.trim()) {
      useStore.getState().showToast("Repository URL is required");
      return;
    }
    const home = await homeDir().catch(() => "/home/user");
    const repoName = gitUrl.split("/").pop()?.replace(/\.git$/, "") || "repo";
    const dest = gitDest.trim() || `${home.replace(/\/$/, "")}/FeatherCode/${repoName}`;
    setBusy(`Cloning ${repoName}…`);
    try {
      const finalDest = await invoke<string>("git_clone", { url: gitUrl.trim(), dest });
      await addWorkspace({
        id: uid(),
        name: repoName,
        kind: "local",
        localPath: finalDest,
        sshHost: null, sshPort: null, sshUser: null, sshAuth: null, sshSecret: null, sshRoot: null,
      });
    } catch (e) {
      useStore.getState().showToast(String(e));
    }
    setBusy("");
  };

  const removeWorkspace = async (ws: Workspace) => {
    await invoke("db_delete_workspace", { id: ws.id });
    if (useStore.getState().activeWsId === ws.id) {
      const rest = workspaces.filter((w) => w.id !== ws.id);
      useStore.setState({ activeWsId: rest[0]?.id ?? null, tabs: [], activePath: null });
      useStore.getState().refreshTree();
    }
    await refreshWorkspaces();
  };

  return (
    <div className="modal-backdrop" onMouseDown={onClose}>
      <div className="modal" onMouseDown={(e) => e.stopPropagation()}>
        <div className="modal-title">Workspaces</div>
        <div className="modal-sub">A workspace is a local folder, a remote SSH directory, or a cloned git repo.</div>

        {workspaces.length > 0 && (
          <>
            <label>Existing workspaces</label>
            {workspaces.map((w) => (
              <div className="provider-row" key={w.id}>
                <div className="grow">
                  <div className="name">
                    {w.kind === "ssh" ? "⇅ " : "▤ "}
                    {w.name} {useStore.getState().activeWsId === w.id && <span className="badge">active</span>}
                  </div>
                  <div className="meta">
                    {w.kind === "ssh"
                      ? `${w.sshUser}@${w.sshHost}:${w.sshPort} → ${w.sshRoot}`
                      : w.localPath}
                  </div>
                </div>
                <button
                  onClick={() => {
                    useStore.getState().setActiveWs(w.id);
                    onClose();
                  }}
                >
                  Open
                </button>
                <button className="danger" onClick={() => removeWorkspace(w)}>
                  Remove
                </button>
              </div>
            ))}
          </>
        )}

        <div className="kind-tabs">
          {(
            [
              ["local", "▤ Local folder"],
              ["ssh", "⇅ SSH remote"],
              ["git", "⬇ Git repo"],
            ] as [Kind, string][]
          ).map(([k, label]) => (
            <div key={k} className={"kind-tab" + (kind === k ? " active" : "")} onClick={() => setKind(k)}>
              {label}
            </div>
          ))}
        </div>

        {kind === "local" && (
          <>
            <label>Folder path</label>
            <div style={{ display: "flex", gap: 6 }}>
              <input value={localPath} placeholder="/home/you/projects/my-app" readOnly />
              <button style={{ whiteSpace: "nowrap" }} onClick={pickFolder}>
                Browse…
              </button>
            </div>
            <div className="modal-actions">
              <button onClick={onClose}>Cancel</button>
              <button className="primary" onClick={addLocal} disabled={!!busy}>
                Add folder
              </button>
            </div>
          </>
        )}

        {kind === "ssh" && (
          <>
            <div className="row2">
              <div>
                <label>Host</label>
                <input value={sshHost} placeholder="server.example.com" onChange={(e) => setSshHost(e.target.value)} />
              </div>
              <div>
                <label>Port</label>
                <input value={sshPort} onChange={(e) => setSshPort(e.target.value)} />
              </div>
            </div>
            <div className="row2">
              <div>
                <label>User</label>
                <input value={sshUser} placeholder="you" onChange={(e) => setSshUser(e.target.value)} />
              </div>
              <div>
                <label>Remote root path</label>
                <input
                  value={sshRoot}
                  placeholder={`/home/${sshUser || "you"}`}
                  onChange={(e) => setSshRoot(e.target.value)}
                />
              </div>
            </div>
            <div className="row2">
              <div>
                <label>Auth method</label>
                <select value={sshAuth} onChange={(e) => setSshAuth(e.target.value as "password" | "key")}>
                  <option value="password">Password</option>
                  <option value="key">Private key file</option>
                </select>
              </div>
              <div>
                <label>{sshAuth === "password" ? "Password" : "Private key path"}</label>
                {sshAuth === "password" ? (
                  <input type="password" value={sshSecret} onChange={(e) => setSshSecret(e.target.value)} />
                ) : (
                  <div style={{ display: "flex", gap: 6 }}>
                    <input value={sshSecret} placeholder="~/.ssh/id_rsa" readOnly />
                    <button style={{ whiteSpace: "nowrap" }} onClick={browseKey}>
                      Browse…
                    </button>
                  </div>
                )}
              </div>
            </div>
            {testResult && <div className="hint" style={{ marginTop: 8 }}>{testResult}</div>}
            <div className="modal-actions">
              <button onClick={testSsh} disabled={!!busy}>
                Test connection
              </button>
              <button onClick={onClose}>Cancel</button>
              <button className="primary" onClick={addSsh} disabled={!!busy}>
                Add remote
              </button>
            </div>
          </>
        )}

        {kind === "git" && (
          <>
            <label>Repository URL</label>
            <input value={gitUrl} placeholder="https://github.com/user/repo.git" onChange={(e) => setGitUrl(e.target.value)} />
            <label>Destination (optional — defaults to ~/FeatherCode/&lt;repo&gt;)</label>
            <input value={gitDest} placeholder="~/FeatherCode/repo" onChange={(e) => setGitDest(e.target.value)} />
            <div className="modal-actions">
              <button onClick={onClose}>Cancel</button>
              <button className="primary" onClick={addGit} disabled={!!busy}>
                Clone & add
              </button>
            </div>
          </>
        )}

        {busy && <div className="hint" style={{ marginTop: 10 }}>{busy}</div>}
      </div>
    </div>
  );
}
