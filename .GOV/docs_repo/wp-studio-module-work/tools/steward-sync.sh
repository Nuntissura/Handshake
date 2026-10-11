#!/usr/bin/env bash
# Steward sync commit (WP-KERNEL-STUDIO workspace steward only; never part of the round path).
# From an in-place export of HEAD (== origin) at Handshake_Artifacts/WP-KERNEL-STUDIO/studio-team-dev/sync-export:
#   optional allowed-edge additions (edges.py), derive Cargo.lock offline from HEAD's committed manifests
#   (`cargo metadata --offline`: minimal, pins kept; never cargo update), registry-sync --write (pins + probe
#   APPROVED; dep-requests gate), license gate, verify `cargo metadata --locked --offline` + registry-sync check +
#   probe per package, then commit ONLY Cargo.lock / root Cargo.toml / tools/workspace-probe.rs with parent = HEAD
#   via a temp index and publish with compare-and-swap update-ref + push. The shared index and working files are
#   never touched (builders commit with pathspec). Prints SYNC_SHA (= round candidate) and writes the export's
#   ROUND_IDENTITY.json for it.
# Usage: steward-sync.sh [--add-edge crate=dep]... [--remove-edge crate=dep]... [--allow-unrequested] [--no-push]
# --allow-unrequested only after reviewing every "unrequested" pair (license gate still runs); report them to the lead.
# Exit: 0 ok, 3 HEAD moved / not pushed (redo), 4 edge error, 5 registry-sync, 6 verify failed, 8 license gate,
#       75 offline lock derivation failed (crate missing from local cargo cache).
set -uo pipefail
TOOLS=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
WORK=$(cd "$TOOLS/.." && pwd)
WT=$(cd "$TOOLS/../../../../.." && pwd)
REPO="$WT/wt-studio-assets"
ART="$WT/Handshake_Artifacts/WP-KERNEL-STUDIO/studio-team-dev"
EX="$ART/sync-export"
W="$EX/src/handshake-studio-assets"
BR=codex/handshake-studio-assets
REQ="$WORK/builders/dep-requests.jsonl"
edges=() push=1 allow=()
while [ $# -gt 0 ]; do
  case "$1" in --add-edge) edges+=("${2:?}"); shift ;; --remove-edge) edges+=("remove:${2:?}"); shift ;; --no-push) push=0 ;; --allow-unrequested) allow=(--allow-unrequested) ;; *) echo "unknown arg $1" >&2; exit 2 ;; esac
  shift
done
lockpk() {
  awk '/^\[\[package\]\]/ { if (n != "") print n "@" v "\t" d; n = v = d = ""; deps = 0; next }
       /^name = /    { n = $3; gsub(/"/, "", n); next }
       /^version = / { v = $3; gsub(/"/, "", v); next }
       /^dependencies = \[/ { deps = 1; next }
       deps && /^\]/ { deps = 0; next }
       deps { x = $0; gsub(/[",]/, "", x); sub(/^ +/, "", x); d = d (d == "" ? "" : ";") x; next }
       END { if (n != "") print n "@" v "\t" d }' "$1" | LC_ALL=C sort
}

cd "$REPO" && git fetch -q origin || exit 1
BASE=$(git rev-parse HEAD)
[ "$BASE" = "$(git rev-parse "origin/$BR")" ] || { echo "HEAD != origin/$BR (unpushed or stale); retry"; exit 3; }
mkdir -p "$EX"
bash "$TOOLS/export-refresh.sh" "$EX" "$BASE" --no-identity || exit 1
rm -f "$EX/ROUND_IDENTITY.json"   # identity is written only for the published sync commit
cd "$W" || exit 1
source "$WORK/builders/studio-env.sh"
cp Cargo.lock "$EX/committed-Cargo.lock"

if [ ${#edges[@]} -gt 0 ]; then python -X utf8 -I "$TOOLS/edges.py" . "${edges[@]}" || exit 4; fi
if ! cargo metadata --offline --format-version 1 >/dev/null 2>"$EX/derive.err"; then
  crate=$(grep -o '`[^`]*`' "$EX/derive.err" | head -n1 | tr -d '`')
  echo "steward-sync: infrastructure: offline lock derivation failed (${crate:-unknown}); see $EX/derive.err" >&2
  exit 75
fi
lockpk "$EX/committed-Cargo.lock" >"$EX/lock-committed.tsv"
lockpk Cargo.lock >"$EX/lock-derived.tsv"
added=$(LC_ALL=C comm -13 <(cut -f1 "$EX/lock-committed.tsv") <(cut -f1 "$EX/lock-derived.tsv") | paste -sd' ')
removed=$(LC_ALL=C comm -23 <(cut -f1 "$EX/lock-committed.tsv") <(cut -f1 "$EX/lock-derived.tsv") | paste -sd' ')
edged=$(LC_ALL=C comm -12 <(LC_ALL=C comm -13 "$EX/lock-committed.tsv" "$EX/lock-derived.tsv" | cut -f1) \
  <(LC_ALL=C comm -12 <(cut -f1 "$EX/lock-committed.tsv") <(cut -f1 "$EX/lock-derived.tsv")) | paste -sd' ')
echo "LOCK_DIFF added=[${added}] removed=[${removed}] dependency_lists_changed=[${edged}]" | tee "$EX/lock-diff.txt"

python -X utf8 -I tools/registry-sync.py . --write --requests "$REQ" ${allow[@]+"${allow[@]}"} >"$EX/registry.json" || { cat "$EX/registry.json"; exit 5; }
python -X utf8 -I -c 'import json,sys; d=json.load(open(sys.argv[1])); print("REGISTRY approved_added=%s spec_defects=%s" % (d["approved_added"], d["spec_defects"]))' "$EX/registry.json"
python -X utf8 -I "$TOOLS/license_gate.py" . || exit 8
cargo metadata --locked --offline --format-version 1 >/dev/null 2>"$EX/verify.err" || { cat "$EX/verify.err"; exit 6; }
python -X utf8 -I tools/registry-sync.py . >/dev/null || { echo "registry-sync check not clean after write"; exit 6; }

mkdir -p "$ART/probe"
rustc --edition=2024 -O tools/workspace-probe.rs -o "$ART/probe/workspace-probe.exe" || exit 6
TREE=$(git -C "$REPO" rev-parse "$BASE^{tree}")
python -X utf8 -I "$TOOLS/probe_provenance.py" "$W" "$BASE" "$TREE" "$EX/probe-provenance.toml" $(ls crates) >/dev/null
: >"$EX/probe-rejects.txt"
for p in $(ls crates); do
  r=$("$ART/probe/workspace-probe.exe" --root "$W" --provenance "$EX/probe-provenance.toml" --revision "$BASE" --tree "$TREE" --package "$p")
  case "$r" in *'"accepted"'*) ;; *) echo "$p $(echo "$r" | sed -n 's/.*"code":"\([^"]*\)".*/\1/p')" >>"$EX/probe-rejects.txt" ;; esac
done
echo "PROBE accepted=$(( $(ls crates | wc -l) - $(wc -l <"$EX/probe-rejects.txt") ))/$(ls crates | wc -l) rejects=[$(paste -sd',' "$EX/probe-rejects.txt")]"

cd "$REPO"
paths=() blobs=()
for f in Cargo.lock Cargo.toml tools/workspace-probe.rs; do
  b=$(git hash-object -w --path="handshake-studio-assets/$f" "$W/$f")
  [ "$b" = "$(git rev-parse "$BASE:handshake-studio-assets/$f")" ] || { paths+=("handshake-studio-assets/$f"); blobs+=("$b"); }
done
if [ ${#paths[@]} -eq 0 ]; then NEW=$BASE; echo "already in sync at $BASE"
else
  printf 'Steward sync: lock, registry pins and probe approvals for %s\n\nDerived offline from the committed manifests of %s (pins kept).\nLock added: %s\nLock removed: %s\nDependency lists changed: %s\nAllowed edge changes: %s\n\nCo-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>\n' \
    "${BASE:0:7}" "$BASE" "${added:-none}" "${removed:-none}" "${edged:-none}" "${edges[*]:-none}" >"$EX/msg.txt"
  export GIT_INDEX_FILE="$EX/commit-index"
  rm -f "$GIT_INDEX_FILE"
  git read-tree "$BASE"
  for i in "${!paths[@]}"; do git update-index --cacheinfo "100644,${blobs[$i]},${paths[$i]}"; done
  NT=$(git write-tree)
  NEW=$(git commit-tree "$NT" -p "$BASE" -F "$EX/msg.txt")
  unset GIT_INDEX_FILE
  git update-ref "refs/heads/$BR" "$NEW" "$BASE" || { echo "CAS failed: $BR moved past $BASE; redo"; exit 3; }
  echo "committed $NEW (${paths[*]})"
  if [ $push -eq 1 ]; then git push origin "$BR" 2>&1 | tail -n1; fi
fi
bash "$TOOLS/export-refresh.sh" "$EX" "$NEW" || exit 1
echo "SYNC_SHA=$NEW"
