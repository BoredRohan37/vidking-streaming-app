import { useEffect, useRef } from "react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import "@xterm/xterm/css/xterm.css";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useStore } from "../state";
import { activeWs, uid, wsRoot } from "../api";

export default function TerminalPanel() {
  const terminalOpen = useStore((s) => s.terminalOpen);
  const hostRef = useRef<HTMLDivElement>(null);
  const initedRef = useRef(false);

  useEffect(() => {
    if (!terminalOpen || initedRef.current || !hostRef.current) return;
    initedRef.current = true;

    const term = new Terminal({
      fontSize: 12.5,
      fontFamily: "'JetBrains Mono','DejaVu Sans Mono',monospace",
      cursorBlink: true,
      theme: {
        background: "#121218",
        foreground: "#d6d6e0",
        cursor: "#4fd1c5",
        selectionBackground: "#31424f",
        black: "#121218",
        green: "#7ddba6",
        blue: "#63b3ed",
        cyan: "#4fd1c5",
        red: "#f56565",
        yellow: "#f0b37e",
      },
    });
    const fit = new FitAddon();
    term.loadAddon(fit);
    term.open(hostRef.current);
    try {
      fit.fit();
    } catch {
      /* ignore fit errors on hidden container */
    }

    const id = uid();
    const ws = activeWs();
    const cwd = ws && ws.kind === "local" ? wsRoot(ws) : undefined;
    invoke("pty_create", { id, cwd: cwd ?? "/" }).catch((e) =>
      term.write(`\r\n\x1b[31m${String(e)}\x1b[0m\r\n`),
    );

    term.onData((d) => invoke("pty_write", { id, data: d }).catch(() => {}));
    term.onResize(({ cols, rows }) => invoke("pty_resize", { id, cols, rows }).catch(() => {}));

    const u1 = listen<{ id: string; data: string }>("pty-output", ({ payload }) => {
      if (payload.id === id) term.write(payload.data);
    });
    const u2 = listen<{ id: string }>("pty-closed", ({ payload }) => {
      if (payload.id === id) term.write("\r\n\x1b[90m[process exited]\x1b[0m\r\n");
    });

    const onWinResize = () => {
      try {
        fit.fit();
      } catch {
        /* noop */
      }
    };
    window.addEventListener("resize", onWinResize);

    return () => {
      window.removeEventListener("resize", onWinResize);
      invoke("pty_kill", { id }).catch(() => {});
      u1.then((f) => f());
      u2.then((f) => f());
      term.dispose();
      initedRef.current = false;
    };
  }, [terminalOpen]);

  useEffect(() => {
    // refit when toggled open
    if (terminalOpen) {
      setTimeout(() => window.dispatchEvent(new Event("resize")), 180);
    }
  }, [terminalOpen]);

  return (
    <div className={"terminal" + (terminalOpen ? " open" : "")}>
      {terminalOpen && <div ref={hostRef} className="term-host" />}
    </div>
  );
}
