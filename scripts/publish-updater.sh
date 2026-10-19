#!/usr/bin/env bash
# Publishes the update channels for release $1 (a v* tag whose latest.json tauri-action wrote).
# The apps read them from the rolling `updater` release (mangas_core::app_update::endpoints):
#   latest-rc.json  every release (rc and stable), so the rc channel also gets full releases
#   latest.json     stable releases only, so the stable channel never sees a release candidate
# A channel file is only replaced by a newer version, so re-running an older tag is harmless.
# Before publishing, the signatures must come from the key whose public half the app ships.
set -euo pipefail
tag="$1"
repo="${GITHUB_REPOSITORY:-kerbaras/mangas}"
channels="updater"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

gh release download "$tag" -R "$repo" -p latest.json -D "$work/release" ||
  { echo "::error::$tag has no latest.json (were the bundles signed?)"; exit 1; }

python3 - "$work/release/latest.json" apps/desktop/src-tauri/tauri.conf.json <<'PY'
import base64, json, sys
manifest, config = (json.load(open(p)) for p in sys.argv[1:3])
def key_id(text):  # minisign: line 2 is base64(alg[2] + key id[8] + …)
    return base64.b64decode(base64.b64decode(text).decode().splitlines()[1])[2:10]
want = key_id(config["plugins"]["updater"]["pubkey"])
bad = [k for k, p in manifest["platforms"].items() if key_id(p["signature"]) != want]
if bad:
    sys.exit(f"::error::signed with a key other than tauri.conf.json's pubkey: {', '.join(bad)}")
print(f"{manifest['version']}: {len(manifest['platforms'])} platforms, signatures match the pubkey")
PY

if ! gh release view "$channels" -R "$repo" >/dev/null 2>&1; then
  gh release create "$channels" -R "$repo" --prerelease --latest=false --target "$(git rev-parse HEAD)" \
    --title "Update channels" --notes "Update manifests read by installed apps. Not a release: download Mangas from the latest release."
fi

newer() { # newer FILE_A FILE_B: whether A's version is above B's (semver, rc.N numeric)
  python3 - "$1" "$2" <<'PY'
import json, re, sys
def key(path):
    v = json.load(open(path))["version"].lstrip("v")
    m = re.fullmatch(r"(\d+)\.(\d+)\.(\d+)(?:-rc\.(\d+))?", v)
    if not m: sys.exit(f"unexpected version {v}")
    x, y, z, rc = m.groups()
    return (int(x), int(y), int(z), 0 if rc else 1, int(rc or 0))
sys.exit(0 if key(sys.argv[1]) > key(sys.argv[2]) else 1)
PY
}

targets=(latest-rc.json)
[[ "$tag" == *-rc.* ]] || targets+=(latest.json)
for name in "${targets[@]}"; do
  mkdir -p "$work/$name.d"
  if gh release download "$channels" -R "$repo" -p "$name" -D "$work/$name.d" 2>/dev/null &&
    ! newer "$work/release/latest.json" "$work/$name.d/$name"; then
    echo "$name stays on a newer or equal release than $tag"
    continue
  fi
  cp "$work/release/latest.json" "$work/$name"
  gh release upload "$channels" -R "$repo" --clobber "$work/$name"
  echo "$name now offers $tag"
done
