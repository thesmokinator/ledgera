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

function replaceOne(path, pattern, replacement, description) {
  const input = fs.readFileSync(path, "utf8");
  if (!pattern.test(input)) {
    throw new Error(`Could not find ${description} in ${path}`);
  }
  fs.writeFileSync(path, input.replace(pattern, replacement));
}

replaceOne(
  "src-tauri/Cargo.toml",
  /(\[package\][\s\S]*?\nversion = ")[^"]+(")/,
  `$1${version}$2`,
  "package version",
);

replaceOne(
  "src-tauri/Cargo.lock",
  /(\[\[package\]\]\nname = "ledgera"\nversion = ")[^"]+(")/,
  `$1${version}$2`,
  "ledgera package version",
);
NODE

echo "Updated Ledgera version to $VERSION"
