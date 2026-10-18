#!/usr/bin/env bash
# Regenerates apps/desktop/src-tauri/icons from the owner's raster master (never redrawn).
#   icons/src/icon-master.png  the icon art, the single source
#   icons/src/macos-tile.svg   the empty macOS squircle tile the art sits in (.icns only)
# Background -> transparent (edge flood fill when the master is opaque), alpha noise dropped,
# trimmed, padded to a 1024 square, then Lanczos-downsampled to every desktop size.
# `tauri icon` lays out the full set (Android/iOS included); the desktop files are then
# re-rendered here. Needs ImageMagick 6, rsvg-convert (librsvg2-bin), python3.
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
icons="$root/apps/desktop/src-tauri/icons"
src="$icons/src"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
object_side=940 # ~92% of the 1024 artboard width (Windows/Linux, bare object; the book is wide)
small_side=993  # ~97% width, no padding, for sizes <=32 px (legibility over padding)
tile_side=650   # ~79% of the 824 px Apple-grid tile
tile_small=800  # ~97% of the tile, for the 16/32 px .icns entries

cutout() {
  if [ "$(convert "$src/icon-master.png" -format '%[fx:p{0,0}.a==1]' info:)" = 1 ]; then
    convert "$src/icon-master.png" -alpha set -bordercolor white -border 1 \
      -fuzz 4% -fill none -draw 'matte 0,0 floodfill' -shave 1x1 "$1"
  else
    cp "$src/icon-master.png" "$1"
  fi
  convert "$1" -channel A -black-threshold 5% +channel -trim +repage "$1"
}
fit() { convert "$1" -filter Lanczos -resize "$2x$2" -background none -gravity center -extent "$3x$3" "$4"; }
down() { convert "$1" -filter Lanczos -resize "$2x$2" -strip "$3"; }
pick() { if [ "$1" -le 32 ]; then echo "$tmp/$2-small.png"; else echo "$tmp/$2.png"; fi; }
mac() { fit "$tmp/cut.png" "$1" 1024 "$tmp/art.png" && convert "$tmp/tile.png" "$tmp/art.png" -composite "$2"; }

cutout "$tmp/cut.png"
fit "$tmp/cut.png" "$object_side" 1024 "$tmp/object.png"
fit "$tmp/cut.png" "$small_side" 1024 "$tmp/object-small.png"
rsvg-convert -w 1024 -h 1024 "$src/macos-tile.svg" -o "$tmp/tile.png"
mac "$tile_side" "$tmp/macos.png"
mac "$tile_small" "$tmp/macos-small.png"

pnpm -C "$root/apps/desktop" tauri icon "$tmp/object.png" -o "$icons"
for f in "$icons"/*.png; do w=$(identify -format %w "$f") && down "$(pick "$w" object)" "$w" "$f"; done
for s in 16 24 32 48 64 256; do down "$(pick "$s" object)" "$s" "$tmp/ico-$s.png"; done
convert "$tmp"/ico-{16,24,32,48,64,256}.png "$icons/icon.ico"
for s in 16 32 64 128 256 512 1024; do down "$(pick "$s" macos)" "$s" "$tmp/icns-$s.png"; done
python3 "$root/scripts/icns.py" "$icons/icon.icns" "$tmp"
echo "icons regenerated in $icons"
