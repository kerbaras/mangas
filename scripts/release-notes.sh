#!/usr/bin/env bash
# Prints the release notes for tag $1 (vX.Y.Z or vX.Y.Z-rc.N): its `## [X.Y.Z…]` section of
# CHANGELOG.md ($2 overrides the path), heading included, up to the next `## [` heading, with
# trailing blank lines and link definitions trimmed. release.yml uses it as the release body,
# which tauri-action also writes into latest.json's `notes` (the in-app "What's new").
# A tag without a section gets just its heading and a warning: the release itself goes on.
set -euo pipefail
tag="$1"
changelog="${2:-CHANGELOG.md}"
version="${tag#v}"
heading="## [$version]"

section=()
found=0
if [[ -f "$changelog" ]]; then
  while IFS= read -r line || [[ -n "$line" ]]; do
    line="${line%$'\r'}"
    if ((found)); then
      [[ "$line" == "## ["* ]] && break
      section+=("$line")
    elif [[ "$line" == "$heading" || "$line" == "$heading "* ]]; then
      found=1
      section+=("$line")
    fi
  done < "$changelog"
fi

if ((!found)); then
  echo "::warning title=No release notes::$changelog has no '$heading' section, so the release notes are only that heading. Add the section to CHANGELOG.md, then edit the release body." >&2
  echo "$heading"
  exit 0
fi

# Drop trailing blank lines and `[x.y.z]: https://…` compare links.
last=$((${#section[@]} - 1))
while ((last > 0)) && [[ -z "${section[last]//[[:space:]]/}" || "${section[last]}" =~ ^\[[^]]+\]:\  ]]; do
  last=$((last - 1))
done
printf '%s\n' "${section[@]:0:last+1}"
