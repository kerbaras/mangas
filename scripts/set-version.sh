#!/usr/bin/env bash
# Writes one version into every manifest: Cargo workspace (tauri.conf.json inherits it) and
# the desktop package.json. Usage: set-version.sh 1.8.3-rc.2
set -euo pipefail
v="$1"
sed -i.bak -E "0,/^version = \"[^\"]+\"/s//version = \"${v}\"/" Cargo.toml && rm Cargo.toml.bak
sed -i.bak -E "s/^(  \"version\": )\"[^\"]+\"/\1\"${v}\"/" apps/desktop/package.json && rm apps/desktop/package.json.bak
cargo update --workspace >/dev/null
echo "version set to ${v}"
