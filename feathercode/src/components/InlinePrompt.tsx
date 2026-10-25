import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import * as monaco from "monaco-editor";
import { useStore } from "../state";
import { stripFences } from "../utils";

export default function InlinePrompt() {
  const inline = useStore((s) => s.inline);
  const [text, setText] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (inline && !inline.busy && !inline.done) {
      setText("");
      setTimeout(() => inputRef.current?.focus(), 30);
    }
  }, [inline?.reqId]); // eslint-disable-line react-hooks/exhaustive-deps

  if (!inline) return null;
  const close = () => useStore.setState({ inline: null });

  const submit = async () => {
    if (!text.trim() || inline.busy) return;
    const s = useStore.getState();
    const provider = s.providers.find((p) => p.id === s.defaultProviderId);
    if (!provider) {
      s.showToast("Add an AI provider in Settings first");
      useStore.setState({ settingsOpen: true });
      return;
    }
    useStore.setState({ inline: { ...inline, busy: true, result: "", done: false } });
    const prompt =
      `File: ${inline.path}\n\nSelected code:\n${inline.sel}\n\nInstruction: ${text}\n\n` +
      "Return ONLY the complete replacement code. No explanations, no markdown fences.";
    try {
      await invoke("ai_chat", {
        requestId: inline.reqId,
        providerId: provider.id,
        messages: [{ role: "user", content: prompt }],
      });
    } catch (e) {
      useStore.setState({
        inline: { ...useStore.getState().inline!, result: String(e), done: true, busy: false },
      });
    }
  };

  const replace = () => {
    const ed = useStore.getState().editor;
    if (!ed) return;
    const r = inline.range;
    ed.executeEdits("inline-ai", [
      {
        range: new monaco.Range(r.startLineNumber, r.startColumn, r.endLineNumber, r.endColumn),
        text: stripFences(inline.result),
        forceMoveMarkers: true,
      },
    ]);
    useStore.setState({ inline: null });
  };

  return (
    <div className="inline-prompt">
      {!inline.busy && !inline.done && (
        <div className="ip-row">
          <input
            ref={inputRef}
            placeholder="Tell the AI how to edit the selection… (Enter to run, Esc to cancel)"
            value={text}
            onChange={(e) => setText(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") submit();
              if (e.key === "Escape") close();
            }}
          />
          <button onClick={submit}>Run</button>
          <button onClick={close}>Esc</button>
        </div>
      )}
      {inline.busy && <div className="ip-status">✦ Thinking…</div>}
      {(inline.done || inline.busy) && (
        <div className="ip-result">
          <pre>{inline.result || "…"}</pre>
          {inline.done && (
            <div className="ip-actions">
              <button className="primary" onClick={replace}>
                Replace selection
              </button>
              <button onClick={() => navigator.clipboard.writeText(stripFences(inline.result))}>Copy</button>
              <button onClick={close}>Close</button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
