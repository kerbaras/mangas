#!/usr/bin/env bash
# Dry run of release-notes.sh, the release body and latest.json `notes` release.yml publishes:
# a stable section, an rc section, a section followed by older ones and compare links, a CRLF
# checkout (Windows runners), a tag without a section, and the real CHANGELOG.md.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
notes="$root/scripts/release-notes.sh"
sandbox="$(mktemp -d)"
trap 'rm -rf "$sandbox"' EXIT

cat > "$sandbox/CHANGELOG.md" <<'MD'
# Changelog

All notable changes to this project will be documented in this file.
## [1.2.0] - 2026-11-02

### Added

- **desktop:** What's new in the update banner (#9)

### Changed

- **export:** Faster pages (#8)

### Fixed

- **ci:** Release notes carry the whole section (#7)
## [1.2.0-rc.1] - 2026-11-01

### Fixed

- **ci:** Release notes carry the whole section (#7)
## [1.1.0] - 2026-10-30

### Fixed

- **core:** Older fix (#3)

## [1.0.0] - 2026-10-29

### Added

- First release

[1.2.0]: https://github.com/kerbaras/mangas/compare/v1.1.0...v1.2.0
[1.1.0]: https://github.com/kerbaras/mangas/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/kerbaras/mangas/releases/tag/v1.0.0
MD
sed 's/$/\r/' "$sandbox/CHANGELOG.md" > "$sandbox/CHANGELOG.crlf.md"

check() { # check NAME TAG CHANGELOG: release-notes.sh's stdout must equal the expected file's
  local name="$1" tag="$2" file="$3"
  echo "== $name: scripts/release-notes.sh $tag"
  "$notes" "$tag" "$file" > "$sandbox/got" 2> "$sandbox/err" ||
    { echo "FAIL: exited $?"; cat "$sandbox/err"; exit 1; }
  if ! diff -u "$sandbox/want" "$sandbox/got"; then echo "FAIL: $name"; exit 1; fi
  echo "ok: $(wc -l < "$sandbox/got") lines, $(grep -c '^- ' "$sandbox/got" || true) entries"
}

sed -n '/^## \[1\.2\.0\] /,/^- \*\*ci/p' "$sandbox/CHANGELOG.md" > "$sandbox/want"
check "stable section, every ### list, stops at the rc heading" v1.2.0 "$sandbox/CHANGELOG.md"
check "same, from a CRLF checkout" v1.2.0 "$sandbox/CHANGELOG.crlf.md"

printf '%s\n' "## [1.2.0-rc.1] - 2026-11-01" "" "### Fixed" "" \
  "- **ci:** Release notes carry the whole section (#7)" > "$sandbox/want"
check "rc section of its own" v1.2.0-rc.1 "$sandbox/CHANGELOG.md"

printf '%s\n' "## [1.1.0] - 2026-10-30" "" "### Fixed" "" "- **core:** Older fix (#3)" > "$sandbox/want"
check "section followed by older ones, trailing blank line trimmed" v1.1.0 "$sandbox/CHANGELOG.md"

printf '%s\n' "## [1.0.0] - 2026-10-29" "" "### Added" "" "- First release" > "$sandbox/want"
check "last section, compare links trimmed" v1.0.0 "$sandbox/CHANGELOG.md"

echo "## [1.3.0]" > "$sandbox/want"
check "tag without a section: heading only" v1.3.0 "$sandbox/CHANGELOG.md"
grep -q '^::warning title=No release notes::' "$sandbox/err" || { echo "FAIL: no warning"; exit 1; }
echo "ok: warns $(cut -d: -f5- "$sandbox/err")"
check "no CHANGELOG.md at all: heading only" v1.3.0 "$sandbox/missing.md"

# The real file: every released section has its ### lists and nothing of the next section.
for tag in v0.6.0 v0.6.0-rc.2 v0.5.0; do
  echo "== CHANGELOG.md: scripts/release-notes.sh $tag"
  (cd "$root" && "$notes" "$tag") > "$sandbox/got"
  head -1 "$sandbox/got" | grep -q "^## \[${tag#v}\] - " || { echo "FAIL: heading"; exit 1; }
  grep -q '^### ' "$sandbox/got" || { echo "FAIL: no ### list"; exit 1; }
  [[ "$(grep -c '^## \[' "$sandbox/got")" == 1 ]] || { echo "FAIL: ran into the next section"; exit 1; }
  echo "ok: $(wc -l < "$sandbox/got") lines, $(grep -c '^- ' "$sandbox/got") entries," \
    "$(grep '^### ' "$sandbox/got" | tr '\n' ' ')"
done
echo "all release-notes cases pass"
