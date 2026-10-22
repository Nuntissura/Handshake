#!/usr/bin/env bash
# Per-lane gameplan (SMB-STEWARD-RETRY-GUARD-LANES, finding F-LEAD-1, 2026-10-16). The gameplan retry guard keys a
# failed gated command on the HEAD of the gameplan's confirm-binding repo. STUDIO-MODULE-BUILD binds to the shared
# edit tree (../wt-studio-assets), whose HEAD lane commits never move, so a lane's fixing commit never cleared its own
# block. This creates (once) a child gameplan .GOV/task_packets/WP-KERNEL-STUDIO/gameplans/lanes/<lane>.yaml with
# parent STUDIO-MODULE-BUILD (same steps, same checks) and confirm_binding repo ../wt-studio-lanes/<lane>, and points
# the lane worktree's .gameplan-active at it. Effect: the retry guard keys on the LANE's HEAD (a lane commit clears
# it) and each lane has its own failures file (one lane's failed command no longer blocks another lane). The hook and
# the parent's steps are unchanged. The child lives in the kernel so the parent's repo-relative paths still resolve;
# lanes/*.yaml and their sidecars are machine-local (lanes/.gitignore).
# Usage: lane-gameplan.sh <lane-id-lowercase>   (idempotent; verifies the hook resolution and the bound HEAD)
set -euo pipefail
lane=${1:?usage: lane-gameplan.sh <lane-id-lowercase>}
case "$lane" in *[!a-z0-9-]*|-*) echo "lane id must be lowercase [a-z0-9-]: $lane" >&2; exit 2 ;; esac
TOOLS=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
K=$(cd "$TOOLS/../../../.." && pwd)                      # wt-gov-kernel
LW="$(cd "$K/.." && pwd)/wt-studio-lanes/$lane"
[ -e "$LW/.git" ] || { echo "no lane worktree at $LW (create it first, see wave-parity-2-common.md)" >&2; exit 2; }
GPY="$HOME/.claude/skills/gameplan/scripts/gameplan.py"
PARENT=.GOV/task_packets/WP-KERNEL-STUDIO/gameplans/STUDIO-MODULE-BUILD.yaml
REL=.GOV/task_packets/WP-KERNEL-STUDIO/gameplans/lanes/$lane.yaml
if [ ! -f "$K/$REL" ]; then
  mkdir -p "$K/$(dirname "$REL")"
  (cd "$K" && python "$GPY" init --scope mt --assignment "LANE-$lane" --parent "$PARENT" \
    --confirm-binding-repo "../wt-studio-lanes/$lane" --path "$REL" >/dev/null)
fi
printf '%s\n' "../../wt-gov-kernel/$REL" >"$LW/.gameplan-active.tmp.$$" && mv -f "$LW/.gameplan-active.tmp.$$" "$LW/.gameplan-active"
# Verify exactly what the hook does: resolve from the lane workspace (no GAMEPLAN_PATH), bind, read HEAD.
env -u GAMEPLAN_PATH python -c 'import importlib.util, sys
spec = importlib.util.spec_from_file_location("gp", sys.argv[1]); m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m)
ctx = m.Context(m.locate_gameplan(sys.argv[2]))
print("LANE_GAMEPLAN", ctx.path); print("CONFIRM_ROOT", ctx.confirm_root()); print("BOUND_HEAD", m.current_head(ctx.confirm_root()))
' "$GPY" "$LW/handshake-studio-assets" >"$LW/.gameplan-active.verify.$$"
cat "$LW/.gameplan-active.verify.$$"
bound=$(sed -n 's/^BOUND_HEAD //p' "$LW/.gameplan-active.verify.$$"); rm -f "$LW/.gameplan-active.verify.$$"
[ "$bound" = "$(git -C "$LW" rev-parse HEAD)" ] || { echo "lane-gameplan: bound HEAD $bound != lane HEAD" >&2; exit 1; }
echo "lane-gameplan: $lane OK (retry guard keys on the lane HEAD)"
