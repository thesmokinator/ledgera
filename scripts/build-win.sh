#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_DIR"

check_tool() {
  local cmd="$1"
  local name="$2"
  if ! command -v "$cmd" &>/dev/null; then
    echo "[win] ❌  ${name} not found. Run: ${3:-brew install ${name}}"
    exit 1
  fi
}

check_tool cargo-xwin   "cargo-xwin"   "cargo install cargo-xwin"
check_tool makensis      "NSIS (makensis)" "brew install nsis"

RUST_TARGET="x86_64-pc-windows-msvc"
if ! rustup target list --installed | grep -q "$RUST_TARGET"; then
  echo "[win] Adding Rust target ${RUST_TARGET}..."
  rustup target add "$RUST_TARGET"
fi

echo "[win] Cross-compiling Windows NSIS installer with cargo-xwin..."
npx tauri build --bundles nsis --target "$RUST_TARGET" --runner cargo-xwin

VERSION=$(node -p "require('./package.json').version")
APP_NAME=$(node -p "require('./package.json').name")

NSIS_DIR="src-tauri/target/${RUST_TARGET}/release/bundle/nsis"
SRC_EXE="${NSIS_DIR}/${APP_NAME}_${VERSION}_x64-setup.exe"

if [ ! -f "$SRC_EXE" ]; then
  # Tauri may use a slightly different naming convention.
  SRC_EXE=$(find "$NSIS_DIR" -maxdepth 1 -name "*_x64-setup.exe" 2>/dev/null | head -1)
fi

if [ -z "${SRC_EXE:-}" ] || [ ! -f "${SRC_EXE:-}" ]; then
  echo "[win] ❌  NSIS installer not found in ${NSIS_DIR}"
  exit 1
fi

OUTPUT_EXE="${APP_NAME}-${VERSION}-win-x64-setup.exe"
cp "$SRC_EXE" "$OUTPUT_EXE"

echo "[win] Done: $OUTPUT_EXE"
echo "[win] ⚠️  Authenticode signing must be performed on a Windows machine."
