#!/usr/bin/env bash
# Prints the next version from conventional commits (git-cliff) for a channel.
#   next-version.sh stable -> 1.8.3        (bump derived from commits since the last tag)
#   next-version.sh rc     -> 1.8.3-rc.2   (same target, rc counter continues the series)
set -euo pipefail
channel="${1:-stable}"

target="$(git cliff --bumped-version 2>/dev/null | sed 's/^v//')"
target="${target%%-*}"                      # a previous rc tag must not pin the target
[[ -n "$target" ]] || { echo "git-cliff could not compute a version" >&2; exit 1; }

case "$channel" in
  stable) echo "$target" ;;
  rc)
    last="$(git tag --list "v${target}-rc.*" --sort=-v:refname | head -n1)"
    n=0; [[ -n "$last" ]] && n="${last##*-rc.}"
    echo "${target}-rc.$((n + 1))" ;;
  *) echo "unknown channel '$channel' (stable|rc)" >&2; exit 1 ;;
esac
