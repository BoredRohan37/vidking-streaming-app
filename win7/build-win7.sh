#!/usr/bin/env bash
# Build MovieBox-TUI for Windows 7 / 8.1 on Linux (works in GitHub Codespaces).
#
#   ./win7/build-win7.sh            # 64-bit  -> dist/MovieBox_Win7_x86_64.zip
#   ./win7/build-win7.sh i686       # 32-bit  -> dist/MovieBox_Win7_i686.zip
#
# Env:
#   RUST_NIGHTLY   toolchain to use (default "nightly"; pin it, e.g. nightly-2026-10-01,
#                  once you have a build that works so results stay reproducible)
set -euo pipefail

ARCH="${1:-x86_64}"
TOOLCHAIN="${RUST_NIGHTLY:-nightly}"

case "$ARCH" in
  x86_64) TARGET="x86_64-win7-windows-gnu"; MINGW="x86_64-w64-mingw32" ;;
  i686)   TARGET="i686-win7-windows-gnu";   MINGW="i686-w64-mingw32"   ;;
  *) echo "usage: $0 [x86_64|i686]" >&2; exit 2 ;;
esac

cd "$(dirname "$0")/.."

echo "==> [1/4] Build tools"
if ! command -v "${MINGW}-gcc-posix" >/dev/null 2>&1 || ! command -v zip >/dev/null 2>&1; then
  sudo apt-get update -y
  sudo apt-get install -y mingw-w64 zip curl
fi
if ! command -v rustup >/dev/null 2>&1; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
  # shellcheck disable=SC1091
  source "$HOME/.cargo/env"
fi
rustup toolchain install "$TOOLCHAIN" --profile minimal -c rust-src

echo "==> [2/4] Building $TARGET with $(rustc +"$TOOLCHAIN" --version)"
TARGET_VAR="${TARGET//-/_}"
TARGET_UPPER="$(echo "$TARGET_VAR" | tr '[:lower:]' '[:upper:]')"
export "CC_${TARGET_VAR}=${MINGW}-gcc-posix"
export "AR_${TARGET_VAR}=${MINGW}-gcc-ar-posix"
export "CARGO_TARGET_${TARGET_UPPER}_LINKER=${MINGW}-gcc-posix"

cargo +"$TOOLCHAIN" build --release --locked \
  -Zbuild-std=std,panic_abort \
  --target "$TARGET"

echo "==> Checking the exe only imports Windows 7 compatible DLLs/APIs"
EXE="target/${TARGET}/release/moviebox-tui.exe"
IMPORTS="$("${MINGW}-objdump" -p "$EXE")"
if echo "$IMPORTS" | grep -qiE 'DLL Name: (combase|api-ms-win-[^ ]*|bcryptprimitives)\.dll'; then
  echo "ERROR: the exe imports a DLL that does not exist on Windows 7:" >&2
  echo "$IMPORTS" | grep -iE 'DLL Name: (combase|api-ms-win-[^ ]*|bcryptprimitives)\.dll' >&2
  echo "Hint: do not run 'cargo update'. Cargo.lock pins dirs-sys to windows-sys 0.59.0 (ole32)" >&2
  echo "      because windows-sys 0.61 links CoTaskMemFree from combase.dll (Windows 8+)." >&2
  exit 1
fi
if echo "$IMPORTS" | grep -qE '(GetSystemTimePreciseAsFileTime|PrefetchVirtualMemory|SetThreadDescription|WaitOnAddress|ProcessPrng)'; then
  echo "ERROR: the exe uses a Windows 8+ only API." >&2
  exit 1
fi

echo "==> [3/4] Packaging"
NAME="MovieBox_Win7_${ARCH}"
DIST="dist/${NAME}"
rm -rf "$DIST" "dist/${NAME}.zip"
mkdir -p "$DIST/tools"
cp "target/${TARGET}/release/moviebox-tui.exe" "$DIST/"
cp win7/run.bat "$DIST/run.bat"
cp win7/README-WIN7.txt "$DIST/README.txt"
cp win7/tools-README.txt "$DIST/tools/README.txt"
cp LICENSE-MIT LICENSE-APACHE "$DIST/"
# Windows batch files want CRLF line endings
sed -i 's/$/\r/' "$DIST/run.bat" "$DIST/README.txt" "$DIST/tools/README.txt"

echo "==> [4/4] Zipping"
( cd dist && zip -qr "${NAME}.zip" "${NAME}" )
echo
echo "Done: dist/${NAME}.zip"
ls -lh "dist/${NAME}.zip" "$DIST/moviebox-tui.exe"
