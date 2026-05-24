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

PORTABLE_ONLY=false
if [ "${1:-}" = "--portable-only" ]; then
  PORTABLE_ONLY=true
elif [ -n "${1:-}" ]; then
  echo "[win] ❌  Unknown option: $1"
  echo "Usage: scripts/build-win.sh [--portable-only]"
  exit 1
fi

check_tool cargo-xwin   "cargo-xwin"   "cargo install cargo-xwin"
check_tool zip          "zip"          "brew install zip"
if [ "$PORTABLE_ONLY" != "true" ]; then
  check_tool makensis    "NSIS (makensis)" "brew install nsis"
fi

RUST_TARGET="x86_64-pc-windows-msvc"
if ! rustup target list --installed | grep -q "$RUST_TARGET"; then
  echo "[win] Adding Rust target ${RUST_TARGET}..."
  rustup target add "$RUST_TARGET"
fi

if [ "$PORTABLE_ONLY" = "true" ]; then
  echo "[win] Cross-compiling Windows executable with cargo-xwin..."
  npx tauri build --no-bundle --target "$RUST_TARGET" --runner cargo-xwin
else
  echo "[win] Cross-compiling Windows NSIS installer with cargo-xwin..."
  npx tauri build --bundles nsis --target "$RUST_TARGET" --runner cargo-xwin
fi

VERSION=$(node -p "require('./package.json').version")
APP_NAME=$(node -p "require('./package.json').name")
PRODUCT_NAME=$(node -p "require('./src-tauri/tauri.conf.json').productName || require('./package.json').name")
TARGET_DIR="src-tauri/target/${RUST_TARGET}/release"

if [ "$PORTABLE_ONLY" != "true" ]; then
  NSIS_DIR="${TARGET_DIR}/bundle/nsis"
  SRC_INSTALLER="${NSIS_DIR}/${APP_NAME}_${VERSION}_x64-setup.exe"

  if [ ! -f "$SRC_INSTALLER" ]; then
    # Tauri may use a slightly different naming convention.
    SRC_INSTALLER=$(find "$NSIS_DIR" -maxdepth 1 -name "*_x64-setup.exe" 2>/dev/null | head -1)
  fi

  if [ -z "${SRC_INSTALLER:-}" ] || [ ! -f "${SRC_INSTALLER:-}" ]; then
    echo "[win] ❌  NSIS installer not found in ${NSIS_DIR}"
    exit 1
  fi

  OUTPUT_EXE="${APP_NAME}-${VERSION}-win-x64-setup.exe"
  cp "$SRC_INSTALLER" "$OUTPUT_EXE"
  echo "[win] Installer: $OUTPUT_EXE"
fi

SRC_APP_EXE="${TARGET_DIR}/${APP_NAME}.exe"
if [ ! -f "$SRC_APP_EXE" ]; then
  SRC_APP_EXE=$(find "$TARGET_DIR" -maxdepth 1 -type f -name "*.exe" ! -name "*setup*.exe" 2>/dev/null | head -1)
fi

if [ -z "${SRC_APP_EXE:-}" ] || [ ! -f "${SRC_APP_EXE:-}" ]; then
  echo "[win] ❌  Windows executable not found in ${TARGET_DIR}"
  exit 1
fi

PORTABLE_ROOT="dist/win-portable"
PORTABLE_DIR="${PORTABLE_ROOT}/${APP_NAME}-${VERSION}-win-x64"
rm -rf "$PORTABLE_DIR"
mkdir -p "$PORTABLE_DIR"
cp "$SRC_APP_EXE" "${PORTABLE_DIR}/${PRODUCT_NAME}.exe"
cat > "${PORTABLE_DIR}/README.txt" <<EOF
${PRODUCT_NAME} portable build for Windows

Run ${PRODUCT_NAME}.exe directly. This ZIP does not install shortcuts,
file associations, or automatic updates.

Requirements:
- Microsoft Edge WebView2 Runtime already installed on this Windows system
  (the setup.exe installer includes the offline WebView2 installer; this portable ZIP does not)
- hledger available in PATH or configured in ${PRODUCT_NAME} settings
EOF

OUTPUT_ZIP="${APP_NAME}-${VERSION}-win-x64-portable.zip"
rm -f "$OUTPUT_ZIP"
(
  cd "$PORTABLE_ROOT"
  zip -qr "$PROJECT_DIR/$OUTPUT_ZIP" "$(basename "$PORTABLE_DIR")"
)

echo "[win] Portable zip: $OUTPUT_ZIP"
echo "[win] Done."
echo "[win] ⚠️  Authenticode signing must be performed on a Windows machine."
