# FeatherCode

A **very lightweight AI code editor for Linux** — like Claude Code / Cursor, but tiny.
Built with **Tauri 2** (native WebView, ~10 MB installer instead of a 300 MB Electron app).

![stack](https://img.shields.io/badge/Tauri-2-blue) ![rust](https://img.shields.io/badge/Rust-backend-orange) ![react](https://img.shields.io/badge/React-18-61dafb)

## Features

| Area | What you get |
|---|---|
| **Editor** | Monaco (VS Code's engine), tabs, syntax highlighting for 100+ languages, dark minimal theme |
| **AI providers** | GLM (Z.ai), OpenAI, Anthropic, and *any* OpenAI-compatible endpoint — all via API keys |
| **AI chat** | Streaming responses, markdown + code blocks, one-click **Copy** / **Apply** (with a real diff review) |
| **File context** | Active file auto-included; right-click any file → "Add to AI context" |
| **Inline AI** | Select code in the editor → `Ctrl+K` → describe the change → review & replace |
| **Workspaces** | Local folder, **remote SSH directory** (SFTP read/write), or **git repo clone** |
| **Terminal** | Built-in terminal (real PTY) with `Ctrl+`` |
| **Storage** | Settings, API keys, workspaces in local SQLite — nothing leaves your machine except AI calls |

## Keyboard shortcuts

| Shortcut | Action |
|---|---|
| `Ctrl+K` | Edit current selection with AI (inline prompt) |
| `Ctrl+S` | Save file |
| `Ctrl+B` | Toggle explorer sidebar |
| `` Ctrl+` `` | Toggle terminal |
| `Enter` / `Shift+Enter` | Send / newline in chat |

## Build from source (Linux)

### 1. System dependencies

Debian / Ubuntu / Mint:

```bash
sudo apt update
sudo apt install -y libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev git
```

Fedora: `sudo dnf install webkit2gtk4.1-devel openssl-devel curl wget file libxdo-devel libappindicator-gtk3-devel librsvg2-devel`

Arch: `sudo pacman -S webkit2gtk-4.1 base-devel curl wget file libxdo openssl appmenu-gtk-module librsvg`

### 2. Rust toolchain

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

### 3. Build & run

```bash
cd feathercode
npm install

# development (hot reload)
npm run tauri dev

# production build → .deb + AppImage
npm run tauri build
```

Artifacts land in `src-tauri/target/release/bundle/`
(`deb/feathercode_0.1.0_amd64.deb`, `appimage/feathercode_0.1.0_amd64.AppImage`).

> Note: the first Rust build compiles bundled SQLite and takes several minutes on a modern PC.

## Building on a low-spec PC (e.g. 2-core CPU, 3 GB RAM)

The Rust backend compiles fine on old hardware, but 3 GB of RAM is tight when the linker runs.
Two options:

### Option A — build in the cloud for free (recommended)

1. Create a GitHub repo, push this project (`git init && git add -A && git commit -m init && git push`).
2. Open the repo → **Actions** tab → **Build Linux packages** → **Run workflow**.
3. When it finishes (~10 min), download the `.deb` / `.AppImage` from the run's **Artifacts**.
4. Copy to your PC and install: `sudo dpkg -i feathercode_0.1.0_amd64.deb`

No Rust compile ever happens on your machine. The workflow is already included at
`.github/workflows/build.yml`.

### Option B — build locally

Requirements & tips:

- **Disk**: ~4 GB free for the Rust build cache (`src-tauri/target`).
- **Swap is essential** with 3 GB RAM (the linker can otherwise OOM):
  ```bash
  sudo fallocate -l 4G /swapfile && sudo chmod 600 /swapfile
  sudo mkswap /swapfile && sudo swapon /swapfile
  ```
- Limit parallel jobs to keep memory low:
  ```bash
  CARGO_BUILD_JOBS=2 npm run tauri build
  ```
- Expect roughly 30–60+ min for the first build on a 2-core Pentium. Subsequent builds are fast.
- `ssh2` uses your system OpenSSL (`libssl-dev`) — if your distro doesn't ship it, switch the
  dependency in `src-tauri/Cargo.toml` to `features = ["vendored-openssl"]` (much slower build).

### Runtime notes for old GPUs (GMA 3100 etc.)

- The webview falls back to software rendering — fine for a text editor, but expect some
  scroll lag with very large files (>1 MB). Keep the minimap off (it already is).
- Close other apps while running; Monaco uses a few hundred MB of RAM.

## Getting started

1. Launch FeatherCode → click **⚙ Settings → + Add provider**
   - **GLM (Z.ai)** — default base URL `https://api.z.ai/api/paas/v4`, model e.g. `glm-4.6`
   - **OpenAI** — `https://api.openai.com/v1`, model e.g. `gpt-4o`
   - **Anthropic** — `https://api.anthropic.com`, model e.g. `claude-sonnet-4-5`
   - **Any OpenAI-compatible** — point at DeepSeek, Groq, Ollama (`http://localhost:11434/v1`), etc.
   - Paste your API key → Save. The default provider is used by chat and inline prompts.
2. Click **+ Workspace** and add:
   - **Local folder** — any directory on this machine
   - **SSH remote** — host/port/user + password *or* private key; files are read/written over SFTP
   - **Git repo** — shallow-cloned into `~/FeatherCode/<repo>` and opened as a local workspace
3. Ask questions in the AI chat, or select code and hit `Ctrl+K`.
4. When the AI proposes file changes, hit **Apply…** on a code block, review the diff, then **Apply**.

## Project structure

```
feathercode/
├── src/                      # React + TypeScript frontend
│   ├── components/           # TitleBar, FileTree, EditorArea, ChatPanel, Terminal,
│   │                         # SettingsModal, WorkspacesModal, DiffModal, InlinePrompt
│   ├── state.ts              # zustand store
│   ├── api.ts                # workspace-aware invoke wrappers (local fs vs SSH sftp)
│   ├── monacoSetup.ts        # Monaco workers + custom "feather" theme
│   └── App.tsx               # layout, shortcuts, AI stream event wiring
├── src-tauri/                # Rust backend
│   └── src/
│       ├── main.rs           # Tauri setup + command registry
│       ├── db.rs             # SQLite (rusqlite): providers, workspaces, settings
│       ├── fsops.rs          # local file operations + workspace text search
│       ├── sshops.rs         # SSH/SFTP remote workspace operations (ssh2 crate)
│       ├── gitops.rs         # shallow git clone
│       ├── aichat.rs         # streaming AI proxy (SSE → events), OpenAI + Anthropic formats
│       └── terminal.rs       # PTY terminal (portable-pty)
└── tauri.conf.json           # window + bundle config (deb, AppImage)
```

## Security notes

- API keys are stored in SQLite at `~/.local/share/dev.feathercode.app/feathercode.db` and are only
  sent directly from the backend to the provider endpoint you configure.
- AI requests are proxied through the Rust backend, so keys never live in the WebView.
- SSH credentials are likewise stored only in the local database.
