#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_DIR"

echo "[linux] Building Linux packages..."
# AppImage is the default when building on Linux.
# deb/rpm require the native tooling (dpkg / rpmbuild).
npx tauri build --bundles deb,appimage

VERSION=$(node -p "require('./package.json').version")
APP_NAME=$(node -p "require('./package.json').name")

BUNDLE_DIR="src-tauri/target/release/bundle"

if [ -f "${BUNDLE_DIR}/deb/${APP_NAME}_${VERSION}_amd64.deb" ]; then
  echo "[linux] deb  : ${BUNDLE_DIR}/deb/${APP_NAME}_${VERSION}_amd64.deb"
fi
if [ -f "${BUNDLE_DIR}/appimage/${APP_NAME}_${VERSION}_amd64.AppImage" ]; then
  echo "[linux] AppImage : ${BUNDLE_DIR}/appimage/${APP_NAME}_${VERSION}_amd64.AppImage"
fi
echo "[linux] Done."
