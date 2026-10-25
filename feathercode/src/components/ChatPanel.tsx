import { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { marked } from "marked";
import DOMPurify from "dompurify";
import { useStore } from "../state";
import { readFile, uid } from "../api";
import { buildSystemPrompt, looksLikePath } from "../utils";
import type { ChatMsg, Workspace } from "../types";

function MessageBody({ content }: { content: string }) {
  const ref = useRef<HTMLDivElement>(null);
  const html = useMemo(() => {
    try {
      return DOMPurify.sanitize(marked.parse(content) as string);
    } catch {
      return content;
    }
  }, [content]);

  useEffect(() => {
    const root = ref.current;
    if (!root) return;
    root.querySelectorAll("pre").forEach((pre) => {
      if (pre.querySelector(".cb-bar")) return;
      const codeEl = pre.querySelector("code");
      const text = codeEl?.textContent ?? "";
      const lang = codeEl?.getAttribute("class")?.replace(/^language-/, "") ?? "";
      const bar = document.createElement("div");
      bar.className = "cb-bar";
      const label = document.createElement("span");
      label.className = "cb-lang";
      label.textContent = lang || "code";
      const copy = document.createElement("button");
      copy.textContent = "Copy";
      copy.onclick = () => {
        navigator.clipboard.writeText(text);
        copy.textContent = "Copied";
        setTimeout(() => (copy.textContent = "Copy"), 1200);
      };
      const apply = document.createElement("button");
      apply.textContent = "Apply…";
      apply.onclick = () => {
        useStore.getState().set({
          diffRequest: {
            code: text,
            target: looksLikePath(lang) ? lang : useStore.getState().activePath,
          },
        });
      };
      bar.append(label, copy, apply);
      pre.prepend(bar);
    });
  }, [html]);

  return <div ref={ref} className="md" dangerouslySetInnerHTML={{ __html: html }} />;
}

function Message({ m }: { m: ChatMsg }) {
  if (m.role === "user") {
    return (
      <div className="msg msg-user">
        <div className="bubble">{m.content}</div>
      </div>
    );
  }
  return (
    <div className="msg msg-assistant">
      <div className="bubble-wrap">
        {m.reasoning && <div className="reasoning">{m.reasoning}</div>}
        {m.content ? (
          <MessageBody content={m.content} />
        ) : (
          <div className="reasoning">{m.done ? "(empty response)" : "Thinking…"}</div>
        )}
        {m.error && <div className="msg-error">Request failed — see message above.</div>}
      </div>
    </div>
  );
}

export default function ChatPanel() {
  const chat = useStore((s) => s.chat);
  const streaming = useStore((s) => s.streaming);
  const providers = useStore((s) => s.providers);
  const defaultProviderId = useStore((s) => s.defaultProviderId);
  const contextPaths = useStore((s) => s.contextPaths);
  const autoIncludeActive = useStore((s) => s.autoIncludeActive);
  const activePath = useStore((s) => s.activePath);
  const [input, setInput] = useState("");
  const bodyRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bodyRef.current?.scrollTo({ top: bodyRef.current.scrollHeight });
  }, [chat]);

  const send = async () => {
    const text = input.trim();
    if (!text || streaming) return;
    const s = useStore.getState();
    const provider = s.providers.find((p) => p.id === s.defaultProviderId);
    if (!provider) {
      s.showToast("Add an AI provider (API key) in Settings first");
      useStore.setState({ settingsOpen: true });
      return;
    }

    // gather context files
    const targets = [
      ...new Set([...(s.autoIncludeActive && s.activePath ? [s.activePath] : []), ...s.contextPaths]),
    ];
    const files: { path: string; content: string }[] = [];
    for (const p of targets) {
      try {
        const c = await readFile(p);
        files.push({ path: p, content: c.slice(0, 60000) });
      } catch {
        /* file unreadable — skip */
      }
    }
    const ws: Workspace | null = s.workspaces.find((w) => w.id === s.activeWsId) ?? null;
    const sys = buildSystemPrompt(ws, files);
    const history = s.chat
      .filter((m) => !m.error && m.content.trim())
      .map((m) => ({ role: m.role, content: m.content }));

    const userMsg: ChatMsg = { id: uid(), role: "user", content: text };
    const aiMsg: ChatMsg = { id: uid(), role: "assistant", content: "" };
    const reqId = uid();

    useStore.setState({
      chat: [...s.chat, userMsg, aiMsg],
      streaming: true,
      streamingReqId: reqId,
      streamingMsgId: aiMsg.id,
      contextPaths: [],
    });
    setInput("");

    try {
      await invoke("ai_chat", {
        requestId: reqId,
        providerId: provider.id,
        messages: [
          { role: "system", content: sys },
          ...history,
          { role: "user", content: text },
        ],
      });
    } catch (e) {
      useStore.getState().updateChat(aiMsg.id, { content: String(e), error: true, done: true });
      useStore.setState({ streaming: false });
    }
  };

  return (
    <div className="chat">
      <div className="chat-head">
        <span className="title">✦ AI Chat</span>
        <select
          className="chat-provider"
          value={defaultProviderId}
          onChange={(e) => {
            useStore.setState({ defaultProviderId: e.target.value });
            invoke("db_set_setting", { key: "defaultProvider", value: e.target.value }).catch(() => {});
          }}
          title="Active AI provider"
        >
          {providers.length === 0 && <option value="">No provider configured</option>}
          {providers.map((p) => (
            <option key={p.id} value={p.id}>
              {p.name} · {p.model}
            </option>
          ))}
        </select>
        <button className="icon-btn" title="Provider settings" onClick={() => useStore.setState({ settingsOpen: true })}>
          ⚙
        </button>
      </div>

      <div className="chat-body" ref={bodyRef}>
        {chat.length === 0 && (
          <div className="chat-empty">
            Ask anything about your workspace.
            <br />
            <span style={{ fontSize: 11.5 }}>
              The active file is included automatically. Right-click files in the explorer to pin more
              context.
            </span>
          </div>
        )}
        {chat.map((m) => (
          <Message key={m.id} m={m} />
        ))}
      </div>

      <div className="chat-input-wrap">
        {contextPaths.length > 0 && (
          <div className="ctx-chips" style={{ padding: 0, marginBottom: 6 }}>
            {contextPaths.map((p) => (
              <span className="chip" key={p} title={p}>
                📄 {p.split("/").pop()}
                <button onClick={() => useStore.getState().removeContext(p)}>×</button>
              </span>
            ))}
          </div>
        )}
        <textarea
          placeholder="Ask the AI… (Enter to send, Shift+Enter for newline)"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.shiftKey) {
              e.preventDefault();
              send();
            }
          }}
        />
        <div className="chat-input-actions">
          <label className="toggle-row" style={{ margin: 0 }}>
            <input
              type="checkbox"
              checked={autoIncludeActive}
              onChange={(e) => useStore.setState({ autoIncludeActive: e.target.checked })}
            />
            include active file {activePath ? `(${activePath.split("/").pop()})` : ""}
          </label>
          <div className="spacer" />
          <button className="primary" onClick={send} disabled={streaming || !input.trim()}>
            {streaming ? "Streaming…" : "Send"}
          </button>
        </div>
      </div>
    </div>
  );
}
