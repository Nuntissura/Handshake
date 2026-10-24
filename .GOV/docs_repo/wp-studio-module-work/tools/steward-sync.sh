#!/usr/bin/env bash
# Steward sync commit (WP-KERNEL-STUDIO workspace steward only; never part of the round path).
# From an in-place export of HEAD (== origin) at Handshake_Artifacts/WP-KERNEL-STUDIO/studio-team-dev/sync-export:
#   optional steward-owned root files overlaid from the shared edit tree (--overlay), optional allowed-edge
#   additions (edges.py), derive Cargo.lock offline from HEAD's committed manifests (`cargo metadata --offline`:
#   minimal, pins kept; never cargo update), registry-sync --write (pins + probe APPROVED; dep-requests gate over
#   the request lines that name a dep), license gate, verify `cargo metadata --locked --offline` + registry-sync
#   check + probe per package AND per .config/round-features.toml feature set (any reject fails the sync), then
#   commit ONLY Cargo.lock / root Cargo.toml / tools/workspace-probe.rs / overlays with parent = HEAD via a temp
#   index and publish with compare-and-swap update-ref + push (lanes that pushed meanwhile: re-parent onto the
#   tip when no derivation input changed, else exit 3). Builders' files are never touched: after the
#   commit only the committed root paths are reconciled in the shared tree (index entry -> new blob when it still
#   held the base blob; working file -> committed content when it held the base or overlay content; the working
#   Cargo.lock stays builders' local state). Prints SYNC_SHA (= round candidate) and writes the export's
#   ROUND_IDENTITY.json for it.
# RULE: export trees never share a target dir with the edit tree. Cargo names path-package artifacts by the path
#   relative to the workspace root, and exports carry current mtimes, so an export build into the builders' target
#   (studio-team-dev/target) leaves artifacts from other sources that the edit tree then treats as fresh (lead root
#   cause 2026-10-11, 5 stale-artifact incidents). This script's cargo runs use sync-export/target and sync-export/tmp.
#   Steward canary (export of SYNC_SHA refreshed in place with export-refresh.sh into studio-team-dev/steward-export),
#   from steward-export/src/handshake-studio-assets after sourcing builders/studio-env.sh:
#     RUSTC_WRAPPER=sccache SCCACHE_DIR=<studio-team-dev>/sccache CARGO_INCREMENTAL=0 \
#     bash tools/run-round.sh --canary --all --results <steward-export>/results/C<n>-<sha7> \
#       --target-dir <steward-export>/target        (never studio-team-dev/target)
# Usage: steward-sync.sh [--overlay RELPATH]... [--add-edge crate=dep]... [--remove-edge crate=dep]...
#                        [--allow-unrequested] [--no-push] [--dry-run]
# --overlay: steward-owned path relative to the workspace root (.config/*, .cargo/config.toml, tools/run-round.sh,
#   tools/workspace-probe.rs, tools/registry-sync.py, tools/disk-build-lock.sh) copied from the shared edit tree
#   into the export first.
# Disk-wide build lock: cargo goes through the studio-env.sh wrapper (metadata is not a build, so unlocked); the
#   probe compile holds the lock; the candidate's tools/disk-build-lock.sh must equal the kernel copy (else exit 6).
# --allow-unrequested only after reviewing every "unrequested" pair (license gate still runs); report them to the lead.
# --dry-run: everything up to and including the probes; prints the paths that would be committed; no commit.
# Exit: 0 ok, 2 usage, 3 HEAD moved / not pushed (redo), 4 edge error, 5 registry-sync, 6 verify failed,
#       7 probe rejected a package or feature set, 8 license gate, 9 committed but push failed,
#       75 offline lock derivation failed (crate missing from local cargo cache).
set -uo pipefail
TOOLS=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
WORK=$(cd "$TOOLS/.." && pwd)
WT=$(cd "$TOOLS/../../../../.." && pwd)
REPO="$WT/wt-studio-assets"
SUB=handshake-studio-assets
ART="$WT/Handshake_Artifacts/WP-KERNEL-STUDIO/studio-team-dev"
EX="$ART/sync-export"
W="$EX/src/$SUB"
BR=codex/handshake-studio-assets
REQ="$WORK/builders/dep-requests.jsonl"
edges=() push=1 allow=() overlays=() dry=0
while [ $# -gt 0 ]; do
  case "$1" in
    --add-edge) edges+=("${2:?}"); shift ;;
    --remove-edge) edges+=("remove:${2:?}"); shift ;;
    --overlay)
      case "${2:?}" in
        .config/*.toml|.cargo/config.toml|tools/run-round.sh|tools/workspace-probe.rs|tools/registry-sync.py|tools/disk-build-lock.sh) overlays+=("$2") ;;
        *) echo "overlay not steward-owned: $2" >&2; exit 2 ;;
      esac
      case "$2" in *..*) echo "overlay path may not contain ..: $2" >&2; exit 2 ;; esac
      shift ;;
    --no-push) push=0 ;;
    --dry-run) dry=1 ;;
    --allow-unrequested) allow=(--allow-unrequested) ;;
    *) echo "unknown arg $1" >&2; exit 2 ;;
  esac
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
declare -A ovblob=()
for rel in ${overlays[@]+"${overlays[@]}"}; do
  [ -f "$REPO/$SUB/$rel" ] || { echo "overlay source missing: $rel" >&2; exit 2; }
  mkdir -p "$(dirname "$W/$rel")"
  cp "$REPO/$SUB/$rel" "$W/$rel"
  ovblob[$rel]=$(git hash-object --path="$SUB/$rel" "$W/$rel")
  echo "OVERLAY $rel ${ovblob[$rel]:0:12}"
done
cd "$W" || exit 1
# Export trees never share a target dir with the edit tree (see header): this export's cargo state stays in sync-export/.
export STUDIO_OWNER=studio-team-dev/sync-export
source "$WORK/builders/studio-env.sh" || exit 2   # tree-binding guard: sync-export/target is bound to this export
[ "$(cd "$CARGO_TARGET_DIR" && pwd)" = "$(cd "$EX/target" 2>/dev/null && pwd)" ] \
  || { echo "steward-sync: export target $CARGO_TARGET_DIR is not $EX/target" >&2; exit 2; }
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

# The request log also carries steward items without a dep (round-features requests); the gate reads dep lines only.
python -X utf8 -I -c 'import json,sys
out = open(sys.argv[2], "w", encoding="utf-8")
for line in open(sys.argv[1], encoding="utf-8"):
    if line.strip():
        r = json.loads(line)
        if r.get("crate") and r.get("dep"):
            out.write(json.dumps({"crate": r["crate"], "dep": r["dep"]}) + "\n")' "$REQ" "$EX/dep-requests.deps.jsonl" || exit 5
python -X utf8 -I tools/registry-sync.py . --write --requests "$EX/dep-requests.deps.jsonl" ${allow[@]+"${allow[@]}"} >"$EX/registry.json" || { cat "$EX/registry.json"; exit 5; }
python -X utf8 -I -c 'import json,sys; d=json.load(open(sys.argv[1])); print("REGISTRY approved_added=%s approved_removed=%s unrequested=%s spec_defects=%s fuzzing_only_skipped=%s" % (d["approved_added"], d["approved_removed"], d["unrequested"], d["spec_defects"], d.get("fuzzing_only_skipped", [])))' "$EX/registry.json"
python -X utf8 -I "$TOOLS/license_gate.py" . || exit 8
cargo metadata --locked --offline --format-version 1 >/dev/null 2>"$EX/verify.err" || { cat "$EX/verify.err"; exit 6; }
python -X utf8 -I tools/registry-sync.py . >/dev/null || { echo "registry-sync check not clean after write"; exit 6; }
cmp -s tools/disk-build-lock.sh "$TOOLS/disk-build-lock.sh" \
  || { echo "disk-build-lock.sh: candidate (product) copy and kernel copy differ; overlay or update one"; exit 6; }

mkdir -p "$ART/probe"
studio_disk_lock_run "steward-sync probe rustc" rustc --edition=2024 -O tools/workspace-probe.rs -o "$ART/probe/workspace-probe.exe" || exit 6
# Probe self-test (fuzzing-only skip rule, lead decision 2026-10-15): both cases must hold before any selection is probed.
"$ART/probe/workspace-probe.exe" --self-test | grep -q '"status":"ok"' || { echo "workspace-probe --self-test failed"; exit 6; }
TREE=$(git -C "$REPO" rev-parse "$BASE^{tree}")
python -X utf8 -I "$TOOLS/probe_provenance.py" "$W" "$BASE" "$TREE" "$EX/probe-provenance.toml" $(ls crates) >/dev/null
# Selections: every package with no features, then every (crate, feature set) of .config/round-features.toml.
{ for p in $(ls crates); do echo "$p"; done
  python -X utf8 -I -c 'import sys, tomllib
for crate, sets in tomllib.load(open(sys.argv[1], "rb")).get("crates", {}).items():
    for s in sets:
        print(" ".join([crate] + s))' .config/round-features.toml | tr -d '\r'
  [ "${PIPESTATUS[0]}" -eq 0 ] || echo "MATRIX_UNREADABLE"
} >"$EX/probe-selections.txt"
: >"$EX/probe-rejects.txt"; : >"$EX/probe-results.jsonl"
while read -r p feats; do
  [ "$p" = MATRIX_UNREADABLE ] && { echo "round-features matrix unreadable" >>"$EX/probe-rejects.txt"; continue; }
  fa=(); for f in $feats; do fa+=(--feature "$p/$f"); done
  r=$("$ART/probe/workspace-probe.exe" --root "$W" --provenance "$EX/probe-provenance.toml" --revision "$BASE" --tree "$TREE" --package "$p" ${fa[@]+"${fa[@]}"})
  echo "$r" >>"$EX/probe-results.jsonl"
  case "$r" in *'"status":"accepted"'*) ;; *) echo "$p${feats:+[$feats]} $(echo "$r" | sed -n 's/.*"code":"\([^"]*\)".*/\1/p')" >>"$EX/probe-rejects.txt" ;; esac
done <"$EX/probe-selections.txt"
total=$(wc -l <"$EX/probe-selections.txt"); bad=$(wc -l <"$EX/probe-rejects.txt")
echo "PROBE accepted=$(( total - bad ))/$total (packages $(ls crates | wc -l), feature sets $(( total - $(ls crates | wc -l) ))) rejects=[$(paste -sd',' "$EX/probe-rejects.txt")]"
[ "$bad" -eq 0 ] || exit 7

cd "$REPO"
files=(Cargo.lock Cargo.toml tools/workspace-probe.rs)
for rel in ${overlays[@]+"${overlays[@]}"}; do case " ${files[*]} " in *" $rel "*) ;; *) files+=("$rel") ;; esac; done
paths=() blobs=() modes=()
for f in "${files[@]}"; do
  wflag=-w; [ $dry -eq 1 ] && wflag=
  b=$(git hash-object $wflag --path="$SUB/$f" "$W/$f")
  [ "$b" = "$(git rev-parse -q --verify "$BASE:$SUB/$f" 2>/dev/null)" ] && continue
  m=$(git ls-tree "$BASE" -- "$SUB/$f" | cut -d' ' -f1); paths+=("$SUB/$f"); blobs+=("$b"); modes+=("${m:-100644}")
done
if [ $dry -eq 1 ]; then echo "DRY_RUN base=$BASE would_commit=[${paths[*]}]"; exit 0; fi
if [ ${#paths[@]} -eq 0 ]; then NEW=$BASE; echo "already in sync at $BASE"
else
  msg() {  # $1 = parent; a re-parent line is added when the parent is not BASE
    local rp=""; [ "$1" = "$BASE" ] || rp="Re-parented onto ${1:0:7}: no derivation input changed after ${BASE:0:7}.
"
    printf "Steward sync: lock, registry pins and probe approvals for %s

Derived offline from the committed manifests of %s (pins kept).
${rp}Lock added: %s
Lock removed: %s
Dependency lists changed: %s
Allowed edge changes: %s
Steward root files overlaid: %s
Probe: %s selections accepted (packages and round-features sets)

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
"       "${BASE:0:7}" "$BASE" "${added:-none}" "${removed:-none}" "${edged:-none}" "${edges[*]:-none}" "${overlays[*]:-none}" "$total" >"$EX/msg.txt"
  }
  # Re-parent instead of redoing when lanes pushed during the sync: allowed only when no derivation input changed
  # between BASE and the current tip (root Cargo.toml/Cargo.lock, rust-toolchain.toml, any crate manifest, the
  # steward tools, .config and .cargo files, or a committed path); the derived blobs are then identical for the tip.
  inputs_re="^$SUB/(Cargo\.toml|Cargo\.lock|rust-toolchain\.toml|crates/[^/]+/Cargo\.toml|tools/|\.config/|\.cargo/)"
  NEW="" PARENT=""
  for attempt in 1 2 3 4 5 6 7 8; do
    # Lanes push from their own worktrees, so origin moves while the probe waits for the disk lock (3 of 3 syncs on
    # 2026-10-16..20 failed the push). Fast-forward the shared branch to the origin tip first (two-tree read-tree
    # refuses if a local edit touches an incoming file); the input check below then decides re-parent vs redo.
    git fetch -q origin "$BR" && TIP=$(git rev-parse FETCH_HEAD) && LOC=$(git rev-parse "refs/heads/$BR")
    if [ -n "${TIP:-}" ] && [ "$TIP" != "$LOC" ] && git merge-base --is-ancestor "$LOC" "$TIP"; then
      git -c core.fscache=false read-tree -m -u "$LOC" "$TIP" && git update-ref "refs/heads/$BR" "$TIP" "$LOC" \
        || { echo "shared tree could not fast-forward $LOC -> origin $TIP (local edit on an incoming file?); redo"; exit 3; }
      echo "fast-forwarded shared tree ${LOC:0:7} -> origin ${TIP:0:7}"
    fi
    PARENT=$(git rev-parse "refs/heads/$BR")
    if [ "$PARENT" != "$BASE" ]; then
      git merge-base --is-ancestor "$BASE" "$PARENT" || { echo "tip $PARENT does not descend from $BASE; redo"; exit 3; }
      moved=$(git diff --name-only "$BASE" "$PARENT" | grep -E "$inputs_re" | paste -sd' ')
      [ -z "$moved" ] || { echo "derivation inputs changed since $BASE ($moved); redo"; exit 3; }
    fi
    msg "$PARENT"
    export GIT_INDEX_FILE="$EX/commit-index"
    rm -f "$GIT_INDEX_FILE"
    git read-tree "$PARENT"
    for i in "${!paths[@]}"; do git update-index --add --cacheinfo "${modes[$i]},${blobs[$i]},${paths[$i]}"; done
    NT=$(git write-tree)
    NEW=$(git commit-tree "$NT" -p "$PARENT" -F "$EX/msg.txt")
    unset GIT_INDEX_FILE
    if git update-ref "refs/heads/$BR" "$NEW" "$PARENT" 2>/dev/null; then break; fi
    echo "CAS attempt $attempt: $BR moved past $PARENT"; NEW=""
  done
  [ -n "$NEW" ] || { echo "CAS failed 8 times; redo"; exit 3; }
  echo "committed $NEW parent $PARENT (${paths[*]})"
  pushed=0
  if [ $push -eq 1 ]; then git push origin "$BR" 2>&1 | tail -n1; pushed=${PIPESTATUS[0]}; fi
  [ "$pushed" -eq 0 ] || echo "PUSH FAILED (exit $pushed): $NEW is committed locally only; push before naming it a candidate"
  # Shared-tree reconcile for the committed root paths only (see header).
  for i in "${!paths[@]}"; do
    p=${paths[$i]} nb=${blobs[$i]} rel=${paths[$i]#"$SUB/"}
    ob=$(git rev-parse -q --verify "$BASE:$p" 2>/dev/null || echo none)
    ib=$(git ls-files -s -- "$p" | cut -d' ' -f2)
    if [ "${ib:-none}" = "$ob" ] || [ -z "$ib" ]; then
      git update-index --add --cacheinfo "${modes[$i]},$nb,$p" || echo "RECONCILE index entry not updated: $p"
    else echo "RECONCILE index entry left (staged by someone else): $p"; fi
    [ "$rel" = Cargo.lock ] && continue
    wb=$(git hash-object --path="$p" "$p" 2>/dev/null || echo none)
    if [ "$wb" = "$ob" ] || [ "$wb" = "${ovblob[$rel]:-x}" ] || [ "$wb" = none ]; then
      cp "$W/$rel" "$p"
    elif [ "$wb" != "$nb" ]; then echo "RECONCILE working file left (edited during sync): $p"; fi
  done
fi
bash "$TOOLS/export-refresh.sh" "$EX" "$NEW" || exit 1
echo "SYNC_SHA=$NEW"
[ "${pushed:-0}" -eq 0 ] || exit 9
