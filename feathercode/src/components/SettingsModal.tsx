import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useStore } from "../state";
import { DEFAULT_BASE, MODEL_HINT } from "../utils";
import type { Provider, ProviderKind } from "../types";

export default function SettingsModal({ onClose }: { onClose: () => void }) {
  const providers = useStore((s) => s.providers);
  const defaultProviderId = useStore((s) => s.defaultProviderId);
  const [editing, setEditing] = useState<Provider | null>(null);

  const reload = async () => {
    const providers = await invoke<Provider[]>("db_list_providers");
    useStore.setState({ providers });
  };

  const save = async () => {
    if (!editing) return;
    if (!editing.name.trim() || !editing.apiKey.trim() || !editing.model.trim()) {
      useStore.getState().showToast("Name, API key and model are required");
      return;
    }
    await invoke("db_save_provider", { provider: editing });
    if (!defaultProviderId) {
      useStore.setState({ defaultProviderId: editing.id });
      await invoke("db_set_setting", { key: "defaultProvider", value: editing.id });
    }
    await reload();
    setEditing(null);
  };

  const remove = async (p: Provider) => {
    await invoke("db_delete_provider", { id: p.id });
    if (defaultProviderId === p.id) {
      const rest = providers.filter((x) => x.id !== p.id);
      useStore.setState({ defaultProviderId: rest[0]?.id ?? "" });
      if (rest[0]) await invoke("db_set_setting", { key: "defaultProvider", value: rest[0].id });
    }
    await reload();
  };

  const setDefault = async (id: string) => {
    useStore.setState({ defaultProviderId: id });
    await invoke("db_set_setting", { key: "defaultProvider", value: id });
  };

  return (
    <div className="modal-backdrop" onMouseDown={onClose}>
      <div className="modal" onMouseDown={(e) => e.stopPropagation()}>
        <div className="modal-title">Settings</div>
        <div className="modal-sub">
          API keys are stored locally in an SQLite database on this machine and sent only to the
          provider you configure.
        </div>

        {editing ? (
          <>
            <label>Display name</label>
            <input
              autoFocus
              value={editing.name}
              placeholder="e.g. Z.ai GLM"
              onChange={(e) => setEditing({ ...editing, name: e.target.value })}
            />
            <div className="row2">
              <div>
                <label>Provider kind</label>
                <select
                  value={editing.kind}
                  onChange={(e) => {
                    const kind = e.target.value as ProviderKind;
                    setEditing({ ...editing, kind, baseUrl: DEFAULT_BASE[kind], model: MODEL_HINT[kind] });
                  }}
                >
                  <option value="glm">GLM (Z.ai)</option>
                  <option value="openai">OpenAI</option>
                  <option value="anthropic">Anthropic</option>
                  <option value="openai_compat">Any OpenAI-compatible</option>
                </select>
              </div>
              <div>
                <label>Model</label>
                <input
                  value={editing.model}
                  placeholder={MODEL_HINT[editing.kind]}
                  onChange={(e) => setEditing({ ...editing, model: e.target.value })}
                />
              </div>
            </div>
            <label>Base URL {editing.kind === "openai_compat" ? "(required)" : "(optional — has a sensible default)"}</label>
            <input
              value={editing.baseUrl}
              placeholder={DEFAULT_BASE[editing.kind] || "https://api.example.com/v1"}
              onChange={(e) => setEditing({ ...editing, baseUrl: e.target.value })}
            />
            <label>API key</label>
            <input
              type="password"
              value={editing.apiKey}
              placeholder="sk-…"
              onChange={(e) => setEditing({ ...editing, apiKey: e.target.value })}
            />
            <div className="modal-actions">
              <button onClick={() => setEditing(null)}>Cancel</button>
              <button className="primary" onClick={save}>
                Save provider
              </button>
            </div>
          </>
        ) : (
          <>
            <label>AI providers</label>
            {providers.length === 0 && (
              <div className="hint" style={{ margin: "8px 0" }}>
                No providers yet. Add one to start chatting with the AI.
              </div>
            )}
            {providers.map((p) => (
              <div className="provider-row" key={p.id}>
                <div className="grow">
                  <div className="name">
                    {p.name} {defaultProviderId === p.id && <span className="badge">default</span>}
                  </div>
                  <div className="meta">
                    {p.kind} · {p.model}
                    {p.baseUrl ? ` · ${p.baseUrl}` : ""}
                  </div>
                </div>
                {defaultProviderId !== p.id && (
                  <button onClick={() => setDefault(p.id)}>Set default</button>
                )}
                <button onClick={() => setEditing(p)}>Edit</button>
                <button className="danger" onClick={() => remove(p)}>
                  Delete
                </button>
              </div>
            ))}
            <div className="modal-actions">
              <button onClick={onClose}>Close</button>
              <button
                className="primary"
                onClick={() =>
                  setEditing({
                    id: crypto.randomUUID(),
                    name: "",
                    kind: "glm",
                    baseUrl: DEFAULT_BASE.glm,
                    apiKey: "",
                    model: MODEL_HINT.glm,
                  })
                }
              >
                + Add provider
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
