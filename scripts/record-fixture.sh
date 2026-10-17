#!/usr/bin/env bash
# Records one provider response as a test fixture: fetches <url> with the headers mangas sends
# (its user agent, plus the referer / session cookie the provider uses) and writes it to
# packages/providers/tests/fixtures/<provider>/<name>.
#
# Usage: scripts/record-fixture.sh [-r referer] [-c cookie] [-a user-agent] [-H 'name: value']
#                                  <provider> <name> <url>
# Example: scripts/record-fixture.sh mangadex contract/search.json \
#            'https://api.mangadex.org/manga?title=eminence&limit=2&includes[]=cover_art'
# For sites behind a browser verification pass the cookies and the user agent of the session
# the desktop app saved (-c 'cf_clearance=...' -a '<that browser UA>').
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
version="$(sed -nE '0,/^version = "([^"]+)"/s//\1/p' "$root/Cargo.toml")"
ua="mangas/${version} (+https://github.com/kerbaras/mangas)"
referer="" cookie=""
extra=()
while getopts "r:c:a:H:" opt; do
  case "$opt" in
    r) referer="$OPTARG" ;;
    c) cookie="$OPTARG" ;;
    a) ua="$OPTARG" ;;
    H) extra+=(-H "$OPTARG") ;;
    *) sed -n '5,7p' "$0" >&2; exit 2 ;;
  esac
done
shift $((OPTIND - 1))
if [ $# -ne 3 ]; then
  sed -n '5,7p' "$0" >&2
  exit 2
fi
provider="$1" name="$2" url="$3"

out="$root/packages/providers/tests/fixtures/$provider/$name"
mkdir -p "$(dirname "$out")"
args=(-fsSL --globoff --compressed -A "$ua")
[ -n "$referer" ] && args+=(-e "$referer")
[ -n "$cookie" ] && args+=(-b "$cookie")
curl "${args[@]}" "${extra[@]}" -o "$out" "$url"
echo "wrote ${out#"$root/"} ($(wc -c <"$out") bytes)"
