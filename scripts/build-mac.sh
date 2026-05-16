#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_DIR"

# ── Optional code signing & notarization ──────────────────────────
# Set SKIP_SIGNING=true to build unsigned for local testing.
SKIP_SIGNING="${SKIP_SIGNING:-false}"

if [ "${SKIP_SIGNING}" != "true" ] && [ -f ".env.local" ]; then
  echo "[mac] Loading signing credentials from .env.local"
  set -a
  source .env.local
  set +a

  export APPLE_SIGNING_IDENTITY="${APPLE_CODESIGN_IDENTITY:-}"
  export APPLE_API_KEY="${APPLE_NOTARY_KEY_ID:-}"
  export APPLE_API_ISSUER="${APPLE_NOTARY_ISSUER_ID:-}"
  export APPLE_API_KEY_PATH="${APPLE_NOTARY_KEY_PATH:-}"

  echo "[mac] Signing identity : ${APPLE_SIGNING_IDENTITY:-<none>}"
  echo "[mac] Notary key      : ${APPLE_API_KEY:-<none>}"

  if [ -z "${APPLE_SIGNING_IDENTITY:-}" ] || [ -z "${APPLE_API_KEY:-}" ]; then
    echo "[mac] ⚠️  Missing signing credentials — building unsigned."
    unset APPLE_SIGNING_IDENTITY APPLE_API_KEY APPLE_API_ISSUER APPLE_API_KEY_PATH
  fi
else
  echo "[mac] Building unsigned .app bundle."
  unset APPLE_SIGNING_IDENTITY APPLE_API_KEY APPLE_API_ISSUER APPLE_API_KEY_PATH
fi

echo "[mac] Building macOS .app bundle..."
npx tauri build --bundles app

VERSION=$(node -p "require('./package.json').version")
APP_NAME=$(node -p "require('./package.json').name")
SRC_APP="src-tauri/target/release/bundle/macos/${APP_NAME}.app"

if [ ! -d "$SRC_APP" ]; then
  echo "[mac] ERROR: .app bundle not found at $SRC_APP"
  exit 1
fi

echo "[mac] Creating zip archive (preserving extended attributes for notarization)..."
OUTPUT_ZIP="${APP_NAME}-${VERSION}-macos.zip"
ditto -c -k --sequesterRsrc --keepParent "$SRC_APP" "$OUTPUT_ZIP"

echo "[mac] Done: $OUTPUT_ZIP"
