#!/usr/bin/env sh
set -eu

VERSION="${1:-}"

if [ -z "$VERSION" ]; then
  echo "Usage: scripts/set-version.sh <version>" >&2
  echo "Example: scripts/set-version.sh 0.1.0-rc.4" >&2
  exit 1
fi

case "$VERSION" in
  v*)
    echo "Version must not include the leading 'v': $VERSION" >&2
    exit 1
    ;;
esac

case "$VERSION" in
  [0-9]*.[0-9]*.[0-9]*|[0-9]*.[0-9]*.[0-9]*-*) ;;
  *)
    echo "Version must look like semver, e.g. 0.1.0 or 0.1.0-rc.4: $VERSION" >&2
    exit 1
    ;;
esac

# --- JS / npm side ---
node - "$VERSION" <<'NODE'
const fs = require("node:fs");

const version = process.argv[2];

function writeJson(path, value) {
  fs.writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

const packageJson = JSON.parse(fs.readFileSync("package.json", "utf8"));
packageJson.version = version;
writeJson("package.json", packageJson);

const packageLock = JSON.parse(fs.readFileSync("package-lock.json", "utf8"));
packageLock.version = version;
if (packageLock.packages?.[""]) {
  packageLock.packages[""].version = version;
}
writeJson("package-lock.json", packageLock);
NODE

# --- Rust / Cargo side ---
if ! command -v cargo &>/dev/null; then
  echo "cargo is required but not installed" >&2
  exit 1
fi

if ! cargo set-version --help &>/dev/null 2>&1; then
  echo "cargo-edit (cargo-set-version) is required but not installed" >&2
  echo "Install it with: cargo install cargo-edit" >&2
  exit 1
fi

cargo set-version \
  --manifest-path src-tauri/Cargo.toml \
  --package ledgera \
  "$VERSION"

echo "Updated Ledgera version to $VERSION"
