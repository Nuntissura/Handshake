#!/usr/bin/env bash
# Refresh a Studio assets export in place so <export_root>/src equals the git tree of <commit>
# (GLOBAL-VALIDATION-023..025): only files whose content differs are rewritten (they get current mtimes),
# files absent from the tree are removed, and <export_root>/ROUND_IDENTITY.json binds the result
# (sha, tree, file_count, manifest_sha256, refresh counts, blob check).
# Uses a dedicated index <export_root>/.export-index; never touches the shared worktree index or files.
# Usage: export-refresh.sh <export_root> <commit> [--no-identity]
set -euo pipefail
TOOLS=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO=$(cd "$TOOLS/../../../../../wt-studio-assets" && pwd)
mp() { local p; p=$(realpath -m -- "$1"); command -v cygpath >/dev/null && p=$(cygpath -m "$p"); printf '%s' "$p"; }
ER=$(mp "${1:?export root}"); REF=${2:?commit}; IDENT=1; [ "${3:-}" = --no-identity ] && IDENT=0
case "$ER" in [cC]:*|/c/*) echo "export-refresh: refusing C: path $ER" >&2; exit 2 ;; esac
listing() { [ -d "$1" ] && (cd "$1" && find . -type f -printf '%P\0' | LC_ALL=C sort -z | xargs -0 -r sha256sum \
  | sed 's/^\([0-9a-f]\{64\}\) [ *]/\1  /') || true; }
mkdir -p "$ER/src"
pre=$(listing "$ER/src")
cd "$REPO"
SHA=$(git rev-parse "$REF^{commit}"); TREE=$(git rev-parse "$SHA^{tree}")
export GIT_INDEX_FILE="$ER/.export-index"
git --work-tree="$ER/src" read-tree -u --reset "$TREE"
git --work-tree="$ER/src" ls-files -z --others | (cd "$ER/src" && xargs -0 -r rm -f --)
find "$ER/src" -mindepth 1 -type d -empty -delete
post=$(listing "$ER/src")
unset GIT_INDEX_FILE
count() { [ -z "$1" ] && echo 0 || printf '%s\n' "$1" | grep -c .; }
p_paths=$(printf '%s\n' "$pre" | sed -n 's/^[0-9a-f]\{64\}  //p' | LC_ALL=C sort)
q_paths=$(printf '%s\n' "$post" | sed -n 's/^[0-9a-f]\{64\}  //p' | LC_ALL=C sort)
added=$(count "$(LC_ALL=C comm -13 <(printf '%s\n' "$p_paths") <(printf '%s\n' "$q_paths") | grep . || true)")
deleted=$(count "$(LC_ALL=C comm -23 <(printf '%s\n' "$p_paths") <(printf '%s\n' "$q_paths") | grep . || true)")
same=$(count "$(LC_ALL=C comm -12 <(printf '%s\n' "$pre" | LC_ALL=C sort) <(printf '%s\n' "$post" | LC_ALL=C sort) | grep . || true)")
files=$(count "$q_paths")
changed=$(( files - added - same ))
# Blob check: every exported file hashes (no filters) to the tree's blob id; no tree file missing.
expected=$(git -c core.quotePath=false ls-tree -r "$TREE" | awk '{ sub(/^[0-9]+ blob /, ""); print }' | LC_ALL=C sort -t "$(printf '	')" -k2,2)
actual=$(paste <(printf '%s\n' "$q_paths" | sed "s|^|$ER/src/|" | git hash-object --no-filters --stdin-paths) \
  <(printf '%s\n' "$q_paths") | LC_ALL=C sort -t "$(printf '	')" -k2,2)
mismatch=$(count "$(diff <(printf '%s\n' "$expected") <(printf '%s\n' "$actual") | grep '^[<>]' || true)")
manifest=$(printf '%s\n' "$post" | sha256sum | cut -c1-64)
echo "export-refresh: $SHA files=$files added=$added changed=$changed unchanged=$same deleted=$deleted blob_mismatch=$mismatch"
[ "$mismatch" -eq 0 ] || { echo "export-refresh: export differs from $SHA" >&2; exit 1; }
[ $IDENT -eq 1 ] || exit 0
cat >"$ER/ROUND_IDENTITY.json" <<JSON
{
 "sha": "$SHA",
 "tree": "$TREE",
 "file_count": $files,
 "manifest_sha256": "$manifest",
 "manifest_format": "sorted lines '<sha256>  <relpath>' over exported files under src/ (tools/run-round.sh)",
 "git_blob_equal": true,
 "blob_mismatch_count": 0,
 "missing_count": 0,
 "refresh": {
  "added": $added,
  "changed": $changed,
  "unchanged": $same,
  "deleted": $deleted
 },
 "exported_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
 "source_repo": "wt-studio-assets $SHA (steward export-refresh.sh, dedicated index)"
}
JSON
