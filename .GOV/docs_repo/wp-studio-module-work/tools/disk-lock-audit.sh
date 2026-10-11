#!/usr/bin/env bash
# Read-only disk-lock audit for lead/workflow ticks (workflow tick-3 proposal 4: detect lock bypass). Prints the
# held lock (owner, command, age vs max hold, holder/child liveness), the queue, recent lock events, every running
# cargo.exe with its command line, and repo-local target dirs. BYPASS_SUSPECT lines name what to chase:
#   - a cargo build-class command (build/check/clippy/test/nextest/run/doc/rustc/bench) running while the lock is
#     free, or more than one distinct build-class cargo command line while it is held (each cargo call shows two
#     cargo.exe: the rustup proxy and the toolchain cargo, same command line);
#   - a target/ directory inside the product workspace (CX-984: build output lives under Handshake_Artifacts).
# Never stops, deletes or changes anything. Usage: disk-lock-audit.sh   Exit 0 (findings are printed, not failed).
set -u
TOOLS=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
WT=$(cd "$TOOLS/../../../../.." && pwd)
R="${STUDIO_DISK_BUILD_LOCK_DIR:-$WT/Handshake_Artifacts/WP-KERNEL-STUDIO/.disk-build-lock}"
source "$TOOLS/disk-build-lock.sh"
DBL_ROOT=$R
dbl_now
echo "lock root: $R ($(date -u +%Y-%m-%dT%H:%M:%SZ))"
held=0
if [ -f "$R/held/owner" ]; then
  held=1
  f="$R/held/owner"
  dbl_get acquired_epoch "$f"; ae=$DBL_V; dbl_get max_hold_s "$f"; mh=$DBL_V
  age=$(( DBL_NOW - ${ae:-$DBL_NOW} ))
  dbl_alive "$f" && ha=alive || ha=dead
  dbl_get child_msys_pid "$f"; if [ -n "$DBL_V" ]; then dbl_alive "$f" child_ && ca=alive || ca=dead; else ca=none; fi
  echo "held: schema=$(dbl_field schema "$f") class=$(dbl_field class "$f") owner=$(dbl_field owner "$f") age=${age}s max=${mh:-?}s holder=$ha child=$ca"
  echo "  command: $(dbl_field command "$f")"
  [ -n "$mh" ] && [ "$age" -gt "$mh" ] && echo "OVER_MAX_HOLD: held ${age}s > ${mh}s (msys pid $(dbl_field msys_pid "$f") start $(dbl_field msys_start "$f") win pid $(dbl_field win_pid "$f")); Operator PROCESS_STOP_APPROVED only"
else
  echo "held: none"
fi
n=0; for t in "$R"/queue/*; do [ -f "$t" ] && n=$((n + 1)); done
echo "queue: $n ticket(s)$( [ $n -gt 0 ] && for t in "$R"/queue/*; do printf '\n  %s %s' "${t##*/}" "$(dbl_field command "$t" | cut -c1-100)"; done)"
[ -f "$R/gap" ] && echo "gap: open (round $(dbl_field round_token "$R/gap"))"
[ -f "$R/events.log" ] && { echo "events (last 8):"; tail -n 8 "$R/events.log" | sed 's/^/  /'; }
echo "cargo.exe processes:"
lines=$(powershell -NoProfile -NonInteractive -Command "Get-CimInstance Win32_Process -Filter \"Name='cargo.exe'\" | ForEach-Object { '{0}|{1}|{2}' -f \$_.ProcessId, \$_.ParentProcessId, \$_.CommandLine }" 2>/dev/null | tr -d '\r')
[ -n "$lines" ] && printf '%s\n' "$lines" | sed 's/^/  /' || echo "  none"
builds=$(printf '%s\n' "$lines" | cut -d'|' -f3- | grep -E '(^|[ "])(build|b|check|c|clippy|test|t|nextest|run|r|doc|rustc|bench)( |$)' | sort -u | grep -c . || true)
# The process snapshot takes seconds: "free" only when the lock was free both before and after it.
[ -f "$R/held/owner" ] && held=1
if [ "$held" -eq 0 ] && [ "${builds:-0}" -gt 0 ]; then echo "BYPASS_SUSPECT: $builds build-class cargo command(s) running while the lock is free (before and after the snapshot)"; fi
if [ "$held" -eq 1 ] && [ "${builds:-0}" -gt 1 ]; then echo "BYPASS_SUSPECT: $builds distinct build-class cargo command lines while one lock is held"; fi
for d in "$WT/wt-studio-assets/handshake-studio-assets/target" "$WT/wt-studio-assets/handshake-studio-assets"/crates/*/target; do
  [ -d "$d" ] && echo "BYPASS_SUSPECT: repo-local target dir $d"
done
exit 0
