#!/usr/bin/env bash
# Dry run of publish-updater.sh against a fake `gh` whose releases are folders. Publishes a
# sequence of tags and checks which version each channel file offers afterwards.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
sandbox="$(mktemp -d)"
trap 'rm -rf "$sandbox"' EXIT
mkdir -p "$sandbox/bin" "$sandbox/releases"
export PATH="$sandbox/bin:$PATH" RELEASES="$sandbox/releases" GITHUB_REPOSITORY=kerbaras/mangas

cat > "$sandbox/bin/gh" <<'GH'
#!/usr/bin/env bash
# gh release view|create|download|upload TAG …, on $RELEASES/TAG/
set -euo pipefail
[[ "$1" == release ]] || exit 2
cmd="$2" dir="$RELEASES/$3"
shift 3
case "$cmd" in
  view) [[ -d "$dir" ]] ;;
  create) mkdir -p "$dir" ;;
  download)
    pattern="" out="."
    while (($#)); do
      case "$1" in -p) pattern="$2"; shift 2 ;; -D) out="$2"; shift 2 ;; *) shift ;; esac
    done
    [[ -f "$dir/$pattern" ]] || { echo "release has no asset $pattern" >&2; exit 1; }
    mkdir -p "$out" && cp "$dir/$pattern" "$out/" ;;
  upload)
    while (($#)); do
      case "$1" in -R) shift 2 ;; --*) shift ;; *) cp "$1" "$dir/"; shift ;; esac
    done ;;
  *) exit 2 ;;
esac
GH
chmod +x "$sandbox/bin/gh"

manifest() { # manifest TAG [wrong]: tauri-action's latest.json, signed by the committed key
  python3 - "$1" "${2:-}" "$root/apps/desktop/src-tauri/tauri.conf.json" <<'PY'
import base64, json, sys
tag, wrong, config = sys.argv[1:4]
pubkey = json.load(open(config))["plugins"]["updater"]["pubkey"]
key_id = bytes(8) if wrong else base64.b64decode(base64.b64decode(pubkey).decode().splitlines()[1])[2:10]
blob = base64.b64encode(b"ED" + key_id + bytes(64)).decode()
signature = base64.b64encode(f"untrusted comment: test\n{blob}\n".encode()).decode()
platforms = {p: {"signature": signature, "url": f"https://example.test/{tag}/{p}"}
             for p in ["linux-x86_64", "darwin-aarch64", "windows-x86_64"]}
print(json.dumps({"version": tag.lstrip("v"), "platforms": platforms}))
PY
}

publish() { # publish TAG [wrong]
  mkdir -p "$RELEASES/$1"
  manifest "$@" > "$RELEASES/$1/latest.json"
  echo "== scripts/publish-updater.sh $1${2:+ (signed by another key)}"
  (cd "$root" && scripts/publish-updater.sh "$1")
}

offers() { # offers FILE: version in the channel file, or "none"
  local file="$RELEASES/updater/$1"
  [[ -f "$file" ]] && python3 -c 'import json, sys; print(json.load(open(sys.argv[1]))["version"])' "$file" || echo none
}

expect() { # expect LATEST LATEST_RC
  local got="latest.json=$(offers latest.json) latest-rc.json=$(offers latest-rc.json)"
  [[ "$got" == "latest.json=$1 latest-rc.json=$2" ]] || { echo "FAIL: $got, want $1 / $2"; exit 1; }
  echo "ok: $got"
}

publish v0.5.0 && expect 0.5.0 0.5.0
publish v0.4.9 && expect 0.5.0 0.5.0
publish v0.5.0-rc.1 && expect 0.5.0 0.5.0
publish v0.5.1-rc.1 && expect 0.5.0 0.5.1-rc.1
publish v0.5.1 && expect 0.5.1 0.5.1
if publish v0.5.2 wrong; then echo "FAIL: published a manifest signed by another key"; exit 1; fi
expect 0.5.1 0.5.1
echo "all publish-updater cases pass"
