#!/usr/bin/env bash
# Regenerates apps/desktop/src-tauri/icons from the SVG masters in icons/src.
#   icon-object.svg  bare object on transparent: PNGs, Square*Logo, .ico, Android, iOS
#   icon-macos.svg   object in the macOS squircle tile: .icns only
#   icon-small.svg   simplified master for <=48 px: overwrites 32x32, Square30/44 and the
#                    16/24/32/48 layers of icon.ico (tauri icon takes a single source PNG)
# Needs rsvg-convert (librsvg2-bin) and ImageMagick. Usage: scripts/icons.sh
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
icons="$root/apps/desktop/src-tauri/icons"
src="$icons/src"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

render() { rsvg-convert -w "$2" -h "$2" "$src/$1.svg" -o "$3"; }

render icon-object 1024 "$tmp/object.png"
render icon-macos 1024 "$tmp/macos.png"
pnpm -C "$root/apps/desktop" tauri icon "$tmp/object.png" -o "$icons"
pnpm -C "$root/apps/desktop" tauri icon "$tmp/macos.png" -o "$tmp/macos"
cp "$tmp/macos/icon.icns" "$icons/icon.icns"

render icon-small 32 "$icons/32x32.png"
render icon-small 30 "$icons/Square30x30Logo.png"
render icon-small 44 "$icons/Square44x44Logo.png"
for s in 16 24 32 48; do render icon-small "$s" "$tmp/ico-$s.png"; done
for s in 64 256; do render icon-object "$s" "$tmp/ico-$s.png"; done
convert "$tmp"/ico-{16,24,32,48,64,256}.png "$icons/icon.ico"
echo "icons regenerated in $icons"
