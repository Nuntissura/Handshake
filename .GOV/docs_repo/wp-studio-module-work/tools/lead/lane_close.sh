#!/usr/bin/env bash
# Guarded close of one completed Studio lane worktree (Codex CX-GIT-001A, Operator 2026-11-07). Dry run unless --apply.
# Usage: lane_close.sh <worktree-dir-name> <LANE-ID in the ledger> [--apply]
# It removes the worktree and its local lane branch with native Git only, never with force. It never deletes a remote
# branch or a cargo target: the target stays for reuse or for an inspected proposal (HSK-ART-005). Any failed check
# stops with no manual cleanup (CX-122).
set -u
dir=${1:?worktree dir name}
lane=${2:?LANE-ID}
apply=${3:-}
H="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../../../.." && pwd -W)"
S="$H/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work"
T="$H/wt-studio-lanes/$dir"
SCOUT="$H/wt-studio-lanes/smb-type-authored-token-1"
SNAP="$H/Handshake_Artifacts/WP-KERNEL-STUDIO/snapshots"
PRODUCT=codex/handshake-studio-assets
stop() { echo "lane_close STOP: $1" >&2; exit 1; }

[ -d "$T" ] || stop "no worktree dir $T"
[ "$T" != "$SCOUT" ] || stop "the scout tree is kept for read-only audits"
top=$(git -C "$T" rev-parse --show-toplevel 2>/dev/null) || stop "not a git worktree: $T"
[ "$top" = "$T" ] || stop "toplevel $top is not $T"

status=$(python -c "import json,sys;print(next((str(x.get('status')) for x in json.load(open(sys.argv[1],encoding='utf-8'))['lanes'] if x.get('lane_id')==sys.argv[2]),'NO_ROW'))" "$S/ledger.json" "$lane")
case "$status" in PASS_V*) ;; *) stop "lane $lane status is $status, not PASS_Vn" ;; esac

[ ! -e "$T/.GOV" ] || stop "$T/.GOV exists: follow CX-122 by hand"
dirty=$(git -C "$T" status --porcelain --ignored) || stop "git status failed"
[ -z "$dirty" ] || { printf '%s\n' "$dirty" | head -n 20 >&2; stop "tree is not clean, ignored files included"; }

git -C "$T" fetch -q origin "$PRODUCT" || stop "fetch failed"
head=$(git -C "$T" rev-parse HEAD) || stop "no HEAD"
origin=$(git -C "$T" rev-parse FETCH_HEAD) || stop "no FETCH_HEAD"
git -C "$T" merge-base --is-ancestor "$head" "$origin" || stop "HEAD $head is not on origin $PRODUCT $origin: not merged"
branch=$(git -C "$T" branch --show-current)
treehash=$(git -C "$T" rev-parse 'HEAD^{tree}')

win=${T//\//\\}
procs=$(powershell -NoProfile -Command "\$p='$win'; @(Get-CimInstance Win32_Process | Where-Object { \$_.ProcessId -ne \$PID -and \$_.CommandLine -and \$_.CommandLine.Contains(\$p) -and \$_.CommandLine -notmatch 'lane_close' }).Count" | tr -d '\r ')
[ "$procs" = "0" ] || stop "$procs process(es) name this tree"

echo "lane_close: $dir lane=$lane status=$status branch=$branch head=${head:0:8} on origin ${origin:0:8}; clean; no process"
if [ "$apply" != "--apply" ]; then
  echo "lane_close: dry run, nothing changed. Would remove worktree $T and local branch $branch."
  exit 0
fi

# CX-119: immutable record outside the repo. The commits are on origin and the clean tree equals this tree hash.
mkdir -p "$SNAP" || stop "cannot create $SNAP"
rec="$SNAP/lane-close-$dir-$(date -u +%Y%m%dT%H%M%SZ).json"
printf '{"lane":"%s","worktree":"%s","branch":"%s","head":"%s","tree":"%s","origin_branch":"%s","origin_tip":"%s","clean":true,"at":"%s"}\n' \
  "$lane" "$dir" "$branch" "$head" "$treehash" "$PRODUCT" "$origin" "$(date -u +%FT%TZ)" >"$rec" || stop "cannot write $rec"
chmod a-w "$rec"

git -C "$SCOUT" worktree remove "$T" || stop "git worktree remove failed; no manual cleanup (CX-122)"
if [ -n "$branch" ]; then
  git -C "$SCOUT" branch --set-upstream-to="origin/$PRODUCT" "$branch" >/dev/null 2>&1
  git -C "$SCOUT" branch -d "$branch" || stop "branch -d $branch refused; worktree is gone, branch kept"
fi
echo "lane_close: removed $dir and $branch; record $rec; cargo target kept for reuse or an inspected proposal"
