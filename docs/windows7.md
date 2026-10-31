# Windows 7 / 8.1 build

MovieBox-TUI can be built for Windows 7 SP1 and Windows 8.1. These are Rust
**Tier 3** targets (`x86_64-win7-windows-gnu`, `i686-win7-windows-gnu`), so they need a
**nightly** toolchain and `-Zbuild-std`.

## Build (Linux / GitHub Codespaces)

```bash
./win7/build-win7.sh          # 64-bit -> dist/MovieBox_Win7_x86_64.zip
./win7/build-win7.sh i686     # 32-bit -> dist/MovieBox_Win7_i686.zip
```

The script installs `mingw-w64`, a nightly toolchain with `rust-src`, builds, and
packages `moviebox-tui.exe`, `run.bat`, `README.txt` and an empty `tools\` folder.
Set `RUST_NIGHTLY=nightly-YYYY-MM-DD` to pin the toolchain. A GitHub Actions
workflow (`.github/workflows/win7.yml`) runs the same script.

## What differs from the normal Windows build

| Area | Normal build | Win7/8.1 build |
|---|---|---|
| Rust | stable 1.90+ | nightly, `-Zbuild-std` |
| Allocator | mimalloc | system allocator |
| Console | assumes a modern terminal | detects a legacy console and uses the 16-color theme, no images |
| Player / tools lookup | WinGet, Scoop, Program Files, PATH | `tools\` next to the exe first, then the usual places |
| Updater | downloads the upstream zip | disabled (manual updates) |
| Packaging | `install.ps1` | zip with `run.bat` |

Everything else (providers, search, history, favorites, cache, proxy, downloads)
is unchanged.

## Runtime tools

Put these in `tools\` (see `win7/tools-README.txt`): `mpv.exe` (a build that runs on
Windows 7), and for downloads `yt-dlp.exe` and `ffmpeg.exe`. VLC is optional.

## Environment variables

- `MOVIEBOX_LEGACY_CONSOLE=1|0` force the legacy 16-color layout on or off.
- `MOVIEBOX_NO_IMAGE=1` disable poster images.

## Keep the Cargo.lock pin

`dirs-sys` accepts any `windows-sys >= 0.59`. Version 0.61 links `CoTaskMemFree` from
`combase.dll`, which does not exist on Windows 7, so the exe would not start. `Cargo.lock`
pins `dirs-sys` to `windows-sys 0.59.0` (links from `ole32.dll`). Do not run a blanket
`cargo update`; `win7/build-win7.sh` fails the build if a Windows 8+ DLL or API shows up
in the exe's import table.

## Notes

- A legacy console (no ANSI support) is detected with crossterm's
  `ansi_support::supports_ansi()`. On such consoles RGB and 256-color values render
  as black, which is why the 16-color theme is forced.
- Use ConEmu or Cmder for a nicer look on Windows 7.
