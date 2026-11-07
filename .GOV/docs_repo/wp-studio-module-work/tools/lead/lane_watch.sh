#!/usr/bin/env bash
# Lead's lane watcher (KB-STEER-004/005). Read-only: run it under the Monitor tool. Each stdout line is one change; an
# unchanged tick prints nothing. Usage: lane_watch.sh <lane-id-lowercase>...
# Watches per lane: worktree HEAD, lane file status/phase/updated_at, newest run log. Plus: both disk lock holders, the
# product origin (every 5th tick) and C: free space. A lane with no change for 30 min prints one SILENT line; that is a
# prompt to look, never proof of a stall (KB-STEER-006).
# A lane that reuses another lane's tree is named new:old (brief's lane id : worktree dir), so the head and logs are read
# from the old dir. The watcher ends itself after WATCH_MAX_S (default 3600): on 2026-11-07, 44 stopped watchers were
# found still running, because stopping the task did not end this script. After stopping one, count what is left.
WATCH_DEADLINE=$(( $(date +%s) + ${WATCH_MAX_S:-3600} ))
W="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../../../.." && pwd -W)"
S="$W/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work"
A="$W/Handshake_Artifacts/WP-KERNEL-STUDIO"
SCOUT="$W/wt-studio-lanes/smb-type-authored-token-1"
TICK_S=60
SILENT_S=1800
declare -A last seen
now() { date -u +%H:%M:%SZ; }
note() { # key value: print when the value changed
  if [ "${last[$1]-__unset}" != "$2" ]; then
    [ "${last[$1]-__unset}" = "__unset" ] || echo "$(now) $1: ${last[$1]} -> $2"
    last[$1]=$2
    return 0
  fi
  return 1
}
holder() { # lock dir: the holder's owner, or none
  local d=$1/held
  if [ -d "$d" ]; then
    { cat "$d"/* 2>/dev/null; } | tr '\n\r' '  ' | sed -E 's/.*owner[=:" ]+([A-Za-z0-9_.-]+).*/\1/' | cut -c1-60
  else
    echo none
  fi
}
n=0
while true; do
  t=$(date +%s)
  if [ "$t" -ge "$WATCH_DEADLINE" ]; then
    echo "$(now) watch ended after ${WATCH_MAX_S:-3600} s; start a new one if still needed"
    exit 0
  fi
  for spec in "$@"; do
    lane=${spec%%:*}
    tree=${spec##*:}
    f=$(ls "$S"/builders/lanes/*"${lane#smb-}".json 2>/dev/null | head -n 1)
    changed=1
    note "$lane head" "$(git -C "$W/wt-studio-lanes/$tree" rev-parse --short=8 HEAD 2>/dev/null)" && changed=0
    if [ -n "$f" ]; then
      note "$lane file" "$(python -c "import json,sys;d=json.load(open(sys.argv[1],encoding='utf-8'));print(d.get('status'),d.get('phase'),d.get('updated_at'))" "$f" 2>/dev/null)" && changed=0
    fi
    log=$(ls -t "$A/$tree"/*.log 2>/dev/null | head -n 1)
    note "$lane log" "$(basename "${log:-none}")" && changed=0
    if [ "$changed" -eq 0 ] || [ -z "${seen[$lane]-}" ]; then
      seen[$lane]=$t
    elif [ $((t - seen[$lane])) -ge "$SILENT_S" ]; then
      echo "$(now) SILENT $lane: no head, lane file or new log for $(((t - seen[$lane]) / 60)) min; look, this is not a stall verdict"
      seen[$lane]=$t
    fi
  done
  note "C: lock" "$(holder C:/HandshakeBuild/.disk-build-lock)"
  note "D: lock" "$(holder "$A/.disk-build-lock")"
  if [ $((n % 5)) -eq 0 ]; then
    note "origin" "$(git -C "$SCOUT" ls-remote origin refs/heads/codex/handshake-studio-assets 2>/dev/null | cut -c1-8)"
    free=$(df -BG /c | tail -n 1 | awk '{print $4}')
    note "C: free band" "$(( ${free%G} / 50 * 50 ))G+"
    note "stray cargo" "$(python "$S/tools/lead/stray_cargo.py" --brief 2>/dev/null)"
  fi
  n=$((n + 1))
  sleep "$TICK_S"
done
