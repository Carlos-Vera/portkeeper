#!/bin/sh
# Print the CHANGELOG.md section for one version, as the notes of its GitHub release.
#   .github/release-notes.sh v0.5.0   (or 0.5.0)
# Exits 1 when the section is missing, so a tag without notes fails before anything is built.
set -eu
ver="${1#v}"
[ -n "$ver" ] || { echo "usage: $0 <version>" >&2; exit 1; }
notes="$(awk -v ver="$ver" '
  /^## / { on = ($0 ~ "^## \\[?" ver "\\]?( |$)") ; next }
  on && /^\[.*\]: / { next }   # link references at the end of the file
  on { print }
' CHANGELOG.md | sed '/./,$!d' | sed -e :a -e '/^\n*$/{$d;N;ba' -e '}')"
if [ -z "$(printf '%s' "$notes" | tr -d '[:space:]')" ]; then
  echo "CHANGELOG.md has no section for $ver. Move the Unreleased notes under '## [$ver] - <date>' first." >&2
  exit 1
fi
printf '%s\n' "$notes"
