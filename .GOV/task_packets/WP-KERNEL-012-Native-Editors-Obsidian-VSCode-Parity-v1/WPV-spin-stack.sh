#!/usr/bin/env bash
# WP-KERNEL-012 offline stack analysis of a BACKEND_CPU_SPIN capture (CX-VAL-007 round tooling; TOOLS.json adopted tools).
#   WPV-spin-stack.sh <40-hex sha> [union|pin-measure]
# Inputs (written by WPV-round-observer.ps1): <lane>/logs/<kind>-<sha>.spin-threads.json (hot TID) and .spin-dumps/*.dmp.
# Steps: dump_syms --store <lane>/symstore on the round's backend PDB (dev profile, HSK_TEST_BACKEND_BIN build), then
# minidump-stackwalk --symbols-path <lane>/symstore per dump into <dump>.stack.txt, then the hot thread's stack into
# <kind>-<sha>.spin-stack.txt. Read-only on the dumps; the validator deletes the dumps after recording the stack (Codex :199).
#   WPV-spin-stack.sh <sha> <kind> --delete-dumps   (only after the stack is recorded in the MT JSON): permanently deletes the
#   *.dmp files (full-memory dumps may hold session tokens/test secrets; never recycled or copied out of the artifact root).
set -euo pipefail
SHA="${1:?usage: WPV-spin-stack.sh <sha> [union|pin-measure] [--delete-dumps]}"; KIND="${2:-union}"; ACTION="${3:-}"
[[ "$SHA" =~ ^[0-9a-f]{40}$ ]] || { echo "full sha required"; exit 2; }
LANE="D:/Projects/LLM projects/Handshake/Handshake Worktrees/Handshake_Artifacts/WP-KERNEL-012/MT-109/wpv-c3x"
TOOLS="D:/Projects/LLM projects/Handshake/Handshake Worktrees/gov_runtime/tools"
DUMP_SYMS="$TOOLS/dump_syms/2.3.9/dump_syms.exe"
STACKWALK="$TOOLS/minidump-stackwalk/0.27.0/minidump-stackwalk.exe"
PREFIX="$LANE/logs/$KIND-$SHA"
PDB="$LANE/backend-bin/debug/handshake_core.pdb"
SYMSTORE="$LANE/symstore"
[ -f "$PREFIX.spin-threads.json" ] || { echo "no spin capture for $SHA ($PREFIX.spin-threads.json)"; exit 3; }
if [ "$ACTION" = "--delete-dumps" ]; then
  [ -s "$PREFIX.spin-stack.txt" ] || { echo "[spin-stack] refusing: stack not extracted yet ($PREFIX.spin-stack.txt)"; exit 4; }
  for d in "$PREFIX.spin-dumps"/*.dmp; do
    [ -f "$d" ] || continue
    echo "[spin-stack] deleting $(basename "$d") bytes=$(stat -c %s "$d") sha256=$(sha256sum "$d" | cut -d' ' -f1)" | tee -a "$PREFIX.spin-dumps-deleted.log"
    rm -f -- "$d"
  done
  exit 0
fi
HOT_TID="$(python -c "import json,sys;print(json.load(open(sys.argv[1],encoding='utf-8')).get('hot_tid') or '')" "$PREFIX.spin-threads.json")"
echo "[spin-stack] hot tid=$HOT_TID pdb=$PDB"
mkdir -p "$SYMSTORE"
"$DUMP_SYMS" --store "$SYMSTORE" "$PDB" > "$PREFIX.spin-dump_syms.log" 2>&1
OUT="$PREFIX.spin-stack.txt"; : > "$OUT"
for d in "$PREFIX.spin-dumps"/*.dmp; do
  [ -f "$d" ] || continue
  "$STACKWALK" --symbols-path "$SYMSTORE" "$d" > "$d.stack.txt" 2> "$d.stackwalk.log" || echo "[spin-stack] stackwalk exit $? for $d"
  {
    echo "=== $(basename "$d") hot tid $HOT_TID"
    awk -v tid="$HOT_TID" '/^Thread [0-9]+ .*tid: /{p = ($NF == tid)} p' "$d.stack.txt"
  } >> "$OUT"
done
echo "[spin-stack] wrote $OUT ($(wc -l < "$OUT") lines)"
